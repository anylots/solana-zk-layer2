use std::vec;

use crate::node::BLOCK_DB;
use anyhow::Result;
use l2_state_client::state_call::{BatchInfo, L2StateClient};
use log::info;
use share::transaction::{Block, BlockDB};

static MAX_BLOCK_COUNT_IN_BATCH: u64 = 256;

pub struct TxBatcher {
    l2_state_client: L2StateClient,
}

impl TxBatcher {
    pub fn new() -> Result<Self> {
        let l2_state_client = L2StateClient::new_local()?;
        Ok(Self { l2_state_client })
    }

    pub fn _new_with_config(rpc_url: String, fee_payer_bytes: &[u8]) -> Result<Self> {
        let l2_state_client = L2StateClient::new(rpc_url, fee_payer_bytes)?;
        Ok(Self { l2_state_client })
    }

    pub async fn smart_submit(&self) -> Result<()> {
        let latest_batch = self.l2_state_client.get_latest_batch()?;
        let mut next_batch = if let Some(batch) = latest_batch {
            BatchInfo {
                batch_index: batch.batch_index + 1,
                blocks: vec![],
                start_block_num: batch.end_block_num + 1,
                end_block_num: 0,
                prev_state_root: batch.post_state_root,
                post_state_root: [0u8; 32],
                withdrawal_root: [0u8; 32],
            }
        } else {
            BatchInfo {
                batch_index: 1,
                blocks: vec![],
                start_block_num: 1,
                end_block_num: 0,
                prev_state_root: [0u8; 32],
                post_state_root: [0u8; 32],
                withdrawal_root: [0u8; 32],
            }
        };

        let block_db = BLOCK_DB.read().await;

        // Get latest block number from database
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

        if latest_block_num == 0 {
            info!("No blocks to submit");
            return Ok(());
        }

        // Determine which blocks to include in the next batch
        let blocks_to_submit = self
            .collect_blocks_for_batch(&block_db, next_batch.start_block_num, latest_block_num)
            .await?;

        if blocks_to_submit.is_empty() {
            info!("No new blocks to submit");
            return Ok(());
        }

        next_batch.post_state_root = blocks_to_submit
            .last()
            .unwrap()
            .post_state_root
            .unwrap_or_default();
        next_batch.withdrawal_root = blocks_to_submit
            .last()
            .unwrap()
            .withdrawal_root
            .unwrap_or_default();

        // Serialize block
        let block_data = blocks_to_submit
            .iter()
            .map(|block| serde_json::to_vec(block).unwrap_or_default())
            .collect();

        next_batch.blocks = block_data;

        // Submit the batch
        info!("Committing batch {} to Solana", next_batch.batch_index);
        self.l2_state_client.commit_batch(next_batch)?;

        Ok(())
    }

    /// Collect blocks from the database for batching
    async fn collect_blocks_for_batch(
        &self,
        block_db: &BlockDB,
        start_block_num: u64,
        latest_block_num_local: u64,
    ) -> Result<Vec<Block>> {
        let blocks_count = if latest_block_num_local - start_block_num > MAX_BLOCK_COUNT_IN_BATCH {
            MAX_BLOCK_COUNT_IN_BATCH
        } else {
            latest_block_num_local - start_block_num
        };

        let mut blocks = vec![];
        for i in start_block_num..start_block_num + blocks_count {
            if let Ok(Some(data)) = block_db.db.get(format!("block_{}", i)) {
                if let Ok(block) = serde_json::from_slice::<Block>(&data) {
                    blocks.push(block);
                }
            } else {
                return Ok(vec![]);
            }
        }

        Ok(blocks)
    }
}
