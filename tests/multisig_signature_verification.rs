use masm_project_template::{
    common::{
        create_basic_account, create_multisig_account, delete_keystore_and_store,
        instantiate_client, prepare_felt_vec,
    },
    constants::{SIGNER_WEIGHTS, THRESHOLD},
};
use miden_client::{
    ClientError, Felt, Word,
    crypto::RpoRandomCoin,
    note::NoteType,
    rpc::Endpoint,
    transaction::{OutputNote, OutputNotes},
};
use miden_lib::note::create_p2id_note;
use std::{fs, path::Path};

#[tokio::test]
async fn signature_verification() -> Result<(), ClientError> {
    delete_keystore_and_store().await;

    // -------------------------------------------------------------------------
    // Instantiate client
    // -------------------------------------------------------------------------
    let endpoint = Endpoint::testnet();
    let (mut client, keystore) = instantiate_client(endpoint).await.unwrap();

    client.sync_state().await.unwrap();

    // Deploy my multisig contract
    let multisig_code = fs::read_to_string(Path::new("./masm/accounts/multisig.masm")).unwrap();

    let (
        multisig_contract,
        multisig_seed,
        multisig_key_pair,
        original_signer_pub_keys,
        original_signer_secret_keys,
    ) = create_multisig_account(
        &mut client,
        &multisig_code,
        THRESHOLD,
        SIGNER_WEIGHTS.to_vec(),
        keystore.clone(),
    )
    .await?;

    client
        .add_account(&multisig_contract, Some(multisig_seed), false)
        .await
        .unwrap();

    // create alice as receiver
    let (alice_account, _) = create_basic_account(&mut client, keystore.clone())
        .await
        .unwrap();

    let note = create_p2id_note(
        multisig_contract.id(),
        alice_account.id(),
        vec![],
        NoteType::Public,
        Felt::new(0),
        &mut RpoRandomCoin::new(prepare_felt_vec(1)),
    )
    .unwrap();
    let output_notes = OutputNotes::new(vec![OutputNote::Full(note.clone())]).unwrap();
    let output_notes_commitment = output_notes.commitment();

    Ok(())
}
