use crate::*;
use crate::{
    blake2b::Blake2bHasher, default_store::DefaultStore, error::Error, merge::MergeValue,
    MerkleProof, SparseMerkleTree,
};
use proptest::prelude::*;
use rand::prelude::{Rng, SliceRandom};

extern crate std;
use std::{println, vec::Vec};

type SMT = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;

#[test]
fn test_default_root() {
    let mut tree = SMT::default();
    assert_eq!(tree.store().branches_map().len(), 0);
    assert_eq!(tree.store().leaves_map().len(), 0);
    assert_eq!(tree.root(), &H256::empty());

    // insert a key-value
    tree.update(H256::empty(), [42u8; 32].into())
        .expect("update");
    assert_ne!(tree.root(), &H256::empty());
    assert_ne!(tree.store().branches_map().len(), 0);
    assert_ne!(tree.store().leaves_map().len(), 0);
    assert_eq!(tree.get(&H256::empty()).expect("get"), [42u8; 32].into());
    // update zero is to delete the key
    tree.update(H256::empty(), H256::empty()).expect("update");
    assert_eq!(tree.root(), &H256::empty());
    assert_eq!(tree.get(&H256::empty()).expect("get"), H256::empty());
}

#[test]
fn test_default_tree() {
    let tree = SMT::default();
    assert_eq!(tree.get(&H256::empty()).expect("get"), H256::empty());
    let proof = tree
        .merkle_proof(vec![H256::empty()])
        .expect("merkle proof");
    let root = proof
        .compute_root::<Blake2bHasher>(vec![(H256::empty(), H256::empty())])
        .expect("root");
    assert_eq!(&root, tree.root());
    let proof = tree
        .merkle_proof(vec![H256::empty()])
        .expect("merkle proof");
    let root2 = proof
        .compute_root::<Blake2bHasher>(vec![(H256::empty(), [42u8; 32].into())])
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
    // assert_ne!(root, H256::empty());
}

