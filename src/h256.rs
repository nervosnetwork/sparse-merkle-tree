use core::cmp::Ordering;
use numext_fixed_hash;

pub type H256 = numext_fixed_hash::H256;

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

pub fn h256_cmp(v1: &H256, v2: &H256) -> Ordering {
    v1.0.iter().rev().cmp(v2.0.iter().rev())
}
