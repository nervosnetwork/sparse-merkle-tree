#[macro_use]
extern crate criterion;

use std::sync::atomic::{AtomicUsize, Ordering};

use rand::{thread_rng, Rng};
use sparse_merkle_tree::{
    blake2b::Blake2bHasher,
    tree::{BranchKey, BranchNode},
    default_store::DefaultStore,
    error::Error,
    traits::{StoreReadOps, StoreWriteOps},
    SparseMerkleTree, H256,
};

#[derive(Debug, Default)]
struct DefaultStoreWithCounters<V> {
    store: DefaultStore<V>,
    counters: Counters,
}

#[derive(Debug, Default)]
struct Counters {
    get_branch_counter: AtomicUsize,
    get_leaf_counter: AtomicUsize,
    insert_branch_counter: AtomicUsize,
    insert_leaf_counter: AtomicUsize,
    remove_branch_counter: AtomicUsize,
    remove_leaf_counter: AtomicUsize,
}

impl<V: Clone> StoreReadOps<V> for DefaultStoreWithCounters<V> {
    fn get_branch(&self, branch_key: &BranchKey) -> Result<Option<BranchNode>, Error> {
        self.counters
            .get_branch_counter
            .fetch_add(1, Ordering::SeqCst);
        self.store.get_branch(branch_key)
    }
    fn get_leaf(&self, leaf_key: &H256) -> Result<Option<V>, Error> {
        self.counters
            .get_leaf_counter
            .fetch_add(1, Ordering::SeqCst);
        self.store.get_leaf(leaf_key)
    }
}

impl<V> StoreWriteOps<V> for DefaultStoreWithCounters<V> {
    fn insert_branch(&mut self, branch_key: BranchKey, branch: BranchNode) -> Result<(), Error> {
        self.counters
            .insert_branch_counter
            .fetch_add(1, Ordering::SeqCst);
        self.store.insert_branch(branch_key, branch)
    }
    fn insert_leaf(&mut self, leaf_key: H256, leaf: V) -> Result<(), Error> {
        self.counters
            .insert_leaf_counter
            .fetch_add(1, Ordering::SeqCst);
        self.store.insert_leaf(leaf_key, leaf)
    }
    fn remove_branch(&mut self, branch_key: &BranchKey) -> Result<(), Error> {
        self.counters
            .remove_branch_counter
            .fetch_add(1, Ordering::SeqCst);
        self.store.remove_branch(branch_key)
    }
    fn remove_leaf(&mut self, leaf_key: &H256) -> Result<(), Error> {
        self.counters
            .remove_leaf_counter
            .fetch_add(1, Ordering::SeqCst);
        self.store.remove_leaf(leaf_key)
    }
}

#[allow(clippy::upper_case_acronyms)]
type SMT = SparseMerkleTree<Blake2bHasher, H256, DefaultStoreWithCounters<H256>>;

fn random_h256(rng: &mut impl Rng) -> H256 {
    let mut buf = [0u8; 32];
    rng.fill(&mut buf);
    buf.into()
}

fn random_smt(update_count: usize, rng: &mut impl Rng) {
    let mut smt = SMT::default();
    let mut keys = Vec::with_capacity(update_count);
    for _ in 0..update_count {
        let key = random_h256(rng);
        let value = random_h256(rng);
        smt.update(key, value).unwrap();
        keys.push(key);
    }
    println!(
        "random update {} keys, store counters: {:?}",
        update_count,
        smt.store().counters
    );
}

fn random_smt_update_all(update_count: usize, rng: &mut impl Rng) {
    let mut smt = SMT::default();
    let mut kvs = Vec::with_capacity(update_count);
    for _ in 0..update_count {
        let key = random_h256(rng);
        let value = random_h256(rng);
        kvs.push((key, value));
    }
    smt.update_all(kvs).unwrap();
    println!(
        "random update_all {} keys, store counters: {:?}",
        update_count,
        smt.store().counters
    );
}

fn main() {
    let mut rng = thread_rng();
    random_smt(100, &mut rng);
    random_smt(10000, &mut rng);
    random_smt_update_all(100, &mut rng);
    random_smt_update_all(10000, &mut rng);
}
