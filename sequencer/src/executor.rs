use anyhow::{anyhow, Result};
use log::info;
use share::{
    state::StateDB,
    transaction::{parsing_instruction, Block, TransferOp},
};
use solana_sdk::transaction::Transaction;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

pub static MAX_MEMPOOL_SIZE: usize = 1024;

pub struct Executor {}

impl Executor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn pending_size(&self) -> usize {
        MEMPOOL.read().await.len()
    }

    pub async fn execute(&self) -> Result<Block> {
        let mut pending_txns = MEMPOOL.write().await;
        let mut transfers = Vec::new();

        for txn in pending_txns.iter() {
            match pre_process(txn) {
                Ok(Some(op)) => transfers.push(op),
                _ => {}
            }
        }
        let mut state_db = STATE.write().await;
        let balances = &mut state_db.state.balances;
        if !transfers.is_empty() {
            let _ = transfer(balances, transfers);
        };

        let block = Block::new(pending_txns.to_vec());
        pending_txns.drain(..);
        Ok(block)
    }
}
pub fn pre_process(txn: &Transaction) -> Result<Option<TransferOp>> {
    let signature = txn.signatures[0].to_string();
    // Parsing each instruction in the transaction
    for (_i, instruction) in txn.message.instructions.iter().enumerate() {
        let op = parsing_instruction(instruction, &txn)?;
        if op.is_some() {
            return Ok(op);
        }
    }

    info!("Transaction processing completed: {}", signature);
    Ok(None)
}

fn transfer(balances: &mut HashMap<String, u128>, ops: Vec<TransferOp>) -> Result<()> {
    for op in ops {
        let from = op.from;
        let to = op.to;
        let amount = op.amount;
        // fetch sender's amount
        let from_balance = balances.get(&from).copied().unwrap_or(0);
        if from_balance < amount {
            return Err(anyhow!("Insufficient balance for transfer"));
        }
        // change the balance
        balances.insert(from, from_balance - amount);
        let to_balance = balances.get(&to).copied().unwrap_or(0);
        balances.insert(to.to_string(), to_balance + amount);
    }
    Ok(())
}

// Global State instance
lazy_static::lazy_static! {
    pub static ref STATE: Arc<RwLock<StateDB>> = {
        let mut state_db = StateDB::new("state_db");
        let account = "AyUFvdbpc4xyJYW1jkCNZVQ54B3bMyqE4reiD45fK7T7";
        if state_db.state.get_balance(account)==0{
            // Initialize dev account with 100 SOL
            let balance_in_lamports = 100_u128 * 1_000_000_000_u128; // 100 SOL in lamports
            state_db.state.set_balance(account.to_string(), balance_in_lamports);// 1000000000
        }
        Arc::new(RwLock::new(state_db))
    };
}

lazy_static::lazy_static! {
        pub static ref MEMPOOL: Arc<RwLock<Vec<Transaction>>>= Arc::new(RwLock::new(Vec::with_capacity(256)));
}
