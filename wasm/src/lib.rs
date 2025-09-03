use blake2b_ref::{Blake2b, Blake2bBuilder};
use sparse_merkle_tree::{
    CompiledMerkleProof, H256, SparseMerkleTree, default_store::DefaultStore, traits::Hasher,
};
use wasm_bindgen::{JsCast, prelude::*, throw_str};

pub struct CkbBlake2bHasher(Blake2b);
impl Default for CkbBlake2bHasher {
    fn default() -> Self {
        // NOTE: here we not set the `personal` since ckb_smt.c linked blake2b implementation from blake2b-rs
        let blake2b = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        CkbBlake2bHasher(blake2b)
    }
}

impl Hasher for CkbBlake2bHasher {
    fn write_byte(&mut self, b: u8) {
        self.0.update(&[b][..]);
    }
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }
    fn finish(self) -> H256 {
        let mut hash = [0u8; 32];
        self.0.finalize(&mut hash);
        hash.into()
    }
}
impl CkbBlake2bHasher {
    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
}

type SMT = SparseMerkleTree<CkbBlake2bHasher, H256, DefaultStore<H256>>;

#[wasm_bindgen]
#[derive()]
pub struct CkbSmt {
    smt: SMT,
}

fn h256_to_str(d: &H256) -> String {
    format!("0x{}", hex::encode(d.as_slice()))
}

fn str_to_h256(d: &str) -> H256 {
    if d.is_empty() {
        return H256::zero();
    }
    let d = if d.starts_with("0x") {
        hex::decode(&d[2..])
    } else {
        hex::decode(d)
    }
    .unwrap_or_else(|e| throw_str(&format!("hex decode failed, err: {:?}, d: {}", e, d)));

    let d: [u8; 32] = d
        .try_into()
        .unwrap_or_else(|e| throw_str(&format!("to [u8; 32] failed, d: {:?}", e)));

    d.into()
}

#[wasm_bindgen]
pub fn hash_data(d: wasm_bindgen::JsValue) -> String {
    if d.is_string() {
        let d = d.as_string().unwrap().as_bytes().to_vec();

        let mut hasher = CkbBlake2bHasher::default();
        hasher.update(&d);
        h256_to_str(&hasher.finish())
    } else {
        throw_str("unsupport type");
    }
}

#[wasm_bindgen]
impl CkbSmt {
    #[wasm_bindgen(constructor)]
    pub fn new() -> CkbSmt {
        let smt = SMT::default();

        CkbSmt { smt }
    }

    pub fn root(&self) -> String {
        let h = self.smt.root();
        h256_to_str(h)
    }

    pub fn update(&mut self, key: &str, val: &str) {
        if let Err(err) = self.smt.update(str_to_h256(key), str_to_h256(val)) {
            throw_str(&format!(
                "smt update failed, Err: {:?}, : key: {:?}, val: {:?}",
                err, key, val
            ));
        }
    }

    pub fn get_proof(&self, keys: Vec<String>) -> String {
        let keys: Vec<H256> = keys.into_iter().map(|f| str_to_h256(&f)).collect();
        let proof = match self.smt.merkle_proof(keys.clone()) {
            Ok(p) => p,
            Err(err) => throw_str(&format!(
                "get smt proof failed, err: {:?}, keys: {:?}",
                err, &keys
            )),
        };
        let compile_proof = match proof.compile(keys.clone()) {
            Ok(p) => p,
            Err(err) => throw_str(&format!(
                "get smt proof compile failed, err: {:?}, keys: {:?}",
                err, &keys
            )),
        };

        hex::encode(compile_proof.0)
    }
}

fn parse_leaves(leaves: wasm_bindgen::JsValue) -> Vec<(H256, H256)> {
    let leaves = leaves.dyn_ref::<js_sys::Array>();
    if leaves.is_none() {
        throw_str(&format!("verify proof failed, parse leaves failed"));
    }

    let leaves = leaves.unwrap();
    leaves
        .iter()
        .map(|pair| {
            if let Some(pair) = pair.dyn_ref::<js_sys::Array>() {
                if pair.length() == 2 {
                    (
                        str_to_h256(&pair.get(0).as_string().unwrap()),
                        str_to_h256(&pair.get(1).as_string().unwrap()),
                    )
                } else {
                    throw_str(&format!("verify proof failed, parse leaves failed"))
                }
            } else {
                throw_str(&format!("verify proof failed, parse leaves failed"))
            }
        })
        .collect()
}

#[wasm_bindgen]
pub fn verify_proof(root: &str, proof: &str, leaves: wasm_bindgen::JsValue) -> bool {
    let proof = match hex::decode(proof) {
        Ok(p) => p,
        Err(err) => throw_str(&format!(
            "decode proof data failed, Err: {:?}, proof: {}",
            err, proof
        )),
    };
    let compile_proof = CompiledMerkleProof(proof);
    match compile_proof.verify::<CkbBlake2bHasher>(&str_to_h256(root), parse_leaves(leaves)) {
        Ok(r) => r,
        Err(err) => throw_str(&format!("verify proof failed: {:?}", err)),
    }
}
