use std::{fs, path::Path};

use masm_project_template::{
    common::{build_and_submit_tx, create_multisig_account, generate_keypair},
    constants::{SIGNER_WEIGHTS, SIGNERS_SLOT, THRESHOLD},
};

use miden_client_tools::{
    create_library, create_tx_script, delete_keystore_and_store, instantiate_client,
};

use miden_client::{rpc::Endpoint, transaction::TransactionRequestBuilder};
use miden_crypto::{Felt, Word, ZERO};
use miden_objects::{account::NetworkId, vm::AdviceMap};
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn add_signer_success() -> Result<(), Box<dyn std::error::Error>> {
    delete_keystore_and_store(None).await;

    // -------------------------------------------------------------------------
    // Instantiate client
    // -------------------------------------------------------------------------
    let endpoint = Endpoint::testnet();
    let mut client = instantiate_client(endpoint, None).await.unwrap();

    client.sync_state().await.unwrap();

    // Deploy my multisig contract
    let multisig_code = fs::read_to_string(Path::new("./masm/accounts/multisig.masm")).unwrap();

    let (multisig_contract, multisig_seed, original_signer_pub_keys, _original_signer_secret_keys) =
        create_multisig_account(
            &mut client,
            &multisig_code,
            THRESHOLD,
            SIGNER_WEIGHTS.to_vec(),
        )
        .await?;

    client
        .add_account(&multisig_contract, Some(multisig_seed), false)
        .await
        .unwrap();

    println!(
        "📄 Multisig contract ID: {}",
        multisig_contract.id().to_bech32(NetworkId::Testnet)
    );

    // -------------------------------------------------------------------------
    // STEP 2: Prepare the Script
    // -------------------------------------------------------------------------
    let script_code = fs::read_to_string(Path::new("./masm/scripts/add_signer.masm")).unwrap();

    let account_code = fs::read_to_string(Path::new("./masm/accounts/multisig.masm")).unwrap();
    let library_path = "external_contract::multisig_contract";

    let library = create_library(account_code, library_path).unwrap();

    let tx_script = create_tx_script(script_code, Some(library)).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare advice map for new signer
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // generate keypair
    let (_, new_signer_public_key) = generate_keypair(&mut client);

    println!("new signer public key: {:?}", new_signer_public_key);

    // insert public key into advice map at index 0
    advice_map.insert(
        [Felt::new(0), ZERO, ZERO, ZERO].into(),
        // new_signer_public_key.into(),
        new_signer_public_key.into(),
    );
    // insert new signer weight into advice map at index 1
    advice_map.insert(
        [Felt::new(1), ZERO, ZERO, ZERO].into(),
        [Felt::new(1), ZERO, ZERO, ZERO].into(),
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
    println!("🚀 Add signer transaction submitted – waiting for finality …");
    sleep(Duration::from_secs(7)).await;

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
    delete_keystore_and_store(None).await;

    // -------------------------------------------------------------------------
    // Instantiate client
    // -------------------------------------------------------------------------
    let endpoint = Endpoint::testnet();
    let mut client = instantiate_client(endpoint, None).await.unwrap();

    client.sync_state().await.unwrap();

    // Deploy my multisig contract
    let multisig_code = fs::read_to_string(Path::new("./masm/accounts/multisig.masm")).unwrap();

    let (multisig_contract, multisig_seed, original_signer_pub_keys, _original_signer_secret_keys) =
        create_multisig_account(
            &mut client,
            &multisig_code,
            THRESHOLD,
            SIGNER_WEIGHTS.to_vec(),
        )
        .await
        .unwrap();

    client
        .add_account(&multisig_contract, Some(multisig_seed), false)
        .await
        .unwrap();

    println!(
        "📄 Multisig contract ID: {}",
        multisig_contract.id().to_bech32(NetworkId::Testnet)
    );

    // -------------------------------------------------------------------------
    // STEP 2: Prepare the Script
    // -------------------------------------------------------------------------
    let script_code = fs::read_to_string(Path::new("./masm/scripts/add_signer.masm")).unwrap();

    let account_code = fs::read_to_string(Path::new("./masm/accounts/multisig.masm")).unwrap();
    let library_path = "external_contract::multisig_contract";

    let library = create_library(account_code, library_path).unwrap();

    let tx_script = create_tx_script(script_code, Some(library)).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare advice map for new signer
    // -------------------------------------------------------------------------
    let mut advice_map = AdviceMap::default();

    // insert public key into advice map at index 0
    advice_map.insert(
        [Felt::new(0), ZERO, ZERO, ZERO].into(),
        original_signer_pub_keys[0].into(),
    );
    advice_map.insert(
        [Felt::new(1), ZERO, ZERO, ZERO].into(),
        [Felt::new(1), ZERO, ZERO, ZERO].into(),
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
    delete_keystore_and_store(None).await;

    // -------------------------------------------------------------------------
    // Instantiate client
    // -------------------------------------------------------------------------
    let endpoint = Endpoint::testnet();
    let mut client = instantiate_client(endpoint, None).await.unwrap();

    client.sync_state().await.unwrap();

    // Deploy my multisig contract
    let multisig_code = fs::read_to_string(Path::new("./masm/accounts/multisig.masm")).unwrap();

    let (multisig_contract, multisig_seed, _original_signer_pub_keys, _original_signer_secret_keys) =
        create_multisig_account(
            &mut client,
            &multisig_code,
            THRESHOLD,
            SIGNER_WEIGHTS.to_vec(),
        )
        .await
        .unwrap();

    client
        .add_account(&multisig_contract, Some(multisig_seed), false)
        .await
        .unwrap();

    println!(
        "📄 Multisig contract ID: {}",
        multisig_contract.id().to_bech32(NetworkId::Testnet)
    );

    // -------------------------------------------------------------------------
    // STEP 2: Prepare the Script
    // -------------------------------------------------------------------------
    let script_code = fs::read_to_string(Path::new("./masm/scripts/add_signer.masm")).unwrap();

    let account_code = fs::read_to_string(Path::new("./masm/accounts/multisig.masm")).unwrap();
    let library_path = "external_contract::multisig_contract";

    let library = create_library(account_code, library_path).unwrap();

    let tx_script = create_tx_script(script_code, Some(library)).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Prepare advice map for new signer
    // -------------------------------------------------------------------------
    // generate keypair
    let (_, new_signer_public_key) = generate_keypair(&mut client);

    let mut advice_map = AdviceMap::default();

    // insert public key into advice map at index 0
    advice_map.insert(
        [Felt::new(0), ZERO, ZERO, ZERO].into(),
        new_signer_public_key.into(),
    );
    advice_map.insert(
        [Felt::new(1), ZERO, ZERO, ZERO].into(),
        [Felt::new(100), ZERO, ZERO, ZERO].into(),
    );

    // -------------------------------------------------------------------------
    // STEP 4: Build & Submit Transaction
    // -------------------------------------------------------------------------
    build_and_submit_tx(tx_script, advice_map, &mut client, multisig_contract.id())
        .await
        .unwrap();
}
