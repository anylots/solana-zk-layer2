use anchor_lang::solana_program::hash::hash;

/*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
/*                            UTIL                            */
/*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/

pub fn hash_nested_vector(data: &Vec<Vec<u8>>) -> [u8; 32] {
    if data.is_empty() {
        return [0u8; 32];
    }
    let concatenated_data = data.concat();
    hash(&concatenated_data).to_bytes()
}

pub fn verify_merkle_proof(
    leaf_hash: [u8; 32],
    proof: Vec<[u8; 32]>,
    index: u64,
    root: [u8; 32],
) -> bool {
    let mut node = leaf_hash;

    for (height, sibling) in proof.iter().enumerate() {
        if (index >> height) & 1 == 1 {
            let mut combined = Vec::with_capacity(64);
            combined.extend_from_slice(sibling);
            combined.extend_from_slice(&node);
            node = hash(&combined).to_bytes();
        } else {
            let mut combined = Vec::with_capacity(64);
            combined.extend_from_slice(&node);
            combined.extend_from_slice(sibling);
            node = hash(&combined).to_bytes();
        }
    }

    node == root
}
