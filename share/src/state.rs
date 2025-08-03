use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub balances: HashMap<String, u128>, // address -> balance
    pub withdrawal_queue: Vec<Withdrawal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Withdrawal {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub index: u64,
}

impl State {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            withdrawal_queue: Vec::new(),
        }
    }
    pub fn get_balance(&self, address: &str) -> u128 {
        *self.balances.get(address).unwrap_or(&0)
    }

    pub fn set_balance(&mut self, address: String, balance: u128) {
        self.balances.insert(address, balance);
    }

    pub fn add_balance(&mut self, address: String, amount: u128) {
        let current_balance = self.get_balance(&address);
        let new_balance = current_balance.saturating_add(amount);
        self.set_balance(address, new_balance);
    }

    pub fn sub_balance(&mut self, address: String, amount: u128) -> bool {
        let current_balance = self.get_balance(&address);
        if current_balance >= amount {
            let new_balance = current_balance - amount;
            self.set_balance(address, new_balance);
            true
        } else {
            false
        }
    }
}

pub struct StateDB {
    pub db: sled::Db,
    pub cache: HashMap<String, u128>,
    pub state: State,
}

impl StateDB {
    pub fn new(db_path: &str) -> Self {
        let db = sled::open(db_path).unwrap();
        StateDB {
            db,
            cache: HashMap::new(),
            state: State::new(),
        }
    }

    pub fn save(&self) {
        let balances = serde_json::to_vec(&self.state.balances).unwrap();
        self.db.insert("balance_state", balances).unwrap();

        let withdrawal = bincode::serialize(&self.state.withdrawal_queue).unwrap();
        self.db.insert("withdrawal_queue", withdrawal).unwrap();
    }

    pub fn load(&mut self) {
        if let Ok(Some(data)) = self.db.get("balance_state") {
            if let Ok(user_balances) = serde_json::from_slice::<HashMap<String, u128>>(&data) {
                self.state.balances = user_balances;
            }
        }
        if let Ok(Some(data)) = self.db.get("withdrawal_queue") {
            if let Ok(withdrawal_queue) = bincode::deserialize(&data) {
                self.state.withdrawal_queue = withdrawal_queue;
            }
        }
    }
}

// Calculate hash for a account's state
fn calculate_user_hash(address: &str, balance: &u128) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // Hash user address
    hasher.update(address.as_bytes());
    // Hash user balance
    hasher.update(&balance.to_be_bytes());

    hasher.finalize().into()
}

// Calculate hash for a withdrawal
fn calculate_withdrawal_hash(withdrawal: &Withdrawal) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // Hash withdrawal fields
    hasher.update(withdrawal.from.as_bytes());
    hasher.update(withdrawal.to.as_bytes());
    hasher.update(&withdrawal.amount.to_be_bytes());
    hasher.update(&withdrawal.index.to_be_bytes());

    hasher.finalize().into()
}

impl State {
    //  State root of binary tree
    pub fn calculate_state_root(&self) -> Option<[u8; 32]> {
        if self.balances.is_empty() {
            return None;
        }

        // Calculate hash for each user's state
        let mut leaf_hashes: Vec<[u8; 32]> = self
            .balances
            .iter()
            .map(|(user_id, value)| calculate_user_hash(user_id, &value))
            .collect();

        // Sorting
        if leaf_hashes.len() % 2 == 1 {
            leaf_hashes.push(leaf_hashes[leaf_hashes.len() - 1]);
        }

        // Build binary tree bottom-up
        let mut nodes: Vec<MerkleNode> =
            leaf_hashes.into_iter().map(MerkleNode::new_leaf).collect();

        // Build tree level by level until we have one root node
        while nodes.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..nodes.len()).step_by(2) {
                let left = nodes[i].clone();
                let right = if i + 1 < nodes.len() {
                    nodes[i + 1].clone()
                } else {
                    left.clone()
                };

                next_level.push(MerkleNode::new_internal(left, right));
            }

