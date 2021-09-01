use crate::{
    error::Error,
    tree::{BranchKey, BranchNode},
    H256,
};

/// Trait for customize hash function
pub trait Hasher {
    fn write_h256(&mut self, h: &H256);
    fn write_byte(&mut self, b: u8);
    fn finish(self) -> H256;
}

/// Trait for define value structures
pub trait Value {
    fn to_h256(&self) -> H256;
    fn zero() -> Self;
}

impl Value for H256 {
    fn to_h256(&self) -> H256 {
        self.clone()
    }
    fn zero() -> Self {
        H256::empty()
    }
}

/// Trait for customize backend storage
pub trait Store<V> {
    fn get_branch(&self, branch_key: &BranchKey) -> Result<Option<BranchNode>, Error>;
    fn get_leaf(&self, leaf_key: &H256) -> Result<Option<V>, Error>;
    fn insert_branch(&mut self, node_key: BranchKey, branch: BranchNode) -> Result<(), Error>;
    fn insert_leaf(&mut self, leaf_key: H256, leaf: V) -> Result<(), Error>;
    fn remove_branch(&mut self, node_key: &BranchKey) -> Result<(), Error>;
    fn remove_leaf(&mut self, leaf_key: &H256) -> Result<(), Error>;
}
