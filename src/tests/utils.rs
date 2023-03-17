use proptest::{
    prelude::prop,
    strategy::{Just, Strategy},
};

use crate::{
    blake2b::Blake2bHasher, default_store::DefaultStore, merge::MergeValue, SparseMerkleTree, H256,
};

#[allow(clippy::upper_case_acronyms)]
pub type SMT = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;

pub fn new_smt(pairs: Vec<(H256, H256)>) -> SMT {
    let mut smt = SMT::default();
    for (key, value) in pairs {
        smt.update(key, value).unwrap();
    }
    smt
}

pub fn leaves(
    min_leaves: usize,
    max_leaves: usize,
) -> impl Strategy<Value = (Vec<(H256, H256)>, usize)> {
    prop::collection::vec(
        prop::array::uniform2(prop::array::uniform32(0u8..)),
        min_leaves..=max_leaves,
    )
    .prop_flat_map(|mut pairs| {
        pairs.dedup_by_key(|[k, _v]| *k);
        let len = pairs.len();
        (
            Just(
                pairs
                    .into_iter()
                    .map(|[k, v]| (k.into(), v.into()))
                    .collect(),
            ),
            core::cmp::min(1, len)..=len,
        )
    })
}

pub fn leaves_bitmap(max_leaves_bitmap: usize) -> impl Strategy<Value = Vec<H256>> {
    prop::collection::vec(prop::array::uniform32(0u8..), max_leaves_bitmap).prop_flat_map(
        |leaves_bitmap| Just(leaves_bitmap.into_iter().map(|item| item.into()).collect()),
    )
}

pub fn merkle_proof(max_proof: usize) -> impl Strategy<Value = Vec<MergeValue>> {
    prop::collection::vec(prop::array::uniform32(0u8..), max_proof).prop_flat_map(|proof| {
        Just(
            proof
                .into_iter()
                .map(|item| MergeValue::from_h256(item.into()))
                .collect(),
        )
    })
}
