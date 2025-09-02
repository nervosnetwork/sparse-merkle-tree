use anyhow::Result;
use anyhow::Result as AnyResult;
use core::ffi::c_void;
use proptest::prelude::*;
use rand::prelude::Rng;
use serde::{Deserialize, Serialize};
use sparse_merkle_tree::blake2b::{Blake2b, Blake2bBuilder};
use sparse_merkle_tree::traits::Hasher;
use sparse_merkle_tree::{default_store::DefaultStore, SparseMerkleTree, H256};
use std::collections::HashMap;
use std::fs;

#[link(name = "dl-c-impl", kind = "static")]
extern "C" {
    fn smt_state_new(capacity: u32) -> *mut c_void;
    fn smt_state_len(state: *mut c_void) -> u32;

    fn smt_state_insert(state: *mut c_void, key: *const u8, value: *const u8) -> isize;
    fn smt_state_fetch(state: *mut c_void, key: *const u8, value: *mut u8) -> isize;
    fn smt_state_normalize(state: *mut c_void);
    #[allow(dead_code)]
    fn smt_calculate_root(
        buffer: *mut u8,
        state: *const c_void,
        proof: *const u8,
        proof_length: u32,
    ) -> isize;
    fn smt_verify(
        hash: *const u8,
        state: *const c_void,
        proof: *const u8,
        proof_length: u32,
    ) -> isize;
}

pub struct SmtCImpl {
    state_ptr: *mut c_void,
}

fn ffi_smt_result<T>(value: T, code: isize) -> Result<T, isize> {
    if code == 0 {
        Ok(value)
    } else {
        Err(code)
    }
}

fn ffi_assert_slice_len(slice: &[u8], expected_len: usize) -> Result<(), isize> {
    if slice.len() == expected_len {
        Ok(())
    } else {
        Err(-999)
    }
}

impl SmtCImpl {
    pub fn new(capacity: u32) -> SmtCImpl {
        let state_ptr = unsafe { smt_state_new(capacity) };
        SmtCImpl { state_ptr }
    }

    pub fn len(&self) -> u32 {
        unsafe { smt_state_len(self.state_ptr) }
    }

    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), isize> {
        ffi_assert_slice_len(key, 32)?;
        ffi_assert_slice_len(value, 32)?;
        let code = unsafe { smt_state_insert(self.state_ptr, key.as_ptr(), value.as_ptr()) };
        ffi_smt_result((), code)
    }

    pub fn fetch(&self, key: &[u8]) -> Result<[u8; 32], isize> {
        ffi_assert_slice_len(key, 32)?;
        let mut value = [0u8; 32];
        let code = unsafe { smt_state_fetch(self.state_ptr, key.as_ptr(), value.as_mut_ptr()) };
        ffi_smt_result(value, code)
    }

    pub fn normalize(&mut self) {
        unsafe {
            smt_state_normalize(self.state_ptr);
        }
    }

    #[allow(dead_code)]
    pub fn calculate_root(&self, proof: &[u8]) -> Result<[u8; 32], isize> {
        let mut hash = [0u8; 32];
        let code = unsafe {
            smt_calculate_root(
                hash.as_mut_ptr(),
                self.state_ptr,
                proof.as_ptr(),
                proof.len() as u32,
            )
        };
        ffi_smt_result(hash, code)
    }

    pub fn verify(&self, root: &[u8], proof: &[u8]) -> Result<(), isize> {
        ffi_assert_slice_len(root, 32)?;
        let code = unsafe {
            smt_verify(
                root.as_ptr(),
                self.state_ptr,
                proof.as_ptr(),
                proof.len() as u32,
            )
        };
        ffi_smt_result((), code)
    }
}

pub type Leave = ([u8; 32], [u8; 32]);

#[derive(Default, Serialize, Deserialize)]
pub struct Proof {
    pub leaves: Vec<Leave>,
    pub compiled_proof: Vec<u8>,
    pub error: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Case {
    pub name: String,
    pub leaves: Vec<Leave>,
    pub root: [u8; 32],
    pub proofs: Vec<Proof>,
}

pub struct CkbBlake2bHasher(Blake2b);

impl Default for CkbBlake2bHasher {
    fn default() -> Self {
        // NOTE: here we not set the `personal` since ckb_smt.c linked blake2b implementation from blake2b-rs
        let blake2b = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        CkbBlake2bHasher(blake2b)
    }
}

impl Hasher for CkbBlake2bHasher {
    fn write_byte(&mut self, b: u8) {
        self.0.update(&[b][..]);
    }
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }
    fn finish(self) -> H256 {
        let mut hash = [0u8; 32];
        self.0.finalize(&mut hash);
        hash.into()
    }
}

