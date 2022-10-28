use crate::{
    branch::{BranchKey, BranchNode},
    collections,
    error::Error,
    traits::{StoreReadOps, StoreWriteOps},
    H256,
};

#[derive(Debug, Clone, Default)]
pub struct StoreWriteCounter {
    insert: usize,
    remove: usize,
}

impl StoreWriteCounter {
    pub fn insert(&self) -> usize {
        self.insert
    }
    pub fn remove(&self) -> usize {
        self.remove
    }

    pub fn reset_insert(&mut self) {
        self.insert = 0
    }

    pub fn reset_remove(&mut self) {
        self.remove = 0
    }

    pub fn reset_all(&mut self) {
        self.insert = 0;
        self.remove = 0;
    }
}

#[derive(Debug, Clone, Default)]
pub struct DefaultStore<V> {
    branches_map: Map<BranchKey, BranchNode>,
    leaves_map: Map<H256, V>,
    enable_counter: bool,
    branches_counter: StoreWriteCounter,
    leaves_counter: StoreWriteCounter,
}

impl<V> DefaultStore<V> {
    pub fn branches_map(&self) -> &Map<BranchKey, BranchNode> {
        &self.branches_map
    }
    pub fn leaves_map(&self) -> &Map<H256, V> {
        &self.leaves_map
    }
    pub fn clear(&mut self) {
        self.branches_map.clear();
        self.leaves_map.clear();
    }

    pub fn enable_counter(&mut self, enable: bool) {
        self.enable_counter = enable;
    }

    pub fn is_counter_on(&self) -> bool {
        self.enable_counter
    }

    pub fn branches_counter(&self) -> &StoreWriteCounter {
        &self.branches_counter
    }

    pub fn leaves_counter(&self) -> &StoreWriteCounter {
        &self.leaves_counter
    }

    pub fn reset_all_counters(&mut self) {
        self.leaves_counter.reset_all();
        self.branches_counter.reset_all()
    }
}

impl<V: Clone> StoreReadOps<V> for DefaultStore<V> {
    fn get_branch(&self, branch_key: &BranchKey) -> Result<Option<BranchNode>, Error> {
        Ok(self.branches_map.get(branch_key).map(Clone::clone))
    }
    fn get_leaf(&self, leaf_key: &H256) -> Result<Option<V>, Error> {
        Ok(self.leaves_map.get(leaf_key).map(Clone::clone))
    }
}

impl<V> StoreWriteOps<V> for DefaultStore<V> {
    fn insert_branch(&mut self, branch_key: BranchKey, branch: BranchNode) -> Result<(), Error> {
        self.branches_map.insert(branch_key, branch);
        if self.enable_counter {
            self.branches_counter.insert += 1;
        }
        Ok(())
    }
    fn insert_leaf(&mut self, leaf_key: H256, leaf: V) -> Result<(), Error> {
        self.leaves_map.insert(leaf_key, leaf);
        if self.enable_counter {
            self.leaves_counter.insert += 1;
        }
        Ok(())
    }
    fn remove_branch(&mut self, branch_key: &BranchKey) -> Result<(), Error> {
        self.branches_map.remove(branch_key);
        if self.enable_counter {
            self.branches_counter.remove += 1;
        }
        Ok(())
    }
    fn remove_leaf(&mut self, leaf_key: &H256) -> Result<(), Error> {
        self.leaves_map.remove(leaf_key);
        if self.enable_counter {
            self.leaves_counter.remove += 1;
        }
        Ok(())
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        pub type Map<K, V> = collections::HashMap<K, V>;
        pub type Entry<'a, K, V> = collections::hash_map::Entry<'a, K, V>;
    } else {
        pub type Map<K, V> = collections::BTreeMap<K, V>;
        pub type Entry<'a, K, V> = collections::btree_map::Entry<'a, K, V>;
    }
}
