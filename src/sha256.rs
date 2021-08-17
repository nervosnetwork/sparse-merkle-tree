use crate::{traits::Hasher, H256};
use sha2::{Digest, Sha256};

#[derive(Clone, Default)]
pub struct Sha256Hasher(Sha256);

impl Hasher for Sha256Hasher {
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }

    fn write_byte(&mut self, b: u8) {
        self.0.update(&[b][..]);
    }

    fn finish(self) -> H256 {
        self.0.finalize().into()
    }
}
