//! Constructs a new `SparseMerkleTree<H, V, S>`.
//!
//! # Examples
//!
//! ```
//! use sparse_merkle_tree::{
//!     blake2b::Blake2bHasher, default_store::DefaultStore,
//!     error::Error, MerkleProof,
//!     SparseMerkleTree, traits::Value, H256
//! };
//! use blake2b_rs::{Blake2b, Blake2bBuilder};
//!
//! // define SMT
//! type SMT = SparseMerkleTree<Blake2bHasher, Word, DefaultStore<Word>>;
//!
//! // define SMT value
//! #[derive(Default, Clone)]
//! pub struct Word(String);
//! impl Value for Word {
//!    fn to_h256(&self) -> H256 {
//!        if self.0.is_empty() {
//!            return H256::zero();
//!        }
//!        let mut buf = [0u8; 32];
//!        let mut hasher = new_blake2b();
//!        hasher.update(self.0.as_bytes());
//!        hasher.finalize(&mut buf);
//!        buf.into()
//!    }
//!    fn zero() -> Self {
//!        Default::default()
//!    }
//! }
//!
//! // helper function
//! fn new_blake2b() -> Blake2b {
//!     Blake2bBuilder::new(32).personal(b"SMT").build()
//! }
//!
//! fn construct_smt() {
//!     let mut tree = SMT::default();
//!     for (i, word) in "The quick brown fox jumps over the lazy dog"
//!         .split_whitespace()
//!         .enumerate()
//!     {
//!         let key: H256 = {
//!             let mut buf = [0u8; 32];
//!             let mut hasher = new_blake2b();
//!             hasher.update(&(i as u32).to_le_bytes());
//!             hasher.finalize(&mut buf);
//!             buf.into()
//!         };
//!         let value = Word(word.to_string());
//!         // insert key value into tree
//!         tree.update(key, value).expect("update");
//!     }
//!
//!     println!("SMT root is {:?} ", tree.root());
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

pub mod blake2b;
#[cfg(feature = "smtc")]
pub mod ckb_smt;
pub mod default_store;
pub mod error;
pub mod h256;
pub mod merge;
pub mod merkle_proof;
#[cfg(test)]
mod tests;
pub mod traits;
pub mod tree;

#[cfg(feature = "smtc")]
pub use ckb_smt::{SMTBuilder, SMT};
pub use h256::H256;
pub use merkle_proof::{CompiledMerkleProof, MerkleProof};
pub use tree::SparseMerkleTree;

/// Expected path size: log2(256) * 2, used for hint vector capacity
pub const EXPECTED_PATH_SIZE: usize = 16;
// Max stack size can be used when verify compiled proof
pub(crate) const MAX_STACK_SIZE: usize = 257;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::collections;
        use std::vec;
        use std::string;
    } else {
        extern crate alloc;
        use alloc::collections;
        use alloc::vec;
        use alloc::string;
    }
}