pub type CkbSMT = SparseMerkleTree<CkbBlake2bHasher, H256, DefaultStore<H256>>;

pub fn new_ckb_smt(pairs: Vec<(H256, H256)>) -> CkbSMT {
    let mut smt = CkbSMT::default();
    for (key, value) in pairs {
        smt.update(key, value).unwrap();
    }
    smt
}

fn h256_by_u8(n: u8) -> [u8; 32] {
    let mut data = [0u8; 32];
    data[31] = n;
    data
}

#[test]
fn test_normalize() {
    // pushed key = 7, value = 1
    // pushed key = 1, value = 1
    // pushed key = 1, value = 2
    // pushed key = 2, value = 1
    // pushed key = 2, value = 2
    // pushed key = 0, value = 1
    // pushed key = 0, value = 2
    // pushed key = 6, value = 1
    let mut smt_state = SmtCImpl::new(256);
    let data_set = [
        (7, 1),
        (1, 1),
        (1, 2),
        (2, 1),
        (2, 2),
        (0, 1),
        (0, 2),
        (6, 1),
    ];
    for (k, v) in &data_set {
        smt_state.insert(&h256_by_u8(*k), &h256_by_u8(*v)).unwrap();
    }
    assert_eq!(smt_state.len() as usize, data_set.len());
    smt_state.normalize();
    assert_eq!(smt_state.len(), 5);
    for (k, v) in &[(0, 2), (1, 2), (2, 2), (6, 1), (7, 1)] {
        assert_eq!(smt_state.fetch(&h256_by_u8(*k)).unwrap(), h256_by_u8(*v));
    }
}

#[test]
fn test_normalize_random() {
    let mut rng = rand::thread_rng();
    for pair_size in vec![1, 2, 100, 256, 512, 1024, 2048] {
        for _ in 0..4 {
            let mut final_map: HashMap<u8, u8> = HashMap::default();
            let mut smt_state = SmtCImpl::new(pair_size);
            let rand_pairs: Vec<(u8, u8)> =
                (0..pair_size).map(|_| (rng.gen(), rng.gen())).collect();
            for (key, value) in &rand_pairs {
                final_map.insert(*key, *value);
                smt_state
                    .insert(&h256_by_u8(*key), &h256_by_u8(*value))
                    .unwrap();
            }
            assert_eq!(smt_state.len(), pair_size);
            smt_state.normalize();
            assert_eq!(smt_state.len() as usize, final_map.len());
            for (key, value) in &final_map {
                let byte32_value = smt_state.fetch(&h256_by_u8(*key)).unwrap();
                assert_eq!(h256_by_u8(*value), byte32_value);
            }
        }
    }
}

fn run_test_case(case: Case) -> AnyResult<()> {
    let Case { leaves, proofs, .. } = case;

    let ckb_smt = new_ckb_smt(
        leaves
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect(),
    );

    for proof in proofs {
        let Proof { leaves, error, .. } = proof;
        let keys: Vec<_> = leaves.iter().map(|(k, _v)| (*k).into()).collect();
        let ckb_actual_proof = match ckb_smt.merkle_proof(keys) {
            Ok(proof) => proof,
            Err(err) => {
                let expected_error = error.expect("expected error");
                assert_eq!(expected_error, format!("{}", err));
                return Ok(());
            }
        };
        let ckb_actual_compiled_proof = ckb_actual_proof
            .clone()
            .compile(leaves.iter().map(|(k, _v)| (*k).into()).collect())?;
        let ckb_actual_compiled_proof_bin: Vec<u8> = ckb_actual_compiled_proof.clone().into();

        let mut smt_state = SmtCImpl::new(leaves.len() as u32);
        for (key, value) in &leaves {
            smt_state.insert(key, value).unwrap();
        }
        for (key, value) in &leaves {
            let fetched_value = smt_state.fetch(key).unwrap();
            assert_eq!(value, &fetched_value);
        }
        smt_state.normalize();
        for (key, value) in &leaves {
            let fetched_value = smt_state.fetch(key).unwrap();
            assert_eq!(value, &fetched_value);
        }

        assert_eq!(smt_state.len(), leaves.len() as u32);
        smt_state
            .verify(ckb_smt.root().as_slice(), &ckb_actual_compiled_proof_bin)
            .unwrap();
    }
    Ok(())
}

