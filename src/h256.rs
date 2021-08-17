use core::cmp::Ordering;

cfg_if::cfg_if! {
    if #[cfg(feature = "serde-rs")] {
        use serde::{Deserialize, Serialize};
    }
}
cfg_if::cfg_if! {
    if #[cfg(feature = "sha256")] {
        use generic_array::{typenum::U32, GenericArray};
    }
}

/// Represent 256 bits
#[cfg_attr(feature = "serde-rs", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Debug, Default, Hash, Clone, Copy)]
pub struct H256([u8; 32]);

const ZERO: H256 = H256([0u8; 32]);
const BYTE_SIZE: u8 = 8;

impl H256 {
    pub const fn zero() -> Self {
        ZERO
    }

    pub fn is_zero(&self) -> bool {
        self == &ZERO
    }

    #[inline]
    pub fn get_bit(&self, i: u8) -> bool {
        let byte_pos = i / BYTE_SIZE;
        let bit_pos = i % BYTE_SIZE;
        let bit = self.0[byte_pos as usize] >> bit_pos & 1;
        bit != 0
    }

    #[inline]
    pub fn set_bit(&mut self, i: u8) {
        let byte_pos = i / BYTE_SIZE;
        let bit_pos = i % BYTE_SIZE;
        self.0[byte_pos as usize] |= 1 << bit_pos as u8;
    }

    #[inline]
    pub fn clear_bit(&mut self, i: u8) {
        let byte_pos = i / BYTE_SIZE;
        let bit_pos = i % BYTE_SIZE;
        self.0[byte_pos as usize] &= !((1 << bit_pos) as u8);
    }

    #[inline]
    pub fn is_right(&self, height: u8) -> bool {
        self.get_bit(height)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0[..]
    }

    /// Treat H256 as a path in a tree
    /// fork height is the number of common bits(from heigher to lower: 255..=0) of two H256
    pub fn fork_height(&self, key: &H256) -> u8 {
        for h in (0..=core::u8::MAX).rev() {
            if self.get_bit(h) != key.get_bit(h) {
                return h;
            }
        }
        0
    }

    /// Treat H256 as a path in a tree
    /// return parent_path of self
    pub fn parent_path(&self, height: u8) -> Self {
        if height == core::u8::MAX {
            H256::zero()
        } else {
            self.copy_bits(height + 1)
        }
    }

    /// Copy bits and return a new H256
    pub fn copy_bits(&self, start: u8) -> Self {
        let mut target = H256::zero();

        let start_byte = (start / BYTE_SIZE) as usize;
        // copy bytes
        target.0[start_byte..].copy_from_slice(&self.0[start_byte..]);

        // reset remain bytes
        let remain = start % BYTE_SIZE;
        if remain > 0 {
            target.0[start_byte] &= 0b11111111 << remain
        }

        target
    }
}

impl PartialOrd for H256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for H256 {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare bits from heigher to lower (255..0)
        self.0.iter().rev().cmp(other.0.iter().rev())
    }
}

impl From<[u8; 32]> for H256 {
    fn from(v: [u8; 32]) -> H256 {
        H256(v)
    }
}

impl Into<[u8; 32]> for H256 {
    fn into(self: H256) -> [u8; 32] {
        self.0
    }
}

#[cfg(feature = "sha256")]
impl From<generic_array::GenericArray<u8, generic_array::typenum::U32>> for H256 {
    fn from(v: GenericArray<u8, U32>) -> H256 {
        H256(v.into())
    }
}
