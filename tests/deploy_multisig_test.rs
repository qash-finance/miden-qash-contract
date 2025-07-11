use masm_project_template::{
    common::create_multisig_account,
    constants::{SIGNER_WEIGHTS, THRESHOLD},
};
use miden_client::{ClientError, rpc::Endpoint};
use miden_client_tools::{delete_keystore_and_store, instantiate_client};
use miden_objects::account::NetworkId;
use std::{fs, path::Path};

#[tokio::test]
async fn deploy_multisig() -> Result<(), ClientError> {
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
        .await?;

    client
        .add_account(&multisig_contract, Some(multisig_seed), false)
        .await
        .unwrap();

    println!(
        "📄 Multisig contract ID: {}",
        multisig_contract.id().to_bech32(NetworkId::Testnet)
    );

    Ok(())
}
