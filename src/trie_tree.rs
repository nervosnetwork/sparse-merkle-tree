use crate::{
    error::{Error, Result},
    merge::{into_merge_value, merge, MergeValue},
    merkle_proof::MerkleProof,
    traits::{Hasher, StoreReadOps, StoreWriteOps, Value},
    tree::{BranchKey, BranchNode},
    vec::Vec,
    H256, MAX_STACK_SIZE,
};
use core::marker::PhantomData;

/// Sparse merkle tree
#[derive(Default, Debug)]
pub struct SparseMerkleTree<H, V, S> {
    store: S,
    root: H256,
    phantom: PhantomData<(H, V)>,
}

impl<H, V, S> SparseMerkleTree<H, V, S> {
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

impl<H: Hasher + Default, V, S: StoreReadOps<V>> SparseMerkleTree<H, V, S> {
    /// Build a merkle tree from store, the root will be calculated automatically
    pub fn new_with_store(store: S) -> Result<SparseMerkleTree<H, V, S>> {
        let root_branch_key = BranchKey::new(core::u8::MAX, H256::zero());
        store
            .get_branch(&root_branch_key)
            .map(|branch_node| {
                branch_node
                    .map(|n| {
                        merge::<H>(core::u8::MAX, &H256::zero(), &n.left, &n.right).hash::<H>()
                    })
                    .unwrap_or_default()
            })
            .map(|root| SparseMerkleTree::new(root, store))
    }
}

impl<H: Hasher + Default, V: Value, S: StoreReadOps<V> + StoreWriteOps<V>>
    SparseMerkleTree<H, V, S>
{
    /// Update a leaf, return new merkle root
    /// set to zero value to delete a key
    pub fn update(&mut self, key: H256, value: V) -> Result<&H256> {
        let value_h256 = value.to_h256();
        // compute and store new leaf
        let node = MergeValue::from_h256(value_h256);
        // notice when value is zero the leaf is deleted, so we do not need to store it
        if !node.is_zero() {
            self.store.insert_leaf(key, value)?;
        } else {
            self.store.remove_leaf(&key)?;
        }

        let mut last_height = core::u8::MAX;
        loop {
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
                        value,
                        height: h,
                    } => {
                        if this_key.eq(&key) {
                            // we update its value
                            if value_h256.is_zero() && another.is_zero() {
                                self.store.remove_branch(&branch_key)?;
                            } else {
                                let target = if value_h256.is_zero() {
                                    MergeValue::from_h256(value_h256)
                                } else {
                                    MergeValue::shortcut(key, value_h256, h)
                                };
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
                            }
                            break;
                        } else if !value_h256.is_zero() {
                            // we need to move this shortcut down

                            // OPTIMIZATION: the shortcut must dropping to the level where [shortcut's new_height + 1] = [insert/delete key's shortcut height + 1]
                            // check definition of H256.fork_height()
                            last_height = this_key.fork_height(&key);

                            let (next_left, next_right) = if key.is_right(last_height) {
                                if last_height != 0 {
                                    (
                                        MergeValue::shortcut(this_key, value, last_height),
                                        MergeValue::shortcut(key, value_h256, last_height),
                                    )
                                } else {
                                    (
                                        MergeValue::from_h256(value),
                                        MergeValue::from_h256(value_h256),
                                    )
                                }
                            } else {
                                if last_height != 0 {
                                    (
                                        MergeValue::shortcut(key, value_h256, last_height),
                                        MergeValue::shortcut(this_key, value, last_height),
                                    )
                                } else {
                                    (
                                        MergeValue::from_h256(value_h256),
                                        MergeValue::from_h256(value),
                                    )
                                }
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
                        } else {
                            // zero insertion meets shortcut, skip
                            break; // go next round
                        }
                    }
                    _ => {
                        if target.is_zero() || last_height == 0 {
                            let insert_value = if last_height == 0 || node.is_zero() {
                                node
                            } else {
                                MergeValue::shortcut(key, value_h256, last_height)
                            };
                            let (left, right) = if key.is_right(last_height) {
                                (another, insert_value)
                            } else {
                                (insert_value, another)
                            };
                            self.store
                                .insert_branch(branch_key, BranchNode { left, right })?;
                            break;
                        } else {
                            // walk down
                            last_height -= 1;
                            continue;
                        }
                    }
                }
            } else if !node.is_zero() {
                let target_node = if last_height != 0 {
                    // adds a shortcut here
                    MergeValue::shortcut(key, value_h256, last_height)
                } else {
                    node
                };
                let (left, right) = if key.is_right(last_height) {
                    (MergeValue::zero(), target_node)
                } else {
                    (target_node, MergeValue::zero())
                };
                self.store
                    .insert_branch(branch_key, BranchNode { left, right })?;
                break; // stop walking
            } else if last_height != 0 {
                last_height -= 1;
            } else {
                // do nothing with a zero insertion
                break;
            }
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
        for current_key in &keys {
            let mut bitmap = H256::zero();
            for height in (0..=core::u8::MAX).rev() {
                let parent_key = current_key.parent_path(height);
                let parent_branch_key = BranchKey::new(height, parent_key);
                if let Some(parent_branch) = self.store.get_branch(&parent_branch_key)? {
                    let (sibling, target) = if current_key.is_right(height) {
                        (parent_branch.left, parent_branch.right)
                    } else {
                        (parent_branch.right, parent_branch.left)
                    };

                    if !sibling.is_zero() {
                        bitmap.set_bit(height);
                    }
                    if let MergeValue::ShortCut { key, .. } = target {
                        if !key.eq(current_key) {
                            let fork_height = key.fork_height(current_key);
                            bitmap.set_bit(fork_height);
                            break;
                        }
                    }
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

            let heights = (0..=fork_height)
                .into_iter()
                .filter(|height| {
                    if stack_top > 0 && stack_fork_height[stack_top - 1] == *height {
                        stack_top -= 1;
                        false
                    } else {
                        true
                    }
                })
                .collect::<Vec<_>>();

            let mut proof_result = Vec::new();
            for height in heights.iter().copied().rev() {
                if height == fork_height && leaf_index + 1 < keys.len() {
                    // If it's not final round, we don't need to merge to root (height=255)
                    continue;
                }

                if leaves_bitmap[leaf_index].get_bit(height) {
                    let parent_key = leaf_key.parent_path(height);
                    let is_right = leaf_key.is_right(height);
                    let parent_branch_key = BranchKey::new(height, parent_key);
                    if let Some(parent_branch) = self.store.get_branch(&parent_branch_key)? {
                        let (sibling, current) = if is_right {
                            (parent_branch.left, parent_branch.right)
                        } else {
                            (parent_branch.right, parent_branch.left)
                        };
                        push_sibling::<H>(&mut proof_result, sibling);
                        if let MergeValue::ShortCut { key, value, .. } = current {
                            if !key.eq(&leaf_key) {
                                // this means key does not exist
                                let fork_height = key.fork_height(&leaf_key);
                                if leaves_bitmap[leaf_index].get_bit(fork_height)
                                    && heights.contains(&fork_height)
                                {
                                    proof_result.push(into_merge_value::<H>(
                                        key,
                                        value,
                                        fork_height,
                                    ))
                                }
                                if fork_height == 1 && leaves_bitmap[leaf_index].get_bit(0) {
                                    proof_result.push(MergeValue::from_h256(value));
                                }
                            }
                            break;
                        }
                    } else {
                        // Maybe we've skipped shortcut node, find from up to down
                        for i in (height..=core::u8::MAX).rev() {
                            let parent_key = leaf_key.parent_path(i);
                            let is_right = leaf_key.is_right(i);
                            let parent_branch_key = BranchKey::new(i, parent_key);
                            if let Some(parent_branch) =
                                self.store.get_branch(&parent_branch_key)?
                            {
                                let current = if is_right {
                                    parent_branch.right
                                } else {
                                    parent_branch.left
                                };

                                match current {
                                    MergeValue::ShortCut { key, value, .. } => {
                                        if !key.eq(&leaf_key) {
                                            let fork_at = key.fork_height(&leaf_key);
                                            if fork_at == height {
                                                proof_result.push(into_merge_value::<H>(
                                                    key, value, fork_at,
                                                ));
                                            }
                                        }
                                        break;
                                    }
                                    _ => {
                                        continue;
                                    }
                                }
                            }
                        }
                        break; // we should stop further looping
                    }
                }
            }

            proof_result.reverse();
            proof.append(&mut proof_result);
            debug_assert!(stack_top < MAX_STACK_SIZE);
            stack_fork_height[stack_top] = fork_height;
            stack_top += 1;
            leaf_index += 1;
        }
        Ok(MerkleProof::new(leaves_bitmap, proof))
    }
}

// Helper function for a merkle_path insertion
fn push_sibling<H: Hasher + Default>(proof_result: &mut Vec<MergeValue>, sibling: MergeValue) {
    match sibling {
        MergeValue::ShortCut { key, value, height } => {
            proof_result.push(into_merge_value::<H>(key, value, height))
        }
        _ => proof_result.push(sibling),
    }
}
