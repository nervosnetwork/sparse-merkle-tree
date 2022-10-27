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
    },
    ShortCut {
        key: H256,
        value: H256,
        height: u8,
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
            MergeValue::MergeWithZero {
                base_node,
                zero_bits: _,
                zero_count: _,
            } => base_node.is_zero(),
            MergeValue::ShortCut {
                key: _,
                value,
                height: _,
            } => value.is_zero(),
        }
    }

    pub fn shortcut(key: H256, value: H256, height: u8) -> Self {
        MergeValue::ShortCut { key, value, height }
    }

    pub fn is_shortcut(&self) -> bool {
        match self {
            MergeValue::ShortCut {
                key: _,
                value: _,
                height: _,
            } => true,
            _ => false,
        }
    }

    pub fn hash<H: Hasher + Default>(&self) -> H256 {
        match self {
            MergeValue::Value(v) => *v,
            MergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count,
            } => {
                let mut hasher = H::default();
                hasher.write_byte(MERGE_ZEROS);
                hasher.write_h256(base_node);
                hasher.write_h256(zero_bits);
                hasher.write_byte(*zero_count);
                hasher.finish()
            }
            MergeValue::ShortCut { key: _, value, height: _ } => {
                // try keep hash same with MergeWithZero
                if value.is_zero() {
                    return H256::zero();
                }
                self.into_merge_with_zero::<H>().hash::<H>()
            }
        }
    }

    pub fn base_node<H: Hasher + Default>(&self) -> H256 {
        match self {
            MergeValue::ShortCut { key, value, height: _ } => {
                let base_key = key.parent_path(0);
                hash_base_node::<H>(0, &base_key, value)
            },
            MergeValue::MergeWithZero { base_node, zero_bits: _, zero_count: _ } => {
                *base_node
            },
            MergeValue::Value(value) => {
                *value
            }
        }
    }

    fn into_merge_with_zero<H: Hasher + Default>(&self) -> MergeValue {
        match self {
            MergeValue::ShortCut { key, value, height } => {
                let base_key = key.parent_path(0);
                let base_node = hash_base_node::<H>(0, &base_key, value);
                let mut zero_bits = key.clone();
                for i in *height..=core::u8::MAX {
                    if key.is_right(i) {
                        zero_bits.clear_bit(i);
                    }
                }
                MergeValue::MergeWithZero {
                    base_node,
                    zero_bits,
                    zero_count: *height,
                }

            },
            MergeValue::MergeWithZero { base_node:_, zero_bits: _, zero_count: _ } => {
                self.clone()
            },
            MergeValue::Value(_) => {
                unreachable!();
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

fn merge_with_zero<H: Hasher + Default>(
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
            }
        }
        MergeValue::MergeWithZero {
            base_node,
            zero_bits,
            zero_count,
        } => {
            let mut zero_bits = *zero_bits;
            if set_bit {
                zero_bits.set_bit(height);
            }
            MergeValue::MergeWithZero {
                base_node: *base_node,
                zero_bits,
                zero_count: zero_count.wrapping_add(1),
            }
        }
        MergeValue::ShortCut {
            key,
            value,
            height: _,
        } => {
            if height == core::u8::MAX {
                let base_key = key.parent_path(0);

                let base_node = hash_base_node::<H>(0, &base_key, value);

                let mut zero_bits = key.clone();
                for i in height..=core::u8::MAX {
                    if key.is_right(i) {
                        zero_bits.clear_bit(i);
                    }
                }
                MergeValue::MergeWithZero {
                    base_node,
                    zero_bits,
                    zero_count: 0,
                }
            }
             else {
                MergeValue::shortcut(*key, *value, height + 1)
            }
        }
    }
}
