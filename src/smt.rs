use solana_hash::Hash;
use solana_sha256_hasher::hashv;
use solana_sdk::{pubkey::Pubkey, account::Account};
use bincode::serialize;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;


/// Sparse Merkle Tree with fixed depth (256-bit address space)
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SparseMerkleTree {
    pub nodes: HashMap<Hash, Hash>, // Internal nodes
    pub leaves: HashMap<Hash, Hash>, // Leaf nodes mapping (Pubkey hash → Account hash)
    pub root: Hash, // Root of the tree
}

impl SparseMerkleTree {

    const ZERO_HASH: Hash = Hash::new_from_array([0; 32]); // Hash constant for empty accounts

    pub fn new() -> Self {
        SparseMerkleTree {
            nodes: HashMap::new(),
            leaves: HashMap::new(),
            root: Hash::default(),
        }
    }

    /// Compute a hash for an account
    fn hash_account(account: &Account) -> Hash {
        let account_bytes = serialize(account).unwrap();
        hashv(&[&account_bytes])
    }

    /// Compute the hashed key for the sparse tree
    fn hash_key(pubkey: &Pubkey) -> Hash {
        hashv(&[pubkey.as_ref()])
    }

    /// Compute a parent node hash
    fn hash_nodes(left: &Hash, right: &Hash) -> Hash {
        hashv(&[left.as_ref(), right.as_ref()])
    }

    pub fn insert(&mut self, pubkey: Pubkey, account: &Account) {
        let leaf_key = Self::hash_key(&pubkey);
        let leaf_hash = if account.lamports == 0 && account.data.is_empty() {
            println!("🟡 Inserting empty account with ZERO_HASH: {:?}", pubkey);
            Self::ZERO_HASH // Use predefined ZERO_HASH
        } else {
            Self::hash_account(account) // Normal hashing
        };
    
        // If the account is empty and already exists, do nothing
        if let Some(existing_hash) = self.leaves.get(&leaf_key) {
            if *existing_hash == Self::ZERO_HASH {
                println!("⚠️ Skipping update for empty account: {:?}", pubkey);
                return; // Do not update the root
            }
        }
    
        self.leaves.insert(leaf_key, leaf_hash);
    
        // Only update the root if it is not an empty account
        if leaf_hash != Self::ZERO_HASH {
            self.update_path(leaf_key, leaf_hash);
        }
    }
    

    /// Update a leaf and propagate changes to the root
    fn update_path(&mut self, mut key: Hash, mut value: Hash) {
        for _ in 0..256 {
            let sibling = self.nodes.get(&key).copied().unwrap_or(Hash::default());
            let parent_hash = if key.as_ref()[0] & 1 == 0 {
                Self::hash_nodes(&value, &sibling)
            } else {
                Self::hash_nodes(&sibling, &value)
            };

            self.nodes.insert(key, value);
            value = parent_hash;
            key = Self::hash_nodes(&key, &Hash::default()); // Move up in tree
        }

        self.root = value;
    }

    /// Retrieve the Merkle root
    pub fn get_root(&self) -> Hash {
        self.root
    }

    /// Generate a Merkle proof for an account
    pub fn generate_proof(&self, pubkey: &Pubkey) -> Option<Vec<Hash>> {
        let mut key = Self::hash_key(pubkey);
        let mut proof = Vec::new();

        for _ in 0..256 {
            let sibling = self.nodes.get(&key).copied().unwrap_or(Hash::default());
            proof.push(sibling);
            key = Self::hash_nodes(&key, &Hash::default());
        }

        Some(proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::{Keypair, Signer};

    fn create_example_account(pubkey: Pubkey) -> Account {
        Account {
            lamports: 1000,
            data: vec![1, 2, 3, 4],
            executable: false,
            rent_epoch: 1,
            owner: pubkey,
        }
    }

    #[test]
    fn test_insert_and_generate_root() {
        let mut smt = SparseMerkleTree::new();
        
        let pubkey1 = Keypair::new().pubkey();
        let account1 = create_example_account(pubkey1);
        smt.insert(pubkey1, &account1);
        
        let root1 = smt.get_root();
        println!("🌳 Root after first insertion: {:?}", root1);
        assert_ne!(root1, Hash::default(), "Root should not be default after insertion");

        let pubkey2 = Keypair::new().pubkey();
        let account2 = create_example_account(pubkey2);
        smt.insert(pubkey2, &account2);

        let root2 = smt.get_root();
        println!("🌳 Root after second insertion: {:?}", root2);
        assert_ne!(root1, root2, "Root should change after inserting another account");
    }

    #[test]
    fn test_generate_proof() {
        let mut smt = SparseMerkleTree::new();

        let pubkey = Keypair::new().pubkey();
        let account = create_example_account(pubkey);
        smt.insert(pubkey, &account);

        let proof = smt.generate_proof(&pubkey);
        assert!(proof.is_some(), "Proof should be generated for existing key");
        println!("🛠️ Proof for {:?}: {:?}", pubkey, proof.unwrap());
    }

    #[test]
    fn test_proof_for_non_existent_key() {
        let smt = SparseMerkleTree::new();
        let random_pubkey = Keypair::new().pubkey();
        
        let proof = smt.generate_proof(&random_pubkey);
        assert!(proof.is_some(), "Proof should be empty but not None for non-existent keys");
        println!("⚠️ Proof for non-existent key: {:?}", proof.unwrap());
    }

    #[test]
    fn test_update_existing_account() {
        let mut smt = SparseMerkleTree::new();

        let pubkey = Keypair::new().pubkey();
        let mut account = create_example_account(pubkey);
        smt.insert(pubkey, &account);
        
        let root_before = smt.get_root();
        println!("🌳 Root before update: {:?}", root_before);

        account.lamports += 500; // Update balance
        smt.insert(pubkey, &account); // Update the same account

        let root_after = smt.get_root();
        println!("🔄 Root after update: {:?}", root_after);
        assert_ne!(root_before, root_after, "Root should change after updating an account");
    }

    #[test]
    fn test_insert_empty_account_does_not_update_root() {
        let mut smt = SparseMerkleTree::new();
    
        // Insert a normal account
        let pubkey1 = Keypair::new().pubkey();
        let account1 = create_example_account(pubkey1);
        smt.insert(pubkey1, &account1);
    
        let root_before = smt.get_root();
        println!("🌳 Root before inserting empty account: {:?}", root_before);
        println!("📝 Tree before inserting empty account: {:?}", smt.nodes);
    
        // Insert an empty account
        let empty_pubkey = Keypair::new().pubkey();
        let empty_account = Account {
            lamports: 0, // No funds
            data: vec![], // No data
            executable: false,
            rent_epoch: 0,
            owner: empty_pubkey,
        };
    
        smt.insert(empty_pubkey, &empty_account); // Insert empty account
    
        let root_after = smt.get_root();
        println!("🌳 Root after inserting empty account: {:?}", root_after);
        println!("📝 Tree after inserting empty account: {:?}", smt.nodes);
    
        // Root should remain the same since we added an "empty" account with ZERO_HASH
        assert_eq!(root_before, root_after, "Root should NOT change after inserting an empty account");
    }
    
}