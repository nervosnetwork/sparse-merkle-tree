use crate::{
    error::{Error, Result},
    merge::{hash_leaf, merge},
    traits::Hasher,
    vec::Vec,
    H256,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    // leaf bitmap, bitmap.get_bit(height) is true means there need a non zero sibling in this height
    leaves_bitmap: Vec<H256>,
    // needed sibling node hash
    merkle_paths: Vec<H256>,
}

impl MerkleProof {
    /// Create MerkleProof
    /// leaves_bitmap: leaf bitmap, bitmap.get_bit(height) is true means there need a non zero sibling in this height
    /// proof: needed sibling node hash
    pub fn new(leaves_bitmap: Vec<H256>, merkle_paths: Vec<H256>) -> Self {
        MerkleProof {
            leaves_bitmap,
            merkle_paths,
        }
    }

    /// Destruct the structure, useful for serialization
    pub fn take(self) -> (Vec<H256>, Vec<H256>) {
        let MerkleProof {
            leaves_bitmap,
            merkle_paths,
        } = self;
        (leaves_bitmap, merkle_paths)
    }

    /// number of leaves required by this merkle proof
    pub fn leaves_count(&self) -> usize {
        self.leaves_bitmap.len()
    }

    /// return the inner leaves_bitmap vector
    pub fn leaves_bitmap(&self) -> &Vec<H256> {
        &self.leaves_bitmap
    }

    /// return sibling node hashes
    pub fn merkle_paths(&self) -> &Vec<H256> {
        &self.merkle_paths
    }

    pub fn compile(self, mut leaves: Vec<(H256, H256)>) -> Result<CompiledMerkleProof> {
        if leaves.is_empty() {
            return Err(Error::EmptyKeys);
        } else if leaves.len() != self.leaves_count() {
            return Err(Error::IncorrectNumberOfLeaves {
                expected: self.leaves_count(),
                actual: leaves.len(),
            });
        }
        // sort leaves
        leaves.sort_unstable_by_key(|(k, _v)| *k);

        let (leaves_bitmap, merkle_paths) = self.take();

        let mut proof: Vec<u8> = Vec::with_capacity(merkle_paths.len() * 33 + leaves.len());
        let mut stack_fork_height = [0u8; 256]; // store fork height
        let mut stack_top = 0;
        let mut leaf_index = 0;
        let mut merkle_path_index = 0;
        while leaf_index < leaves.len() {
            let (leaf_key, _value) = leaves[leaf_index];
            let fork_height = if leaf_index + 1 < leaves.len() {
                leaf_key.fork_height(&leaves[leaf_index + 1].0)
            } else {
                core::u8::MAX
            };
            proof.push(0x4C);
            let mut zero_count = 0u16;
            for height in 0..=fork_height {
                if height == fork_height && leaf_index + 1 < leaves.len() {
                    // If it's not final round, we don't need to merge to root (height=255)
                    break;
                }
                let (op_code_opt, sibling_node_opt) =
                    if stack_top > 0 && stack_fork_height[stack_top - 1] == height {
                        stack_top -= 1;
                        (Some(0x48), None)
                    } else if leaves_bitmap[leaf_index].get_bit(height) {
                        if merkle_path_index >= merkle_paths.len() {
                            return Err(Error::CorruptedProof);
                        }
                        let node_hash = merkle_paths[merkle_path_index];
                        merkle_path_index += 1;
                        (Some(0x50), Some(node_hash))
                    } else {
                        zero_count += 1;
                        if zero_count > 256 {
                            return Err(Error::CorruptedProof);
                        }
                        (None, None)
                    };
                if let Some(op_code) = op_code_opt {
                    if zero_count > 0 {
                        let n = if zero_count == 256 {
                            0
                        } else {
                            zero_count as u8
                        };
                        proof.push(0x4F);
                        proof.push(n);
                        zero_count = 0;
                    }
                    proof.push(op_code);
                }
                if let Some(hash) = sibling_node_opt {
                    proof.extend(hash.as_slice());
                }
            }
            if zero_count > 0 {
                let n = if zero_count == 256 {
                    0
                } else {
                    zero_count as u8
                };
                proof.push(0x4F);
                proof.push(n);
            }
            debug_assert!(stack_top < 256);
            stack_fork_height[stack_top] = fork_height;
            stack_top += 1;
            leaf_index += 1;
        }

        if stack_top != 1 {
            return Err(Error::CorruptedProof);
        }
        if leaf_index != leaves.len() {
            return Err(Error::CorruptedProof);
        }
        if merkle_path_index != merkle_paths.len() {
            return Err(Error::CorruptedProof);
        }
        Ok(CompiledMerkleProof(proof))
    }

