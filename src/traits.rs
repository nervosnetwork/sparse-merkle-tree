use crate::{
    error::Error,
    h256::SmtH256,
    tree::{BranchKey, BranchNode},
};

/// Trait for customize hash function
pub trait Hasher {
    fn write_h256(&mut self, h: &SmtH256);
    fn write_byte(&mut self, b: u8);
    fn finish(self) -> SmtH256;
}

/// Trait for define value structures
pub trait Value {
    fn to_h256(&self) -> SmtH256;
    fn zero() -> Self;
}

impl Value for SmtH256 {
    fn to_h256(&self) -> SmtH256 {
        self.clone()
    }
    fn zero() -> Self {
        SmtH256::empty()
    }
}

/// Trait for customize backend storage
pub trait Store<V> {
    fn get_branch(&self, branch_key: &BranchKey) -> Result<Option<BranchNode>, Error>;
    fn get_leaf(&self, leaf_key: &SmtH256) -> Result<Option<V>, Error>;
    fn insert_branch(&mut self, node_key: BranchKey, branch: BranchNode) -> Result<(), Error>;
    fn insert_leaf(&mut self, leaf_key: SmtH256, leaf: V) -> Result<(), Error>;
    fn remove_branch(&mut self, node_key: &BranchKey) -> Result<(), Error>;
    fn remove_leaf(&mut self, leaf_key: &SmtH256) -> Result<(), Error>;
}
