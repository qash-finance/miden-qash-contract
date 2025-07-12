use std::{fs, path::Path};

use masm_project_template::{
    common::{build_and_submit_tx, create_multisig_account, create_public_immutable_contract, generate_keypair, prepare_felt_vec},
    constants::{
        INVALID_WEIGHT, NEW_SIGNER_PUBKEY_KEY_SLOT, NEW_SIGNER_WEIGHT_KEY_SLOT, SIGNERS_SLOT, SIGNER_WEIGHTS, THRESHOLD
    },
};

use miden_client_tools::{
    create_basic_account, create_library, create_tx_script, delete_keystore_and_store, instantiate_client
};

use miden_client::{keystore::FilesystemKeyStore, rpc::Endpoint, transaction::{TransactionRequestBuilder, TransactionScript}};
use miden_crypto::Word;
use miden_lib::transaction::TransactionKernel;
use miden_objects::{account::NetworkId, vm::AdviceMap};
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn set_spending_limit() -> Result<(), Box<dyn std::error::Error>> {
    delete_keystore_and_store(None).await;

    // -------------------------------------------------------------------------
    // Instantiate client
    // -------------------------------------------------------------------------
    let endpoint = Endpoint::testnet();
    let mut client = instantiate_client(endpoint, None).await.unwrap();

    client.sync_state().await.unwrap();

    // Deploy my spending limit contract
    let spending_limit_code = fs::read_to_string(Path::new("./masm/accounts/spending_limit.masm")).unwrap();

    let (spending_limit_contract, spending_limit_seed) =
        create_public_immutable_contract(&mut client, &spending_limit_code)
            .await
            .unwrap();

    println!(
        "contract id: {:?}",
        spending_limit_contract.id().to_bech32(NetworkId::Testnet)
    );

    client
        .add_account(&spending_limit_contract, Some(spending_limit_seed), false)
        .await
        .unwrap();

    // -------------------------------------------------------------------------
    // STEP 2: Prepare the Script
    // -------------------------------------------------------------------------

    let library_path = "external_contract::spending_limit_contract";

    let library = create_library(spending_limit_code, library_path).unwrap();
    let assembler = TransactionKernel::assembler().with_library(&library).unwrap();

    let tx_script_code =
        fs::read_to_string(Path::new("./masm/scripts/set_spending_script.masm")).unwrap();
    let tx_script = TransactionScript::compile(&tx_script_code, Vec::new(), assembler).unwrap();

    // -------------------------------------------------------------------------
    // STEP 3: Build & Submit Transaction
    // -------------------------------------------------------------------------
    let update_spending_request = TransactionRequestBuilder::new()
        .with_custom_script(tx_script)
        .build()
        .unwrap();

    let tx_result = client
        .new_transaction(spending_limit_contract.id(), update_spending_request)
        .await
        .unwrap();

    let _ = client.submit_transaction(tx_result).await?;

    Ok(())
}