use miden_client::{
    Client, ClientError, Felt, Word, ZERO,
    account::{
        Account, AccountBuilder, AccountId, AccountStorageMode, AccountType, StorageMap,
        StorageSlot,
    },
    asset::{Asset, FungibleAsset, TokenSymbol},
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
use miden_lib::{
    account::{auth::RpoFalcon512, faucets::BasicFungibleFaucet, wallets::BasicWallet},
    transaction::TransactionKernel,
};
use miden_objects::{
    Hasher,
    assembly::{Assembler, DefaultSourceManager, Library, LibraryPath, Module, ModuleKind},
};
use miden_objects::{
    NoteError,
    account::{AccountComponent, NetworkId},
    vm::AdviceMap,
};
use rand::{RngCore, rngs::StdRng};
use serde::de::value::Error;
use std::{
    fs,
    path::{Path, PathBuf},
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

use crate::constants::{MULTISIG_CODE_PATH, NETWORK_ID, SIGNER_WEIGHTS, THRESHOLD, TOTAL_WEIGHT};

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
pub async fn instantiate_client(
    endpoint: Endpoint,
) -> Result<(Client, FilesystemKeyStore<StdRng>), ClientError> {
    let timeout_ms = 10_000;
    let keystore = FilesystemKeyStore::new("./keystore".into()).unwrap();

    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));
    let client = ClientBuilder::new()
        .rpc(rpc_api.clone())
        .filesystem_keystore("./keystore")
        .in_debug_mode(true)
        .build()
        .await?;

    Ok((client, keystore))
}

pub async fn initialize_client_and_multisig()
-> Result<(Client, Account, Word, Vec<Word>, Vec<SecretKey>), Box<dyn std::error::Error>> {
    let endpoint = if NETWORK_ID == NetworkId::Testnet {
        Endpoint::testnet()
    } else {
        Endpoint::devnet()
    };

    let (mut client, keystore) = instantiate_client(endpoint).await.unwrap();

    client.sync_state().await.unwrap();

    // Deploy my multisig contract
    let multisig_code = fs::read_to_string(Path::new(MULTISIG_CODE_PATH)).unwrap();

    let (
        multisig_contract,
        multisig_seed,
        multisig_key_pair,
        original_signer_pub_keys,
        original_signer_secret_keys,
    ) = create_multisig_account(
        &mut client,
        &multisig_code,
        THRESHOLD,
        SIGNER_WEIGHTS.to_vec(),
        keystore.clone(),
    )
    .await?;

    client
        .add_account(&multisig_contract, Some(multisig_seed.into()), false)
        .await
        .unwrap();

    println!(
        "📄 Multisig contract ID: {}",
        multisig_contract.id().to_hex()
    );

    Ok((
        client,
        multisig_contract,
        multisig_seed,
        original_signer_pub_keys,
        original_signer_secret_keys,
    ))
}

// Creates library
pub fn create_library(
    account_code: String,
    library_path: &str,
) -> Result<miden_objects::assembly::Library, Box<dyn std::error::Error>> {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);
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
    let serial_num = rng.inner_mut().draw_word();
    let note_script = NoteScript::compile(note_code, assembler).unwrap();
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
        .own_output_notes(vec![OutputNote::Full(note.clone())])
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

pub async fn create_basic_faucet(
    client: &mut Client,
    keystore: FilesystemKeyStore<StdRng>,
) -> Result<miden_client::account::Account, ClientError> {
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);
    let key_pair = SecretKey::with_rng(client.rng());
    let symbol = TokenSymbol::new("MID").unwrap();
    let decimals = 8;
    let max_supply = Felt::new(1_000_000_000);
    let builder = AccountBuilder::new(init_seed)
        .account_type(AccountType::FungibleFaucet)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(RpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicFungibleFaucet::new(symbol, decimals, max_supply).unwrap());
    let (account, seed) = builder.build().unwrap();
    client.add_account(&account, Some(seed), false).await?;
    keystore
        .add_key(&AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();
    Ok(account)
}

// Creates basic account
pub async fn create_basic_account(
    client: &mut Client,
    keystore: FilesystemKeyStore<StdRng>,
) -> Result<(miden_client::account::Account, SecretKey), ClientError> {
    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let key_pair = SecretKey::with_rng(client.rng());
    let builder = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(RpoFalcon512::new(key_pair.public_key().clone()))
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
    keystore: FilesystemKeyStore<StdRng>,
) -> Result<(Account, Word, SecretKey, Vec<Word>, Vec<SecretKey>), ClientError> {
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

    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let multisig_key_pair = SecretKey::with_rng(client.rng());

    let auth_componnet: AccountComponent = RpoFalcon512::new(multisig_key_pair.public_key()).into();

    let (multisig_contract, multisig_seed) = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(auth_componnet)
        .with_component(multisig_component.clone())
        .build()
        .unwrap();
    keystore
        .add_key(&AuthSecretKey::RpoFalcon512(multisig_key_pair.clone()))
        .unwrap();
    Ok((
        multisig_contract,
        multisig_seed,
        multisig_key_pair,
        signer_pub_keys,
        signers_secret_keys,
    ))
}