            nodes = next_level;
        }

        if let Some(root) = nodes.into_iter().next() {
            Some(root.hash)
        } else {
            None
        }
    }

    pub fn calculate_withdrawal_root(&self) -> Option<[u8; 32]> {
        if self.withdrawal_queue.is_empty() {
            return None;
        }

        // Calculate hash for each withdrawal
        let mut leaf_hashes: Vec<[u8; 32]> = self
            .withdrawal_queue
            .iter()
            .map(|withdrawal| calculate_withdrawal_hash(withdrawal))
            .collect();

        if leaf_hashes.len() % 2 == 1 {
            leaf_hashes.push([0u8; 32]);
        }

        // Build binary tree bottom-up
        let mut nodes: Vec<MerkleNode> =
            leaf_hashes.into_iter().map(MerkleNode::new_leaf).collect();

        // Build tree level by level until we have one root node
        while nodes.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..nodes.len()).step_by(2) {
                let left = nodes[i].clone();
                let right = if i + 1 < nodes.len() {
                    nodes[i + 1].clone()
                } else {
                    left.clone()
                };

                next_level.push(MerkleNode::new_internal(left, right));
            }

            nodes = next_level;
        }

        if let Some(root) = nodes.into_iter().next() {
            Some(root.hash)
        } else {
            None
        }
    }

    pub fn generate_withdrawal_merkle_proof(
        &self,
        index: u64,
        range: u64, // Batch boundary
    ) -> Option<(
        [u8; 32],      // leaf_hash
        Vec<[u8; 32]>, // proof
        u64,           // index
        [u8; 32],      // root
    )> {
        if self.withdrawal_queue.is_empty() || index as usize >= self.withdrawal_queue.len() {
            return None;
        }
        let history_queue = &self.withdrawal_queue[0..range as usize];

        // Calculate hash for each withdrawal
        let mut leaf_hashes: Vec<[u8; 32]> = history_queue
            .iter()
            .map(|withdrawal| calculate_withdrawal_hash(withdrawal))
            .collect();

        let leaf_hash = leaf_hashes[index as usize];

        // If odd number of leaves, duplicate the last one
        if leaf_hashes.len() % 2 == 1 {
            leaf_hashes.push([0u8; 32]);
        }

        // Build binary tree bottom-up and collect proof
        let mut nodes: Vec<MerkleNode> =
            leaf_hashes.into_iter().map(MerkleNode::new_leaf).collect();

        let mut proof = Vec::new();
        let mut current_index = index;

        // Build tree level by level and collect sibling hashes for the proof
        while nodes.len() > 1 {
            let mut next_level = Vec::new();

            // Find sibling for current index
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            // Add sibling hash to proof if it exists
            if (sibling_index as usize) < nodes.len() {
                proof.push(nodes[sibling_index as usize].hash);
            } else {
                // If no sibling, use the same node (for odd number of nodes)
                proof.push(nodes[current_index as usize].hash);
            }

            // Build next level
            for i in (0..nodes.len()).step_by(2) {
                let left = nodes[i].clone();
                let right = if i + 1 < nodes.len() {
                    nodes[i + 1].clone()
                } else {
                    left.clone()
                };

                next_level.push(MerkleNode::new_internal(left, right));
            }

            nodes = next_level;
            current_index = current_index / 2;
        }

        if let Some(root) = nodes.into_iter().next() {
            Some((leaf_hash, proof, index, root.hash))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
}

impl MerkleNode {
    fn new_leaf(hash: [u8; 32]) -> Self {
        MerkleNode {
            hash,
            left: None,
            right: None,
        }
    }

    fn new_internal(left: MerkleNode, right: MerkleNode) -> Self {
        let mut hasher = Sha256::new();

        hasher.update(&left.hash);
        hasher.update(&right.hash);

        MerkleNode {
            hash: hasher.finalize().into(),
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}
