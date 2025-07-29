use std::time::Duration;

use masm_project_template::common::{
    build_and_submit_tx, create_evm_account, delete_keystore_and_store, prepare_felt_vec,
    prepare_script, wait_for_notes,
};
use masm_project_template::constants::{
    EVM_CODE_PATH, EVM_LIBRARY_PATH, LIBRARY_PATH, SYNC_STATE_WAIT_TIME, VERIFY_ECDSA_SCRIPT_PATH,
};
use masm_project_template::{
    common::{create_gift_note_recallable, instantiate_client, setup_accounts_and_faucets},
    constants::NETWORK_ID,
};
use miden_client::Felt;
use miden_client::account::AccountStorageMode;
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

struct Point {
    x: Vec<u64>,
    y: Vec<u64>,
    z: Vec<u64>,
}

impl Point {
    fn new(x: Vec<u64>, y: Vec<u64>, z: Vec<u64>) -> Self {
        Self { x, y, z }
    }
}

struct Signature {
    r: Vec<u64>,
    s: Vec<u64>,
}

impl Signature {
    fn new(r: Vec<u64>, s: Vec<u64>) -> Self {
        Self { r, s }
    }
}

#[tokio::test]
async fn verify_ecdsa_success() -> Result<(), Box<dyn std::error::Error>> {
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

    // create evm account
    let evm_account = create_evm_account(&mut client, AccountStorageMode::Public)
        .await
        .unwrap();

    // print address
    println!(
        "evm account: {:?}",
        evm_account.id().to_bech32(NetworkId::Testnet)
    );

    // prepare script
    let tx_script =
        prepare_script(VERIFY_ECDSA_SCRIPT_PATH, EVM_CODE_PATH, EVM_LIBRARY_PATH).unwrap();

    let public_key = Point::new(
        vec![
            1187647059, 1135132293, 1524607722, 3257770169, 1812770566, 4163599075, 3343690625,
            2983146250,
        ],
        vec![
            694970425, 3961647168, 2962892522, 3871680339, 479244527, 2106589630, 3531004100,
            487738481,
        ],
        vec![
            1718928786, 2222219308, 1537333708, 969814285, 1600645591, 2744076726, 1359599981,
            1095895041,
        ],
    );
    let message_hash: Vec<u64> = vec![
        1915140291, 1682821516, 1088031394, 2866424576, 2852209138, 1159876682, 234168247,
        3360002988,
    ];

    let signature = Signature::new(
        vec![
            1494159694, 3668493121, 2315165624, 353127114, 974571799, 2051320959, 3421809437,
            3258836281,
        ],
        vec![
            1259054195, 60155476, 2236955964, 2106542718, 1332177784, 1407189293, 11489664,
            3695133146,
        ],
    );

    let mut advice_map = AdviceMap::default();

    // -------------------------------------------------------------------------
    // Signature - S
    // -------------------------------------------------------------------------
    let signature_s_first_half: Vec<Felt> = signature.s[..4]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    let signature_s_second_half: Vec<Felt> = signature.s[4..]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    advice_map.insert(prepare_felt_vec(0 as u64).into(), signature_s_second_half);
    advice_map.insert(prepare_felt_vec(1 as u64).into(), signature_s_first_half);

    // -------------------------------------------------------------------------
    // Signature - R
    // -------------------------------------------------------------------------
    let signature_r_first_half: Vec<Felt> = signature.r[..4]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    let signature_r_second_half: Vec<Felt> = signature.r[4..]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    advice_map.insert(prepare_felt_vec(2 as u64).into(), signature_r_second_half);
    advice_map.insert(prepare_felt_vec(3 as u64).into(), signature_r_first_half);

    // -------------------------------------------------------------------------
    // Message - Hash
    // -------------------------------------------------------------------------
    let message_hash_first_half: Vec<Felt> = message_hash[..4]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    let message_hash_second_half: Vec<Felt> = message_hash[4..]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    advice_map.insert(prepare_felt_vec(4 as u64).into(), message_hash_second_half);
    advice_map.insert(prepare_felt_vec(5 as u64).into(), message_hash_first_half);

    // -------------------------------------------------------------------------
    // Public Key Z
    // -------------------------------------------------------------------------
    let public_key_z_first_half: Vec<Felt> = public_key.z[..4]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    let public_key_z_second_half: Vec<Felt> = public_key.z[4..]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    advice_map.insert(prepare_felt_vec(6 as u64).into(), public_key_z_second_half);
    advice_map.insert(prepare_felt_vec(7 as u64).into(), public_key_z_first_half);

    // -------------------------------------------------------------------------
    // Public Key Y
    // -------------------------------------------------------------------------
    let public_key_y_first_half: Vec<Felt> = public_key.y[..4]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    let public_key_y_second_half: Vec<Felt> = public_key.y[4..]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    advice_map.insert(prepare_felt_vec(8 as u64).into(), public_key_y_second_half);
    advice_map.insert(prepare_felt_vec(9 as u64).into(), public_key_y_first_half);

    // -------------------------------------------------------------------------
    // Public Key X
    // -------------------------------------------------------------------------
    let public_key_x_first_half: Vec<Felt> = public_key.x[..4]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    let public_key_x_second_half: Vec<Felt> = public_key.x[4..]
        .iter()
        .map(|&x| Felt::new(x))
        .rev()
        .collect();
    advice_map.insert(prepare_felt_vec(10 as u64).into(), public_key_x_second_half);
    advice_map.insert(prepare_felt_vec(11 as u64).into(), public_key_x_first_half);

    build_and_submit_tx(tx_script, advice_map, &mut client, evm_account.id())
        .await
        .unwrap();

    println!("verify ecdsa success");
    Ok(())
}
