use crate::{
    error::{Error, Result},
    merge::{merge, MergeValue},
    merkle_proof::MerkleProof,
    traits::{Hasher, Store, Value},
    vec::Vec,
    H256, MAX_STACK_SIZE,
};
use core::cmp::Ordering;
use core::marker::PhantomData;

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

/// Sparse merkle tree
#[derive(Default, Debug)]
pub struct SparseMerkleTree<H, V, S> {
    store: S,
    root: H256,
    phantom: PhantomData<(H, V)>,
}

impl<H: Hasher + Default, V: Value, S: Store<V>> SparseMerkleTree<H, V, S> {
    /// Build a merkle tree from root and store
    pub fn new(root: H256, store: S) -> SparseMerkleTree<H, V, S> {
        SparseMerkleTree {
            root,
            store,
            phantom: PhantomData,
        }
    }

    /// Merkle root
    pub fn root(&self) -> &H256 {
        &self.root
    }

    /// Check empty of the tree
    pub fn is_empty(&self) -> bool {
        self.root.is_zero()
    }

    /// Destroy current tree and retake store
    pub fn take_store(self) -> S {
        self.store
    }

    /// Get backend store
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Get mutable backend store
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// Update a leaf, return new merkle root
    /// set to zero value to delete a key
    pub fn update(&mut self, key: H256, value: V) -> Result<&H256> {
        // compute and store new leaf
        let node = MergeValue::from_h256(value.to_h256());
        // notice when value is zero the leaf is deleted, so we do not need to store it
        if !node.is_zero() {
            self.store.insert_leaf(key, value)?;
        } else {
            self.store.remove_leaf(&key)?;
        }

        // recompute the tree from bottom to top
        let mut current_key = key;
        let mut current_node = node;
        for height in 0..=core::u8::MAX {
            let parent_key = current_key.parent_path(height);
            let parent_branch_key = BranchKey::new(height, parent_key);
            let (left, right) =
                if let Some(parent_branch) = self.store.get_branch(&parent_branch_key)? {
                    if current_key.is_right(height) {
                        (parent_branch.left, current_node)
                    } else {
                        (current_node, parent_branch.right)
                    }
                } else if current_key.is_right(height) {
                    (MergeValue::zero(), current_node)
                } else {
                    (current_node, MergeValue::zero())
                };

            if !left.is_zero() || !right.is_zero() {
                // insert or update branch
                self.store.insert_branch(
                    parent_branch_key,
                    BranchNode {
                        left: left.clone(),
                        right: right.clone(),
                    },
                )?;
            } else {
                // remove empty branch
                self.store.remove_branch(&parent_branch_key)?;
            }
            // prepare for next round
            current_key = parent_key;
            current_node = merge::<H>(height, &parent_key, &left, &right);
        }

        self.root = current_node.hash::<H>();
        Ok(&self.root)
    }

    /// Update multiple leaves at once
    pub fn update_all(&mut self, mut leaves: Vec<(H256, V)>) -> Result<&H256> {
        // Dedup(only keep the last of each key) and sort leaves
        leaves.reverse();
        leaves.sort_by_key(|(a, _)| a.clone());
        leaves.dedup_by_key(|(a, _)| a.clone());

        let mut nodes: Vec<(H256, MergeValue)> = Vec::new();
        for (k, v) in leaves {
            let value = MergeValue::from_h256(v.to_h256());
            if !value.is_zero() {
                self.store.insert_leaf(k, v)?;
            } else {
                self.store.remove_leaf(&k)?;
            }
            nodes.push((k, value));
        }

        for height in 0..=core::u8::MAX {
            let mut next_nodes: Vec<(H256, MergeValue)> = Vec::new();
            let mut i = 0;
            while i < nodes.len() {
                let (current_key, current_merge_value) = &nodes[i];
                i += 1;
                let parent_key = current_key.parent_path(height);
                let parent_branch_key = BranchKey::new(height, parent_key);

                // Test for neighbors
                let mut right = None;
                if i < nodes.len() && (!current_key.is_right(height)) {
                    let (neighbor_key, neighbor_value) = &nodes[i];
                    let mut right_key = current_key.clone();
                    right_key.set_bit(height);
                    if right_key == *neighbor_key {
                        right = Some(neighbor_value.clone());
                        i += 1;
                    }
                }

                let (left, right) = if let Some(right_merge_value) = right {
                    (current_merge_value.clone(), right_merge_value)
                } else {
                    // In case neighbor is not available, fetch from store
                    if let Some(parent_branch) = self.store.get_branch(&parent_branch_key)? {
                        if current_key.is_right(height) {
                            (parent_branch.left, current_merge_value.clone())
                        } else {
                            (current_merge_value.clone(), parent_branch.right)
                        }
                    } else if current_key.is_right(height) {
                        (MergeValue::zero(), current_merge_value.clone())
                    } else {
                        (current_merge_value.clone(), MergeValue::zero())
                    }
                };

                if !left.is_zero() || !right.is_zero() {
                    self.store.insert_branch(
                        parent_branch_key,
                        BranchNode {
                            left: left.clone(),
                            right: right.clone(),
                        },
                    )?;
                } else {
                    self.store.remove_branch(&parent_branch_key)?;
                }
                next_nodes.push((parent_key, merge::<H>(height, &parent_key, &left, &right)));
            }
            nodes = next_nodes;
        }

        assert!(nodes.len() == 1);
        self.root = nodes[0].1.hash::<H>();
        Ok(&self.root)
    }

