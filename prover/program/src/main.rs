#![no_main]
sp1_zkvm::entrypoint!(main);
use share::{transaction::calculate_txns_root, zkvm::ZkVMInput};
use tiny_keccak::{Hasher, Sha3};

pub fn main() {
    // Read the input.
    let x = sp1_zkvm::io::read::<ZkVMInput>();
    let mut state = x.state;
    let blocks = x.blocks;
    let prev_state_root = blocks.first().unwrap().prev_state_root.unwrap_or_default();
    let post_state_root = blocks.last().unwrap().post_state_root.unwrap_or_default();

    let mut txns_roots: Vec<[u8; 32]> = vec![];
    let mut current_state_root = prev_state_root;
    for block in blocks {
        assert!(
            current_state_root == block.prev_state_root.unwrap_or_default(),
            "blocks[n-1].post_state_root == blocks[n].prev_state_root"
        );
        current_state_root = block.post_state_root.unwrap_or_default();

        // Calculate txns root
        let txns_root = calculate_txns_root(&block.txns);
        assert!(
            txns_root == block.txns_root.unwrap_or_default(),
            "txns_root == block.txns_root"
        );
        txns_roots.push(txns_root);

        for _txn in block.txns {
            state.add_balance("user1".to_owned(), 100);
        }
        // Calculate current block state root
        let state_root = state.calculate_state_root().unwrap_or_default();
        assert!(
            state_root == block.post_state_root.unwrap_or_default(),
            "block_post_state_root == block.state_root"
        );
    }

    // Replace versioned_hash with all txn hashes
    let da_hash = calculate_da_hash(&txns_roots);

    // calculate pi hash
    let pi_hash = calculate_pi_hash(&prev_state_root, &post_state_root, &da_hash);

    // Commit public input.
    sp1_zkvm::io::commit(&pi_hash);
}

// Helper function to calculate hash with all blocks' txns for DA.
fn calculate_da_hash(txns_roots: &Vec<[u8; 32]>) -> [u8; 32] {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    for txns_root in txns_roots {
        sha3.update(txns_root);
    }

    sha3.finalize(&mut output);
    output
}

// Helper function to calculate public input for zk proof.
fn calculate_pi_hash(
    prev_state_root: &[u8; 32],
    post_state_root: &[u8; 32],
    da_hash: &[u8; 32],
) -> [u8; 32] {
    let mut sha3 = Sha3::v256();
    let mut output = [0u8; 32];

    sha3.update(prev_state_root);
    sha3.update(post_state_root);
    sha3.update(da_hash);

    sha3.finalize(&mut output);
    output
}