pub fn create_tx_script(
    script_code: String,
    library: Option<Library>,
) -> Result<TransactionScript, Error> {
    let assembler = TransactionKernel::assembler();

    let assembler = match library {
        Some(lib) => assembler.with_library(&lib),
        None => Ok(assembler.with_debug_mode(true)),
    }
    .unwrap();
    let tx_script = TransactionScript::compile(script_code, assembler).unwrap();

    Ok(tx_script)
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

pub async fn build_and_submit_tx(
    tx_script: TransactionScript,
    advice_map: AdviceMap,
    client: &mut Client,
    account_id: AccountId,
) -> Result<(), ClientError> {
    let tx_add_signer_request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
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

pub fn prepare_felt_vec(element: u64) -> [Felt; 4] {
    [Felt::new(element), ZERO, ZERO, ZERO]
}

pub fn prepare_script(
    script_path: &str,
    account_code_path: &str,
    library_path: &str,
) -> Result<TransactionScript, Error> {
    let script_code = fs::read_to_string(Path::new(script_path)).unwrap();

    let account_code = fs::read_to_string(Path::new(account_code_path)).unwrap();

    let library = create_library(account_code, library_path).unwrap();

    let tx_script = create_tx_script(script_code, Some(library)).unwrap();

    Ok(tx_script)
}

pub async fn wait_for_notes(
    client: &mut Client,
    account_id: &miden_client::account::Account,
    expected: usize,
) -> Result<(), ClientError> {
    loop {
        client.sync_state().await?;
        let notes = client.get_consumable_notes(Some(account_id.id())).await?;
        if notes.len() >= expected {
            break;
        }
        println!(
            "{} consumable notes found for account {}. Waiting...",
            notes.len(),
            account_id.id().to_hex()
        );
        sleep(Duration::from_secs(3)).await;
    }
    Ok(())
}

/// Creates [num_accounts] accounts, [num_faucets] faucets, and mints the given [balances].
///
/// - `balances[a][f]`: how many tokens faucet `f` should mint for account `a`.
/// - Returns: a tuple of `(Vec<Account>, Vec<Account>)` i.e. (accounts, faucets).
pub async fn setup_accounts_and_faucets(
    client: &mut Client,
    keystore: FilesystemKeyStore<StdRng>,
    num_accounts: usize,
    num_faucets: usize,
    balances: Vec<Vec<u64>>,
) -> Result<(Vec<Account>, Vec<Account>), ClientError> {
    // ---------------------------------------------------------------------
    // 1)  Create basic accounts
    // ---------------------------------------------------------------------
    let mut accounts = Vec::with_capacity(num_accounts);
    for i in 0..num_accounts {
        let (account, _) = create_basic_account(client, keystore.clone()).await?;
        accounts.push(account);
    }

    // ---------------------------------------------------------------------
    // 2)  Create basic faucets
    // ---------------------------------------------------------------------
    let mut faucets = Vec::with_capacity(num_faucets);
    for j in 0..num_faucets {
        let faucet = create_basic_faucet(client, keystore.clone()).await?;
        faucets.push(faucet);
    }

    // Tell the client about the new accounts/faucets
    client.sync_state().await?;

    // ---------------------------------------------------------------------
    // 3)  Mint tokens
    // ---------------------------------------------------------------------
    // `minted_notes[i]` collects the notes minted **for** `accounts[i]`
    let mut minted_notes: Vec<Vec<Note>> = vec![Vec::new(); num_accounts];

    for (acct_idx, account) in accounts.iter().enumerate() {
        for (faucet_idx, faucet) in faucets.iter().enumerate() {
            let amount = balances[acct_idx][faucet_idx];
            if amount == 0 {
                continue;
            }

            // Build & submit the mint transaction
            let asset = FungibleAsset::new(faucet.id(), amount).unwrap();
            let tx_request = TransactionRequestBuilder::new()
                .build_mint_fungible_asset(asset, account.id(), NoteType::Public, client.rng())
                .unwrap();

            let tx_exec = client.new_transaction(faucet.id(), tx_request).await?;
            client.submit_transaction(tx_exec.clone()).await?;

            // Remember the freshly-created note so we can consume it later
            let minted_note = match tx_exec.created_notes().get_note(0) {
                OutputNote::Full(n) => n.clone(),
                _ => panic!("Expected OutputNote::Full, got something else"),
            };
            minted_notes[acct_idx].push(minted_note);
        }
    }

    // ---------------------------------------------------------------------
    // 4)  ONE wait-phase – ensure every account can now see all its notes
    // ---------------------------------------------------------------------
    for (acct_idx, account) in accounts.iter().enumerate() {
        let expected = minted_notes[acct_idx].len();
        if expected > 0 {
            wait_for_notes(client, account, expected).await?;
        }
    }
    client.sync_state().await?;

    // ---------------------------------------------------------------------
    // 5)  Consume notes so the tokens live in the public vaults
    // ---------------------------------------------------------------------
    for (acct_idx, account) in accounts.iter().enumerate() {
        for note in &minted_notes[acct_idx] {
            let consume_req = TransactionRequestBuilder::new()
                .authenticated_input_notes([(note.id(), None)])
                .build()
                .unwrap();

            let tx_exec = client.new_transaction(account.id(), consume_req).await?;
            client.submit_transaction(tx_exec).await?;
        }
    }
    client.sync_state().await?;

    Ok((accounts, faucets))
}

pub fn create_gift_note_recallable(
    creator: AccountId,
    offered_asset: Asset,
    secret: [Felt; 4],
    serial_num: [Felt; 4],
) -> Result<Note, NoteError> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = [manifest_dir, "masm", "notes", "gift.masm"]
        .iter()
        .collect();

    let note_code = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("Error reading {}: {}", path.display(), err));

    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    let note_script = NoteScript::compile(note_code, assembler).unwrap();
    let note_type = NoteType::Public;

    let gift_tag = NoteTag::for_public_use_case(0, 0, NoteExecutionMode::Local)?;

    let mut secret_vals = vec![secret[0], secret[1], secret[2], secret[3]];
    secret_vals.splice(0..0, Word::default().iter().cloned());
    let digest = Hasher::hash_elements(&secret_vals);
    println!("digest: {:?}", digest);

    let inputs = NoteInputs::new(digest.to_vec())?;

    let aux = Felt::new(0);

    // build the outgoing note
    let metadata = NoteMetadata::new(
        creator,
        note_type,
        gift_tag,
        NoteExecutionHint::always(),
        aux,
    )?;

    let assets = NoteAssets::new(vec![offered_asset])?;
    let recipient = NoteRecipient::new(serial_num, note_script.clone(), inputs.clone());
    let note = Note::new(assets.clone(), metadata, recipient.clone());

    println!(
        "inputlen: {:?}, NoteInputs: {:?}",
        inputs.num_values(),
        inputs.values()
    );
    println!("tag: {:?}", note.metadata().tag());
    println!("aux: {:?}", note.metadata().aux());
    println!("note type: {:?}", note.metadata().note_type());
    println!("hint: {:?}", note.metadata().execution_hint());
    println!("recipient: {:?}", note.recipient().digest());

    Ok(note)
}

