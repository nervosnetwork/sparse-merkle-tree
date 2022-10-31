use crate::{
    branch::*,
    error::{Error, Result},
    merge::{merge, MergeValue},
    merkle_proof::MerkleProof,
    traits::{Hasher, StoreReadOps, StoreWriteOps, Value},
    vec::Vec,
    H256,
};
use core::marker::PhantomData;

/// Sparse merkle tree
#[derive(Default, Debug)]
pub struct SparseMerkleTree<H, V, S> {
    store: S,
    root: H256,
    phantom: PhantomData<(H, V)>,
}

impl<H, V, S: StoreReadOps<V>> SparseMerkleTree<H, V, S> {
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
}

impl<H: Hasher + Default, V: Value + PartialEq, S: StoreReadOps<V> + StoreWriteOps<V>>
    SparseMerkleTree<H, V, S>
{
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

        let mut last_height = core::u8::MAX;
        while last_height > 0 {
            // walk from top to bottom
            let node_key = key.parent_path(last_height);
            let branch_key = BranchKey::new(last_height, node_key); // this represents a position in the tree
            if let Some(branch) = self.store.get_branch(&branch_key)? {
                // if we we found a record in here
                // we need to determine whether is it a shortcut
                let (target, another) = if key.is_right(last_height) {
                    (branch.right, branch.left)
                } else {
                    (branch.left, branch.right)
                };

                match target {
                    MergeValue::ShortCut {
                        key: this_key,
                        value: val,
                        height: h,
                    } => {
                        if this_key.eq(&key) {
                            // we update its value
                            let target = MergeValue::shortcut(key, node.hash::<H>(), h);

                            let new_branch = if key.is_right(last_height) {
                                BranchNode {
                                    left: another,
                                    right: target,
                                }
                            } else {
                                BranchNode {
                                    left: target,
                                    right: another,
                                }
                            };

                            // update this shortcut's value
                            self.store.insert_branch(branch_key, new_branch)?;
                            break;
                        } else {
                            // we need to move this shortcut down

                            // OPTIMIZATION: the shortcut must dropping to the level where [shortcut's new_height + 1] = [insert/delete key's shortcut height + 1]
                            // check definition of H256.fork_height()
                            last_height = this_key.fork_height(&key);

                            let (next_left, next_right) = if key.is_right(last_height) {
                                (
                                    MergeValue::shortcut(this_key, val, last_height),
                                    MergeValue::shortcut(key, node.hash::<H>(), last_height),
                                )
                            } else {
                                (
                                    MergeValue::shortcut(key, node.hash::<H>(), last_height),
                                    MergeValue::shortcut(this_key, val, last_height),
                                )
                            };

                            let next_branch_key =
                                BranchKey::new(last_height, this_key.parent_path(last_height));

                            self.store.insert_branch(
                                next_branch_key,
                                BranchNode {
                                    left: next_left,
                                    right: next_right,
                                },
                            )?;
                            break; // go next round
                        }
                    }
                    _ => {
                        let insert_value = if last_height == 0 {
                            node.clone()
                        } else {
                            MergeValue::shortcut(key, node.hash::<H>(), last_height)
                        };
                        let (left, right) = if key.is_right(last_height) {
                            (another, insert_value)
                        } else {
                            (insert_value, another)
                        };
                        self.store
                            .insert_branch(branch_key, BranchNode { left, right })?;
                    }
                }
            } else if !node.is_zero() {
                // adds a shortcut here
                let shortcut = MergeValue::shortcut(key, node.hash::<H>(), last_height);
                let (left, right) = if key.is_right(last_height) {
                    (MergeValue::zero(), shortcut)
                } else {
                    (shortcut, MergeValue::zero())
                };
                self.store
                    .insert_branch(branch_key, BranchNode { left, right })?;
                break; // stop walking
            } // do nothing with a zero insertion
            last_height -= 1;
        }

        for height in last_height..=core::u8::MAX {
            // update tree hash from insert pos to top
            let node_key = key.parent_path(height);
            let branch_key = BranchKey::new(height, node_key);

            let new_merge = if let Some(branch) = self.store.get_branch(&branch_key)? {
                merge::<H>(height, &node_key, &branch.left, &branch.right)
            } else {
                MergeValue::zero()
            };
            if height == core::u8::MAX {
                // updating root
                self.root = new_merge.hash::<H>();
            } else {
                // updates parent branch
                let parent_key = key.parent_path(height + 1);
                let parent_branch_key = BranchKey::new(height + 1, parent_key);
                if new_merge.is_shortcut() {
                    // move up
                    self.store.remove_branch(&branch_key)?;
                }
                if let Some(parent_branch) = self.store.get_branch(&parent_branch_key)? {
                    let (left, right) = if key.is_right(height + 1) {
                        (parent_branch.left, new_merge)
                    } else {
                        (new_merge, parent_branch.right)
                    };
                    if left.is_zero() && right.is_zero() {
                        self.store.remove_branch(&parent_branch_key)?;
                    } else {
                        let new_parent_branch = BranchNode { left, right };
                        self.store
                            .insert_branch(parent_branch_key, new_parent_branch)?;
                    }
                } else if !new_merge.is_zero() {
                    let new_parent_branch = if key.is_right(height + 1) {
                        BranchNode {
                            left: MergeValue::zero(),
                            right: new_merge,
                        }
                    } else {
                        BranchNode {
                            left: new_merge,
                            right: MergeValue::zero(),
                        }
                    };
                    self.store
                        .insert_branch(parent_branch_key, new_parent_branch)?;
                }
            }
        }

        Ok(&self.root)
    }

    /// Update multiple leaves at once
    pub fn update_all(&mut self, mut leaves: Vec<(H256, V)>) -> Result<&H256> {
        // Dedup(only keep the last of each key) and sort leaves
        leaves.reverse();
        leaves.sort_by_key(|(a, _)| *a);
        leaves.dedup_by_key(|(a, _)| *a);

        for (key, value) in leaves {
            self.update(key, value)?;
        }

        Ok(&self.root)
    }
}

impl<H: Hasher + Default, V: Value, S: StoreReadOps<V>> SparseMerkleTree<H, V, S> {
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
        let mut proof: Vec<MergeValue> = Default::default();
        for current_key in &keys {
            let mut bitmap = H256::zero();
            for height in (0..=core::u8::MAX).rev() {}

            leaves_bitmap.push(bitmap);
        }

        Ok(MerkleProof::new(leaves_bitmap, proof))
    }
}