fn hex2bin(src: String) -> Vec<u8> {
    hex::decode(src).unwrap_or(Vec::new())
}

#[test]
fn test_smt_c_verify1() {
    let key =
        hex2bin("381dc5391dab099da5e28acd1ad859a051cf18ace804d037f12819c6fbc0e18b".to_owned());
    let value =
        hex2bin("9158ce9b0e11dd150ba2ae5d55c1db04b1c5986ec626f2e38a93fe8ad0b2923b".to_owned());
    let root_hash =
        hex2bin("ebe0fab376cd802d364eeb44af20c67a74d6183a33928fead163120ef12e6e06".to_owned());
    let proof = hex2bin(
        "4c4fff51ff322de8a89fe589987f97220cfcb6820bd798b31a0b56ffea221093d35f909e580b00000000000000000000000000000000000000000000000000000000000000".to_owned());

    unsafe {
        let changes = smt_state_new(32);
        smt_state_insert(changes, key.as_ptr(), value.as_ptr());
        smt_state_normalize(changes);

        let verify_ref = smt_verify(
            root_hash.as_ptr(),
            changes,
            proof.as_ptr(),
            proof.len() as u32,
        );
        assert_eq!(0, verify_ref);
    }
}

#[test]
fn test_smt_c_verify2() {
    let key =
        hex2bin("a9bb945be71f0bd2757d33d2465b6387383da42f321072e47472f0c9c7428a8a".to_owned());
    let value =
        hex2bin("a939a47335f777eac4c40fbc0970e25f832a24e1d55adc45a7b76d63fe364e82".to_owned());
    let root_hash =
        hex2bin("6e5c722644cd55cef8c4ed886cd8b44027ae9ed129e70a4b67d87be1c6857842".to_owned());
    let proof = hex2bin(
        "4c4fff51fa8aaa2aece17b92ec3f202a40a09f7286522bae1e5581a2a49195ab6781b1b8090000000000000000000000000000000000000000000000000000000000000000".to_owned());

    unsafe {
        let changes = smt_state_new(32);
        smt_state_insert(changes, key.as_ptr(), value.as_ptr());
        smt_state_normalize(changes);

        let verify_ref = smt_verify(
            root_hash.as_ptr(),
            changes,
            proof.as_ptr(),
            proof.len() as u32,
        );
        assert_eq!(0, verify_ref);
    }
}

// FIXME: uncomment this later
// pub const FIXTURES_DIR: &str = "../deps/sparse-merkle-tree/fixtures";
// #[test]
// fn test_fixtures() {
//     for i in 0..100 {
//         let path = format!("{}/basic/case-{}.json", FIXTURES_DIR, i);
//         let content = fs::read(&path).expect("read");
//         let case: Case = serde_json::from_slice(&content).expect("parse json");
//         run_test_case(case).expect("test case c impl");
//         println!("pass {}", i);
//     }
// }

proptest! {
    #[test]
    fn test_random_merkle_proof(key: [u8; 32], value: [u8;32]) {
        let key = H256::from(key);
        let value = H256::from(value);
        const EXPECTED_PROOF_SIZE: usize = 16;

        let mut tree = CkbSMT::default();
        tree.update(key, value).expect("update");
        if !tree.is_empty() {
            let proof = tree.merkle_proof(vec![key]).expect("proof");
            let compiled_proof = proof
                .clone()
                .compile(vec![key])
                .expect("compile proof");
            assert!(proof.merkle_path().len() < EXPECTED_PROOF_SIZE);
            assert!(proof
                    .verify::<CkbBlake2bHasher>(tree.root(), vec![(key, value)])
                    .expect("verify"));
            assert!(compiled_proof
                    .verify::<CkbBlake2bHasher>(tree.root(), vec![(key, value)])
                    .expect("compiled verify"));

            let compiled_proof_bin: Vec<u8> = compiled_proof.into();
            let mut smt_state = SmtCImpl::new(8);
            smt_state.insert(key.as_slice(), value.as_slice()).unwrap();
            smt_state.normalize();
            smt_state
                .verify(tree.root().as_slice(), &compiled_proof_bin)
                .expect("verify with c");
        }
    }
}
