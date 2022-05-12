use std::convert::TryInto;

use crate::*;
use blake2b_rs::{Blake2b, Blake2bBuilder};
use default_store::DefaultStore;
use hex::decode;
use proptest::prelude::*;
use traits::Hasher;

fn str_to_h256(src: &str) -> H256 {
    let src = decode(src).unwrap();
    assert!(src.len() == 32);
    let data: [u8; 32] = src.try_into().unwrap();
    H256::from(data)
}

fn str_to_vec(src: &str) -> Vec<u8> {
    decode(src).unwrap()
}

#[test]
fn test_ckb_smt_verify1() {
    let key = str_to_h256("381dc5391dab099da5e28acd1ad859a051cf18ace804d037f12819c6fbc0e18b");
    let val = str_to_h256("9158ce9b0e11dd150ba2ae5d55c1db04b1c5986ec626f2e38a93fe8ad0b2923b");
    let root_hash = str_to_h256("ebe0fab376cd802d364eeb44af20c67a74d6183a33928fead163120ef12e6e06");
    let proof = str_to_vec(
        "4c4fff51ff322de8a89fe589987f97220cfcb6820bd798b31a0b56ffea221093d35f909e580b00000000000000000000000000000000000000000000000000000000000000");

    let builder = SMTBuilder::new();
    let builder = builder.insert(&key, &val).unwrap();

    let smt = builder.build().unwrap();
    assert!(smt.verify(&root_hash, &proof).is_ok());
}

#[test]
fn test_ckb_smt_verify2() {
    let key = str_to_h256("a9bb945be71f0bd2757d33d2465b6387383da42f321072e47472f0c9c7428a8a");
    let val = str_to_h256("a939a47335f777eac4c40fbc0970e25f832a24e1d55adc45a7b76d63fe364e82");
    let root_hash = str_to_h256("6e5c722644cd55cef8c4ed886cd8b44027ae9ed129e70a4b67d87be1c6857842");
    let proof = str_to_vec(
        "4c4fff51fa8aaa2aece17b92ec3f202a40a09f7286522bae1e5581a2a49195ab6781b1b8090000000000000000000000000000000000000000000000000000000000000000");

    let builder = SMTBuilder::new();
    let builder = builder.insert(&key, &val).unwrap();

    let smt = builder.build().unwrap();
    assert!(smt.verify(&root_hash, &proof).is_ok());
}

#[test]
fn test_ckb_smt_verify3() {
    let key = str_to_h256("e8c0265680a02b680b6cbc880348f062b825b28e237da7169aded4bcac0a04e5");
    let val = str_to_h256("2ca41595841e46ce8e74ad749e5c3f1d17202150f99c3d8631233ebdd19b19eb");
    let root_hash = str_to_h256("c8f513901e34383bcec57c368628ce66da7496df0a180ee1e021df3d97cb8f7b");
    let proof = str_to_vec(
        "4c4fff51fa8aaa2aece17b92ec3f202a40a09f7286522bae1e5581a2a49195ab6781b1b8090000000000000000000000000000000000000000000000000000000000000000");

    let builder = SMTBuilder::new();
    let builder = builder.insert(&key, &val).unwrap();

    let smt = builder.build().unwrap();
    assert!(smt.verify(&root_hash, &proof).is_ok());
}

#[test]
fn test_ckb_smt_verify_invalid() {
    let key = str_to_h256("e8c0265680a02b680b6cbc880348f062b825b28e237da7169aded4bcac0a04e5");
    let val = str_to_h256("2ca41595841e46ce8e74ad749e5c3f1d17202150f99c3d8631233ebdd19b19eb");
    let root_hash = str_to_h256("a4cbf1b69a848396ac759f362679e2b185ac87a17cba747d2db1ef6fd929042f");
    let proof =
        str_to_vec("4c50fe32845309d34f132cd6f7ac6a7881962401adc35c19a18d4fffeb511b97eabf86");

    let builder = SMTBuilder::new();
    let builder = builder.insert(&key, &val).unwrap();

    let smt = builder.build().unwrap();
    assert!(smt.verify(&root_hash, &proof).is_err());
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
            let smt_state = SMTBuilder::new();
            let smt_state = smt_state.insert(&key, &value).unwrap();
            let smt = smt_state.build().unwrap();
            smt.verify(tree.root(), &compiled_proof_bin).expect("verify with c");
        }
    }
}