#[test]
fn test_merkle_root() {
    fn new_blake2b() -> blake2b_rs::Blake2b {
        blake2b_rs::Blake2bBuilder::new(32).personal(b"SMT").build()
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
            hasher.update(&word.as_bytes());
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
    let value = H256::empty();
    tree.update(key, value).unwrap();
    assert_eq!(tree.root(), &H256::empty());
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
    assert_ne!(tree.root(), &H256::empty());
    let root = tree.root().clone();
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
    assert_ne!(tree.root(), &H256::empty());
    let root = tree.root().clone();
    let store = tree.store().clone();

    // insert a leaf
    let key: H256 = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    let value = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1,
    ]
    .into();
    tree.update(key.clone(), value).unwrap();
    assert_ne!(tree.root(), &root);

    // delete a leaf
    tree.update(key, H256::empty()).unwrap();
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
        assert_eq!(H256::empty(), tree.get(&sibling_key).unwrap());
    }

    {
        let mut tree = SMT::default();
        let key = H256::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let value = H256::from([1u8; 32]);
        tree.update(key.clone(), value.clone()).expect("update");

        let sibling_key = H256::from([
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        let sibling_value = H256::from([2u8; 32]);
        tree.update(sibling_key.clone(), sibling_value.clone())
            .expect("update");
        // get sibling key should return corresponding value
        assert_eq!(value, tree.get(&key).unwrap());
        assert_eq!(sibling_value, tree.get(&sibling_key).unwrap());
    }
}

fn test_construct(key: H256, value: H256) {
    // insert same value to sibling key will construct a different root

    let mut tree = SMT::default();
    tree.update(key.clone(), value.clone()).expect("update");

    let mut sibling_key = key;
    if sibling_key.bit(0).unwrap_or(false) {
        sibling_key.set_bit(0, false);
    } else {
        sibling_key.set_bit(0, true);
    }
    let mut tree2 = SMT::default();
    tree2.update(sibling_key, value).expect("update");
    assert_ne!(tree.root(), tree2.root());
}

fn test_update(key: H256, value: H256) {
    let mut tree = SMT::default();
    tree.update(key.clone(), value.clone()).expect("update");
    assert_eq!(tree.get(&key), Ok(value));
}

fn test_update_tree_store(key: H256, value: H256, value2: H256) {
    const EXPECTED_LEAVES_LEN: usize = 1;

    let mut tree = SMT::default();
    tree.update(key.clone(), value).expect("update");
    assert_eq!(tree.store().branches_map().len(), 256);
    assert_eq!(tree.store().leaves_map().len(), EXPECTED_LEAVES_LEN);
    tree.update(key.clone(), value2.clone()).expect("update");
    assert_eq!(tree.store().branches_map().len(), 256);
    assert_eq!(tree.store().leaves_map().len(), EXPECTED_LEAVES_LEN);
    assert_eq!(tree.get(&key), Ok(value2));
}

fn test_merkle_proof(key: H256, value: H256) {
    const EXPECTED_MERKLE_PATH_SIZE: usize = 1;

    let mut tree = SMT::default();
    tree.update(key.clone(), value.clone()).expect("update");
    if !tree.is_empty() {
        let proof = tree.merkle_proof(vec![key.clone()]).expect("proof");
        let compiled_proof = proof
            .clone()
            .compile(vec![(key.clone(), value.clone())])
            .expect("compile proof");
        assert!(proof.merkle_path().len() < EXPECTED_MERKLE_PATH_SIZE);
        assert!(proof
            .verify::<Blake2bHasher>(tree.root(), vec![(key.clone(), value.clone())])
            .expect("verify"));
        assert!(compiled_proof
            .verify::<Blake2bHasher>(tree.root(), vec![(key, value)])
            .expect("compiled verify"));
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
        let mut list1: Vec<H256> = vec![H256::from(key) , H256::from(key2)];
        let mut list2 = list1.clone();
        // sort H256
        list1.sort_unstable_by_key(|k| k.clone());
        h256::smt_sort_unstable(&mut list1);
        // sort by high bits to lower bits
        list2.sort_unstable_by(|k1, k2| {
            for i in (0u8..=255).rev() {
                let b1 = if (*k1).bit(i.into()).unwrap_or(false) { 1 } else { 0 };
                let b2 = if (*k2).bit(i.into()).unwrap_or(false) { 1 } else { 0 };
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
        let target = h256::copy_bits(&one, start);
        for i in start..=core::u8::MAX {
            assert_eq!(one.bit(i.into()).unwrap_or(false), target.bit(i.into()).unwrap_or(false));
        }
        for i in 0..start {
            assert!(!target.bit(i.into()).unwrap_or(false));
        }
    }

    #[test]
    fn test_random_update(key: [u8; 32], value: [u8;32]) {
        test_update(key.into(), value.into());
    }

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
            let proof = smt.merkle_proof(vec![k.clone()]).expect("gen proof");
            let compiled_proof = proof.clone().compile(vec![(k.clone(), v.clone())]).expect("compile proof");
            assert!(proof.verify::<Blake2bHasher>(smt.root(), vec![(k.clone(), v.clone())]).expect("verify proof"));
            assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), vec![(k, v)]).expect("verify compiled proof"));
        }
    }

    #[test]
    fn test_smt_single_leaf_large((pairs, _n) in leaves(50, 100)){
        let smt = new_smt(pairs.clone());
        for (k, v) in pairs {
            let proof = smt.merkle_proof(vec![k.clone()]).expect("gen proof");
            let compiled_proof = proof.clone().compile(vec![(k.clone(), v.clone())]).expect("compile proof");
            assert!(proof.verify::<Blake2bHasher>(smt.root(), vec![(k.clone(), v.clone())]).expect("verify proof"));
            assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), vec![(k, v)]).expect("verify compiled proof"));
        }
    }

    #[test]
    fn test_smt_multi_leaves_small((pairs, _n) in leaves(1, 50)){
        let smt = new_smt(pairs.clone());
        let proof = smt.merkle_proof(pairs.iter().map(|(k, _v)| k.clone()).collect()).expect("gen proof");
        let data: Vec<(H256, H256)> = pairs.into_iter().collect();
        let compiled_proof = proof.clone().compile(data.clone()).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), data).expect("verify compiled proof"));
    }

    #[test]
    fn test_smt_multi_leaves_large((pairs, _n) in leaves(50, 100)){
        let n = 20;
        let smt = new_smt(pairs.clone());
        let proof = smt.merkle_proof(pairs.iter().take(n).map(|(k, _v)| k.clone()).collect()).expect("gen proof");
        let data: Vec<(H256, H256)> = pairs.into_iter().take(n).collect();
        let compiled_proof = proof.clone().compile(data.clone()).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), data).expect("verify compiled proof"));
    }

    #[test]
    fn test_smt_non_exists_leaves((pairs, _n) in leaves(1, 20), (pairs2, _n2) in leaves(1, 5)){
        let smt = new_smt(pairs);
        let non_exists_keys: Vec<_> = pairs2.into_iter().map(|(k, _v)|k).collect();
        let proof = smt.merkle_proof(non_exists_keys.clone()).expect("gen proof");
        let data: Vec<(H256, H256)> = non_exists_keys.into_iter().map(|k|(k, H256::empty())).collect();
        let compiled_proof = proof.clone().compile(data.clone()).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), data).expect("verify compiled proof"));
    }

    #[test]
    fn test_smt_non_exists_leaves_mix((pairs, _n) in leaves(1, 20), (pairs2, _n2) in leaves(1, 5)){
        let smt = new_smt(pairs.clone());
        let exists_keys: Vec<_> = pairs.into_iter().map(|(k, _v)|k).collect();
        let non_exists_keys: Vec<_> = pairs2.into_iter().map(|(k, _v)|k).collect();
        let exists_keys_len = std::cmp::max(exists_keys.len() / 2, 1);
        let non_exists_keys_len = std::cmp::max(non_exists_keys.len() / 2, 1);
        let mut keys: Vec<_> = exists_keys.into_iter().take(exists_keys_len).chain(non_exists_keys.into_iter().take(non_exists_keys_len)).collect();
        keys.dedup();
        let proof = smt.merkle_proof(keys.clone()).expect("gen proof");
        let data: Vec<(H256, H256)> = keys.into_iter().map(|k|(k.clone(), smt.get(&k).expect("get"))).collect();
        let compiled_proof = proof.clone().compile(data.clone()).expect("compile proof");
        assert!(proof.verify::<Blake2bHasher>(smt.root(), data.clone()).expect("verify proof"));
        assert!(compiled_proof.verify::<Blake2bHasher>(smt.root(), data).expect("verify compiled proof"));
    }

    #[test]
    fn test_update_smt_tree_store((pairs, n) in leaves(1, 20)) {
        let smt = new_smt(pairs.clone());
        for (k, v) in pairs.into_iter().take(n) {
            assert_eq!(smt.get(&k), Ok(v));
        }
    }

    #[test]
    fn test_smt_random_insert_order((pairs, _n) in leaves(5, 50)){
        let smt = new_smt(pairs.clone());
        let root = smt.root().clone();

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
                assert_eq!(&smt2.get(&k).unwrap(), v, "key value must be consisted");

                let origin_proof = smt.merkle_proof(vec![k.clone()]).unwrap();
                let proof = smt2.merkle_proof(vec![k.clone()]).unwrap();
                assert_eq!(origin_proof, proof, "merkle proof must be consisted");

                let calculated_root = proof.compute_root::<Blake2bHasher>(vec![(k.clone(), v.clone())]).unwrap();
                assert_eq!(root, calculated_root, "root must be consisted");
            }
        }
    }

    #[test]
    fn test_smt_update_with_zero_values((pairs, _n) in leaves(5, 30)){
        let mut rng = rand::thread_rng();
        let len =  rng.gen_range(0, pairs.len());
        let mut smt = new_smt(pairs[..len].to_vec());
        let root = smt.root().clone();

        // insert zero values
        for (k, _v) in pairs[len..].iter() {
            smt.update(k.clone(), H256::empty()).unwrap();
        }
        // check root
        let current_root = smt.root().clone();
        assert_eq!(root, current_root);
        // check inserted pairs
        for (k, v) in pairs[..len].iter() {
            let value = smt.get(&k).unwrap();
            assert_eq!(v, &value);
        }
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
        let _result = proof.compile(leaves);
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
    .map(parse_h256)
    .collect::<Vec<_>>();
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
    .map(parse_h256)
    .collect::<Vec<_>>();
    let mut pairs = keys
        .into_iter()
        .map(|k| k.clone())
        .into_iter()
        .zip(values.into_iter())
        .collect::<Vec<_>>();
    let smt = new_smt(pairs.clone());
    let base_root = smt.root().clone();

    // insert in random order
    let mut rng = rand::thread_rng();
    for _i in 0..10 {
        pairs.shuffle(&mut rng);
        let smt = new_smt(pairs.clone());
        let current_root = smt.root().clone();
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
        (key1.clone(), existing),
        (key2.clone(), non_existing.clone()),
        (key3.clone(), non_existing.clone()),
        (key4.clone(), non_existing.clone()),
    ];
    let smt = new_smt(pairs);
    let leaf_a_bl = vec![(key1, H256::empty())];
    let leaf_c = vec![(key3.clone(), non_existing)];
    let leaf_other = vec![(key3, other_value)];
    let proofc = smt
        .merkle_proof(leaf_c.clone().into_iter().map(|(k, _)| k).collect())
        .expect("gen proof");
    let compiled_proof = proofc
        .clone()
        .compile(leaf_c.clone())
        .expect("compile proof");

    println!("verify ok case");
    assert!(proofc
        .clone()
        .verify::<Blake2bHasher>(smt.root(), leaf_c)
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
}

#[test]
fn test_sibling_leaf() {
    fn gen_rand_h256() -> H256 {
        let mut rng = rand::thread_rng();
        let rand_data: [u8; 32] = rng.gen();
        H256::from(rand_data)
    }
    let rand_key = gen_rand_h256();
    let mut sibling_key = rand_key.clone();
    if rand_key.bit(0).unwrap_or(false) {
        sibling_key.set_bit(0, false);
    } else {
        sibling_key.set_bit(0, true);
    }
    let pairs = vec![
        (rand_key.clone(), gen_rand_h256()),
        (sibling_key.clone(), gen_rand_h256()),
    ];
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
        let mut key = H256::empty();
        for h in height..=255 {
            key.set_bit(h.into(), true);
        }
        key
    }
    let mut pairs: Vec<_> = (0..=255)
        .map(|height| (gen_h256(height), gen_h256(1)))
        .collect();
    // Most left key
    pairs.push((H256::empty(), gen_h256(1)));
    {
        // A pair of sibling keys in between
        let mut left_key = H256::empty();
        for h in 12..56 {
            left_key.set_bit(h, true);
        }
        let mut right_key = left_key.clone();
        right_key.set_bit(0, true);
        pairs.push((left_key, gen_h256(1)));
        pairs.push((right_key, gen_h256(1)));
    }

    let keys: Vec<_> = pairs.iter().map(|(key, _)| key.clone()).collect();
    let smt = new_smt(pairs.clone());
    let proof = smt.merkle_proof(keys).expect("gen proof");
    let compiled_proof = proof.compile(pairs.clone()).expect("compile proof");
    assert!(compiled_proof
        .verify::<Blake2bHasher>(smt.root(), pairs)
        .expect("verify"));
}
