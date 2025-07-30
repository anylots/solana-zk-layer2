use std::time::Duration;

use anyhow::Result;
use l2_state_client::{BatchProof, L2StateClient};
use share::state::StateDB;
use share::transaction::load_blocks;
use tokio::time::sleep;

mod gen_proof;

#[tokio::main]
async fn main() -> Result<()> {
    let mut state_db = StateDB::new("state_db");
    state_db.load();
    let state = state_db.state;
    let l2_state_client = L2StateClient::new_local()?;

    loop {
        sleep(Duration::from_secs(300)).await;
        let last_finalized_index = l2_state_client.get_last_finalized_batch_index()?;
        let next_batch_index = last_finalized_index + 1;

        let Some(batch) = l2_state_client.get_committed_batch(next_batch_index)? else {
            continue;
        };

        let block_count = batch.end_block_num - batch.start_block_num + 1;
        let blocks = load_blocks(batch.start_block_num, block_count).unwrap_or_default();
        if blocks.is_empty() {
            continue;
        }
        let proof = gen_proof::prove(state.clone(), blocks)?;
        let Some(proof) = proof else {
            continue;
        };

        let batch_proof = BatchProof {
            batch_index: next_batch_index,
            proof,
        };

        let _ = l2_state_client.prove_batch(batch_proof);
    }
}
