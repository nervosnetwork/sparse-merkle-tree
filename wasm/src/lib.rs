use blake2b_ref::{Blake2b, Blake2bBuilder};
use js_sys::{Array, Uint8Array};
use sparse_merkle_tree::{
    CompiledMerkleProof, H256, SparseMerkleTree, default_store::DefaultStore, traits::Hasher,
};
use wasm_bindgen::{prelude::*, throw_str};

pub struct CkbBlake2bHasher(Blake2b);
impl Default for CkbBlake2bHasher {
    fn default() -> Self {
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

type Smt = SparseMerkleTree<CkbBlake2bHasher, H256, DefaultStore<H256>>;

#[wasm_bindgen]
pub fn ckb_blake2b_256(d: wasm_bindgen::JsValue) -> Uint8Array {
    if d.is_string() {
        let d = d.as_string().unwrap().as_bytes().to_vec();

        let mut hasher = CkbBlake2bHasher::default();
        hasher.update(&d);
        Uint8Array::from(hasher.finish().as_slice())
    } else if d.is_instance_of::<Uint8Array>() {
        let mut hasher = CkbBlake2bHasher::default();
        hasher.update(&Uint8Array::from(d).to_vec());
        Uint8Array::from(hasher.finish().as_slice())
    } else {
        throw_str("unsupport type");
    }
}

#[wasm_bindgen]
#[derive(Default)]
pub struct CkbSmt {
    smt: Smt,
}

fn u8a_to_h256(d: &Uint8Array) -> H256 {
    let d: [u8; 32] = d.to_vec().try_into().unwrap();
    d.into()
}

#[wasm_bindgen]
impl CkbSmt {
    #[wasm_bindgen(constructor)]
    pub fn new() -> CkbSmt {
        Default::default()
    }

    pub fn root(&self) -> Uint8Array {
        let h = self.smt.root();
        Uint8Array::from(h.as_slice())
    }

    pub fn update(&mut self, key: &Uint8Array, val: &Uint8Array) {
        if let Err(err) = self.smt.update(u8a_to_h256(key), u8a_to_h256(val)) {
            throw_str(&format!(
                "smt update failed, Err: {err:?}, : key: {key:?}, val: {val:?}"
            ));
        }
    }

    pub fn get_proof(&self, keys: Vec<Uint8Array>) -> Uint8Array {
        let keys: Vec<H256> = keys.into_iter().map(|f| u8a_to_h256(&f)).collect();
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

        Uint8Array::from(compile_proof.0.as_slice())
    }
}

#[wasm_bindgen]
pub fn verify_proof(root: &Uint8Array, proof: &Uint8Array, leaves: Array) -> bool {
    let compile_proof = CompiledMerkleProof(proof.to_vec());

    let mut lv = Vec::<(H256, H256)>::with_capacity(leaves.length() as usize);
    for x in leaves.iter() {
        let pair = js_sys::Array::from(&x);
        let k = Uint8Array::from(pair.get(0));
        let v = Uint8Array::from(pair.get(1));
        lv.push((u8a_to_h256(&k), u8a_to_h256(&v)));
    }

    match compile_proof.verify::<CkbBlake2bHasher>(&u8a_to_h256(root), lv) {
        Ok(r) => r,
        Err(err) => throw_str(&format!("verify proof failed: {err:?}")),
    }
}
