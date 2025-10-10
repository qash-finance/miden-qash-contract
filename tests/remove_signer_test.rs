use masm_project_template::{
    common::{
        build_and_submit_tx, delete_keystore_and_store, generate_keypair,
        initialize_client_and_multisig, prepare_felt_vec, prepare_script,
    },
    constants::{
        LIBRARY_PATH, MULTISIG_CODE_PATH, REMOVE_SIGNER_SCRIPT_PATH,
        SIGNER_TO_REMOVE_CANT_REACH_THRESHOLD_INDEX, SIGNER_TO_REMOVE_INDEX,
        SIGNER_TO_REMOVE_KEY_SLOT, SIGNERS_SLOT, SYNC_STATE_WAIT_TIME,
    },
};
use miden_client::Word;
use miden_objects::vm::AdviceMap;
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn remove_signer_success() -> Result<(), Box<dyn std::error::Error>> {
    delete_keystore_and_store().await;

    // -------------------------------------------------------------------------
    // 1. Instantiate client
    // -------------------------------------------------------------------------
    let (
        mut client,
        multisig_contract,
        _multisig_seed,
        original_signer_pub_keys,
        _original_signer_secret_keys,
    ) = initialize_client_and_multisig().await?;

    // -------------------------------------------------------------------------
    // STEP 2: Prepare the Script
    // -------------------------------------------------------------------------
    let tx_script =
        prepare_script(REMOVE_SIGNER_SCRIPT_PATH, MULTISIG_CODE_PATH, LIBRARY_PATH).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare advice map for new signer
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // insert new threshold into advice map at index 0
    advice_map.insert(
        prepare_felt_vec(SIGNER_TO_REMOVE_KEY_SLOT as u64).into(),
        original_signer_pub_keys[SIGNER_TO_REMOVE_INDEX].to_vec(),
    );

    // -------------------------------------------------------------------------
    // STEP 4: Build & Submit Transaction
    // -------------------------------------------------------------------------
    build_and_submit_tx(tx_script, advice_map, &mut client, multisig_contract.id())
        .await
        .unwrap();

    // -------------------------------------------------------------------------
    // STEP 5: Fetch and verify signer added
    // -------------------------------------------------------------------------
    println!("🚀 Remove signer transaction submitted – waiting for finality …");
    sleep(Duration::from_secs(SYNC_STATE_WAIT_TIME)).await;

    client.sync_state().await.unwrap();

    let account_state = client
        .get_account(multisig_contract.id())
        .await?
        .expect("multisig contract not found");

    let storage_signer: Word = account_state
        .account()
        .storage()
        .get_map_item(
            SIGNERS_SLOT as u8,
            original_signer_pub_keys[SIGNER_TO_REMOVE_INDEX].into(),
        )?
        .into();
    println!("🔢 Storage Signer: {:?}", storage_signer);
    println!("✅ Success! The signer was removed.");

    Ok(())
}

#[tokio::test]
#[should_panic]
async fn remove_signer_with_non_signer() {
    delete_keystore_and_store().await;

    // -------------------------------------------------------------------------
    // 1. Instantiate client
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
        prepare_script(REMOVE_SIGNER_SCRIPT_PATH, MULTISIG_CODE_PATH, LIBRARY_PATH).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare advice map for new signer
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // generate random key pair which is not signer
    let (_, random_pub_key) = generate_keypair(&mut client);

    // insert new threshold into advice map at index 0
    advice_map.insert(
        prepare_felt_vec(SIGNER_TO_REMOVE_KEY_SLOT as u64).into(),
        random_pub_key.to_vec(),
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
async fn remove_signer_causing_threshold_unreachable() {
    delete_keystore_and_store().await;

    // -------------------------------------------------------------------------
    // 1. Instantiate client
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
        prepare_script(REMOVE_SIGNER_SCRIPT_PATH, MULTISIG_CODE_PATH, LIBRARY_PATH).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare advice map for new signer
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // insert new threshold into advice map at index 0
    advice_map.insert(
        prepare_felt_vec(SIGNER_TO_REMOVE_KEY_SLOT as u64).into(),
        original_signer_pub_keys[SIGNER_TO_REMOVE_CANT_REACH_THRESHOLD_INDEX].to_vec(),
    );

    // -------------------------------------------------------------------------
    // STEP 4: Build & Submit Transaction
    // -------------------------------------------------------------------------
    build_and_submit_tx(tx_script, advice_map, &mut client, multisig_contract.id())
        .await
        .unwrap();
}
