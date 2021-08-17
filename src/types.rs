use core::cmp::{Eq, Ord, PartialEq, PartialOrd};
use core::convert::{From, Into};
use serde::{Deserialize, Serialize};

/// Key/value of a sparse merkle tree
/// byteorder(bigendian): v[0]=highest,v[31]=lowest
#[derive(Clone, Default, Copy, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "serde-rs", derive(Serialize, Deserialize))]
pub struct H256([u8; 32]);

impl core::ops::Index<usize> for H256 {
    type Output = u8;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

impl core::ops::IndexMut<usize> for H256 {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.0[idx]
    }
}

impl From<[u8; 32]> for H256 {
    fn from(data: [u8; 32]) -> Self {
        Self(data)
    }
}

impl H256 {
    pub fn is_zero(&self) -> bool {
        self.0.iter().fold(0, |a, x| a | x) == 0
    }

    /// shift right 1 bit
    pub fn shr1(&self) -> Self {
        let mut data = self.clone();
        let mut is_least_bit_one = false;
        for i in 0..32 {
            if data[i] & 0x01 == 0x01 {
                data[i] >>= 1;
                if is_least_bit_one {
                    data[i] |= 0x80;
                }
                is_least_bit_one = true;
            } else {
                data[i] >>= 1;
            }
        }
        data
    }

    /// x & 0x01
    pub fn is_least_bit_high(&self) -> bool {
        self.0[31] & 0x01 == 0x01
    }

    /// negate the lowerest bit
    pub fn negate_least_bit(&self) -> H256 {
        let mut data = self.clone();
        if self.is_least_one() {
            data[31] &= 0xfe;
        } else {
            data[31] |= 0x01;
        }
        data
    }

    // pub fn common_prefix(&self, other: &Self) -> Option<Self> {
    // }

    pub fn split(&self) -> (u128, u128) {
        let mut l: [u8; 16] = Default::default();
        l.copy_from_slice(&self.0[..16]);
        let mut r: [u8; 16] = Default::default();
        r.copy_from_slice(&self.0[16..]);
        (u128::from_be_bytes(l), u128::from_be_bytes(r))
    }
}
