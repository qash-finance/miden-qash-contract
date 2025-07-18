use miden_objects::account::NetworkId;

// multisig setup
pub const THRESHOLD: usize = 3;
pub const TOTAL_WEIGHT: usize = 4;
pub const SIGNER_WEIGHTS: [usize; 3] = [2, 1, 1];
pub const SIGNER_TO_REMOVE_INDEX: usize = 2;
pub const SIGNER_TO_REMOVE_CANT_REACH_THRESHOLD_INDEX: usize = 0;

// contract storage slots
pub const PUBKEY_SLOT: usize = 0;
pub const THRESHOLD_SLOT: usize = 1;
pub const TOTAL_WEIGHT_SLOT: usize = 2;
pub const SIGNERS_SLOT: usize = 3;
pub const MESSAGE_HASH_SLOT: usize = 4;

// error
pub const INVALID_WEIGHT: usize = 100;

// advice map location for change threshold
pub const NEW_THRESHOLD_AS_KEY_SLOT: usize = 0;

// advice map location for add signer
pub const NEW_SIGNER_PUBKEY_KEY_SLOT: usize = 0;
pub const NEW_SIGNER_WEIGHT_KEY_SLOT: usize = 1;

// advice map location for remove signer
pub const SIGNER_TO_REMOVE_KEY_SLOT: usize = 0;

// file location
pub const MULTISIG_CODE_PATH: &str = "./masm/accounts/multisig.masm";
pub const CHANGE_THRESHOLD_SCRIPT_PATH: &str = "./masm/scripts/change_threshold.masm";
pub const ADD_SIGNER_SCRIPT_PATH: &str = "./masm/scripts/add_signer.masm";
pub const REMOVE_SIGNER_SCRIPT_PATH: &str = "./masm/scripts/remove_signer.masm";
pub const LIBRARY_PATH: &str = "external_contract::multisig_contract";

// miden client
pub const SYNC_STATE_WAIT_TIME: u64 = 7;
pub const NETWORK_ID: NetworkId = NetworkId::Testnet;
