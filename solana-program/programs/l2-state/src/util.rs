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
