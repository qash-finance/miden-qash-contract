use sha2::{Digest, Sha256};
use std::time::Duration;

use masm_project_template::common::{
    create_basic_account, create_sha256_note, delete_keystore_and_store, prepare_felt_vec,
    wait_for_notes,
};
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
use miden_objects::vm::AdviceMap;
use tokio::time::sleep;

#[tokio::test]
async fn sha256_test() -> Result<(), Box<dyn std::error::Error>> {
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

    let (account, secret) = create_basic_account(&mut client, keystore).await?;
    let rng = client.rng();
    let serial_num = rng.inner_mut().draw_word();

    let bytes = [
        165, 205, 129, 202, 250, 29, 226, 202, 146, 255, 58, 58, 211, 113, 214, 143, 25, 176, 173,
        99, 19, 6, 78, 36, 246, 76, 52, 132, 6, 224, 102, 237,
    ];
    // let bytes = [
    //     0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //     0, 0,
    // ];

    // Calculate SHA256 hash using Rust's sha2 crate for comparison
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    println!("Expected SHA256 hash: 0x{}", hex::encode(result));

    // Convert hex result back to array of bytes for comparison
    let result_bytes: Vec<u8> = result.to_vec();
    println!("Expected SHA256 result as bytes: {:?}", result_bytes);

    // Also show as array of u32 words (big-endian) for easier comparison with Miden output
    let mut result_words = Vec::new();
    for i in 0..8 {
        let word = ((result_bytes[i * 4] as u32) << 24)
            | ((result_bytes[i * 4 + 1] as u32) << 16)
            | ((result_bytes[i * 4 + 2] as u32) << 8)
            | (result_bytes[i * 4 + 3] as u32);
        result_words.push(word);
    }
    println!("Expected SHA256 result as u32 words: {:?}", result_words);

    // alice create gift
    let sha256_note = create_sha256_note(
        account.id(),
        result_words.iter().map(|&x| Felt::new(x as u64)).collect(),
        serial_num,
    )?;

    let output_note = OutputNote::Full(sha256_note.clone());

    // submit gift note to alice account
    let tx_request = TransactionRequestBuilder::new()
        .own_output_notes(vec![output_note])
        .build()
        .unwrap();
    let tx_exec = client.new_transaction(account.id(), tx_request).await?;
    client.submit_transaction(tx_exec.clone()).await?;

    // wait for 7 seconds
    sleep(Duration::from_secs(SYNC_STATE_WAIT_TIME)).await;

    client.sync_state().await?;

    // check if the note holding the correct asset
    let created_note = tx_exec.created_notes().get_note(0);
    // check created note
    println!("created_note assets: {:?}", created_note.assets().unwrap());

    // Pack bytes into 32-bit words in big-endian format
    // Each word contains 4 bytes: [byte0, byte1, byte2, byte3] -> word = (byte0 << 24) | (byte1 << 16) | (byte2 << 8) | byte3
    let mut words = Vec::new();
    for i in 0..8 {
        let word = (bytes[i * 4] as u64) << 24
            | (bytes[i * 4 + 1] as u64) << 16
            | (bytes[i * 4 + 2] as u64) << 8
            | (bytes[i * 4 + 3] as u64);
        words.push(Felt::new(word));
    }

    println!("words: {:?}", words);

    let mut advice_map = AdviceMap::default();
    advice_map.insert(
        prepare_felt_vec(0 as u64).into(),
        vec![words[3], words[2], words[1], words[0]],
    );
    advice_map.insert(
        prepare_felt_vec(1 as u64).into(),
        vec![words[7], words[6], words[5], words[4]],
    );

    let consume_req = TransactionRequestBuilder::new()
        .unauthenticated_input_notes([(sha256_note, None)])
        .extend_advice_map(advice_map)
        .build()
        .unwrap();

    let tx_exec = client.new_transaction(account.id(), consume_req).await?;
    client.submit_transaction(tx_exec).await?;

    Ok(())
}
