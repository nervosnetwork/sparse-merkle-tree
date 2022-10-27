use crate::{merge::MergeValue, H256};
use core::cmp::Ordering;

/// The branch key
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BranchKey {
    pub height: u8,
    pub node_key: H256,
}

impl BranchKey {
    pub fn new(height: u8, node_key: H256) -> BranchKey {
        BranchKey { height, node_key }
    }
}

impl PartialOrd for BranchKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BranchKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.height.cmp(&other.height) {
            Ordering::Equal => self.node_key.cmp(&other.node_key),
            ordering => ordering,
        }
    }
}

/// A branch in the SMT
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct BranchNode {
    pub left: MergeValue,
    pub right: MergeValue,
}

impl BranchNode {
    /// Create a new empty branch
    pub fn new_empty() -> BranchNode {
        BranchNode {
            left: MergeValue::zero(),
            right: MergeValue::zero(),
        }
    }

    /// Determine whether a node did not store any value
    pub fn is_empty(&self) -> bool {
        self.left.is_zero() && self.right.is_zero()
    }
}