    /// Compute root from proof
    /// leaves: a vector of (key, value)
    ///
    /// return EmptyProof error when proof is empty
    /// return CorruptedProof error when proof is invalid
    pub fn compute_root<H: Hasher + Default>(self, mut leaves: Vec<(H256, H256)>) -> Result<H256> {
        if leaves.is_empty() {
            return Err(Error::EmptyKeys);
        } else if leaves.len() != self.leaves_count() {
            return Err(Error::IncorrectNumberOfLeaves {
                expected: self.leaves_count(),
                actual: leaves.len(),
            });
        }
        // sort leaves
        leaves.sort_unstable_by_key(|(k, _v)| *k);

        let (leaves_bitmap, merkle_paths) = self.take();

        let mut stack_fork_height = [0u8; 256]; // store fork height
        let mut stack = [H256::zero(); 256]; // store node hash
        let mut stack_top = 0;
        let mut leaf_index = 0;
        let mut merkle_path_index = 0;
        while leaf_index < leaves.len() {
            let (leaf_key, value) = leaves[leaf_index];
            let fork_height = if leaf_index + 1 < leaves.len() {
                leaf_key.fork_height(&leaves[leaf_index + 1].0)
            } else {
                core::u8::MAX
            };
            let mut current_node = hash_leaf::<H>(&leaf_key, &value);
            for height in 0..=fork_height {
                if height == fork_height && leaf_index + 1 < leaves.len() {
                    // If it's not final round, we don't need to merge to root (height=255)
                    break;
                }
                let parent_key = leaf_key.parent_path(height);
                let is_right = leaf_key.is_right(height);
                let sibling_node = if stack_top > 0 && stack_fork_height[stack_top - 1] == height {
                    let node_hash = stack[stack_top - 1];
                    stack_top -= 1;
                    node_hash
                } else if leaves_bitmap[leaf_index].get_bit(height) {
                    if merkle_path_index >= merkle_paths.len() {
                        return Err(Error::CorruptedProof);
                    }
                    let node_hash = merkle_paths[merkle_path_index];
                    merkle_path_index += 1;
                    node_hash
                } else {
                    H256::zero()
                };
                let (left, right) = if is_right {
                    (sibling_node, current_node)
                } else {
                    (current_node, sibling_node)
                };
                current_node = merge::<H>(height, &parent_key, &left, &right);
            }
            debug_assert!(stack_top < 256);
            stack_fork_height[stack_top] = fork_height;
            stack[stack_top] = current_node;
            stack_top += 1;
            leaf_index += 1;
        }

        if stack_top != 1 {
            return Err(Error::CorruptedProof);
        }
        if leaf_index != leaves.len() {
            return Err(Error::CorruptedProof);
        }
        if merkle_path_index != merkle_paths.len() {
            return Err(Error::CorruptedProof);
        }
        Ok(stack[0])
    }

    /// Verify merkle proof
    /// see compute_root_from_proof
    pub fn verify<H: Hasher + Default>(
        self,
        root: &H256,
        leaves: Vec<(H256, H256)>,
    ) -> Result<bool> {
        let calculated_root = self.compute_root::<H>(leaves)?;
        Ok(&calculated_root == root)
    }
}

/// An structure optimized for verify merkle proof
#[derive(Debug, Clone)]
pub struct CompiledMerkleProof(pub Vec<u8>);

