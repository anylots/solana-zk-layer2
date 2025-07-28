use serde::{Deserialize, Serialize};

use crate::{state::State, transaction::Block};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkVMInput {
    pub blocks: Vec<Block>,
    pub state: State,
}
