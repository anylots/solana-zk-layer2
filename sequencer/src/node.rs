use anyhow::Result;
use share::transaction::{calculate_txns_root, Block};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;

use crate::executor::{Executor, STATE};

static BLOCK_TIME_INTERVAL: Duration = Duration::from_millis(200);

// For generate block and execute txn.
pub struct Node {
    pub block_db: sled::Db,
    pub executor: Executor,
    pub latest_block_num: u64,
    pub latest_state_root: [u8; 32],
    pub last_block_time: Arc<RwLock<Instant>>,
}

impl Node {
    pub fn new(db_path: Option<String>) -> Result<Self> {
        let block_db = sled::open(db_path.unwrap_or("block_db".to_owned()))?;
        let executor = Executor::new();

        // Initialize block number from database or start from 0
        let latest_block_num = match block_db.get("latest_block_num")? {
            Some(bytes) => {
                let num_bytes: [u8; 8] = bytes
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid block number format"))?;
                u64::from_be_bytes(num_bytes)
            }
            None => 0,
        };

        let latest_state_root = match block_db.get("latest_state_root")? {
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
            block_db,
            executor,
            latest_block_num,
            latest_state_root,
            last_block_time: Arc::new(RwLock::new(Instant::now())),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
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

        let txns_root = calculate_txns_root(&block.txns);
        block.txns_root = Some(txns_root);

        // Serialize block
        let block_data = serde_json::to_vec(block)
            .map_err(|e| anyhow::anyhow!("Failed to serialize block: {}", e))?;

        // Save block
        let block_key = format!("block_{}", block.block_num);
        self.block_db.insert(block_key.as_bytes(), block_data)?;

        // Update latest_block_num & latest_state_root
        let block_num_bytes = block.block_num.to_be_bytes();
        self.block_db
            .insert("latest_block_num", &block_num_bytes[..])?;
        self.block_db
            .insert("latest_state_root", &block.post_state_root.unwrap())?;

        // Save balance state
        state_db.save();

        // Flush to ensure data is persisted
        self.block_db.flush()?;

        Ok(())
    }
}
