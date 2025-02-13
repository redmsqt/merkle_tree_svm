use solana_hash::Hash;
use solana_sha256_hasher::hashv;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use bincode::serialize;
use serde::{Serialize, Deserialize};

const TREE_DEPTH: usize = 256;

// Structure to represent a transaction
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionData {
    pub sender: Pubkey,
    pub receiver: Pubkey,
    pub amount: u64,
}

#[derive(Clone, Debug)]
pub struct SparseMerkleTree {
    tree: HashMap<u64, Hash>, // Stores tree nodes
    root: Hash, // Tree root
    next_index: u64, // Position for the next transaction
}

impl SparseMerkleTree {
    pub fn new() -> Self {
        SparseMerkleTree {
            tree: HashMap::new(),
            root: Hash::default(),
            next_index: 0,
        }
    }

    // Hash child nodes
    pub fn hash_nodes(left: &Hash, right: &Hash) -> Hash {
        hashv(&[left.as_ref(), right.as_ref()])
    }

    // Insert a transaction into the tree
    pub fn insert(&mut self, transaction_data: &TransactionData) {
        let transaction_bytes = serialize(transaction_data).unwrap();
        let transaction_hash = hashv(&[&transaction_bytes]);

        let mut current_hash = transaction_hash;
        let mut index = self.next_index;
        self.next_index += 1;

        // Update Merkle tree nodes
        for _level in 0..TREE_DEPTH {
            let sibling_index = index ^ 1; // Find the sibling (even/odd index)
            let sibling_hash = self.tree.get(&sibling_index).cloned().unwrap_or(Hash::default());

            if index % 2 == 0 {
                current_hash = Self::hash_nodes(&current_hash, &sibling_hash);
            } else {
                current_hash = Self::hash_nodes(&sibling_hash, &current_hash);
            }

            self.tree.insert(index, transaction_hash);
            index /= 2; // Move up the tree
        }

        self.root = current_hash; // Update the root
    }

    // Verify the root
    pub fn verify_root(&self, root: &Hash) -> bool {
        &self.root == root
    }

    // Get the current root
    pub fn get_root(&self) -> Hash {
        self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    // Utility function to generate a random Pubkey
    fn generate_random_pubkey() -> Pubkey {
        let mut rng = rand::thread_rng();
        let mut random_bytes = [0u8; 32];
        rng.fill(&mut random_bytes);
        Pubkey::new_from_array(random_bytes)
    }

    // Utility function to generate a random amount
    fn generate_random_amount() -> u64 {
        rand::thread_rng().gen_range(1..10000)
    }

    #[test]
    fn test_transaction_insertion() {
        let mut smt = SparseMerkleTree::new();

        let transaction_data = TransactionData {
            sender: generate_random_pubkey(),
            receiver: generate_random_pubkey(),
            amount: generate_random_amount(),
        };

        smt.insert(&transaction_data);
        
        let root = smt.get_root();
        println!("Root after first insertion: {:?}", root);
        assert!(smt.verify_root(&root), "The Merkle tree root is incorrect after insertion.");
    }

    #[test]
    fn test_multiple_transactions() {
        let mut smt = SparseMerkleTree::new();

        let transaction_data1 = TransactionData {
            sender: generate_random_pubkey(),
            receiver: generate_random_pubkey(),
            amount: generate_random_amount(),
        };

        let transaction_data2 = TransactionData {
            sender: generate_random_pubkey(),
            receiver: generate_random_pubkey(),
            amount: generate_random_amount(),
        };

        smt.insert(&transaction_data1);
        let root1 = smt.get_root();
        
        smt.insert(&transaction_data2);
        let root2 = smt.get_root();

        println!("Root after first insertion: {:?}", root1);
        println!("Root after second insertion: {:?}", root2);

        assert_ne!(root1, root2, "Roots should not be identical after multiple insertions.");
        assert!(smt.verify_root(&root2), "The root after multiple insertions is incorrect.");
    }
}
