use crate::h256::H256;
use crate::traits::Hasher;

const MERGE_NORMAL: u8 = 1;
const MERGE_ZEROS: u8 = 2;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum MergeValue {
    Value(H256),
    MergeWithZero {
        base_node: H256,
        zero_bits: H256,
        zero_count: u8,
        value: H256,
    },
}

impl MergeValue {
    pub fn from_h256(v: H256) -> Self {
        MergeValue::Value(v)
    }

    pub fn zero() -> Self {
        MergeValue::Value(H256::zero())
    }

    pub fn is_zero(&self) -> bool {
        match self {
            MergeValue::Value(v) => v.is_zero(),
            MergeValue::MergeWithZero { .. } => false,
        }
    }

    pub fn dec_zero(&mut self, height: u8) {
        if let MergeValue::MergeWithZero {
            base_node: _,
            zero_bits,
            zero_count,
            value: _,
        } = self
        {
            zero_bits.clear_bit(height);
            *zero_count -= 1;
        }
    }

    pub fn hash<H: Hasher + Default>(&self) -> H256 {
        match self {
            MergeValue::Value(v) => *v,
            MergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count,
                value: _,
            } => {
                let mut hasher = H::default();
                hasher.write_byte(MERGE_ZEROS);
                hasher.write_h256(base_node);
                hasher.write_h256(zero_bits);
                hasher.write_byte(*zero_count);
                hasher.finish()
            }
        }
    }
}

/// Hash base node into a H256
pub fn hash_base_node<H: Hasher + Default>(
    base_height: u8,
    base_key: &H256,
    base_value: &H256,
) -> H256 {
    let mut hasher = H::default();
    hasher.write_byte(base_height);
    hasher.write_h256(base_key);
    hasher.write_h256(base_value);
    hasher.finish()
}

/// Merge two hash with node information
/// this function optimized for ZERO_HASH
/// if lhs and rhs both are ZERO_HASH return ZERO_HASH, otherwise hash all info.
pub fn merge<H: Hasher + Default>(
    height: u8,
    node_key: &H256,
    lhs: &MergeValue,
    rhs: &MergeValue,
) -> MergeValue {
    if lhs.is_zero() && rhs.is_zero() {
        return MergeValue::zero();
    }
    if lhs.is_zero() {
        return merge_with_zero::<H>(height, node_key, rhs, true);
    }
    if rhs.is_zero() {
        return merge_with_zero::<H>(height, node_key, lhs, false);
    }
    let mut hasher = H::default();
    hasher.write_byte(MERGE_NORMAL);
    hasher.write_byte(height);
    hasher.write_h256(node_key);
    hasher.write_h256(&lhs.hash::<H>());
    hasher.write_h256(&rhs.hash::<H>());
    MergeValue::Value(hasher.finish())
}

/// Merge two hash with node information
/// this function optimized for ZERO_HASH
/// if lhs and rhs both are ZERO_HASH return ZERO_HASH, otherwise hash all info.
pub fn _merge<H: Hasher + Default>(
    height: u8,
    node_key: &H256,
    lhs: &MergeValue,
    rhs: &MergeValue,
) -> MergeValue {
    if lhs.is_zero() && rhs.is_zero() {
        return MergeValue::zero();
    }
    if lhs.is_zero() {
        return merge_with_zero::<H>(height, node_key, rhs, true);
    }
    if rhs.is_zero() {
        return merge_with_zero::<H>(height, node_key, lhs, false);
    }
    let mut hasher = H::default();
    hasher.write_byte(MERGE_NORMAL);
    hasher.write_byte(height);
    hasher.write_h256(node_key);
    hasher.write_h256(&lhs.hash::<H>());
    hasher.write_h256(&rhs.hash::<H>());
    MergeValue::Value(hasher.finish())
}

pub fn merge_with_zero<H: Hasher + Default>(
    height: u8,
    node_key: &H256,
    value: &MergeValue,
    set_bit: bool,
) -> MergeValue {
    match value {
        MergeValue::Value(v) => {
            let mut zero_bits = H256::zero();
            if set_bit {
                zero_bits.set_bit(height);
            }
            let base_node = hash_base_node::<H>(height, node_key, v);
            MergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count: 1,
                value: *v,
            }
        }
        MergeValue::MergeWithZero {
            base_node,
            zero_bits,
            zero_count,
            value,
        } => {
            let mut zero_bits = *zero_bits;
            if set_bit {
                zero_bits.set_bit(height);
            }
            MergeValue::MergeWithZero {
                base_node: *base_node,
                zero_bits,
                zero_count: zero_count.wrapping_add(1),
                value: *value,
            }
        }
    }
}

/// Merge node with zeros
pub fn merge_with_zeros<H: Hasher + Default>(
    key: H256,
    value: MergeValue,
    target_height: u8,
    zeros: u8,
) -> MergeValue {
    if target_height == 0 || zeros == 0 || value.is_zero() {
        return value;
    }
    let base_height = target_height - zeros;
    match value {
        MergeValue::Value(value) => {
            let base_key = key.parent_path(base_height);
            let base_node = hash_base_node::<H>(base_height, &base_key, &value);
            let mut zero_bits = H256::zero();
            for i in base_height..target_height {
                if key.get_bit(i) {
                    zero_bits.set_bit(i);
                }
            }
            MergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count: zeros,
                value,
            }
        }
        MergeValue::MergeWithZero {
            base_node,
            zero_bits,
            zero_count,
            value,
        } => {
            let mut zero_bits = zero_bits;
            for i in base_height..target_height {
                if key.get_bit(i) {
                    zero_bits.set_bit(i);
                }
            }
            MergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count: zero_count.wrapping_add(zeros),
                value,
            }
        }
    }
}
