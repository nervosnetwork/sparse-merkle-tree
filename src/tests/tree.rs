use crate::*;
use crate::{
    blake2b::Blake2bHasher, default_store::DefaultStore, error::Error, merge::MergeValue,
    MerkleProof,
};
use proptest::prelude::*;
use rand::prelude::{Rng, SliceRandom};
use std::collections::HashMap;

#[allow(clippy::upper_case_acronyms)]
type SMT = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;

#[test]
fn test_default_root() {
    let mut tree = SMT::default();
    assert_eq!(tree.store().branches_map().len(), 0);
    assert_eq!(tree.store().leaves_map().len(), 0);
    assert_eq!(tree.root(), &H256::zero());

    // insert a key-value
    tree.update(H256::zero(), [42u8; 32].into())
        .expect("update");
    assert_ne!(tree.root(), &H256::zero());
    assert_ne!(tree.store().branches_map().len(), 0);
    assert_ne!(tree.store().leaves_map().len(), 0);
    assert_eq!(tree.get(&H256::zero()).expect("get"), [42u8; 32].into());
    // update zero is to delete the key
    tree.update(H256::zero(), H256::zero()).expect("update");
    assert_eq!(tree.root(), &H256::zero());
    assert_eq!(tree.get(&H256::zero()).expect("get"), H256::zero());
}

#[test]
fn test_default_tree() {
    let tree = SMT::default();
    assert_eq!(tree.get(&H256::zero()).expect("get"), H256::zero());
    let proof = tree.merkle_proof(vec![H256::zero()]).expect("merkle proof");
    let root = proof
        .compute_root::<Blake2bHasher>(vec![(H256::zero(), H256::zero())])
        .expect("root");
    assert_eq!(&root, tree.root());
    let proof = tree.merkle_proof(vec![H256::zero()]).expect("merkle proof");
    let root2 = proof
        .compute_root::<Blake2bHasher>(vec![(H256::zero(), [42u8; 32].into())])
        .expect("root");
    assert_ne!(&root2, tree.root());
}

#[test]
fn test_default_merkle_proof() {
    let proof = MerkleProof::new(Default::default(), Default::default());
    let result = proof.compute_root::<Blake2bHasher>(vec![([42u8; 32].into(), [42u8; 32].into())]);
    assert_eq!(
        result.unwrap_err(),
        Error::IncorrectNumberOfLeaves {
            expected: 0,
            actual: 1
        }
    );

    // FIXME: makes room for leaves
    // let proof = MerkleProof::new(vec![Vec::new()], Default::default());
    // let root = proof
    //     .compute_root::<Blake2bHasher>(vec![([42u8; 32].into(), [42u8; 32].into())])
    //     .expect("compute root");
    // assert_ne!(root, H256::zero());
}

#[test]
fn test_merkle_root() {
    fn new_blake2b() -> crate::blake2b::Blake2b {
        crate::blake2b::Blake2bBuilder::new(32)
            .personal(b"SMT")
            .build()
    }

    let mut tree = SMT::default();
    for (i, word) in "The quick brown fox jumps over the lazy dog"
        .split_whitespace()
        .enumerate()
    {
        let key: H256 = {
            let mut buf = [0u8; 32];
            let mut hasher = new_blake2b();
            hasher.update(&(i as u32).to_le_bytes());
            hasher.finalize(&mut buf);
            buf.into()
        };
        let value: H256 = {
            let mut buf = [0u8; 32];
            let mut hasher = new_blake2b();
            hasher.update(word.as_bytes());
            hasher.finalize(&mut buf);
            buf.into()
        };
        tree.update(key, value).expect("update");
    }

    let expected_root: H256 = [
        209, 214, 1, 128, 166, 207, 49, 89, 206, 78, 169, 88, 18, 243, 130, 61, 150, 45, 43, 54,
        208, 20, 237, 20, 98, 69, 130, 120, 241, 169, 248, 211,
    ]
    .into();
    assert_eq!(tree.store().leaves_map().len(), 9);
    assert_eq!(tree.root(), &expected_root);
}

#[test]
fn test_zero_value_donot_change_root() {
    let mut tree = SMT::default();
    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    let value = H256::zero();
    tree.update(key, value).unwrap();
    assert_eq!(tree.root(), &H256::zero());
    assert_eq!(tree.store().leaves_map().len(), 0);
    assert_eq!(tree.store().branches_map().len(), 0);
}

#[test]
fn test_zero_value_donot_change_store() {
    let mut tree = SMT::default();
    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let value = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    tree.update(key, value).unwrap();
    assert_ne!(tree.root(), &H256::zero());
    let root = *tree.root();
    let store = tree.store().clone();

    // insert a zero value leaf
    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    let value = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    tree.update(key, value).unwrap();
    assert_eq!(tree.root(), &root);
    assert_eq!(tree.store().leaves_map(), store.leaves_map());
    assert_eq!(tree.store().branches_map(), store.branches_map());
}

