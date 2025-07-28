use anyhow::{anyhow, Result};
use log::info;
use share::{state::StateDB, transaction::Block};
use solana_sdk::transaction::Transaction;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

static MAX_MEMPOOL_SIZE: usize = 1024;

pub struct Executor {
    pub mempool: Arc<RwLock<Vec<Transaction>>>,
}

struct TransferOp {
    from: String,
    to: String,
    amount: u128,
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
            self.parsing_instruction(instruction, &txn)?;
        }

        info!("Transaction processing completed: {}", signature);
        Ok(None)
    }

    fn parsing_instruction(
        &self,
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
            self.parsing_sys_instruction(instruction, txn)?;
        } else {
            // for other instructions, only basic logging is done
            info!("Processing instruction for program: {}", program_id);
        }

        Ok(None)
    }

    fn parsing_sys_instruction(
        &self,
        instruction: &solana_sdk::instruction::CompiledInstruction,
        txn: &Transaction,
    ) -> Result<Option<TransferOp>> {
        if instruction.data.is_empty() {
            return Ok(None);
        }

        // transfer Instructions
        if instruction.accounts.len() >= 2 {
            let from_index = instruction.accounts[0] as usize;
            let to_index = instruction.accounts[1] as usize;

            if from_index < txn.message.account_keys.len()
                && to_index < txn.message.account_keys.len()
            {
                let from_pubkey = txn.message.account_keys[from_index].to_string();
                let to_pubkey = txn.message.account_keys[to_index].to_string();

                // generate transfer operation
                return Ok(Some(TransferOp {
                    from: from_pubkey,
                    to: to_pubkey,
                    amount: 1000000,
                }));
            }
        }
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