    /// Get value of a leaf
    /// return zero value if leaf not exists
    pub fn get(&self, key: &H256) -> Result<V> {
        if self.is_empty() {
            return Ok(V::zero());
        }
        Ok(self.store.get_leaf(key)?.unwrap_or_else(V::zero))
    }

    /// Generate merkle proof
    pub fn merkle_proof(&self, mut keys: Vec<H256>) -> Result<MerkleProof> {
        if keys.is_empty() {
            return Err(Error::EmptyKeys);
        }

        // sort keys
        keys.sort_unstable();

        // Collect leaf bitmaps
        let mut leaves_bitmap: Vec<H256> = Default::default();
        for current_key in &keys {
            let mut bitmap = H256::zero();
            for height in 0..=core::u8::MAX {
                let parent_key = current_key.parent_path(height);
                let parent_branch_key = BranchKey::new(height, parent_key);
                if let Some(parent_branch) = self.store.get_branch(&parent_branch_key)? {
                    let sibling = if current_key.is_right(height) {
                        parent_branch.left
                    } else {
                        parent_branch.right
                    };
                    if !sibling.is_zero() {
                        bitmap.set_bit(height);
                    }
                } else {
                    // The key is not in the tree (support non-inclusion proof)
                }
            }
            leaves_bitmap.push(bitmap);
        }

        let mut proof: Vec<MergeValue> = Default::default();
        let mut stack_fork_height = [0u8; MAX_STACK_SIZE]; // store fork height
        let mut stack_top = 0;
        let mut leaf_index = 0;
        while leaf_index < keys.len() {
            let leaf_key = keys[leaf_index];
            let fork_height = if leaf_index + 1 < keys.len() {
                leaf_key.fork_height(&keys[leaf_index + 1])
            } else {
                core::u8::MAX
            };
            for height in 0..=fork_height {
                if height == fork_height && leaf_index + 1 < keys.len() {
                    // If it's not final round, we don't need to merge to root (height=255)
                    break;
                }
                let parent_key = leaf_key.parent_path(height);
                let is_right = leaf_key.is_right(height);

                // has non-zero sibling
                if stack_top > 0 && stack_fork_height[stack_top - 1] == height {
                    stack_top -= 1;
                } else if leaves_bitmap[leaf_index].get_bit(height) {
                    let parent_branch_key = BranchKey::new(height, parent_key);
                    if let Some(parent_branch) = self.store.get_branch(&parent_branch_key)? {
                        let sibling = if is_right {
                            parent_branch.left
                        } else {
                            parent_branch.right
                        };
                        if !sibling.is_zero() {
                            proof.push(sibling);
                        } else {
                            unreachable!();
                        }
                    } else {
                        // The key is not in the tree (support non-inclusion proof)
                    }
                }
            }
            debug_assert!(stack_top < MAX_STACK_SIZE);
            stack_fork_height[stack_top] = fork_height;
            stack_top += 1;
            leaf_index += 1;
        }
        assert_eq!(stack_top, 1);
        Ok(MerkleProof::new(leaves_bitmap, proof))
    }
}
