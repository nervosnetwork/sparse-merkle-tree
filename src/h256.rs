use core::cmp::Ordering;
use numext_fixed_hash;

pub(crate) type H256 = numext_fixed_hash::H256;

const BYTE_SIZE: u8 = 8;

pub fn fork_height(data: &H256, key: &H256) -> u8 {
    for h in (0..=core::u8::MAX).rev() {
        if data.bit(h.into()).unwrap_or(false) != key.bit(h.into()).unwrap_or(false) {
            return h;
        }
    }
    0
}

pub fn parent_path(data: &H256, height: u8) -> H256 {
    if height == core::u8::MAX {
        H256::empty()
    } else {
        copy_bits(data, height + 1)
    }
}

/// Copy bits and return a new H256
pub fn copy_bits(data: &H256, start: u8) -> H256 {
    // It can also be implemented with And, but the performance is not as good as this
    let mut target = H256::empty();

    let start_byte = (start / BYTE_SIZE) as usize;
    // copy bytes
    target.0[start_byte..].copy_from_slice(&data.0[start_byte..]);

    // reset remain bytes
    let remain = start % BYTE_SIZE;
    if remain > 0 {
        target.0[start_byte] &= 0b11111111 << remain
    }

    target
}

#[derive(Eq, PartialEq, Debug, Default, Hash, Clone)]
pub(crate) struct H256Ord {
    pub inner: H256,
}

impl From<[u8; 32]> for H256Ord {
    fn from(v: [u8; 32]) -> H256Ord {
        H256Ord {
            inner: H256::from(v),
        }
    }
}

impl From<H256> for H256Ord {
    fn from(v: H256) -> H256Ord {
        H256Ord { inner: v }
    }
}

impl From<&H256> for H256Ord {
    fn from(v: &H256) -> H256Ord {
        H256Ord { inner: v.clone() }
    }
}

impl PartialOrd for H256Ord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for H256Ord {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare bits from heigher to lower (255..0)
        self.inner.0.iter().rev().cmp(other.inner.0.iter().rev())
    }
}
