use anyhow::{anyhow, Result};
use log::info;
use share::{
    state::StateDB,
    transaction::{parsing_instruction, Block, TransferOp},
};
use solana_sdk::transaction::Transaction;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

static MAX_MEMPOOL_SIZE: usize = 1024;

pub struct Executor {
    pub mempool: Arc<RwLock<Vec<Transaction>>>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            mempool: Arc::new(RwLock::new(Vec::<Transaction>::new())),
        }
    }

    pub async fn pending_size(&self) -> usize {
        self.mempool.read().await.len()
    }

    pub async fn add_tnx(&self, txn: Transaction) -> Result<()> {
        let mut pending_txns = self.mempool.write().await;
        if pending_txns.len() > MAX_MEMPOOL_SIZE {
            return Err(anyhow!("mempool is full"));
        }
        pending_txns.push(txn);
        Ok(())
    }

    pub async fn execute(&self) -> Result<Block> {
        let pending_txns = self.mempool.write().await;
        let mut transfers = Vec::new();
        for txn in pending_txns.iter() {
            transfers.push(self.pre_process(txn).unwrap().unwrap());
        }
        let mut state_db = STATE.write().await;
        let balances = &mut state_db.state.balances;
        let _ = transfer(balances, transfers);

        let block = Block::new(pending_txns.to_vec());
        Ok(block)
    }

    fn pre_process(&self, txn: &Transaction) -> Result<Option<TransferOp>> {
        let signature = txn.signatures[0].to_string();
        info!("Processing txn: {}", signature);

        // Parsing each instruction in the transaction
        for (i, instruction) in txn.message.instructions.iter().enumerate() {
            info!("Processing instruction {}", i);
            parsing_instruction(instruction, &txn)?;
        }

        info!("Transaction processing completed: {}", signature);
        Ok(None)
    }
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
    pub static ref STATE: Arc<RwLock<StateDB>> = Arc::new(RwLock::new(StateDB::new("state_db")));
}
