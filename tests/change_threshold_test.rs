use masm_project_template::common::delete_keystore_and_store;
use masm_project_template::{
    common::{
        build_and_submit_tx, initialize_client_and_multisig, prepare_felt_vec, prepare_script,
    },
    constants::{
        CHANGE_THRESHOLD_SCRIPT_PATH, LIBRARY_PATH, MULTISIG_CODE_PATH, NEW_THRESHOLD_AS_KEY_SLOT,
        SYNC_STATE_WAIT_TIME, THRESHOLD_SLOT,
    },
};
use miden_client::Word;
use miden_objects::{account::NetworkId, vm::AdviceMap};
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn change_threshold_success() -> Result<(), Box<dyn std::error::Error>> {
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
    ) = initialize_client_and_multisig().await?;

    println!(
        "📄 Multisig contract ID: {}",
        multisig_contract.id().to_bech32(NetworkId::Testnet)
    );

    // -------------------------------------------------------------------------
    // STEP 1: Prepare the Script for change threshold
    // -------------------------------------------------------------------------
    let tx_script = prepare_script(
        CHANGE_THRESHOLD_SCRIPT_PATH,
        MULTISIG_CODE_PATH,
        LIBRARY_PATH,
    )
    .unwrap();

    // -------------------------------------------------------------------------
    // STEP 2: Prepare advice map for change threshold
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // insert new threshold into advice map at index 0
    advice_map.insert(
        prepare_felt_vec(NEW_THRESHOLD_AS_KEY_SLOT as u64).into(),
        prepare_felt_vec(4).into(),
    );

    // -------------------------------------------------------------------------
    // STEP 3: Build & Submit Transaction
    // -------------------------------------------------------------------------
    build_and_submit_tx(tx_script, advice_map, &mut client, multisig_contract.id())
        .await
        .unwrap();

    // -------------------------------------------------------------------------
    // STEP 4: Fetch and verify threshold changed
    // -------------------------------------------------------------------------
    println!("🚀 Change threshold transaction submitted – waiting for finality …");
    sleep(Duration::from_secs(SYNC_STATE_WAIT_TIME)).await;

    client.sync_state().await.unwrap();

    let account_state = client
        .get_account(multisig_contract.id())
        .await?
        .expect("multisig contract not found");

    let storage_threshold: Word = account_state
        .account()
        .storage()
        .get_item(THRESHOLD_SLOT as u8)?
        .into();
    println!("🔢 Storage threshold: {:?}", storage_threshold);
    println!("✅ Success! The threshold was changed.");

    Ok(())
}

#[tokio::test]
#[should_panic]
async fn change_threshold_with_same_threshold() {
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

    println!(
        "📄 Multisig contract ID: {}",
        multisig_contract.id().to_bech32(NetworkId::Testnet)
    );

    // -------------------------------------------------------------------------
    // STEP 1: Prepare the Script for change threshold
    // -------------------------------------------------------------------------
    let tx_script = prepare_script(
        CHANGE_THRESHOLD_SCRIPT_PATH,
        MULTISIG_CODE_PATH,
        LIBRARY_PATH,
    )
    .unwrap();

    // -------------------------------------------------------------------------
    // STEP 2: Prepare advice map for change threshold
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // insert new threshold into advice map at index 0
    advice_map.insert(
        prepare_felt_vec(NEW_THRESHOLD_AS_KEY_SLOT as u64).into(),
        prepare_felt_vec(3).into(),
    );

    // -------------------------------------------------------------------------
    // STEP 3: Build & Submit Transaction
    // -------------------------------------------------------------------------
    build_and_submit_tx(tx_script, advice_map, &mut client, multisig_contract.id())
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic]
async fn change_threshold_with_invalid_threshold() {
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
    // STEP 1: Prepare the Script for change threshold
    // -------------------------------------------------------------------------
    let tx_script = prepare_script(
        CHANGE_THRESHOLD_SCRIPT_PATH,
        MULTISIG_CODE_PATH,
        LIBRARY_PATH,
    )
    .unwrap();

    // -------------------------------------------------------------------------
    // STEP 2: Prepare advice map for change threshold
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // insert new threshold into advice map at index 0
    advice_map.insert(
        prepare_felt_vec(NEW_THRESHOLD_AS_KEY_SLOT as u64).into(),
        prepare_felt_vec(100).into(),
    );

    // -------------------------------------------------------------------------
    // STEP 3: Build & Submit Transaction
    // -------------------------------------------------------------------------
    build_and_submit_tx(tx_script, advice_map, &mut client, multisig_contract.id())
        .await
        .unwrap();
}
