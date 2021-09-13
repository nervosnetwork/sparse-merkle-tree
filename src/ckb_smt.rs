use crate::H256;
use core::{ptr, result::Result};

extern crate std;
use std::{boxed::Box, vec::Vec};

#[repr(C)]
struct smt_pair_t {
    key: [u8; 32],
    value: [u8; 32],
    order: u32,
}

#[repr(C)]
struct smt_state_t {
    pairs: *mut smt_pair_t,
    len: u32,
    capacity: u32,
}

#[link(name = "smt-c-impl", kind = "static")]
extern "C" {
    fn smt_state_init(state: *mut smt_state_t, buffer: *const smt_pair_t, capacity: u32);

    fn smt_state_insert(state: *mut smt_state_t, key: *const u8, value: *const u8) -> i32;
    fn smt_state_normalize(state: *mut smt_state_t);
    fn smt_verify(
        hash: *const u8,
        state: *const smt_state_t,
        proof: *const u8,
        proof_length: u32,
    ) -> i32;
}

pub struct SMTBuilder {
    data: Vec<(H256, H256)>,
}

pub struct SMT {
    state: Box<smt_state_t>,
    _buffer: Vec<smt_pair_t>,
}

impl SMTBuilder {
    pub fn new() -> SMTBuilder {
        SMTBuilder { data: Vec::new() }
    }

    pub fn insert(self, key: &H256, value: &H256) -> Result<Self, i32> {
        let mut ret = self;
        ret.data.push((*key, *value));
        Ok(ret)
    }

    pub fn build(self) -> Result<SMT, i32> {
        let capacity = self.data.len();
        let mut smt = SMT {
            state: Box::new(smt_state_t {
                pairs: ptr::null_mut(),
                len: 0,
                capacity: 0,
            }),
            _buffer: Vec::with_capacity(capacity as usize),
        };
        unsafe {
            smt_state_init(smt.state.as_mut(), smt._buffer.as_ptr(), capacity as u32);

            for (key, value) in self.data {
                let ret = smt_state_insert(
                    smt.state.as_mut(),
                    key.as_slice().as_ptr(),
                    value.as_slice().as_ptr(),
                );
                if ret != 0 {
                    return Err(ret);
                }
            }

            smt_state_normalize(smt.state.as_mut());
        }
        Ok(smt)
    }
}

impl SMT {
    pub fn verify(&self, root: &H256, proof: &[u8]) -> Result<(), i32> {
        unsafe {
            let verify_ret = smt_verify(
                root.as_slice().as_ptr(),
                self.state.as_ref(),
                proof.as_ptr(),
                proof.len() as u32,
            );
            if 0 != verify_ret {
                return Err(verify_ret);
            }
        }
        Ok(())
    }
}
