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
use std::collections::BTreeMap;

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

    pub fn update_all(&mut self, leaves: Vec<(H256, V)>) -> Result<&H256> {
        if leaves.is_empty() {
            return Ok(&self.root);
        }

        let leaf_pairs = leaves
            .into_iter()
            .map(|(key, value)| {
                let node = MergeValue::from_h256(value.to_h256());
                (key, value, node)
            })
            .collect::<Vec<_>>();

        let mut delta_tree: BTreeMap<BranchKey, (Option<MergeValue>, Option<MergeValue>)> =
            BTreeMap::default();
        for (leaf_key, leaf_value, leaf_node) in leaf_pairs {
            if !leaf_node.is_zero() {
                self.store.insert_leaf(leaf_key, leaf_value)?;
            } else {
                self.store.remove_leaf(&leaf_key)?;
            }

            // recompute the tree from bottom to top
            let mut current_key = leaf_key;
            for height in 0..=core::u8::MAX {
                let parent_key = current_key.parent_path(height);
                let parent_branch_key = BranchKey::new(height, parent_key);
                let is_right = current_key.is_right(height);

                let (new_left, new_right) =
                    if let Some((old_left, old_right)) = delta_tree.get(&parent_branch_key) {
                        if height == 0 {
                            if is_right {
                                (old_left.clone(), Some(leaf_node.clone()))
                            } else {
                                (Some(leaf_node.clone()), old_right.clone())
                            }
                        } else {
                            match (old_left, old_right, is_right) {
                                (None, Some(_right), true) => (None, None),
                                (Some(_left), None, false) => (None, None),
                                // duplicated key in right side
                                (Some(_), _, true) => {
                                    break;
                                }
                                // duplicated key in left side
                                (_, Some(_), false) => {
                                    break;
                                }
                                // All ancestors are processed
                                (None, None, _) => {
                                    break;
                                }
                            }
                        }
                    } else {
                        let (branch_left, branch_right) = self
                            .store
                            .get_branch(&parent_branch_key)?
                            .map(|parent_branch| (parent_branch.left, parent_branch.right))
                            .unwrap_or_else(|| (MergeValue::zero(), MergeValue::zero()));
                        match (height, is_right) {
                            (0, true) => (Some(branch_left), Some(leaf_node.clone())),
                            (0, false) => (Some(leaf_node.clone()), Some(branch_right)),
                            (_, true) => (Some(branch_left), None),
                            (_, false) => (None, Some(branch_right)),
                        }
                    };
                delta_tree.insert(parent_branch_key, (new_left, new_right));
                current_key = parent_key;
            }
        }

        let mut root_node = MergeValue::zero();
        let keys = delta_tree.keys().cloned().collect::<Vec<BranchKey>>();
        for parent_branch_key in keys {
            let BranchKey { height, node_key } = parent_branch_key;
            let (left_opt, right_opt) = delta_tree.get(&parent_branch_key).unwrap();
            match (left_opt, right_opt) {
                (Some(left), Some(right)) => {
                    if !left.is_zero() || !right.is_zero() {
                        // insert or update branch
                        self.store.insert_branch(
                            BranchKey { height, node_key },
                            BranchNode {
                                left: left.clone(),
                                right: right.clone(),
                            },
                        )?;
                    } else {
                        // remove empty branch
                        self.store.remove_branch(&parent_branch_key)?;
                    }
                    // update parent node in delta_tree
                    let node = merge::<H>(height, &node_key, left, right);
                    if height < core::u8::MAX {
                        let parent_height = height + 1;
                        let parent_parent_key = node_key.parent_path(parent_height);
                        let is_right = node_key.is_right(parent_height);
                        let (parent_left_opt, parent_right_opt) = delta_tree
                            .get_mut(&BranchKey {
                                height: parent_height,
                                node_key: parent_parent_key,
                            })
                            .unwrap();
                        if is_right {
                            *parent_right_opt = Some(node);
                        } else {
                            *parent_left_opt = Some(node);
                        }
                    } else {
                        root_node = node;
                    }
                }
                _ => panic!("children nodes not all filled"),
            }
        }

        self.root = root_node.hash::<H>();
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
