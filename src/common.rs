use miden_assembly::{
    Assembler, DefaultSourceManager, Library, LibraryPath,
    ast::{Module, ModuleKind},
};
use miden_client::{
    Client, ClientError,
    account::{
        Account, AccountBuilder, AccountId, AccountStorageMode, AccountType, StorageMap,
        StorageSlot,
    },
    auth::AuthSecretKey,
    builder::ClientBuilder,
    crypto::SecretKey,
    keystore::FilesystemKeyStore,
    note::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteInputs, NoteMetadata,
        NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    rpc::{Endpoint, TonicRpcClient},
    transaction::{OutputNote, TransactionRequestBuilder, TransactionScript},
};
use miden_crypto::rand::FeltRng;
use miden_crypto::{Felt, Word};
use miden_lib::{
    account::{auth::RpoFalcon512, wallets::BasicWallet},
    transaction::TransactionKernel,
};
use miden_objects::{account::AccountComponent, vm::AdviceMap};
use rand::{RngCore, rngs::StdRng};
use serde::de::value::Error;
use std::sync::Arc;

use crate::constants::{THRESHOLD, TOTAL_WEIGHT};

// Clears keystore & default sqlite file
pub async fn delete_keystore_and_store() {
    let store_path = "./store.sqlite3";
    if tokio::fs::metadata(store_path).await.is_ok() {
        if let Err(e) = tokio::fs::remove_file(store_path).await {
            eprintln!("failed to remove {}: {}", store_path, e);
        } else {
            println!("cleared sqlite store: {}", store_path);
        }
    } else {
        println!("store not found: {}", store_path);
    }

    let keystore_dir = "./keystore";
    match tokio::fs::read_dir(keystore_dir).await {
        Ok(mut dir) => {
            while let Ok(Some(entry)) = dir.next_entry().await {
                let file_path = entry.path();
                if let Err(e) = tokio::fs::remove_file(&file_path).await {
                    eprintln!("failed to remove {}: {}", file_path.display(), e);
                } else {
                    println!("removed file: {}", file_path.display());
                }
            }
        }
        Err(e) => eprintln!("failed to read directory {}: {}", keystore_dir, e),
    }
}

// Helper to instantiate Client
pub async fn instantiate_client(endpoint: Endpoint) -> Result<Client, ClientError> {
    let timeout_ms = 10_000;
    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    let client = ClientBuilder::new()
        .with_rpc(rpc_api.clone())
        .with_filesystem_keystore("./keystore")
        .in_debug_mode(true)
        .build()
        .await?;

    Ok(client)
}

// Creates library
pub fn create_library(
    account_code: String,
    library_path: &str,
) -> Result<miden_assembly::Library, Box<dyn std::error::Error>> {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let source_manager = Arc::new(DefaultSourceManager::default());
    let module = Module::parser(ModuleKind::Library).parse_str(
        LibraryPath::new(library_path)?,
        account_code,
        &source_manager,
    )?;
    let library = assembler.clone().assemble_library([module])?;
    Ok(library)
}

// Creates public note
pub async fn create_public_note(
    client: &mut Client,
    note_code: String,
    account_library: Library,
    creator_account: Account,
    assets: NoteAssets,
) -> Result<Note, Error> {
    let assembler = TransactionKernel::assembler()
        .with_library(&account_library)
        .unwrap()
        .with_debug_mode(true);
    let rng = client.rng();
    let serial_num = rng.draw_word();
    let note_script = NoteScript::compile(note_code, assembler.clone()).unwrap();
    let note_inputs = NoteInputs::new([].to_vec()).unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, note_inputs.clone());
    let tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local).unwrap();
    let metadata = NoteMetadata::new(
        creator_account.id(),
        NoteType::Public,
        tag,
        NoteExecutionHint::always(),
        Felt::new(0),
    )
    .unwrap();

    let note = Note::new(assets, metadata, recipient);

    let note_req = TransactionRequestBuilder::new()
        .with_own_output_notes(vec![OutputNote::Full(note.clone())])
        .build()
        .unwrap();
    let tx_result = client
        .new_transaction(creator_account.id(), note_req)
        .await
        .unwrap();

    let _ = client.submit_transaction(tx_result).await;
    client.sync_state().await.unwrap();

    Ok(note)
}

