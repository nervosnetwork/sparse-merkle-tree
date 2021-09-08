use crate::collections::linked_list::LinkedList;
use crate::H256;
use core::{ffi::c_void, result::Result};

#[link(name = "dl-c-impl", kind = "static")]
extern "C" {
    fn smt_state_new(capacity: u32) -> *mut c_void;
    fn smt_state_del(state: *mut c_void);

    fn smt_state_insert(state: *mut c_void, key: *const u8, value: *const u8) -> isize;
    //fn smt_state_fetch(state: *mut c_void, key: *const u8, value: *mut u8) -> isize;
    fn smt_state_normalize(state: *mut c_void);
    fn smt_verify(
        hash: *const u8,
        state: *const c_void,
        proof: *const u8,
        proof_length: u32,
    ) -> isize;
}

struct OriginCKBSmt {
    state: *mut c_void,
}

impl OriginCKBSmt {
    pub fn new(capacity: u32) -> OriginCKBSmt {
        unsafe {
            OriginCKBSmt {
                state: smt_state_new(capacity),
            }
        }
    }

    pub fn insert(&self, key: *const u8, value: *const u8) -> isize {
        unsafe { smt_state_insert(self.state, key, value) }
    }

    pub fn normalize(&self) {
        unsafe {
            smt_state_normalize(self.state);
        }
    }

    pub fn verify(&self, root: &[u8], proof: &[u8]) -> isize {
        unsafe {
            smt_verify(
                root.as_ptr(),
                self.state,
                proof.as_ptr(),
                proof.len() as u32,
            )
        }
    }
}

impl Drop for OriginCKBSmt {
    fn drop(&mut self) {
        unsafe { smt_state_del(self.state) }
    }
}

pub struct CkbSmt {
    origin_smt: OriginCKBSmt,
}

#[derive(Default)]
pub struct CkbSmtBuilder {
    data: LinkedList<(H256, H256)>,
}

impl CkbSmtBuilder {
    pub fn new() -> CkbSmtBuilder {
        CkbSmtBuilder {
            data: LinkedList::new(),
        }
    }

    pub fn insert(&mut self, key: &H256, value: &H256) {
        self.data.push_back((*key, *value));
    }

    pub fn build(&self) -> Result<CkbSmt, i32> {
        let smt = CkbSmt {
            origin_smt: OriginCKBSmt::new(self.data.len() as u32),
        };

        for (key, val) in &self.data {
            let insert_ret = smt
                .origin_smt
                .insert(key.as_slice().as_ptr(), val.as_slice().as_ptr());
            if insert_ret != 0 {
                return Err(insert_ret as i32);
            }
        }

        smt.origin_smt.normalize();
        Ok(smt)
    }
}

impl CkbSmt {
    pub fn verify(&self, root: &H256, proof: &[u8]) -> Result<(), i32> {
        let verify_ret = self.origin_smt.verify(root.as_slice(), proof);
        if verify_ret != 0 {
            return Err(verify_ret as i32);
        }
        Ok(())
    }
}