pub async fn create_no_auth_component() -> Result<AccountComponent, Error> {
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);
    let no_auth_code = fs::read_to_string(Path::new("./masm/auth/no_auth.masm")).unwrap();
    let no_auth_component = AccountComponent::compile(no_auth_code, assembler.clone(), vec![])
        .unwrap()
        .with_supports_all_types();

    Ok(no_auth_component)
}

pub async fn create_no_auth_faucet(
    client: &mut Client,
    token_symbol: &str,
    max_supply: u64,
    decimals: u8,
    storage_mode: AccountStorageMode,
) -> Result<Account, ClientError> {
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let no_auth_component = create_no_auth_component().await.unwrap();

    let symbol = TokenSymbol::new(token_symbol).unwrap();

    let (new_account, seed) = AccountBuilder::new(init_seed)
        .account_type(AccountType::FungibleFaucet)
        .storage_mode(storage_mode.into())
        .with_auth_component(no_auth_component)
        .with_component(BasicFungibleFaucet::new(symbol, decimals, Felt::new(max_supply)).unwrap())
        .build()
        .unwrap();
    client.add_account(&new_account, Some(seed), false).await?;
    Ok(new_account)
}

pub async fn create_evm_account(
    client: &mut Client,
    storage_mode: AccountStorageMode,
) -> Result<Account, ClientError> {
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let no_auth_component = create_no_auth_component().await.unwrap();
    let account_code = fs::read_to_string(Path::new("./masm/accounts/evm.masm")).unwrap();
    let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);

    let evm_component = AccountComponent::compile(account_code.clone(), assembler.clone(), vec![])
        .unwrap()
        .with_supports_all_types();

    let (new_account, seed) = AccountBuilder::new(init_seed)
        .account_type(AccountType::FungibleFaucet)
        .storage_mode(storage_mode.into())
        .with_auth_component(no_auth_component)
        .with_component(evm_component)
        .build()
        .unwrap();

    client.add_account(&new_account, Some(seed), false).await?;
    Ok(new_account)
}
