use numext_fixed_hash;

use core::cmp::Ordering;

/// Represent 256 bits
#[derive(Eq, PartialEq, Debug, Default, Hash, Clone)]
pub struct H256 {
    inner: numext_fixed_hash::H256,
}

const BYTE_SIZE: u8 = 8;

impl H256 {
    pub const fn empty() -> Self {
        H256 { inner: numext_fixed_hash::H256::empty() }
    }

    pub fn is_zero(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    pub fn get_bit(&self, i: u8) -> bool {
        self.inner.bit(i.into()).unwrap_or(false)
    }

    #[inline]
    pub fn set_bit(&mut self, i: u8) {
        self.inner.set_bit(i.into(), true);
    }

    #[inline]
    pub fn clear_bit(&mut self, i: u8) {
        self.inner.set_bit(i.into(), false);
    }

    #[inline]
    pub fn is_right(&self, height: u8) -> bool {
        self.get_bit(height)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.inner.as_bytes()
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
            H256::empty()
        } else {
            self.copy_bits(height + 1)
        }
    }

    /// Copy bits and return a new H256
    pub fn copy_bits(&self, start: u8) -> Self {
        // It can also be implemented with And, but the performance is not as good as this 
        let mut target = H256::empty();

        let start_byte = (start / BYTE_SIZE) as usize;
        // copy bytes
        target.inner.0[start_byte..].copy_from_slice(&self.inner.0[start_byte..]);

        // reset remain bytes
        let remain = start % BYTE_SIZE;
        if remain > 0 {
            target.inner.0[start_byte] &= 0b11111111 << remain
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
        self.inner.0.iter().rev().cmp(other.inner.0.iter().rev())
    }
}

impl From<[u8; 32]> for H256 {
    fn from(v: [u8; 32]) -> H256 {
        H256{
            inner: numext_fixed_hash::H256::from(v),
        }
    }
}

impl Into<[u8; 32]> for H256 {
    fn into(self: H256) -> [u8; 32] {
        self.inner.0
    }
}
