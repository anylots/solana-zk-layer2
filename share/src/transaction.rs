use serde::{Deserialize, Serialize};
use solana_sdk::transaction::Transaction;
use tiny_keccak::{Hasher, Sha3};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub block_num: u64,
    pub txns: Vec<Transaction>,
    pub txns_root: Option<[u8; 32]>,
    pub prev_state_root: Option<[u8; 32]>,
    pub post_state_root: Option<[u8; 32]>,
}

impl Block {
    pub fn new(txns: Vec<Transaction>) -> Self {
        Self {
            block_num: 0,
            txns,
            txns_root: None,
            prev_state_root: None,
            post_state_root: None,
        }
    }
}

/// Calculate txns root for the block
pub fn calculate_txns_root(txns: &[Transaction]) -> [u8; 32] {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    // Hash all transactions in the block
    for txn in txns {
        if let Ok(txn_data) = serde_json::to_vec(txn) {
            sha3.update(&txn_data);
        }
    }

    sha3.finalize(&mut output);
    output
}

pub fn load_blocks(start: u64, length: u64) -> Option<Vec<Block>> {
    let db = sled::open("block_db").ok()?;
    let mut blocks = vec![];
    for i in start..start + length {
        if let Ok(Some(data)) = db.get(format!("block_{}", i)) {
            if let Ok(block) = serde_json::from_slice::<Block>(&data) {
                blocks.push(block);
            }
        } else {
            return None;
        }
    }
    Some(blocks)
}
