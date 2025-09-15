use std::fs;

use crate::{
    blake2b::Blake2bHasher, default_store::DefaultStore, CompiledMerkleProof, SparseMerkleTree,
    H256,
};
use anyhow::Result;
use rand::{prelude::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};

type Leave = ([u8; 32], [u8; 32]);

#[derive(Default, Serialize, Deserialize)]
struct Proof {
    leaves: Vec<Leave>,
    compiled_proof: Vec<u8>,
    error: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct Case {
    name: String,
    leaves: Vec<Leave>,
    root: [u8; 32],
    proofs: Vec<Proof>,
}

type Smt = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;

fn new_smt(pairs: Vec<(H256, H256)>) -> Smt {
    let mut smt = Smt::default();
    for (key, value) in pairs {
        smt.update(key, value).unwrap();
    }
    smt
}

fn leaves(min_leaves: usize, max_leaves: usize) -> Vec<(H256, H256)> {
    let mut rng = thread_rng();
    let size = rng.gen_range(min_leaves..=max_leaves);
    let mut pairs: Vec<_> = (0..size)
        .map(|_| {
            let mut k = [0u8; 32];
            let mut v = [0u8; 32];
            rng.fill(&mut k);
            rng.fill(&mut v);
            (k.into(), v.into())
        })
        .collect();
    pairs.dedup_by_key(|(k, _v)| *k);
    pairs
}

fn gen_test_case(name: String) -> Case {
    let leaves = leaves(1, 50);
    let smt = new_smt(leaves.clone());
    let mut rng = thread_rng();

    let mut proofs = Vec::new();
    for _i in 0..5 {
        let amount = rng.gen_range(0..=leaves.len());
        let leaves_to_proof: Vec<_> = leaves.choose_multiple(&mut rng, amount).cloned().collect();
        let keys: Vec<_> = leaves_to_proof.iter().map(|(k, _v)| *k).collect();
        let proof = match smt.merkle_proof(keys.clone()) {
            Ok(proof) => {
                let compiled_proof = proof.clone().compile(keys).expect("compile proof");
                Proof {
                    leaves: leaves_to_proof
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                    compiled_proof: compiled_proof.into(),
                    error: None,
                }
            }
            Err(err) => Proof {
                leaves: Default::default(),
                compiled_proof: Default::default(),
                error: Some(format!("{}", err)),
            },
        };
        proofs.push(proof);
    }

    Case {
        name,
        root: (*smt.root()).into(),
        leaves: leaves
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect(),
        proofs,
    }
}

// use `cargo test test_gen_fixtures -- --ignored` to regenerate fixtures
#[test]
#[ignore]
fn test_gen_fixtures() {
    for i in 0..100 {
        let name = format!("case-{}", i);
        let case = gen_test_case(name.clone());
        let content = serde_json::to_vec_pretty(&case).expect("to json");
        let path = format!("{}/basic/{}.json", FIXTURES_DIR, name);
        fs::write(&path, content).expect("write");
        println!("write {}", &path);
    }
}

fn run_test_case(case: Case) -> Result<()> {
    let Case {
        name: _name,
        leaves,
        root,
        proofs,
    } = case;
    let smt = new_smt(
        leaves
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect(),
    );
    assert_eq!(smt.root(), &root.into(), "root");

    for proof in proofs {
        let Proof {
            leaves,
            compiled_proof,
            error,
        } = proof;
        let keys = leaves.iter().map(|(k, _v)| (*k).into()).collect();
        let actual_compiled_proof: Vec<u8> = match smt.merkle_proof(keys) {
            Ok(proof) => proof
                .compile(leaves.iter().map(|(k, _v)| (*k).into()).collect())?
                .into(),
            Err(err) => {
                let expected_error = error.expect("expected error");
                assert_eq!(expected_error, format!("{}", err));
                return Ok(());
            }
        };

        assert_eq!(compiled_proof, actual_compiled_proof, "proof");

        let proof = CompiledMerkleProof(compiled_proof);
        let proof_root = proof
            .compute_root::<Blake2bHasher>(
                leaves
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            )
            .expect("compute_root");
        assert_eq!(smt.root(), &proof_root, "proof root");
    }

    Ok(())
}

const FIXTURES_DIR: &str = "fixtures";

#[test]
fn test_fixtures() {
    for i in 0..100 {
        let path = format!("{}/basic/case-{}.json", FIXTURES_DIR, i);
        let content = fs::read(&path).expect("read");
        let case: Case = serde_json::from_slice(&content).expect("parse json");
        run_test_case(case).expect("test case");
        println!("pass {}", i);
    }
}
