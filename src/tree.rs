use crate::{
    error::{Error, Result},
    merge::{
        // hex_merge_value,
        merge,
        merge_with_zeros,
        MergeValue,
    },
    merkle_proof::MerkleProof,
    traits::{Hasher, StoreReadOps, StoreWriteOps, Value},
    vec::Vec,
    H256, MAX_STACK_SIZE,
};
use core::marker::PhantomData;

/// Suggested merkle path capacity
const MERKLE_PATH_CAPACITY: usize = 16;

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
    pub fn new_with_store(store: S, root: H256) -> SparseMerkleTree<H, V, S> {
        SparseMerkleTree::new(root, store)
    }
}

impl<H: Hasher + Default, V: Value, S: StoreReadOps<V> + StoreWriteOps<V>>
    SparseMerkleTree<H, V, S>
{
    /// Update a leaf, return new merkle root
    /// set to zero value to delete a key
    pub fn update(&mut self, key: H256, value: V) -> Result<&H256> {
        let is_delete = value.to_h256().is_zero();

        // search siblings
        let MerklePath {
            height,
            mut merkle_path,
            ..
        } = self.merkle_path(&key)?;

        // if height isn't 0, there is no leaf in the tree
        if height != 0 && is_delete {
            return Ok(&self.root);
        }

        let mut node = MergeValue::Value(value.to_h256());
        // insert leaf
        if !is_delete {
            self.store.insert_leaf(node.hash::<H>(), value)?;
        }

        let mut node_height = 0;
        // the new leaf becomes descendent
        while let Some((height, sibling)) = merkle_path.pop() {
            // FIXME allign to fork_height????
            assert!(node_height <= height);
            if node_height < height {
                node = merge_with_zeros::<H>(key, node, height, height - node_height);
                node_height = height;
            }

            let (lhs, rhs) = if key.is_right(height) {
                (sibling, node)
            } else {
                (node, sibling)
            };
            let node_key = key.parent_path(height);
            node = merge::<H>(height, &node_key, &lhs, &rhs);

            let is_merge_with_zero = lhs.is_zero() || rhs.is_zero();
            let next_sibling_is_zero = height != u8::MAX;
            if !(is_merge_with_zero && next_sibling_is_zero) {
                self.store.insert_branch(
                    node.hash::<H>(),
                    BranchNode {
                        left: lhs,
                        right: rhs,
                    },
                )?;
            }
            if height == u8::MAX {
                self.root = node.hash::<H>();
                return Ok(&self.root);
            }
            node_height += 1;
        }

        node = merge_with_zeros::<H>(key, node, u8::MAX, u8::MAX - node_height);
        let height = u8::MAX;
        let node_key = key.parent_path(height);
        let (lhs, rhs) = if key.is_right(height) {
            (MergeValue::zero(), node)
        } else {
            (node, MergeValue::zero())
        };
        node = merge::<H>(height, &node_key, &lhs, &rhs);
        self.store.insert_branch(
            node.hash::<H>(),
            BranchNode {
                left: lhs,
                right: rhs,
            },
        )?;

        self.root = node.hash::<H>();
        Ok(&self.root)
    }

    /// Update multiple leaves at once
    #[deprecated(since = "0.6.1", note = "use update instead")]
    pub fn update_all(&mut self, leaves: Vec<(H256, V)>) -> Result<&H256> {
        // unimplemented!();
        for (key, value) in leaves {
            self.update(key, value)?;
        }
        Ok(&self.root)
    }
}

struct MerklePath {
    node: H256,
    height: u8,
    bitmap: H256,
    merkle_path: Vec<(u8, MergeValue)>,
}

