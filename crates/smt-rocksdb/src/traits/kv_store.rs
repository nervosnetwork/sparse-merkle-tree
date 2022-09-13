use crate::db::Col;
use crate::error::Error;
pub trait KVStoreRead {
    fn get(&self, col: Col, key: &[u8]) -> Option<Box<[u8]>>;
}

pub trait KVStoreWrite {
    fn insert_raw(&self, col: Col, key: &[u8], value: &[u8]) -> Result<(), Error>;
    fn delete(&self, col: Col, key: &[u8]) -> Result<(), Error>;
}

pub trait KVStore: KVStoreRead + KVStoreWrite {}
