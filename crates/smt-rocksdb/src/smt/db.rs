#![allow(clippy::mutable_key_type)]

use rocksdb::DBPinnableSlice;

use crate::db::Col;
use crate::traits::kv_store::KVStoreRead;
use crate::traits::kv_store::{KVStore, KVStoreWrite};
use crate::{error::Error, iter::DBIter, DBIterator, IteratorMode, RocksDBTransaction};
use crate::{RocksDB, RocksDBSnapshot};

#[derive(Clone)]
pub struct Store {
    db: RocksDB,
}

impl<'a> Store {
    pub fn new(db: RocksDB) -> Self {
        Store { db }
    }

    pub fn open_tmp(columns: u32) -> Self {
        let db = RocksDB::open_tmp(columns);
        Self::new(db)
    }

    fn get(&'a self, col: Col, key: &[u8]) -> Option<DBPinnableSlice<'a>> {
        self.db
            .get_pinned(col, key)
            .expect("db operation should be ok")
    }

    pub fn begin_transaction(&self) -> StoreTransaction {
        StoreTransaction {
            inner: self.db.transaction(),
        }
    }

    pub fn get_snapshot(&self) -> StoreSnapshot {
        StoreSnapshot::new(self.db.get_snapshot())
    }
}

impl KVStoreRead for Store {
    fn get(&self, col: Col, key: &[u8]) -> Option<Box<[u8]>> {
        self.get(col, key).map(|v| Box::<[u8]>::from(v.as_ref()))
    }
}

pub struct StoreTransaction {
    pub(crate) inner: RocksDBTransaction,
}

impl KVStoreRead for StoreTransaction {
    fn get(&self, col: Col, key: &[u8]) -> Option<Box<[u8]>> {
        self.inner
            .get(col, key)
            .expect("db operation should be ok")
            .map(|v| Box::<[u8]>::from(v.as_ref()))
    }
}

impl KVStoreWrite for StoreTransaction {
    fn insert_raw(&self, col: Col, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.inner.put(col, key, value)
    }

    fn delete(&self, col: Col, key: &[u8]) -> Result<(), Error> {
        self.inner.delete(col, key)
    }
}

impl KVStore for StoreTransaction {}

impl StoreTransaction {
    pub fn commit(&self) -> Result<(), Error> {
        self.inner.commit()
    }

    pub fn rollback(&self) -> Result<(), Error> {
        self.inner.rollback()
    }

    pub fn get_iter(&self, col: Col, mode: IteratorMode) -> DBIter {
        self.inner
            .iter(col, mode)
            .expect("db operation should be ok")
    }
}

pub struct StoreSnapshot {
    inner: RocksDBSnapshot,
}

impl StoreSnapshot {
    pub(crate) fn new(inner: RocksDBSnapshot) -> Self {
        Self { inner }
    }
}

impl KVStoreRead for StoreSnapshot {
    fn get(&self, col: Col, key: &[u8]) -> Option<Box<[u8]>> {
        self.inner
            .get_pinned(col, key)
            .expect("db operation should be ok")
            .map(|v| Box::<[u8]>::from(v.as_ref()))
    }
}
