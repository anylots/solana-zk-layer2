use anyhow::Result;
use share::transaction::{calculate_txns_root, Block, BlockDB};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;

use crate::batcher::tx_batcher::TxBatcher;
use crate::executor::{Executor, STATE};

static BLOCK_TIME_INTERVAL: Duration = Duration::from_millis(200);

// For generate block and execute txn.
pub struct Node {
    pub executor: Executor,
    pub batcher: Arc<TxBatcher>,
    pub latest_block_num: u64,
    pub latest_state_root: [u8; 32],
    pub last_block_time: Arc<RwLock<Instant>>,
}

impl Node {
    pub async fn new() -> Result<Self> {
        let block_db = BLOCK_DB.read().await;
        let executor = Executor::new();
        let batcher = TxBatcher::new()?;

        // Initialize block number from database or start from 0
        let latest_block_num = match block_db.db.get("latest_block_num")? {
            Some(bytes) => {
                let num_bytes: [u8; 8] = bytes
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid block number format"))?;
                u64::from_be_bytes(num_bytes)
            }
            None => 0,
        };

        let latest_state_root = match block_db.db.get("latest_state_root")? {
            Some(bytes) => {
                let num_bytes: [u8; 32] = bytes
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid latest state root format"))?;
                num_bytes
            }
            None => Default::default(),
        };

        Ok(Self {
            executor,
            batcher: Arc::new(batcher),
            latest_block_num,
            latest_state_root,
            last_block_time: Arc::new(RwLock::new(Instant::now())),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        // Step1. Start batcher
        let batcher = self.batcher.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(60)).await;
                if let Err(e) = batcher.smart_submit().await {
                    log::info!("Batcher error: {:?}", e);
                };
            }
        });

        // Step2. Start building block
        loop {
            let should_generate_block = {
                let last_time = *self.last_block_time.read().await;
                let time_elapsed = last_time.elapsed() >= BLOCK_TIME_INTERVAL;
                (time_elapsed && self.executor.pending_size().await > 0)
                    || last_time.elapsed() > 10 * BLOCK_TIME_INTERVAL
            };

            if should_generate_block {
                // Generate and save block
                let mut block = self.create_block().await;
                let _ = self.save_block(&mut block).await;

                log::info!(
                    "Generated block #{} with {} transactions",
                    block.block_num,
                    block.txns.len()
                );

                *self.last_block_time.write().await = Instant::now();
            }

            // Sleep for a short interval before checking again
            sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn create_block(&mut self) -> Block {
        let mut block = self.executor.execute().await.unwrap();
        block.prev_state_root = Some(self.latest_state_root);
        self.latest_block_num += 1;
        block.block_num = self.latest_block_num;

        block
    }

    /// Save block to local storage
    async fn save_block(&self, block: &mut Block) -> Result<()> {
        let state_db = STATE.read().await;
        let state = &state_db.state;
        let state_root = state.calculate_state_root().unwrap_or_default();
        block.post_state_root = Some(state_root);
        let withdrawal_root = state.calculate_withdrawal_root().unwrap_or_default();
        block.withdrawal_root = Some(withdrawal_root);

        let txns_root = calculate_txns_root(&block.txns);
        block.txns_root = Some(txns_root);

        // Serialize block
        let block_data = serde_json::to_vec(block)
            .map_err(|e| anyhow::anyhow!("Failed to serialize block: {}", e))?;

        let mut block_db = BLOCK_DB.write().await;

        // Save cache
        if block_db.cache.len() == 128 {
            block_db.cache.pop_front();
        }
        block_db.cache.push_back(block.clone());

        // Save block
        let block_key = format!("block_{}", block.block_num);
        block_db.db.insert(block_key.as_bytes(), block_data)?;

        // Update latest_block_num & latest_state_root
        let block_num_bytes = block.block_num.to_be_bytes();
        block_db
            .db
            .insert("latest_block_num", &block_num_bytes[..])?;
        block_db
            .db
            .insert("latest_state_root", &block.post_state_root.unwrap())?;

        // Save balance state
        state_db.save();

        // Flush to ensure data is persisted
        block_db.db.flush()?;

        Ok(())
    }
}

// Global block db instance
lazy_static::lazy_static! {
    pub static ref BLOCK_DB: Arc<RwLock<BlockDB>> = Arc::new(RwLock::new(BlockDB::new("block_db")));
}
