#[macro_use]
extern crate criterion;

use criterion::Criterion;
use rand::{thread_rng, Rng};
use smt_rocksdb::{
    config::StoreConfig,
    smt::{
        db::{Store, StoreTransaction},
        smt_store::SMTStore,
    },
    RocksDB,
};
use sparse_merkle_tree::{blake2b::Blake2bHasher, tree::SparseMerkleTree, H256};

const TARGET_LEAVES_COUNT: usize = 20;
const LEAF_COL: u8 = 0;
const BRANCH_COL: u8 = 1;
const COLUMNS: u32 = 2;

#[allow(clippy::upper_case_acronyms)]
type SMT<'a> = SparseMerkleTree<Blake2bHasher, H256, SMTStore<'a, StoreTransaction>>;

fn random_h256(rng: &mut impl Rng) -> H256 {
    let mut buf = [0u8; 32];
    rng.fill(&mut buf);
    buf.into()
}

fn random_smt<'a>(
    db: &'a StoreTransaction,
    update_count: usize,
    rng: &mut impl Rng,
) -> (SMT<'a>, Vec<H256>) {
    let store = SMTStore::new(LEAF_COL, BRANCH_COL, db);
    let mut smt = SparseMerkleTree::new(H256::default(), store);
    let mut keys = Vec::with_capacity(update_count);
    for _ in 0..update_count {
        let key = random_h256(rng);
        let value = random_h256(rng);
        smt.update(key, value).unwrap();
        keys.push(key);
    }
    (smt, keys)
}

fn random_smt_update_all<'a>(db: &'a StoreTransaction, update_count: usize, rng: &mut impl Rng) {
    let store = SMTStore::new(LEAF_COL, BRANCH_COL, db);
    let mut smt = SparseMerkleTree::<Blake2bHasher, _, _>::new(H256::default(), store);
    let mut kvs = Vec::with_capacity(update_count);
    for _ in 0..update_count {
        let key = random_h256(rng);
        let value = random_h256(rng);
        kvs.push((key, value));
    }
    smt.update_all(kvs).unwrap();
}

fn open_db() -> Store {
    if let Err(err) = std::fs::remove_dir_all(".tmp/db") {
        println!("clean db: {}", err);
    }
    if let Err(err) = std::fs::create_dir_all(".tmp") {
        println!("create tmp dir: {}", err);
    }
    let config = StoreConfig {
        path: ".tmp/db".into(),
        cache_size: Some(512 * 1024 * 1024),
        ..Default::default()
    };
    Store::new(RocksDB::open(&config, COLUMNS))
}

fn bench(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "SMT update",
        |b, &&size| {
            let store = open_db();
            b.iter(|| {
                let mut rng = thread_rng();
                let db = store.begin_transaction();
                random_smt(&db, size, &mut rng);
                db.commit().expect("ok")
            });
        },
        &[100, 10_000],
    );

    c.bench_function_over_inputs(
        "SMT update_all",
        |b, &&size| {
            let store = open_db();
            b.iter(|| {
                let mut rng = thread_rng();
                let db = store.begin_transaction();
                random_smt_update_all(&db, size, &mut rng);
                db.commit().expect("ok")
            });
        },
        &[100, 10_000],
    );

    c.bench_function_over_inputs(
        "SMT get",
        |b, &&size| {
            let store = open_db();
            let mut rng = thread_rng();
            let db = store.begin_transaction();
            let (smt, _keys) = random_smt(&db, size, &mut rng);
            db.commit().expect("ok");
            b.iter(|| {
                let key = random_h256(&mut rng);
                smt.get(&key).unwrap();
            });
        },
        &[5_000, 10_000],
    );

    c.bench_function("SMT generate merkle proof", |b| {
        let mut rng = thread_rng();
        let store = open_db();
        let db = store.begin_transaction();
        let (smt, mut keys) = random_smt(&db, 10_000, &mut rng);
        db.commit().expect("ok");
        keys.dedup();
        let keys: Vec<_> = keys.into_iter().take(TARGET_LEAVES_COUNT).collect();
        b.iter(|| {
            smt.merkle_proof(keys.clone()).unwrap();
        });
    });

    c.bench_function("SMT verify merkle proof", |b| {
        let mut rng = thread_rng();
        let store = open_db();
        let db = store.begin_transaction();
        let (smt, mut keys) = random_smt(&db, 10_000, &mut rng);
        db.commit().expect("ok");
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
