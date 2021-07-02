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
    proof: Vec<H256>,
}

impl MerkleProof {
    /// Create MerkleProof
    /// leaves_bitmap: leaf bitmap, bitmap.get_bit(height) is true means there need a non zero sibling in this height
    /// proof: needed sibling node hash
    pub fn new(leaves_bitmap: Vec<H256>, proof: Vec<H256>) -> Self {
        MerkleProof {
            leaves_bitmap,
            proof,
        }
    }

    /// Destruct the structure, useful for serialization
    pub fn take(self) -> (Vec<H256>, Vec<H256>) {
        let MerkleProof {
            leaves_bitmap,
            proof,
        } = self;
        (leaves_bitmap, proof)
    }

    /// number of leaves required by this merkle proof
    pub fn leaves_count(&self) -> usize {
        self.leaves_bitmap.len()
    }

    /// return the inner leaves_bitmap vector
    pub fn leaves_bitmap(&self) -> &Vec<H256> {
        &self.leaves_bitmap
    }

    /// return proof merkle path
    pub fn proof(&self) -> &Vec<H256> {
        &self.proof
    }

    /// convert merkle proof into CompiledMerkleProof
    pub fn compile(self) -> CompiledMerkleProof {
        let (leaves_bitmap, proof) = self.take();
        let leaves_len = leaves_bitmap.len();
        let mut data = vec![0u8; (leaves_len + proof.len()) * 32];
        for (idx, bitmap) in leaves_bitmap.into_iter().enumerate() {
            let offset = idx * 32;
            data[offset..offset + 32].copy_from_slice(bitmap.as_slice());
        }
        for (idx, sibling_node_hash) in proof.into_iter().enumerate() {
            let offset = (leaves_len + idx) * 32;
            data[offset..offset + 32].copy_from_slice(sibling_node_hash.as_slice());
        }
        CompiledMerkleProof(data)
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

        let (leaves_bitmap, proof) = self.take();

        let mut stack_fork_height = vec![0u8; 256]; // store fork height
        let mut stack = vec![H256::zero(); 256]; // store node hash
        let mut stack_top = 0;
        let mut leaf_index = 0;
        let mut proof_index = 0;
        while leaf_index < leaves.len() {
            let (leaf_key, value) = leaves[leaf_index];
            let fork_height = if leaf_index + 1 < leaves.len() {
                leaf_key.fork_height(&leaves[leaf_index + 1].0)
            } else {
                255
            };
            let mut current_node = hash_leaf::<H>(&leaf_key, &value);
            for height in 0..=fork_height {
                if height == fork_height && leaf_index + 1 < leaves.len() {
                    break;
                }
                let parent_key = leaf_key.parent_path(height);
                let is_right = leaf_key.is_right(height);
                let sibling_node = if stack_top > 0 && stack_fork_height[stack_top - 1] == height {
                    let node_hash = stack[stack_top - 1];
                    stack_top -= 1;
                    node_hash
                } else if leaves_bitmap[leaf_index].get_bit(height) {
                    if proof_index >= proof.len() {
                        return Err(Error::CorruptedProof);
                    }
                    let node_hash = proof[proof_index];
                    proof_index += 1;
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
        if proof_index != proof.len() {
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

/// An structure for verify merkle proof by raw binary
#[derive(Debug, Clone)]
pub struct CompiledMerkleProof(pub Vec<u8>);

impl CompiledMerkleProof {
    pub fn compute_root<H: Hasher + Default>(&self, leaves: Vec<(H256, H256)>) -> Result<H256> {
        if self.0.len() % 32 != 0 {
            return Err(Error::CorruptedProof);
        }
        if self.0.len() / 32 < leaves.len() {
            return Err(Error::CorruptedProof);
        }

        let sibling_node_size = self.0.len() / 32 - leaves.len();
        let mut data = [0u8; 32];
        let mut leaves_bitmap = Vec::with_capacity(leaves.len());
        let mut proof = Vec::with_capacity(sibling_node_size);
        for idx in 0..leaves.len() {
            let offset = idx * 32;
            data.copy_from_slice(&self.0[offset..offset + 32]);
            leaves_bitmap.push(H256::from(data));
        }
        for idx in 0..sibling_node_size {
            let offset = (idx + leaves.len()) * 32;
            data.copy_from_slice(&self.0[offset..offset + 32]);
            proof.push(H256::from(data));
        }
        MerkleProof::new(leaves_bitmap, proof).compute_root::<H>(leaves)
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
