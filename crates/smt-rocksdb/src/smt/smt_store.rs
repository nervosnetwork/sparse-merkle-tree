//! Implement SMTStore trait

use std::convert::TryInto;

use crate::db::Col;
use crate::traits::kv_store::KVStore;
use sparse_merkle_tree::{
    error::Error as SMTError,
    traits::Store,
    tree::{BranchKey, BranchNode},
    H256,
};

use super::serde::{branch_key_to_vec, branch_node_to_vec, slice_to_branch_node};

pub struct SMTStore<'a, DB: KVStore> {
    leaf_col: Col,
    branch_col: Col,
    store: &'a DB,
}

impl<'a, DB: KVStore> SMTStore<'a, DB> {
    pub fn new(leaf_col: Col, branch_col: Col, store: &'a DB) -> Self {
        SMTStore {
            leaf_col,
            branch_col,
            store,
        }
    }

    pub fn inner_store(&self) -> &DB {
        self.store
    }
}

impl<'a, DB: KVStore> Store<H256> for SMTStore<'a, DB> {
    fn get_branch(&self, branch_key: &BranchKey) -> Result<Option<BranchNode>, SMTError> {
        match self
            .store
            .get(self.branch_col, &branch_key_to_vec(branch_key))
        {
            Some(slice) => Ok(Some(slice_to_branch_node(&slice))),
            None => Ok(None),
        }
    }

    fn get_leaf(&self, leaf_key: &H256) -> Result<Option<H256>, SMTError> {
        match self.store.get(self.leaf_col, leaf_key.as_slice()) {
            Some(slice) if 32 == slice.len() => {
                let leaf: [u8; 32] = slice.as_ref().try_into().unwrap();
                Ok(Some(H256::from(leaf)))
            }
            Some(_) => Err(SMTError::Store("get corrupted leaf".to_string())),
            None => Ok(None),
        }
    }

    fn insert_branch(&mut self, branch_key: BranchKey, branch: BranchNode) -> Result<(), SMTError> {
        self.store
            .insert_raw(
                self.branch_col,
                &branch_key_to_vec(&branch_key),
                &branch_node_to_vec(&branch),
            )
            .map_err(|err| SMTError::Store(format!("insert error {}", err)))?;

        Ok(())
    }

    fn insert_leaf(&mut self, leaf_key: H256, leaf: H256) -> Result<(), SMTError> {
        self.store
            .insert_raw(self.leaf_col, leaf_key.as_slice(), leaf.as_slice())
            .map_err(|err| SMTError::Store(format!("insert error {}", err)))?;

        Ok(())
    }

    fn remove_branch(&mut self, branch_key: &BranchKey) -> Result<(), SMTError> {
        self.store
            .delete(self.branch_col, &branch_key_to_vec(branch_key))
            .map_err(|err| SMTError::Store(format!("delete error {}", err)))?;

        Ok(())
    }

    fn remove_leaf(&mut self, leaf_key: &H256) -> Result<(), SMTError> {
        self.store
            .delete(self.leaf_col, leaf_key.as_slice())
            .map_err(|err| SMTError::Store(format!("delete error {}", err)))?;

        Ok(())
    }
}
