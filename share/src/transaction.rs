use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solana_sdk::{system_instruction::SystemInstruction, transaction::Transaction};

pub struct TransferOp {
    pub from: String,
    pub to: String,
    pub amount: u128,
}

pub fn parsing_instruction(
    instruction: &solana_sdk::instruction::CompiledInstruction,
    txn: &Transaction,
) -> Result<Option<TransferOp>> {
    let program_id_index = instruction.program_id_index as usize;

    if program_id_index >= txn.message.account_keys.len() {
        return Err(anyhow!("Invalid program_id_index"));
    }

    let program_id = &txn.message.account_keys[program_id_index];

    // parsing system program instructions
    if program_id == &solana_sdk::system_program::ID {
        parsing_sys_instruction(instruction, txn)?;
    } else {
        // for other instructions, only basic logging is done
        info!("Processing instruction for program: {}", program_id);
    }

    Ok(None)
}

fn parsing_sys_instruction(
    instruction: &solana_sdk::instruction::CompiledInstruction,
    txn: &Transaction,
) -> Result<Option<TransferOp>> {
    if instruction.data.is_empty() {
        return Ok(None);
    }

    // transfer Instructions
    match bincode::deserialize::<SystemInstruction>(&instruction.data) {
        Ok(SystemInstruction::Transfer { lamports }) => {
            // transfer ins
            if instruction.accounts.len() >= 2 {
                let from_index = instruction.accounts[0] as usize;
                let to_index = instruction.accounts[1] as usize;

                if from_index < txn.message.account_keys.len()
                    && to_index < txn.message.account_keys.len()
                {
                    let from_pubkey = txn.message.account_keys[from_index].to_string();
                    let to_pubkey = txn.message.account_keys[to_index].to_string();

                    return Ok(Some(TransferOp {
                        from: from_pubkey,
                        to: to_pubkey,
                        amount: lamports as u128,
                    }));
                }
            }
        }
        Ok(_) => {
            info!("Non-transfer system instruction");
        }
        Err(e) => {
            warn!("Failed to deserialize system instruction: {}", e);
        }
    }
    Ok(None)
}
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
    let mut hasher = Sha256::new();

    // Hash all transactions in the block
    for txn in txns {
        if let Ok(txn_data) = serde_json::to_vec(txn) {
            hasher.update(&txn_data);
        }
    }

    hasher.finalize().into()
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

pub struct BlockDB {
    pub db: sled::Db,
}

impl BlockDB {
    pub fn new(db_path: &str) -> Self {
        let block_db = sled::open(db_path).unwrap();
        Self { db: block_db }
    }
}
