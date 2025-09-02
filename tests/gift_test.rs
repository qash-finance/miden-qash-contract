use std::time::Duration;

use masm_project_template::common::{delete_keystore_and_store, wait_for_notes};
use masm_project_template::constants::SYNC_STATE_WAIT_TIME;
use masm_project_template::{
    common::{create_gift_note_recallable, instantiate_client, setup_accounts_and_faucets},
    constants::NETWORK_ID,
};
use miden_client::Felt;
use miden_client::rpc::Endpoint;
use miden_client::transaction::OutputNote;
use miden_client::{
    asset::{Asset, FungibleAsset},
    keystore::FilesystemKeyStore,
    transaction::TransactionRequestBuilder,
};
use miden_objects::account::NetworkId;
use tokio::time::sleep;

#[tokio::test]
async fn create_and_open_gift_success() -> Result<(), Box<dyn std::error::Error>> {
    delete_keystore_and_store().await;

    // -------------------------------------------------------------------------
    // 1. Instantiate client
    // -------------------------------------------------------------------------
    let endpoint = if NETWORK_ID == NetworkId::Testnet {
        Endpoint::testnet()
    } else {
        Endpoint::devnet()
    };

    let (mut client, keystore) = instantiate_client(endpoint).await.unwrap();
    client.sync_state().await.unwrap();

    let gift_amount = 100;

    let balances = vec![
        vec![100, 0], // For account[0] => Alice
        vec![0, 100], // For account[1] => Bob
    ];
    let (accounts, faucets) =
        setup_accounts_and_faucets(&mut client, keystore, 2, 2, balances).await?;

    // rename for clarity
    let alice_account = accounts[0].clone();
    let bob_account = accounts[1].clone();
    let faucet = faucets[0].clone();

    let rng = client.rng();
    let serial_num = rng.inner_mut().draw_word();
    // let secret = rng.inner_mut().draw_word();
    // let secret = rng.inner_mut().draw_word();
    let secret = [
        Felt::new(1209008168),
        Felt::new(1192048525),
        Felt::new(1539272724),
        Felt::new(1649632662),
    ];

    println!("serial_num: {:?}", serial_num);
    println!("secret: {:?}", secret);

    // alice create gift
    let gift_note = create_gift_note_recallable(
        alice_account.id(),
        Asset::Fungible(FungibleAsset::new(faucet.id(), gift_amount).unwrap()),
        secret,
        serial_num,
    )?;

    // turn note into output note
    let output_note = OutputNote::Full(gift_note.clone());

    // submit gift note to alice account
    let tx_request = TransactionRequestBuilder::new()
        .own_output_notes(vec![output_note])
        .build()
        .unwrap();
    let tx_exec = client
        .new_transaction(alice_account.id(), tx_request)
        .await?;
    client.submit_transaction(tx_exec.clone()).await?;

    // wait for 7 seconds
    sleep(Duration::from_secs(SYNC_STATE_WAIT_TIME)).await;

    client.sync_state().await?;

    // check if the note holding the correct asset
    let created_note = tx_exec.created_notes().get_note(0);
    // check created note
    println!("created_note assets: {:?}", created_note.assets().unwrap());

    // now bob need to open the gift
    let consume_req = TransactionRequestBuilder::new()
        .unauthenticated_input_notes([(gift_note, secret.into())])
        .build()
        .unwrap();

    let tx_exec = client
        .new_transaction(bob_account.id(), consume_req)
        .await?;
    client.submit_transaction(tx_exec).await?;

    sleep(Duration::from_secs(SYNC_STATE_WAIT_TIME)).await;

    client.sync_state().await.unwrap();

    // bob should have `gift_amount` tokens
    // Get updated account states from the client
    let alice_account_state = client
        .get_account(alice_account.id())
        .await?
        .expect("alice account not found");
    let bob_account_state = client
        .get_account(bob_account.id())
        .await?
        .expect("bob account not found");

    let balance_alice = alice_account_state
        .account()
        .vault()
        .get_balance(faucet.id())
        .unwrap();
    let balance_bob = bob_account_state
        .account()
        .vault()
        .get_balance(faucet.id())
        .unwrap();
    println!("balance_bob: {:?}", balance_bob);
    println!("balance_alice: {:?}", balance_alice);

    assert_eq!(balance_bob, gift_amount);

    Ok(())
}

#[tokio::test]
#[should_panic]
async fn open_gift_with_wrong_secret() {
    delete_keystore_and_store().await;

    // -------------------------------------------------------------------------
    // 1. Instantiate client
    // -------------------------------------------------------------------------
    let endpoint = if NETWORK_ID == NetworkId::Testnet {
        Endpoint::testnet()
    } else {
        Endpoint::devnet()
    };

    let (mut client, keystore) = instantiate_client(endpoint).await.unwrap();
    client.sync_state().await.unwrap();

    let balances = vec![
        vec![100, 0], // For account[0] => Alice
        vec![0, 100], // For account[1] => Bob
    ];
    let (accounts, faucets) = setup_accounts_and_faucets(&mut client, keystore, 2, 2, balances)
        .await
        .unwrap();

    // rename for clarity
    let alice_account = accounts[0].clone();
    let bob_account = accounts[1].clone();
    let faucet = faucets[0].clone();

    let rng = client.rng();
    let serial_num = rng.inner_mut().draw_word();
    let secret = rng.inner_mut().draw_word();

    println!("serial_num: {:?}", serial_num);
    println!("secret: {:?}", secret);

    // alice create gift
    let gift_note = create_gift_note_recallable(
        alice_account.id(),
        Asset::Fungible(FungibleAsset::new(faucet.id(), 100).unwrap()),
        secret,
        serial_num,
    )
    .unwrap();

    // turn note into output note
    let output_note = OutputNote::Full(gift_note.clone());

    // submit gift note to alice account
    let tx_request = TransactionRequestBuilder::new()
        .own_output_notes(vec![output_note])
        .build()
        .unwrap();
    let tx_exec = client
        .new_transaction(alice_account.id(), tx_request)
        .await
        .unwrap();
    client.submit_transaction(tx_exec).await.unwrap();

    // wait for 7 seconds
    sleep(Duration::from_secs(SYNC_STATE_WAIT_TIME)).await;

    client.sync_state().await.unwrap();

    // now bob need to open the gift
    let consume_req = TransactionRequestBuilder::new()
        .unauthenticated_input_notes([(
            gift_note,
            Some([Felt::new(1), secret[0], secret[1], secret[2]]),
        )])
        .build()
        .unwrap();

    let tx_exec = client
        .new_transaction(bob_account.id(), consume_req)
        .await
        .unwrap();
    client.submit_transaction(tx_exec).await.unwrap();
}
