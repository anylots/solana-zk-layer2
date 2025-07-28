use share::state::StateDB;
use share::load_blocks;

mod gen_proof;
fn main() {
    let mut state_db = StateDB::new("state_db");
    state_db.load();
    let state = state_db.state;
    let blocks = load_blocks(101, 10).unwrap();

    let _ = gen_proof::prove(state, blocks);
}


pub fn load_blocks(start: u64, length: u64) -> Option<Vec<Block>> {
    let db = sled::open("block_db").ok()?;
    let mut blocks = vec![];
    for i in start..start + length {
        if let Ok(Some(data)) = db.get(format!("block_{}", i)) {
            if let Ok(block) = serde_json::from_slice::<Block>(&data) {
                blocks.push(block);
            }
        } else {
            return None;
        }
    }
    Some(blocks)
}
