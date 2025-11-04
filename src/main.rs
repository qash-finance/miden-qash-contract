use masm_project_template::common::{create_no_auth_faucet, instantiate_client};
use miden_client::{
    account::{AccountIdAddress, AccountStorageMode, Address, AddressInterface},
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
        "QWAP",
        1000000000000000000,
        8,
        AccountStorageMode::Public,
    )
    .await?;

    let addr = AccountIdAddress::new(account.id(), AddressInterface::Unspecified);

    // build address of faucet
    let address = Address::AccountId(addr);
    println!("account: {:?}", address.to_bech32(NetworkId::Testnet));

    let addr = Address::from_bech32("mtst1qzx905defy842yr7sgnh7pxpqpcqq86aypm")
        .unwrap()
        .1;

    // Extract account ID from the address
    let account_id = match addr {
        Address::AccountId(account_id_address) => account_id_address.id(),
        _ => panic!("Invalid address"),
    };

    // mint qash to
    let transaction_request = TransactionRequestBuilder::new()
        .build_mint_fungible_asset(
            FungibleAsset::new(account.id(), 100000000000000).unwrap(),
            account_id,
            NoteType::Public,
            client.rng(),
        )
        .unwrap();

    let tx_execution_result = client
        .new_transaction(account.id(), transaction_request)
        .await?;
    client.submit_transaction(tx_execution_result).await?;
    println!("Minted 100 tokens for mtst1qzx905defy842yr7sgnh7pxpqpcqq86aypm.",);
    Ok(())
}
