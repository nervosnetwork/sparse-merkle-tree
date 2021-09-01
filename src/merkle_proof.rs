use crate::{
    error::{Error, Result},
    h256,
    h256::H256Ord,
    merge::{merge, MergeValue},
    traits::Hasher,
    vec::Vec,
    H256, MAX_STACK_SIZE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    // leaf bitmap, bitmap.get_bit(height) is true means there need a non zero sibling in this height
    leaves_bitmap: Vec<H256>,
    // needed sibling node hash
    merkle_path: Vec<MergeValue>,
}

impl MerkleProof {
    /// Create MerkleProof
    /// leaves_bitmap: leaf bitmap, bitmap.get_bit(height) is true means there need a non zero sibling in this height
    /// proof: needed sibling node hash
    pub fn new(leaves_bitmap: Vec<H256>, merkle_path: Vec<MergeValue>) -> Self {
        MerkleProof {
            leaves_bitmap,
            merkle_path,
        }
    }

    /// Destruct the structure, useful for serialization
    pub fn take(self) -> (Vec<H256>, Vec<MergeValue>) {
        let MerkleProof {
            leaves_bitmap,
            merkle_path,
        } = self;
        (leaves_bitmap, merkle_path)
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
    pub fn merkle_path(&self) -> &Vec<MergeValue> {
        &self.merkle_path
    }

    pub fn compile(self, leaves: Vec<(H256, H256)>) -> Result<CompiledMerkleProof> {
        let mut leaves_ord: Vec<(H256Ord, H256)> = leaves
            .iter()
            .take(leaves.len())
            .map(|(k, v)| (H256Ord { inner: k.clone() }, v.clone()))
            .collect::<Vec<_>>();
        if leaves_ord.is_empty() {
            return Err(Error::EmptyKeys);
        } else if leaves_ord.len() != self.leaves_count() {
            return Err(Error::IncorrectNumberOfLeaves {
                expected: self.leaves_count(),
                actual: leaves_ord.len(),
            });
        }
        // sort leaves
        leaves_ord.sort_unstable_by_key(|(k, _v)| k.clone());

        let (leaves_bitmap, merkle_path) = self.take();

        let mut proof: Vec<u8> = Vec::with_capacity(merkle_path.len() * 33 + leaves_ord.len());
        let mut stack_fork_height = [0u8; MAX_STACK_SIZE]; // store fork height
        let mut stack_top = 0;
        let mut leaf_index = 0;
        let mut merkle_path_index = 0;
        while leaf_index < leaves_ord.len() {
            let (leaf_key, _value) = leaves_ord[leaf_index].clone();
            let fork_height = if leaf_index + 1 < leaves_ord.len() {
                h256::fork_height(&leaf_key.inner, &leaves_ord[leaf_index + 1].0.inner)
            } else {
                core::u8::MAX
            };
            proof.push(0x4C);
            let mut zero_count = 0u16;
            for height in 0..=fork_height {
                if height == fork_height && leaf_index + 1 < leaves_ord.len() {
                    // If it's not final round, we don't need to merge to root (height=255)
                    break;
                }
                let (op_code_opt, sibling_data_opt): (_, Option<Vec<u8>>) =
                    if stack_top > 0 && stack_fork_height[stack_top - 1] == height {
                        stack_top -= 1;
                        (Some(0x48), None)
                    } else if leaves_bitmap[leaf_index]
                        .bit(height.into())
                        .unwrap_or(false)
                    {
                        if merkle_path_index >= merkle_path.len() {
                            return Err(Error::CorruptedProof);
                        }
                        let node = &merkle_path[merkle_path_index];
                        merkle_path_index += 1;
                        match node {
                            MergeValue::Value(v) => (Some(0x50), Some(v.as_bytes().to_vec())),
                            MergeValue::MergeWithZero {
                                base_node,
                                zero_bits,
                                zero_count,
                            } => {
                                let mut buffer = Vec::new();
                                buffer.push(*zero_count);
                                buffer.extend_from_slice(base_node.as_bytes());
                                buffer.extend_from_slice(zero_bits.as_bytes());
                                (Some(0x51), Some(buffer))
                            }
                        }
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
                if let Some(data) = sibling_data_opt {
                    proof.extend(&data);
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
            debug_assert!(stack_top < MAX_STACK_SIZE);
            stack_fork_height[stack_top] = fork_height;
            stack_top += 1;
            leaf_index += 1;
        }

        if stack_top != 1 {
            return Err(Error::CorruptedProof);
        }
        if leaf_index != leaves_ord.len() {
            return Err(Error::CorruptedProof);
        }
        if merkle_path_index != merkle_path.len() {
            return Err(Error::CorruptedProof);
        }
        Ok(CompiledMerkleProof(proof))
    }

    /// Compute root from proof
    /// leaves: a vector of (key, value)
    ///
    /// return EmptyProof error when proof is empty
    /// return CorruptedProof error when proof is invalid
    pub fn compute_root<H: Hasher + Default>(self, leaves: Vec<(H256, H256)>) -> Result<H256> {
        self.compile(leaves.clone())?.compute_root::<H>(leaves)
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
    pub fn compute_root<H: Hasher + Default>(&self, leaves: Vec<(H256, H256)>) -> Result<H256> {
        let mut leaves_ord: Vec<(H256Ord, H256)> = leaves
            .iter()
            .take(leaves.len())
            .map(|(k, v)| (H256Ord { inner: k.clone() }, v.clone()))
            .collect();
        leaves_ord.sort_unstable_by_key(|(k, _v)| k.clone());
        let mut program_index = 0;
        let mut leaf_index = 0;
        let mut stack: Vec<(u16, H256, MergeValue)> = Vec::new();
        while program_index < self.0.len() {
            let code = self.0[program_index];
            program_index += 1;
            match code {
                // L : push leaf value
                0x4C => {
                    if leaf_index >= leaves_ord.len() {
                        return Err(Error::CorruptedStack);
                    }
                    let (k, v) = leaves_ord[leaf_index].clone();
                    stack.push((0, k.inner, MergeValue::from_h256(v)));
                    leaf_index += 1;
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
                    let sibling_node = MergeValue::from_h256(H256::from(data));
                    let (height_u16, key, value) = stack.pop().unwrap();
                    if height_u16 > 255 {
                        return Err(Error::CorruptedProof);
                    }
                    let height = height_u16 as u8;
                    let parent_key = h256::parent_path(&key, height);
                    let parent = if key.bit(height.into()).unwrap_or(false) {
                        merge::<H>(height, &parent_key, &sibling_node, &value)
                    } else {
                        merge::<H>(height, &parent_key, &value, &sibling_node)
                    };
                    stack.push((height_u16 + 1, parent_key, parent));
                }
                // Q : hash stack top item with sibling node in proof,
                // this is similar to P except that proof comes in using
                // MergeWithZero format.
                0x51 => {
                    if stack.is_empty() {
                        return Err(Error::CorruptedStack);
                    }
                    if program_index + 65 > self.0.len() {
                        return Err(Error::CorruptedProof);
                    }
                    let zero_count = self.0[program_index];
                    let base_node = {
                        let mut data = [0u8; 32];
                        data.copy_from_slice(&self.0[program_index + 1..program_index + 33]);
                        H256::from(data)
                    };
                    let zero_bits = {
                        let mut data = [0u8; 32];
                        data.copy_from_slice(&self.0[program_index + 33..program_index + 65]);
                        H256::from(data)
                    };
                    program_index += 65;
                    let sibling_node = MergeValue::MergeWithZero {
                        base_node,
                        zero_bits,
                        zero_count,
                    };
                    let (height_u16, key, value) = stack.pop().unwrap();
                    if height_u16 > 255 {
                        return Err(Error::CorruptedProof);
                    }
                    let height = height_u16 as u8;
                    let parent_key = h256::parent_path(&key, height);
                    let parent = if key.bit(height.into()).unwrap_or(false) {
                        merge::<H>(height, &parent_key, &sibling_node, &value)
                    } else {
                        merge::<H>(height, &parent_key, &value, &sibling_node)
                    };
                    stack.push((height_u16 + 1, parent_key, parent));
                }
                // H : pop 2 items in stack hash them then push the result
                0x48 => {
                    if stack.len() < 2 {
                        return Err(Error::CorruptedStack);
                    }
                    let (height_b, key_b, value_b) = stack.pop().unwrap();
                    let (height_a, key_a, value_a) = stack.pop().unwrap();
                    if height_a != height_b {
                        return Err(Error::CorruptedProof);
                    }
                    if height_a > 255 {
                        return Err(Error::CorruptedProof);
                    }
                    let height_u16 = height_a;
                    let height = height_u16 as u8;
                    let parent_key_a = h256::parent_path(&key_a, height);
                    let parent_key_b = h256::parent_path(&key_b, height);
                    if parent_key_a != parent_key_b {
                        return Err(Error::CorruptedProof);
                    }
                    let parent = if key_a.bit(height.into()).unwrap_or(false) {
                        merge::<H>(height, &parent_key_a, &value_b, &value_a)
                    } else {
                        merge::<H>(height, &parent_key_a, &value_a, &value_b)
                    };
                    stack.push((height_u16 + 1, parent_key_a, parent));
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
                    if base_height > 255 {
                        return Err(Error::CorruptedProof);
                    }
                    let mut parent_key = key.clone();
                    let mut height_u16 = base_height;
                    for idx in 0..zero_count {
                        if base_height + idx > 255 {
                            return Err(Error::CorruptedProof);
                        }
                        height_u16 = base_height + idx;
                        let height = height_u16 as u8;
                        parent_key = h256::parent_path(&key, height);
                        value = if key.bit(height.into()).unwrap_or(false) {
                            merge::<H>(height, &parent_key, &MergeValue::zero(), &value)
                        } else {
                            merge::<H>(height, &parent_key, &value, &MergeValue::zero())
                        };
                    }
                    stack.push((height_u16 + 1, parent_key, value));
                }
                _ => return Err(Error::InvalidCode(code)),
            }
            debug_assert!(stack.len() <= MAX_STACK_SIZE);
        }
        if stack.len() != 1 {
            return Err(Error::CorruptedStack);
        }
        if stack[0].0 != 256 {
            return Err(Error::CorruptedProof);
        }
        if leaf_index != leaves_ord.len() {
            return Err(Error::CorruptedProof);
        }
        Ok(stack[0].2.hash::<H>())
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
