// multisig setup
pub const THRESHOLD: usize = 3;
pub const TOTAL_WEIGHT: usize = 4;
pub const SIGNER_WEIGHTS: [usize; 3] = [2, 1, 1];

// contract storage slots
pub const THRESHOLD_SLOT: usize = 0;
pub const TOTAL_WEIGHT_SLOT: usize = 1;
pub const SIGNERS_SLOT: usize = 2;
pub const MESSAGE_HASH_SLOT: usize = 3;

// error
pub const INVALID_WEIGHT: usize = 100;

// advice map location for change threshold
pub const NEW_THRESHOLD_AS_KEY_SLOT: usize = 0;

// advice map location for add signer
pub const NEW_SIGNER_PUBKEY_KEY_SLOT: usize = 0;
pub const NEW_SIGNER_WEIGHT_KEY_SLOT: usize = 1;