// Creates basic account
pub async fn create_basic_account(
    client: &mut Client,
    keystore: FilesystemKeyStore<StdRng>,
) -> Result<(miden_client::account::Account, SecretKey), ClientError> {
    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let key_pair = SecretKey::with_rng(client.rng());
    let anchor_block = client.get_latest_epoch_block().await.unwrap();
    let builder = AccountBuilder::new(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_component(RpoFalcon512::new(key_pair.public_key().clone()))
        .with_component(BasicWallet);
    let (account, seed) = builder.build().unwrap();
    client.add_account(&account, Some(seed), false).await?;
    keystore
        .add_key(&AuthSecretKey::RpoFalcon512(key_pair.clone()))
        .unwrap();

    Ok((account, key_pair))
}

pub async fn create_multisig_account(
    client: &mut Client,
    account_code: &String,
    num_signers: usize,
    signer_weights: Vec<usize>,
) -> Result<(Account, Word, Vec<Word>, Vec<SecretKey>), ClientError> {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);

    // generate keypairs for signers
    let (signers_secret_keys, signer_pub_keys) = generate_keypairs(num_signers, client);

    let mut storage_map_signers = StorageMap::new();
    let storage_map_message_hash = StorageMap::new();
    // loop through signers pub key
    for (i, pub_key) in signer_pub_keys.iter().enumerate() {
        let weight = signer_weights[i];
        storage_map_signers.insert(
            pub_key.into(),
            [
                Felt::new(weight as u64),
                Felt::new(0 as u64),
                Felt::new(0 as u64),
                Felt::new(0 as u64),
            ],
        );
    }

    let storage_slot_map_signers = StorageSlot::Map(storage_map_signers.clone());
    let storage_slot_map_message_hash = StorageSlot::Map(storage_map_message_hash.clone());

    let threshold = Felt::new(THRESHOLD as u64);
    let total_weight = Felt::new(TOTAL_WEIGHT as u64);

    let multisig_component = AccountComponent::compile(
        account_code.clone(),
        assembler.clone(),
        vec![
            StorageSlot::Value([threshold, Felt::new(0), Felt::new(0), Felt::new(0)]),
            StorageSlot::Value([total_weight, Felt::new(0), Felt::new(0), Felt::new(0)]),
            storage_slot_map_signers,
            storage_slot_map_message_hash,
        ],
    )
    .unwrap()
    .with_supports_all_types();

    let anchor_block = client.get_latest_epoch_block().await.unwrap();

    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let (multisig_contract, multisig_seed) = AccountBuilder::new(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_component(multisig_component.clone())
        .build()
        .unwrap();

    Ok((
        multisig_contract,
        multisig_seed,
        signer_pub_keys,
        signers_secret_keys,
    ))
}

pub fn generate_keypairs(num_keys: usize, client: &mut Client) -> (Vec<SecretKey>, Vec<Word>) {
    let mut keys = Vec::new();
    let mut signer_pub_keys: Vec<Word> = Vec::new();

    for _ in 0..num_keys {
        let key = SecretKey::with_rng(client.rng());
        keys.push(key.clone());

        signer_pub_keys.push(key.public_key().into());
    }

    (keys, signer_pub_keys)
}

pub fn generate_keypair(client: &mut Client) -> (SecretKey, Word) {
    let private_key = SecretKey::with_rng(client.rng());
    let public_key = private_key.public_key();

    (private_key, public_key.into())
}

pub fn create_tx_script(
    script_code: String,
    library: Option<Library>,
) -> Result<TransactionScript, Error> {
    let assembler = TransactionKernel::assembler();

    let assembler = match library {
        Some(lib) => assembler.with_library(lib),
        None => Ok(assembler.with_debug_mode(true)),
    }
    .unwrap();
    let tx_script = TransactionScript::compile(script_code, [], assembler).unwrap();

    Ok(tx_script)
}

pub async fn build_and_submit_tx(
    tx_script: TransactionScript,
    advice_map: AdviceMap,
    client: &mut Client,
    account_id: AccountId,
) -> Result<(), ClientError> {
    let tx_add_signer_request = TransactionRequestBuilder::new()
        .with_custom_script(tx_script)
        .extend_advice_map(advice_map)
        .build()
        .unwrap();

    let tx_result = client
        .new_transaction(account_id, tx_add_signer_request)
        .await
        .unwrap();

    let _ = client.submit_transaction(tx_result).await?;
    Ok(())
}