#[test]
fn test_delete_a_leaf() {
    let mut tree = SMT::default();
    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let value = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    tree.update(key, value).unwrap();
    assert_ne!(tree.root(), &H256::zero());

    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 2,
    ]
    .into();

    tree.update(key, value).unwrap();

    let root = *tree.root();
    let store = tree.store().clone();

    // insert a leaf
    let key = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    let value = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    tree.update(key, value).unwrap();
    assert_ne!(tree.root(), &root);

    // delete a leaf
    tree.update(key, H256::zero()).unwrap();
    assert_eq!(tree.root(), &root);
    assert_eq!(tree.store().leaves_map(), store.leaves_map());
    assert_eq!(tree.store().branches_map(), store.branches_map());
}

#[test]
fn test_sibling_key_get() {
    {
        let mut tree = SMT::default();
        let key = H256::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let value = H256::from([1u8; 32]);
        tree.update(key, value).expect("update");

        let sibling_key = H256::from([
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        // get non exists sibling key should return zero value;
        assert_eq!(H256::zero(), tree.get(&sibling_key).unwrap());
    }

    {
        let mut tree = SMT::default();
        let key = H256::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let value = H256::from([1u8; 32]);
        tree.update(key, value).expect("update");

        let sibling_key = H256::from([
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let sibling_value = H256::from([2u8; 32]);
        tree.update(sibling_key, sibling_value).expect("update");
        // get sibling key should return corresponding value
        assert_eq!(value, tree.get(&key).unwrap());
        assert_eq!(sibling_value, tree.get(&sibling_key).unwrap());
    }
}

fn test_construct(key: H256, value: H256) {
    // insert same value to sibling key will construct a different root

    let mut tree = SMT::default();
    tree.update(key, value).expect("update");

    let mut sibling_key = key;
    if sibling_key.get_bit(0) {
        sibling_key.clear_bit(0);
    } else {
        sibling_key.set_bit(0);
    }
    let mut tree2 = SMT::default();
    tree2.update(sibling_key, value).expect("update");
    assert_ne!(tree.root(), tree2.root());
}

fn test_update(key: H256, value: H256) {
    let mut tree = SMT::default();
    tree.update(key, value).expect("update");
    assert_eq!(tree.get(&key), Ok(value));
}

#[cfg(not(feature = "trie"))]
fn test_update_tree_store(key: H256, value: H256, value2: H256) {
    const EXPECTED_LEAVES_LEN: usize = 1;

    let mut tree = SMT::default();
    tree.update(key, value).expect("update");
    assert_eq!(tree.store().branches_map().len(), 256);
    assert_eq!(tree.store().leaves_map().len(), EXPECTED_LEAVES_LEN);
    tree.update(key, value2).expect("update");
    assert_eq!(tree.store().branches_map().len(), 256);
    assert_eq!(tree.store().leaves_map().len(), EXPECTED_LEAVES_LEN);
    assert_eq!(tree.get(&key), Ok(value2));
}

fn test_merkle_proof(key: H256, value: H256) {
    const EXPECTED_MERKLE_PATH_SIZE: usize = 1;

    let mut tree = SMT::default();
    tree.update(key, value).expect("update");
    if !tree.is_empty() {
        let proof = tree.merkle_proof(vec![key]).expect("proof");
        let compiled_proof = proof.clone().compile(vec![key]).expect("compile proof");
        assert!(proof.merkle_path().len() < EXPECTED_MERKLE_PATH_SIZE);
        assert!(proof
            .verify::<Blake2bHasher>(tree.root(), vec![(key, value)])
            .expect("verify"));
        assert!(compiled_proof
            .verify::<Blake2bHasher>(tree.root(), vec![(key, value)])
            .expect("compiled verify"));

        let single_compiled_proof = compiled_proof
            .extract_proof::<Blake2bHasher>(vec![(key, value, true)])
            .expect("compute one proof");
        assert!(single_compiled_proof
            .verify::<Blake2bHasher>(tree.root(), vec![(key, value)])
            .expect("verify compiled proof"));
    }
}

fn new_smt(pairs: Vec<(H256, H256)>) -> SMT {
    let mut smt = SMT::default();
    for (key, value) in pairs {
        smt.update(key, value).unwrap();
    }
    smt
}

fn leaves(
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

fn leaves_bitmap(max_leaves_bitmap: usize) -> impl Strategy<Value = Vec<H256>> {
    prop::collection::vec(prop::array::uniform32(0u8..), max_leaves_bitmap).prop_flat_map(
        |leaves_bitmap| Just(leaves_bitmap.into_iter().map(|item| item.into()).collect()),
    )
}

fn merkle_proof(max_proof: usize) -> impl Strategy<Value = Vec<MergeValue>> {
    prop::collection::vec(prop::array::uniform32(0u8..), max_proof).prop_flat_map(|proof| {
        Just(
            proof
                .into_iter()
                .map(|item| MergeValue::from_h256(item.into()))
                .collect(),
        )
    })
}

proptest! {
    #[test]
    fn test_h256(key: [u8; 32], key2: [u8; 32]) {
        let mut list1: Vec<H256> = vec![key.into() , key2.into()];
        let mut list2 = list1.clone();
        // sort H256
        list1.sort_unstable_by_key(|k| *k);
        // sort by high bits to lower bits
        list2.sort_unstable_by(|k1, k2| {
            for i in (0u8..=255).rev() {
                let b1 = if k1.get_bit(i) { 1 } else { 0 };
                let b2 = if k2.get_bit(i) { 1 } else { 0 };
                let o = b1.cmp(&b2);
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            std::cmp::Ordering::Equal
        });
        assert_eq!(list1, list2);
    }

    #[test]
    fn test_h256_copy_bits(start: u8) {
        let one: H256 = [255u8; 32].into();
        let target = one.copy_bits(start);
        for i in start..=core::u8::MAX {
            assert_eq!(one.get_bit(i), target.get_bit(i));
        }
        for i in 0..start {
            assert!(!target.get_bit(i));
        }
    }

    #[test]
    fn test_random_update(key: [u8; 32], value: [u8;32]) {
        test_update(key.into(), value.into());
    }

    #[cfg(not(feature = "trie"))]
    #[test]
    fn test_random_update_tree_store(key: [u8;32], value: [u8;32], value2: [u8;32]) {
        test_update_tree_store(key.into(), value.into(), value2.into());
    }

    #[test]
    fn test_random_construct(key: [u8;32], value: [u8;32]) {
        test_construct(key.into(), value.into());
    }

    #[test]
    fn test_random_merkle_proof(key: [u8; 32], value: [u8;32]) {
        test_merkle_proof(key.into(), value.into());
    }

    #[test]
    fn test_smt_single_leaf_small((pairs, _n) in leaves(1, 50)){
        let smt = new_smt(pairs.clone());
        for (k, v) in pairs {
            let proof = smt.merkle_proof(vec![k]).expect("gen proof");
            let compiled_proof = proof.clone().compile(vec![k]).expect("compile proof");
            assert!(proof.verify::<Blake2bHasher>(smt.root(), vec![(k, v)]).expect("verify proof"));
            assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), vec![(k, v)]).expect("verify compiled proof"));

            let single_compiled_proof = compiled_proof
                .extract_proof::<Blake2bHasher>(vec![(k, v, true)])
                .expect("compute one proof");
            assert!(single_compiled_proof.verify::<Blake2bHasher>(smt.root(), vec![(k, v)]).expect("verify compiled one proof"));
        }
    }

    #[test]
    fn test_smt_single_leaf_large((pairs, _n) in leaves(50, 100)){
        let smt = new_smt(pairs.clone());
        for (k, v) in pairs {
            let proof = smt.merkle_proof(vec![k]).expect("gen proof");
            let compiled_proof = proof.clone().compile(vec![k]).expect("compile proof");
            assert!(proof.verify::<Blake2bHasher>(smt.root(), vec![(k, v)]).expect("verify proof"));
            assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), vec![(k, v)]).expect("verify compiled proof"));

            let single_compiled_proof = compiled_proof
                .extract_proof::<Blake2bHasher>(vec![(k, v, true)])
                .expect("compute one proof");
            assert!(single_compiled_proof.verify::<Blake2bHasher>(smt.root(), vec![(k, v)]).expect("verify compiled one proof"));
        }
    }

    #[test]
    fn test_smt_multi_leaves_small((pairs, n) in leaves(1, 50)){
        let smt = new_smt(pairs.clone());
        let keys: Vec<_> = pairs.iter().take(n).map(|(k, _v)| *k).collect();
        let proof = smt.merkle_proof(keys.clone()).expect("gen proof");
        let data: Vec<(H256, H256)> = pairs.into_iter().take(n).collect();
        let compiled_proof = proof.clone().compile(keys).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify compiled proof"));

        test_sub_proof(&compiled_proof, &smt, &data, 20);
    }

    #[test]
    fn test_smt_multi_leaves_large((pairs, _n) in leaves(50, 100)){
        let n = 20;
        let smt = new_smt(pairs.clone());
        let keys: Vec<_> = pairs.iter().take(n).map(|(k, _v)| *k).collect();
        let proof = smt.merkle_proof(keys.clone()).expect("gen proof");
        let data: Vec<(H256, H256)> = pairs.into_iter().take(n).collect();
        let compiled_proof = proof.clone().compile(keys).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify compiled proof"));

        test_sub_proof(&compiled_proof, &smt, &data, 20);
    }

    #[test]
    fn test_smt_non_exists_leaves((pairs, _n) in leaves(1, 20), (pairs2, _n2) in leaves(1, 5)){
        if pairs.iter().all(|(k, _v)| pairs2.iter().all(|(k2, _v2)| k2 != k)) {
            let smt = new_smt(pairs);
            let non_exists_keys: Vec<_> = pairs2.into_iter().map(|(k, _v)|k).collect();
            let proof = smt.merkle_proof(non_exists_keys.clone()).expect("gen proof");
            let data: Vec<(H256, H256)> = non_exists_keys.iter().map(|k|(*k, H256::zero())).collect();
            let compiled_proof = proof.clone().compile(non_exists_keys).expect("compile proof");
            assert!(proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify proof"));
            assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify compiled proof"));

            test_sub_proof(&compiled_proof, &smt, &data, 20);
        }
    }

    #[test]
    fn test_smt_non_existssss_leaves_mix((pairs, _n) in leaves(1, 20), (pairs2, _n2) in leaves(1, 5)){
        if pairs.iter().all(|(k, _v)| pairs2.iter().all(|(k2, _v2)| k2 != k)) {
            let smt = new_smt(pairs.clone());
            let exists_keys: Vec<_> = pairs.into_iter().map(|(k, _v)|k).collect();
            let non_exists_keys: Vec<_> = pairs2.into_iter().map(|(k, _v)|k).collect();
            let exists_keys_len = std::cmp::max(exists_keys.len() / 2, 1);
            let non_exists_keys_len = std::cmp::max(non_exists_keys.len() / 2, 1);
            let mut keys: Vec<_> = exists_keys.into_iter().take(exists_keys_len).chain(non_exists_keys.into_iter().take(non_exists_keys_len)).collect();
            keys.dedup();
            let proof = smt.merkle_proof(keys.clone()).expect("gen proof");
            let data: Vec<(H256, H256)> = keys.iter().map(|k|(*k, smt.get(k).expect("get"))).collect();
            let compiled_proof = proof.clone().compile(keys.clone()).expect("compile proof");
            assert!(proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify proof"));
            assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify compiled proof"));

            test_sub_proof(&compiled_proof, &smt, &data, 20);
        }
    }

    #[test]
    fn test_update_smt_tree_store((pairs, n) in leaves(1, 20)) {
        let smt = new_smt(pairs.clone());
        for (k, v) in pairs.into_iter().take(n) {
            assert_eq!(smt.get(&k), Ok(v));
        }
    }

    #[test]
    fn test_from_store((pairs, _n) in leaves(1, 20)) {
        let smt = new_smt(pairs.clone());
        let smt2 = SMT::new_with_store(smt.store().clone()).expect("from store");
        assert_eq!(smt.root(), smt2.root());
    }

    #[test]
    fn test_smt_update_all((pairs, _n) in leaves(1, 20), (pairs2, _n2) in leaves(1, 10)){
        let mut smt = new_smt(pairs.clone());
        for (k, v) in pairs2.clone().into_iter() {
            smt.update(k, v).expect("update");
        }
        let mut smt2 = new_smt(pairs);
        smt2.update_all(pairs2).expect("update all");
        assert_eq!(smt.root(), smt2.root());
    }

    #[test]
    fn test_smt_random_insert_order((pairs, _n) in leaves(5, 50)){
        let smt = new_smt(pairs.clone());
        let root = *smt.root();

        let mut pairs = pairs;
        let mut rng = rand::thread_rng();
        for _i in 0..5 {
            // shuffle
            pairs.shuffle(&mut rng);

            // insert to smt in random order
            let smt2 = new_smt(pairs.clone());
            assert_eq!(root, *smt2.root());

            // check leaves
            for (k, v) in &pairs {
                assert_eq!(&smt2.get(k).unwrap(), v, "key value must be consisted");

                let origin_proof = smt.merkle_proof(vec![*k]).unwrap();
                let proof = smt2.merkle_proof(vec![*k]).unwrap();
                assert_eq!(origin_proof, proof, "merkle proof must be consisted");

                let calculated_root = proof.compute_root::<Blake2bHasher>(vec![(*k, *v)]).unwrap();
                assert_eq!(root, calculated_root, "root must be consisted");
            }
        }
    }

    #[test]
    fn test_smt_update_with_zero_values((pairs, _n) in leaves(5, 30)){
        let mut rng = rand::thread_rng();
        let len =  rng.gen_range(0..pairs.len());
        let mut smt = new_smt(pairs[..len].to_vec());
        let root = *smt.root();

        // insert zero values
        for (k, _v) in pairs[len..].iter() {
            smt.update(*k, H256::zero()).unwrap();
        }
        // check root
        let current_root = *smt.root();
        assert_eq!(root, current_root);
        // check inserted pairs
        for (k, v) in pairs[..len].iter() {
            let value = smt.get(k).unwrap();
            assert_eq!(v, &value);
        }
    }

    #[test]
    fn test_zero_value_should_delete_branch((pairs, _n) in leaves(5, 30)){
        let mut smt = new_smt(pairs.clone());
        for (k, _v) in pairs {
            smt.update(k, H256::zero()).unwrap();
        }
        assert_eq!(0, smt.store().branches_map().len());
    }

    #[test]
    fn test_smt_not_crash(
        (leaves, _n) in leaves(0, 30),
        leaves_bitmap in leaves_bitmap(30),
        proof in merkle_proof(50)
    ){
        let proof = MerkleProof::new(leaves_bitmap, proof);
        // test compute_root not crash
        let _result = proof.clone().compute_root::<Blake2bHasher>(leaves.clone());
        // test compile not crash
        let _result = proof.compile(leaves.iter().map(|(k, _v)| *k).collect());
    }

    #[test]
    fn test_try_crash_compiled_merkle_proof((leaves, _n) in leaves(0, 30)) {
        // construct cases to crush compiled merkle proof
        let case1 = [0x50, 0x48, 0x4C].to_vec();
        let case2 = [0x48, 0x4C].to_vec();
        let case3 = [0x4C, 0x50].to_vec();
        let case4 = [0x4C, 0x48].to_vec();
        let case5 = [0x50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                     0, 0, 0, 0].to_vec();
        let case6 = [0x48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                     0, 0, 0, 0].to_vec();
        let case7 = [0x4C, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                     0, 0, 0, 0].to_vec();

        for case in [case1, case2, case3, case4, case5, case6, case7].iter() {
            let proof = CompiledMerkleProof(case.to_vec());
            // test compute root not crash
            let _result = proof.compute_root::<Blake2bHasher>(leaves.clone());
        }
    }
}

fn parse_h256(s: &str) -> H256 {
    let data = hex::decode(s).unwrap();
    let mut inner = [0u8; 32];
    inner.copy_from_slice(&data);
    H256::from(inner)
}

#[test]
fn test_v0_2_broken_sample() {
    let keys = vec![
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000002",
        "0000000000000000000000000000000000000000000000000000000000000003",
        "0000000000000000000000000000000000000000000000000000000000000004",
        "0000000000000000000000000000000000000000000000000000000000000005",
        "0000000000000000000000000000000000000000000000000000000000000006",
        "000000000000000000000000000000000000000000000000000000000000000e",
        "f652222313e28459528d920b65115c16c04f3efc82aaedc97be59f3f377c0d3f",
        "f652222313e28459528d920b65115c16c04f3efc82aaedc97be59f3f377c0d40",
        "5eff886ea0ce6ca488a3d6e336d6c0f75f46d19b42c06ce5ee98e42c96d256c7",
        "6d5257204ebe7d88fd91ae87941cb2dd9d8062b64ae5a2bd2d28ec40b9fbf6df",
    ]
    .into_iter()
    .map(parse_h256);

    let values = vec![
        "000000000000000000000000c8328aabcd9b9e8e64fbc566c4385c3bdeb219d7",
        "000000000000000000000001c8328aabcd9b9e8e64fbc566c4385c3bdeb219d7",
        "0000384000001c2000000e1000000708000002580000012c000000780000003c",
        "000000000000000000093a80000546000002a300000151800000e10000007080",
        "000000000000000000000000000000000000000000000000000000000000000f",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "00000000000000000000000000000000000000000000000000071afd498d0000",
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000000",
    ]
    .into_iter()
    .map(parse_h256);

    let mut pairs = keys.zip(values).collect::<Vec<_>>();
    let smt = new_smt(pairs.clone());
    let base_root = *smt.root();

    // insert in random order
    let mut rng = rand::thread_rng();
    for _i in 0..10 {
        pairs.shuffle(&mut rng);
        let smt = new_smt(pairs.clone());
        let current_root = *smt.root();
        assert_eq!(base_root, current_root);
    }
}

#[test]
fn test_v0_3_broken_sample() {
    let k1 = [
        0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let v1 = [
        108, 153, 9, 238, 15, 28, 173, 182, 146, 77, 52, 203, 162, 151, 125, 76, 55, 176, 192, 104,
        170, 5, 193, 174, 137, 255, 169, 176, 132, 64, 199, 115,
    ];
    let k2 = [
        1, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let v2 = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let k3 = [
        1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let v3 = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];

    let mut smt = SMT::default();
    // inserted keys shouldn't interfere with each other
    assert_ne!(k1, k2);
    assert_ne!(k2, k3);
    assert_ne!(k1, k3);
    smt.update(k1.into(), v1.into()).unwrap();
    smt.update(k2.into(), v2.into()).unwrap();
    smt.update(k3.into(), v3.into()).unwrap();
    assert_eq!(smt.get(&k1.into()).unwrap(), v1.into());
}

#[test]
fn test_trie_broken_sample() {
    let keys = vec![
        "f652222313e28459528d920b65115c16c04f3efc82aaedc97be59f3f377c0d40",
        "5eff886ea0ce6ca488a3d6e336d6c0f75f46d19b42c06ce5ee98e42c96d256c7",
        "6d5257204ebe7d88fd91ae87941cb2dd9d8062b64ae5a2bd2d28ec40b9fbf6df",
    ]
    .into_iter()
    .map(parse_h256);

    let values = vec![
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000002",
        "0000000000000000000000000000000000000000000000000000000000000003",
    ]
    .into_iter()
    .map(parse_h256);

    let mut pairs = keys.zip(values).collect::<Vec<_>>();
    let smt = new_smt(pairs.clone());
    let base_branches = smt.store().branches_map();
    pairs.reverse();
    let smt = new_smt(pairs.clone());
    let current_branches = smt.store().branches_map();
    assert_eq!(base_branches, current_branches);
}

#[test]
fn test_trie_broken_sample_02() {
    let key1: H256 = [
        1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let key2: H256 = [
        2, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let key3: H256 = [
        0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();

    let pairs = vec![
        (key1, [1; 32].into()),
        (key2, [2; 32].into()),
        (key3, [0u8; 32].into()),
    ];
    let smt = new_smt(pairs);
    let kv_state: [([u8; 32], [u8; 32]); 1] = [(
        [
            3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ],
        [0; 32],
    )];

    for (k, v) in kv_state {
        assert_eq!(smt.get(&k.into()).unwrap(), v.into());
    }

    let keys: Vec<H256> = kv_state.iter().map(|kv| kv.0.into()).collect();

    let proof = smt
        .merkle_proof(keys.clone())
        .unwrap()
        .compile(keys)
        .unwrap();

    let root1 = proof
        .compute_root::<Blake2bHasher>(
            kv_state
                .iter()
                .map(|(k, v)| (k.clone().into(), v.clone().into()))
                .collect(),
        )
        .unwrap();
    assert_eq!(smt.root(), &root1);
}

#[test]
fn test_trie_broken_sample_03() {
    let mut smt = SMT::default();
    smt.update(
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]
        .into(),
        [
            231, 17, 197, 236, 8, 0, 141, 194, 15, 253, 234, 189, 224, 53, 255, 173, 117, 8, 221,
            5, 34, 5, 198, 250, 99, 32, 229, 13, 222, 40, 203, 90,
        ]
        .into(),
    )
    .unwrap();
    smt.update(
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]
        .into(),
        [
            231, 17, 197, 236, 8, 0, 141, 194, 15, 253, 234, 189, 224, 53, 255, 173, 117, 8, 221,
            5, 34, 5, 198, 250, 99, 32, 229, 13, 222, 40, 203, 90,
        ]
        .into(),
    )
    .unwrap();
    smt.update(
        [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]
        .into(),
        [
            105, 112, 48, 175, 83, 217, 158, 108, 243, 136, 9, 21, 192, 91, 211, 190, 218, 240, 89,
            241, 63, 137, 128, 133, 65, 169, 51, 33, 49, 123, 118, 132,
        ]
        .into(),
    )
    .unwrap();
    smt.update(
        [
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]
        .into(),
        [
            189, 150, 22, 8, 143, 248, 180, 169, 68, 195, 31, 28, 34, 180, 182, 223, 195, 37, 117,
            197, 229, 144, 229, 64, 230, 250, 21, 205, 225, 32, 135, 195,
        ]
        .into(),
    )
    .unwrap();
    smt.update(
        [
            3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]
        .into(),
        [
            153, 75, 31, 235, 146, 228, 224, 228, 237, 250, 34, 227, 139, 8, 213, 118, 25, 114, 82,
            242, 215, 172, 184, 100, 205, 85, 47, 116, 140, 238, 175, 190,
        ]
        .into(),
    )
    .unwrap();
    smt.update(
        [
            4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]
        .into(),
        [
            242, 174, 6, 108, 205, 74, 137, 57, 15, 248, 35, 35, 255, 58, 93, 74, 183, 47, 194, 40,
            134, 3, 215, 100, 80, 51, 28, 251, 96, 19, 201, 170,
        ]
        .into(),
    )
    .unwrap();
    smt.update(
        [
            5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]
        .into(),
        [
            88, 83, 226, 107, 201, 255, 207, 189, 197, 145, 113, 95, 209, 238, 110, 9, 82, 215,
            232, 183, 203, 220, 194, 167, 21, 189, 239, 238, 178, 149, 153, 44,
        ]
        .into(),
    )
    .unwrap();
    smt.update(
        [
            6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]
        .into(),
        [
            80, 177, 52, 81, 182, 121, 67, 120, 77, 19, 201, 42, 75, 136, 19, 238, 112, 23, 204,
            103, 20, 157, 53, 235, 80, 70, 126, 79, 9, 35, 11, 158,
        ]
        .into(),
    )
    .unwrap();
    let key7 = H256::from([
        7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]);
    let v7 = H256::from([
        103, 245, 93, 107, 47, 213, 28, 173, 216, 92, 109, 17, 151, 57, 101, 4, 44, 145, 116, 215,
        185, 218, 144, 244, 131, 160, 148, 58, 247, 226, 240, 55,
    ]);
    let proof = smt
        .merkle_proof(vec![key7])
        .unwrap()
        .compile(vec![key7])
        .unwrap();
    // Compute root with different value than actually in smt.
    let root = proof
        .compute_root::<Blake2bHasher>(vec![(key7, v7)])
        .unwrap();
    // Compute root by updating smt.
    smt.update(key7, v7).unwrap();
    // Expect them to be the same.
    assert_eq!(*smt.root(), root);
}

#[test]
fn test_replay_to_pass_proof() {
    let key1: H256 = [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let key2: H256 = [
        2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let key3: H256 = [
        3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let key4: H256 = [
        4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();

    let existing: H256 = [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let non_existing: H256 = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ]
    .into();
    let other_value: H256 = [
        0, 0, 0xff, 0, 0, 0, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0xff,
    ]
    .into();
    let pairs = vec![
        (key1, existing),
        (key2, non_existing),
        (key3, non_existing),
        (key4, non_existing),
    ];
    let smt = new_smt(pairs);
    let leaf_a_bl = vec![(key1, H256::zero())];
    let leaf_c = vec![(key3, non_existing)];
    let leaf_other = vec![(key3, other_value)];
    let proofc = smt
        .merkle_proof(leaf_c.clone().into_iter().map(|(k, _)| k).collect())
        .expect("gen proof");
    let compiled_proof = proofc.clone().compile(vec![key3]).expect("compile proof");

    println!("verify ok case");
    assert!(proofc
        .clone()
        .verify::<Blake2bHasher>(smt.root(), leaf_c.clone())
        .expect("verify"));
    println!("verify not ok case");
    assert!(!proofc
        .clone()
        .verify::<Blake2bHasher>(smt.root(), leaf_other)
        .expect("verify"));

    println!("merkle proof, leaf is faked");
    assert!(!proofc
        .verify::<Blake2bHasher>(smt.root(), leaf_a_bl.clone())
        .expect("verify"));

    println!("compiled merkle proof, leaf is faked");
    assert!(!compiled_proof
        .verify::<Blake2bHasher>(smt.root(), leaf_a_bl)
        .expect("verify compiled proof"));

    test_sub_proof(&compiled_proof, &smt, &leaf_c, 20);
}

#[test]
fn test_sibling_leaf() {
    fn gen_rand_h256() -> H256 {
        let mut rng = rand::thread_rng();
        let rand_data: [u8; 32] = rng.gen();
        H256::from(rand_data)
    }
    let rand_key = gen_rand_h256();
    let mut sibling_key = rand_key;
    if rand_key.is_right(0) {
        sibling_key.clear_bit(0);
    } else {
        sibling_key.set_bit(0);
    }
    let pairs = vec![(rand_key, gen_rand_h256()), (sibling_key, gen_rand_h256())];
    let keys = vec![rand_key, sibling_key];
    let smt = new_smt(pairs.clone());
    let proof = smt.merkle_proof(keys).expect("gen proof");
    assert!(proof
        .verify::<Blake2bHasher>(smt.root(), pairs)
        .expect("verify"));
}

#[test]
fn test_max_stack_size() {
    fn gen_h256(height: u8) -> H256 {
        // The key path is first go right `256 - height` times then go left `height` times.
        let mut key = H256::zero();
        for h in height..=255 {
            key.set_bit(h);
        }
        key
    }
    let mut pairs: Vec<_> = (0..=255)
        .map(|height| (gen_h256(height), gen_h256(1)))
        .collect();
    // Most left key
    pairs.push((H256::zero(), gen_h256(1)));
    {
        // A pair of sibling keys in between
        let mut left_key = H256::zero();
        for h in 12..56 {
            left_key.set_bit(h);
        }
        let mut right_key = left_key;
        right_key.set_bit(0);
        pairs.push((left_key, gen_h256(1)));
        pairs.push((right_key, gen_h256(1)));
    }

    let keys: Vec<_> = pairs.iter().map(|(key, _)| *key).collect();
    let smt = new_smt(pairs.clone());
    let proof = smt.merkle_proof(keys.clone()).expect("gen proof");
    let compiled_proof = proof.compile(keys).expect("compile proof");

    assert!(compiled_proof
        .verify::<Blake2bHasher>(smt.root(), pairs.clone())
        .expect("verify"));

    test_sub_proof(&compiled_proof, &smt, &pairs, 20);
}

#[test]
fn test_simple_non_exists_sub_proof() {
    let pairs = vec![(
        H256::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]),
        H256::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]),
    )];
    let pairs2 = vec![(
        H256::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ]),
        H256::from([
            120, 94, 121, 42, 43, 185, 121, 215, 19, 188, 112, 111, 16, 124, 59, 43, 189, 203, 55,
            192, 159, 233, 56, 217, 126, 150, 113, 232, 27, 66, 255, 10,
        ]),
    )];
    let smt = new_smt(pairs.clone());
    let exists_keys: Vec<_> = pairs.into_iter().map(|(k, _v)| k).collect();
    let non_exists_keys: Vec<_> = pairs2.into_iter().map(|(k, _v)| k).collect();
    let exists_keys_len = std::cmp::max(exists_keys.len() / 2, 1);
    let non_exists_keys_len = std::cmp::max(non_exists_keys.len() / 2, 1);
    let mut keys: Vec<_> = exists_keys
        .into_iter()
        .take(exists_keys_len)
        .chain(non_exists_keys.into_iter().take(non_exists_keys_len))
        .collect();
    keys.dedup();
    let proof = smt.merkle_proof(keys.clone()).expect("gen proof");
    let data: Vec<(H256, H256)> = keys
        .iter()
        .map(|k| (*k, smt.get(k).expect("get")))
        .collect();
    let compiled_proof = proof.compile(keys.clone()).expect("compile proof");
    test_sub_proof(&compiled_proof, &smt, &data, 20);
}

fn test_sub_proof(
    compiled_proof: &CompiledMerkleProof,
    smt: &SMT,
    data: &[(H256, H256)],
    test_multi_round: usize,
) {
    let mut keys = data.iter().map(|(k, _v)| *k).collect::<Vec<_>>();

    // test sub proof with single leaf
    for key in &keys {
        let single_compiled_proof = compiled_proof
            .extract_proof::<Blake2bHasher>(data.iter().map(|(k, v)| (*k, *v, k == key)).collect())
            .expect("compiled one proof");
        let expected_compiled_proof = smt
            .merkle_proof(vec![*key])
            .unwrap()
            .compile(vec![*key])
            .unwrap();
        assert_eq!(expected_compiled_proof.0, single_compiled_proof.0);

        let value = smt.get(key).unwrap();
        assert!(single_compiled_proof
            .verify::<Blake2bHasher>(smt.root(), vec![(*key, value)])
            .expect("verify compiled one proof"));
    }

    if data.len() < 2 {
        return;
    }

    // test sub proof with multiple leaves
    let mut rng = rand::thread_rng();
    for _ in 0..test_multi_round {
        keys.shuffle(&mut rng);
        let selected_number = rng.gen_range(2..=keys.len());
        let selected_pairs: HashMap<_, _> = keys
            .iter()
            .take(selected_number)
            .map(|key| (*key, smt.get(key).unwrap()))
            .collect();

        let sub_proof = compiled_proof
            .extract_proof::<Blake2bHasher>(
                data.iter()
                    .map(|(k, v)| (*k, *v, selected_pairs.contains_key(k)))
                    .collect(),
            )
            .expect("compiled sub proof");
        let selected_keys = selected_pairs.keys().cloned().collect::<Vec<_>>();
        let expected_compiled_proof = smt
            .merkle_proof(selected_keys.clone())
            .unwrap()
            .compile(selected_keys)
            .unwrap();
        assert_eq!(expected_compiled_proof.0, sub_proof.0);

        assert!(sub_proof
            .verify::<Blake2bHasher>(smt.root(), selected_pairs.into_iter().collect())
            .expect("verify compiled sub proof"));
    }
}
