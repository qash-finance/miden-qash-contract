// use masm_project_template::common::delete_keystore_and_store;
// use masm_project_template::{
//     common::{create_gift_note_recallable, instantiate_client, setup_accounts_and_faucets},
//     constants::NETWORK_ID,
// };
// use miden_client::{
//     asset::{Asset, FungibleAsset},
//     keystore::FilesystemKeyStore,
//     rpc::Endpoint,
//     transaction::TransactionRequestBuilder,
// };
// use miden_crypto::rand::FeltRng;
// use miden_objects::account::NetworkId;

// #[tokio::test]
// async fn create_and_open_gift_success() -> Result<(), Box<dyn std::error::Error>> {
//     delete_keystore_and_store().await;

//     // -------------------------------------------------------------------------
//     // 1. Instantiate client
//     // -------------------------------------------------------------------------
//     let endpoint = if NETWORK_ID == NetworkId::Testnet {
//         Endpoint::testnet()
//     } else {
//         Endpoint::devnet()
//     };

//     let mut client = instantiate_client(endpoint).await.unwrap();
//     client.sync_state().await.unwrap();

//     let keystore = FilesystemKeyStore::new("./keystore".into()).unwrap();

//     let balances = vec![
//         vec![100, 0], // For account[0] => Alice
//         vec![0, 100], // For account[1] => Bob
//     ];
//     let (accounts, faucets) =
//         setup_accounts_and_faucets(&mut client, keystore, 2, 2, balances).await?;

//     // rename for clarity
//     let alice_account = accounts[0].clone();
//     let bob_account = accounts[1].clone();
//     let faucet = faucets[0].clone();

//     let serial_num = client.rng().draw_word();
//     let secret_hash = client.rng().draw_word();

//     // alice create gift
//     let gift_note = create_gift_note_recallable(
//         alice_account.id(),
//         Asset::Fungible(FungibleAsset::new(faucet.id(), 100).unwrap()),
//         secret_hash,
//         serial_num,
//     )?;

//     client.sync_state().await?;

//     // now bob need to open the gift
//     let consume_req = TransactionRequestBuilder::new()
//         .with_authenticated_input_notes([(gift_note.id(), secret_hash.into())])
//         .build()
//         .unwrap();

//     let tx_exec = client
//         .new_transaction(bob_account.id(), consume_req)
//         .await?;
//     client.submit_transaction(tx_exec).await?;

//     Ok(())
// }
