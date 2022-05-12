#[macro_use]
extern crate criterion;

use criterion::Criterion;
use rand::{thread_rng, Rng};
use sparse_merkle_tree::{
    blake2b::Blake2bHasher, default_store::DefaultStore, tree::SparseMerkleTree, H256,
};

const TARGET_LEAVES_COUNT: usize = 20;

#[allow(clippy::upper_case_acronyms)]
type SMT = SparseMerkleTree<Blake2bHasher, H256, DefaultStore<H256>>;

fn random_h256(rng: &mut impl Rng) -> H256 {
    let mut buf = [0u8; 32];
    rng.fill(&mut buf);
    buf.into()
}

fn random_smt(update_count: usize, rng: &mut impl Rng) -> (SMT, Vec<H256>) {
    let mut smt = SparseMerkleTree::default();
    let mut keys = Vec::with_capacity(update_count);
    for _ in 0..update_count {
        let key = random_h256(rng);
        let value = random_h256(rng);
        smt.update(key, value).unwrap();
        keys.push(key);
    }
    (smt, keys)
}

fn bench(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "SMT update",
        |b, &&size| {
            b.iter(|| {
                let mut rng = thread_rng();
                random_smt(size, &mut rng)
            });
        },
        &[100, 10_000],
    );

    c.bench_function_over_inputs(
        "SMT get",
        |b, &&size| {
            let mut rng = thread_rng();
            let (smt, _keys) = random_smt(size, &mut rng);
            b.iter(|| {
                let key = random_h256(&mut rng);
                smt.get(&key).unwrap();
            });
        },
        &[5_000, 10_000],
    );

    c.bench_function("SMT generate merkle proof", |b| {
        let mut rng = thread_rng();
        let (smt, mut keys) = random_smt(10_000, &mut rng);
        keys.dedup();
        let keys: Vec<_> = keys.into_iter().take(TARGET_LEAVES_COUNT).collect();
        b.iter(|| {
            smt.merkle_proof(keys.clone()).unwrap();
        });
    });

    c.bench_function("SMT verify merkle proof", |b| {
        let mut rng = thread_rng();
        let (smt, mut keys) = random_smt(10_000, &mut rng);
        keys.dedup();
        let leaves: Vec<_> = keys
            .iter()
            .take(TARGET_LEAVES_COUNT)
            .map(|k| (*k, smt.get(k).unwrap()))
            .collect();
        let proof = smt
            .merkle_proof(keys.into_iter().take(TARGET_LEAVES_COUNT).collect())
            .unwrap();
        let root = smt.root();
        b.iter(|| {
            let valid = proof.clone().verify::<Blake2bHasher>(root, leaves.clone());
            assert!(valid.expect("verify result"));
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench
);
criterion_main!(benches);
