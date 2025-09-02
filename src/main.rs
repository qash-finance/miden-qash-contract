use masm_project_template::common::{create_no_auth_faucet, instantiate_client};
use miden_client::{
    account::{AccountId, AccountStorageMode},
    asset::FungibleAsset,
    note::NoteType,
    rpc::Endpoint,
    transaction::TransactionRequestBuilder,
};
use miden_objects::account::NetworkId;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut client, _) = instantiate_client(Endpoint::testnet()).await?;

    client.sync_state().await?;

    let sync_height = client.get_sync_height().await?;
    println!("sync_height: {}", sync_height);

    // deploy fungible assets without the need of auth
    let account = create_no_auth_faucet(
        &mut client,
        "QLAB",
        1000000000000000000,
        8,
        AccountStorageMode::Public,
    )
    .await?;

    println!("account: {:?}", account.id().to_bech32(NetworkId::Testnet));

    // mint qash to
    let transaction_request = TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            FungibleAsset::new(account.id(), 100).unwrap(),
            AccountId::from_bech32("mtst1qps470fhfg77kyzc2k0he44g8uem0yyy")
                .unwrap()
                .1,
            NoteType::Public,
            client.rng(),
        )
        .unwrap();

    let tx_execution_result = client
        .new_transaction(account.id(), transaction_request)
        .await?;
    client.submit_transaction(tx_execution_result).await?;
    println!("Minted 100 tokens for mtst1qps470fhfg77kyzc2k0he44g8uem0yyy.",);
    Ok(())
}