impl<H: Hasher + Default, V: Value, S: StoreReadOps<V>> SparseMerkleTree<H, V, S> {
    /// Get merkle path of a key
    ///
    /// # Arguments
    /// - key
    ///
    /// # Returns
    /// - node: internal node of the key
    /// - height: height of the node
    /// - bitmap: bitmap of the key
    /// - merkle_path: merkle path in reverse order
    ///
    fn merkle_path(&self, key: &H256) -> Result<MerklePath> {
        let mut merkle_path = Vec::with_capacity(MERKLE_PATH_CAPACITY);
        let mut node = self.root;
        let mut height = u8::MAX;

        let mut bitmap = H256::zero();

        if self.is_empty() {
            return Ok(MerklePath {
                node,
                height,
                bitmap,
                merkle_path,
            });
        }

        // search siblings from root
        loop {
            let branch = self
                .store
                .get_branch(&node)?
                .ok_or(Error::MissingBranch(height, node))?;
            // push sibling
            let next;
            let sibling;
            if key.is_right(height) {
                next = branch.right.clone();
                sibling = branch.left.clone();
            } else {
                next = branch.left.clone();
                sibling = branch.right.clone();
            }

            if !sibling.is_zero() {
                bitmap.set_bit(height);
                merkle_path.push((height, sibling));
            }
            if next.is_zero() {
                break;
            }

            // goto next
            match next {
                MergeValue::MergeWithZero {
                    base_node,
                    mut zero_bits,
                    mut zero_count,
                    value,
                } => {
                    while zero_count > 0 {
                        zero_count -= 1;
                        height -= 1;
                        // if we are on a zero node, go descendent to zero
                        let descendent_to_zero = zero_bits.is_right(height) != key.is_right(height);
                        zero_bits.clear_bit(height);
                        if descendent_to_zero {
                            bitmap.set_bit(height);
                            let sibling = if zero_count == 0 {
                                MergeValue::Value(value)
                            } else {
                                MergeValue::MergeWithZero {
                                    base_node,
                                    zero_bits,
                                    zero_count,
                                    value,
                                }
                            };
                            merkle_path.push((height, sibling));
                            // we are at 0 branch, so all siblings from there are zeros
                            height = 0;
                            node = H256::zero();
                            // skip descendent zeros
                            return Ok(MerklePath {
                                node,
                                height,
                                bitmap,
                                merkle_path,
                            });
                        }
                    }
                    if height == 0 {
                        node = value;
                        break;
                    }
                    assert_eq!(zero_count, 0);
                    node = value;
                    height -= 1;
                }
                MergeValue::Value(n) => {
                    node = n;
                    if height == 0 {
                        // found leaf
                        break;
                    }
                    height -= 1;
                }
            }
        }
        Ok(MerklePath {
            node,
            height,
            bitmap,
            merkle_path,
        })
    }

    /// Get value of a leaf
    /// return zero value if leaf not exists
    pub fn get(&self, key: &H256) -> Result<V> {
        let MerklePath { node, height, .. } = self.merkle_path(key)?;

        if height == 0 {
            Ok(self.store.get_leaf(&node)?.unwrap_or(V::zero()))
        } else {
            Ok(V::zero())
        }
    }

    /// Generate merkle proof
    pub fn merkle_proof(&self, mut keys: Vec<H256>) -> Result<MerkleProof> {
        if keys.is_empty() {
            return Err(Error::EmptyKeys);
        }

        // Sort keys
        keys.sort_unstable();

        // Collect leaf bitmaps and merkle path
        let mut leaves_bitmap: Vec<H256> = Default::default();
        let mut leaves_merkle_path: Vec<_> = Default::default();
        for key in &keys {
            let MerklePath {
                bitmap,
                merkle_path,
                ..
            } = self.merkle_path(key)?;
            leaves_bitmap.push(bitmap);
            leaves_merkle_path.push(merkle_path);
        }

        // Iterate all leaves to compile proof
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

                // has non-zero sibling
                if stack_top > 0 && stack_fork_height[stack_top - 1] == height {
                    stack_top -= 1;
                    // pop unused merkle path
                    if leaves_bitmap[leaf_index].get_bit(height) {
                        leaves_merkle_path[leaf_index].pop().unwrap();
                    }
                } else if leaves_bitmap[leaf_index].get_bit(height) {
                    let sibling = leaves_merkle_path[leaf_index].pop().unwrap().1;
                    assert!(!sibling.is_zero());
                    proof.push(sibling);
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
