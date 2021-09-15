use crate::{string, H256};

pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    MissingBranch(u8, H256),
    MissingLeaf(H256),
    CorruptedProof,
    EmptyProof,
    EmptyKeys,
    IncorrectNumberOfLeaves { expected: usize, actual: usize },
    Store(string::String),
    CorruptedStack,
    NonSiblings,
    InvalidCode(u8),
    NonMergableRange,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::MissingBranch(height, key) => {
                write!(
                    f,
                    "Corrupted store, missing branch height:{}, key:{:?}",
                    height, key
                )?;
            }
            Error::MissingLeaf(key) => {
                write!(f, "Corrupted store, missing leaf {:?}", key)?;
            }
            Error::CorruptedProof => {
                write!(f, "Corrupted proof")?;
            }
            Error::EmptyProof => {
                write!(f, "Empty proof")?;
            }
            Error::EmptyKeys => {
                write!(f, "Empty keys")?;
            }
            Error::IncorrectNumberOfLeaves { expected, actual } => {
                write!(
                    f,
                    "Incorrect number of leaves, expected {} actual {}",
                    expected, actual
                )?;
            }
            Error::Store(err_msg) => {
                write!(f, "Backend store error: {}", err_msg)?;
            }
            Error::CorruptedStack => {
                write!(f, "Corrupted serialized proof stack")?;
            }
            Error::NonSiblings => {
                write!(f, "Merging non-siblings in serialized stack")?;
            }
            Error::InvalidCode(code) => {
                write!(f, "Invalid serialized proof code: {}", code)?;
            }
            Error::NonMergableRange => {
                write!(f, "Ranges can not be merged")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for Error {}