impl CompiledMerkleProof {
    pub fn compute_root<H: Hasher + Default>(&self, mut leaves: Vec<(H256, H256)>) -> Result<H256> {
        leaves.sort_unstable_by_key(|(k, _v)| *k);
        let mut program_index = 0;
        let mut leave_index = 0;
        let mut stack: Vec<(u8, H256, H256)> = Vec::new();
        while program_index < self.0.len() {
            let code = self.0[program_index];
            program_index += 1;
            match code {
                // L : hash leaf
                0x4C => {
                    if leave_index >= leaves.len() {
                        return Err(Error::CorruptedStack);
                    }
                    let (k, v) = leaves[leave_index];
                    stack.push((0, k, hash_leaf::<H>(&k, &v)));
                    leave_index += 1;
                }
                // P : hash stack top item with sibling node in proof
                0x50 => {
                    if stack.is_empty() {
                        return Err(Error::CorruptedStack);
                    }
                    if program_index + 32 > self.0.len() {
                        return Err(Error::CorruptedProof);
                    }
                    let mut data = [0u8; 32];
                    data.copy_from_slice(&self.0[program_index..program_index + 32]);
                    program_index += 32;
                    let sibling_node = H256::from(data);
                    let (height, key, value) = stack.pop().unwrap();
                    let parent_key = key.parent_path(height);
                    let parent = if key.get_bit(height) {
                        merge::<H>(height, &parent_key, &sibling_node, &value)
                    } else {
                        merge::<H>(height, &parent_key, &value, &sibling_node)
                    };
                    stack.push((height.wrapping_add(1), parent_key, parent));
                }
                // H : pop 2 items in stack hash them then push the result
                0x48 => {
                    if stack.len() < 2 {
                        return Err(Error::CorruptedStack);
                    }
                    let (height_b, _key_b, value_b) = stack.pop().unwrap();
                    let (height_a, key_a, value_a) = stack.pop().unwrap();
                    if height_a != height_b {
                        return Err(Error::CorruptedProof);
                    }
                    let (height, key) = (height_a, key_a);
                    let parent_key = key.parent_path(height);
                    let parent = if key.get_bit(height) {
                        merge::<H>(height, &parent_key, &value_b, &value_a)
                    } else {
                        merge::<H>(height, &parent_key, &value_a, &value_b)
                    };
                    stack.push((height.wrapping_add(1), parent_key, parent));
                }
                // O : hash stack top item with n zero values
                0x4F => {
                    if stack.is_empty() {
                        return Err(Error::CorruptedStack);
                    }
                    if program_index >= self.0.len() {
                        return Err(Error::CorruptedProof);
                    }
                    let n = self.0[program_index];
                    program_index += 1;
                    let zero_count: u16 = if n == 0 { 256 } else { n as u16 };
                    let (base_height, key, mut value) = stack.pop().unwrap();
                    let mut parent_key = key;
                    let mut height = base_height;
                    for idx in 0..zero_count {
                        if base_height as u16 + idx > 255 {
                            return Err(Error::CorruptedProof);
                        }
                        height = base_height + idx as u8;
                        parent_key = key.parent_path(height);
                        value = if key.get_bit(height) {
                            merge::<H>(height, &parent_key, &H256::zero(), &value)
                        } else {
                            merge::<H>(height, &parent_key, &value, &H256::zero())
                        };
                    }
                    stack.push((height.wrapping_add(1), parent_key, value));
                }
                _ => return Err(Error::InvalidCode(code)),
            }
        }
        if stack.len() != 1 {
            return Err(Error::CorruptedStack);
        }
        if stack[0].0 != 0 {
            return Err(Error::CorruptedProof);
        }
        Ok(stack[0].2)
    }

    pub fn verify<H: Hasher + Default>(
        &self,
        root: &H256,
        leaves: Vec<(H256, H256)>,
    ) -> Result<bool> {
        let calculated_root = self.compute_root::<H>(leaves)?;
        Ok(&calculated_root == root)
    }
}

impl Into<Vec<u8>> for CompiledMerkleProof {
    fn into(self) -> Vec<u8> {
        self.0
    }
}
