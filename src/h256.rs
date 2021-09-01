use core::cmp::Ordering;
use numext_fixed_hash;

pub type SmtH256 = numext_fixed_hash::H256;

const BYTE_SIZE: u8 = 8;

pub fn fork_height(data: &SmtH256, key: &SmtH256) -> u8 {
    for h in (0..=core::u8::MAX).rev() {
        if data.bit(h.into()).unwrap_or(false) != key.bit(h.into()).unwrap_or(false) {
            return h;
        }
    }
    0
}

pub fn parent_path(data: &SmtH256, height: u8) -> SmtH256 {
    if height == core::u8::MAX {
        SmtH256::empty()
    } else {
        copy_bits(data, height + 1)
    }
}

/// Copy bits and return a new SmtH256
pub fn copy_bits(data: &SmtH256, start: u8) -> SmtH256 {
    // It can also be implemented with And, but the performance is not as good as this
    let mut target = SmtH256::empty();

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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SmtH256Ord {
    pub inner: SmtH256,
}

impl From<[u8; 32]> for SmtH256Ord {
    fn from(v: [u8; 32]) -> SmtH256Ord {
        SmtH256Ord {
            inner: SmtH256::from(v),
        }
    }
}

impl From<SmtH256> for SmtH256Ord {
    fn from(v: SmtH256) -> SmtH256Ord {
        SmtH256Ord { inner: v }
    }
}

impl From<&SmtH256> for SmtH256Ord {
    fn from(v: &SmtH256) -> SmtH256Ord {
        SmtH256Ord { inner: v.clone() }
    }
}

impl PartialOrd for SmtH256Ord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SmtH256Ord {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare bits from heigher to lower (255..0)
        self.inner.0.iter().rev().cmp(other.inner.0.iter().rev())
    }
}

pub fn h256_cmp(v1: &SmtH256, v2: &SmtH256) -> Ordering {
    v1.0.iter().rev().cmp(v2.0.iter().rev())
}
