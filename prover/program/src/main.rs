#![no_main]
sp1_zkvm::entrypoint!(main);
use sha2::{Digest, Sha256};
use share::{
    transaction::{calculate_txns_root, parsing_instruction},
    zkvm::ZkVMInput,
};

pub fn main() {
    // Read the input.
    let x = sp1_zkvm::io::read::<ZkVMInput>();
    let mut state = x.state;
    let blocks = x.blocks;
    let prev_state_root = blocks.first().unwrap().prev_state_root.unwrap_or_default();
    let post_state_root = blocks.last().unwrap().post_state_root.unwrap_or_default();

    let mut blocks_bytes: Vec<u8> = vec![];
    let mut current_state_root = prev_state_root;
    for block in blocks {
        assert!(
            current_state_root == block.prev_state_root.unwrap_or_default(),
            "blocks[n-1].post_state_root == blocks[n].prev_state_root"
        );
        current_state_root = block.post_state_root.unwrap_or_default();
        blocks_bytes.extend_from_slice(&serde_json::to_vec(&block).unwrap_or_default());

        // Calculate txns root
        let txns_root = calculate_txns_root(&block.txns);
        assert!(
            txns_root == block.txns_root.unwrap_or_default(),
            "txns_root == block.txns_root"
        );

        for txn in block.txns {
            for (_, instruction) in txn.message.instructions.iter().enumerate() {
                let op = parsing_instruction(&instruction, &txn)
                    .unwrap()
                    .expect("valid instruction");
                let from = op.from;
                let to = op.to;
                let amount = op.amount;
                // check sender's amount
                let from_balance = state.get_balance(&from);
                assert!(from_balance >= amount, "Insufficient balance for transfer");
                // change the balance
                state.sub_balance(from, amount);
                state.add_balance(to, amount);
            }
        }
        // Calculate current block state root
        let state_root = state.calculate_state_root().unwrap_or_default();
        assert!(
            state_root == block.post_state_root.unwrap_or_default(),
            "block_post_state_root == block.state_root"
        );
    }

    // Replace versioned_hash with all txn hashes
    let da_hash = calculate_da_hash(&blocks_bytes);

    // calculate pi hash
    let pi_hash = calculate_pi_hash(&prev_state_root, &post_state_root, &da_hash);

    // Commit public input.
    sp1_zkvm::io::commit(&pi_hash);
}

// Helper function to calculate hash with all blocks' txns for DA.
fn calculate_da_hash(blocks_bytes: &Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(blocks_bytes);
    hasher.finalize().into()
}

// Helper function to calculate public input for zk proof.
fn calculate_pi_hash(
    prev_state_root: &[u8; 32],
    post_state_root: &[u8; 32],
    da_hash: &[u8; 32],
) -> [u8; 32] {
    let mut hasher = Sha256::new();

    hasher.update(prev_state_root);
    hasher.update(post_state_root);
    hasher.update(da_hash);

    hasher.finalize().into()
}
