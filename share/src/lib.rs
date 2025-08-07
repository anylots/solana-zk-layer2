pub mod state;
pub mod transaction;
pub mod utils;
pub mod zkvm;

// L2 Widhdrawal address
pub static WITHDRAWAL_ADDRESS: &str = "AF111111111111111111111111111111";
// L2 Sys Program ID.
pub static L2_SYS_PROGRAM_ID: &str = "My11111111111111111111111111111111111111111";
pub static DEFAULT_L1_RPC: &str = "http://localhost:8898";
pub static DEFAULT_L1_WS: &str = "ws://127.0.0.1:8900";
pub static DEFAULT_L2_RPC: &str = "http://localhost:8899";
pub static UNSAFE_PRIVATE_KEY: &str =
    "2bCxRJ2GSYnbEHMPsxcf6dWzFtNzaQLFXbNDSAqr2aWaSBgFbhnFejoC4z9LHcLGzkjvY6ZtBFWDoEzcVqq82PSo";
