use crate::{
    collections,
    error::Error,
    traits::Store,
    tree::{BranchKey, BranchNode},
    H256,
};

#[derive(Debug, Clone, Default)]
pub struct DefaultStore<V> {
    branches_map: Map<BranchKey, BranchNode>,
    leaves_map: Map<H256, V>,
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
}

impl<V: Clone> Store<V> for DefaultStore<V> {
    fn get_branch(&self, branch_key: &BranchKey) -> Result<Option<BranchNode>, Error> {
        Ok(self.branches_map.get(branch_key).map(Clone::clone))
    }
    fn get_leaf(&self, leaf_key: &H256) -> Result<Option<V>, Error> {
        Ok(self.leaves_map.get(leaf_key).map(Clone::clone))
    }
    fn insert_branch(&mut self, branch_key: BranchKey, branch: BranchNode) -> Result<(), Error> {
        self.branches_map.insert(branch_key, branch);
        Ok(())
    }
    fn insert_leaf(&mut self, leaf_key: H256, leaf: V) -> Result<(), Error> {
        self.leaves_map.insert(leaf_key, leaf);
        Ok(())
    }
    fn remove_branch(&mut self, branch_key: &BranchKey) -> Result<(), Error> {
        self.branches_map.remove(branch_key);
        Ok(())
    }
    fn remove_leaf(&mut self, leaf_key: &H256) -> Result<(), Error> {
        self.leaves_map.remove(leaf_key);
        Ok(())
    }
}

pub type Map<K, V> = collections::BTreeMap<K, V>;
pub type Entry<'a, K, V> = collections::btree_map::Entry<'a, K, V>;
