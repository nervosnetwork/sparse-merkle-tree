import { CkbSmt, ckb_blake2b_256, verify_proof } from "sparse-merkle-tree-wasm";
import { ZERO_HASH } from "../src/misc";

beforeAll(async () => {
    // await init();
});

test("ckbs smt", () => {
    const smt = new CkbSmt();

    const k1 = ckb_blake2b_256("aaa");
    const v1 = ckb_blake2b_256("123aa");

    const k2 = ckb_blake2b_256("bbb");
    const v2 = ckb_blake2b_256(new Uint8Array([0xaa, 0xbb, 0xcc]));

    const k3 = ckb_blake2b_256("ccc");
    const v3 = ckb_blake2b_256("232cc");

    smt.update(k1, v1);

    smt.update(k2, v2);
    const root2 = smt.root();

    smt.update(k3, v3);
    const root3 = smt.root();

    let proof = smt.get_proof([k1, k3]);

    console.assert(verify_proof(root2, proof, [[k1, v1], [k3, ZERO_HASH]]));
    console.assert(verify_proof(root3, proof, [[k1, v1], [k3, v3]]));
});
