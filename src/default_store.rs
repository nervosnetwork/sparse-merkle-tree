use crate::{
    collections,
    error::Error,
    traits::{StoreReadOps, StoreWriteOps},
    tree::BranchNode,
    H256,
};

#[derive(Debug, Clone, Default)]
pub struct DefaultStore<V> {
    nodes: Map<H256, BranchNode>,
    leaves: Map<H256, V>,
}

impl<V> DefaultStore<V> {
    pub fn branches_map(&self) -> &Map<H256, BranchNode> {
        &self.nodes
    }
    pub fn leaves_map(&self) -> &Map<H256, V> {
        &self.leaves
    }
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.leaves.clear();
    }
}

impl<V: Clone> StoreReadOps<V> for DefaultStore<V> {
    fn get_branch(&self, key: &H256) -> Result<Option<BranchNode>, Error> {
        Ok(self.nodes.get(key).map(Clone::clone))
    }
    fn get_leaf(&self, key: &H256) -> Result<Option<V>, Error> {
        Ok(self.leaves.get(key).map(Clone::clone))
    }
}

impl<V> StoreWriteOps<V> for DefaultStore<V> {
    fn insert_branch(&mut self, key: H256, branch: BranchNode) -> Result<(), Error> {
        self.nodes.insert(key, branch);
        Ok(())
    }
    fn insert_leaf(&mut self, key: H256, leaf: V) -> Result<(), Error> {
        self.leaves.insert(key, leaf);
        Ok(())
    }
    fn remove_branch(&mut self, key: &H256) -> Result<(), Error> {
        self.nodes.remove(key);
        Ok(())
    }
    fn remove_leaf(&mut self, key: &H256) -> Result<(), Error> {
        self.leaves.remove(key);
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
