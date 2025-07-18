use masm_project_template::common::{
    create_basic_account, create_basic_faucet, delete_keystore_and_store,
};
use masm_project_template::{
    common::{
        build_and_submit_tx, generate_keypair, initialize_client_and_multisig, prepare_felt_vec,
        prepare_script,
    },
    constants::{
        ADD_SIGNER_SCRIPT_PATH, INVALID_WEIGHT, LIBRARY_PATH, MULTISIG_CODE_PATH,
        NEW_SIGNER_PUBKEY_KEY_SLOT, NEW_SIGNER_WEIGHT_KEY_SLOT, SIGNERS_SLOT, SYNC_STATE_WAIT_TIME,
    },
};
use miden_client::asset::FungibleAsset;
use miden_client::keystore::FilesystemKeyStore;
use miden_client::note::NoteType;
use miden_client::transaction::TransactionRequestBuilder;
use miden_objects::{Word, account::NetworkId, vm::AdviceMap};
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn add_signer_success() -> Result<(), Box<dyn std::error::Error>> {
    delete_keystore_and_store().await;

    // -------------------------------------------------------------------------
    // Instantiate client
    // -------------------------------------------------------------------------
    let (
        mut client,
        multisig_contract,
        _multisig_seed,
        original_signer_pub_keys,
        _original_signer_secret_keys,
    ) = initialize_client_and_multisig().await?;

    println!(
        "📄 Multisig contract ID: {}",
        multisig_contract.id().to_bech32(NetworkId::Testnet)
    );

    // -------------------------------------------------------------------------
    // STEP 2: Prepare the Script
    // -------------------------------------------------------------------------
    let tx_script =
        prepare_script(ADD_SIGNER_SCRIPT_PATH, MULTISIG_CODE_PATH, LIBRARY_PATH).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare advice map for new signer
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // generate keypair
    let (_, new_signer_public_key) = generate_keypair(&mut client);

    println!("new signer public key: {:?}", new_signer_public_key);

    // insert public key into advice map at index 0
    advice_map.insert(
        prepare_felt_vec(NEW_SIGNER_PUBKEY_KEY_SLOT as u64).into(),
        new_signer_public_key.into(),
    );
    // insert new signer weight into advice map at index 1
    advice_map.insert(
        prepare_felt_vec(NEW_SIGNER_WEIGHT_KEY_SLOT as u64).into(),
        prepare_felt_vec(1).into(),
    );

    // -------------------------------------------------------------------------·
    // STEP 4: Build & Submit Transaction
    // -------------------------------------------------------------------------
    build_and_submit_tx(tx_script, advice_map, &mut client, multisig_contract.id())
        .await
        .unwrap();

    // -------------------------------------------------------------------------
    // STEP 5: Fetch and verify signer added
    // -------------------------------------------------------------------------
    println!("🚀 Add signer transaction submitted – waiting for finality …");
    sleep(Duration::from_secs(SYNC_STATE_WAIT_TIME)).await;

    client.sync_state().await.unwrap();

    let account_state = client
        .get_account(multisig_contract.id())
        .await?
        .expect("multisig contract not found");

    // loop through the original signer
    for i in 0..original_signer_pub_keys.len() {
        let storage_signer: Word = account_state
            .account()
            .storage()
            .get_map_item(SIGNERS_SLOT as u8, original_signer_pub_keys[i].into())?
            .into();
        println!(
            "Storage Original Signer: {:?}, Weight: {:?}",
            original_signer_pub_keys[i], storage_signer
        );
    }
    let storage_new_signer: Word = account_state
        .account()
        .storage()
        .get_map_item(SIGNERS_SLOT as u8, new_signer_public_key.into())?
        .into();

    println!(
        "🔢 Storage New Signer Public Key: {:?}, Weight: {:?}",
        new_signer_public_key, storage_new_signer
    );
    println!("✅ Success! The signer was added.");

    Ok(())
}

#[tokio::test]
#[should_panic]
async fn add_signer_with_same_public_key() {
    delete_keystore_and_store().await;

    // -------------------------------------------------------------------------
    // Instantiate client
    // -------------------------------------------------------------------------
    let (
        mut client,
        multisig_contract,
        _multisig_seed,
        original_signer_pub_keys,
        _original_signer_secret_keys,
    ) = initialize_client_and_multisig().await.unwrap();

    // -------------------------------------------------------------------------
    // STEP 2: Prepare the Script
    // -------------------------------------------------------------------------
    let tx_script =
        prepare_script(ADD_SIGNER_SCRIPT_PATH, MULTISIG_CODE_PATH, LIBRARY_PATH).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare advice map for new signer
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // insert public key into advice map at index 0
    advice_map.insert(
        prepare_felt_vec(NEW_SIGNER_PUBKEY_KEY_SLOT as u64).into(),
        original_signer_pub_keys[0].into(),
    );
    advice_map.insert(
        prepare_felt_vec(NEW_SIGNER_WEIGHT_KEY_SLOT as u64).into(),
        prepare_felt_vec(1).into(),
    );

    // -------------------------------------------------------------------------
    // STEP 4: Build & Submit Transaction
    // -------------------------------------------------------------------------
    build_and_submit_tx(tx_script, advice_map, &mut client, multisig_contract.id())
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic]
async fn add_signer_with_invalid_weight() {
    delete_keystore_and_store().await;

    // -------------------------------------------------------------------------
    // Instantiate client
    // -------------------------------------------------------------------------
    let (
        mut client,
        multisig_contract,
        _multisig_seed,
        _original_signer_pub_keys,
        _original_signer_secret_keys,
    ) = initialize_client_and_multisig().await.unwrap();

    // -------------------------------------------------------------------------
    // STEP 2: Prepare the Script
    // -------------------------------------------------------------------------
    let tx_script =
        prepare_script(ADD_SIGNER_SCRIPT_PATH, MULTISIG_CODE_PATH, LIBRARY_PATH).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare advice map for new signer
    // -------------------------------------------------------------------------
    // generate keypair
    let (_, new_signer_public_key) = generate_keypair(&mut client);

    let mut advice_map = AdviceMap::default();

    // insert public key into advice map at index 0
    advice_map.insert(
        prepare_felt_vec(NEW_SIGNER_PUBKEY_KEY_SLOT as u64).into(),
        new_signer_public_key.into(),
    );
    advice_map.insert(
        prepare_felt_vec(NEW_SIGNER_WEIGHT_KEY_SLOT as u64).into(),
        prepare_felt_vec(INVALID_WEIGHT as u64).into(),
    );

    // -------------------------------------------------------------------------
    // STEP 4: Build & Submit Transaction
    // -------------------------------------------------------------------------
    build_and_submit_tx(tx_script, advice_map, &mut client, multisig_contract.id())
        .await
        .unwrap();
}
