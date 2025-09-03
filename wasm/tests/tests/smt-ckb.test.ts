import { Resource, Verifier } from "ckb-testtool";
import { hexFrom, Transaction, hashCkb, WitnessArgs } from "@ckb-ccc/core";
import { readFileSync } from "fs";

import { CkbSmt, hash_data, verify_proof } from "sparse-merkle-tree-wasm";

import { createJSScript, createScript, SCRIPT_ALWAYS_SUCCESS } from "../src/misc";

const SCRIPT_SMT = readFileSync('ckb-contract/dist/ckb-test-smt-wasm.bc')

test('ckb-smt success', () => {
    const resource = Resource.default();
    const tx = Transaction.default();
    const lockScript = createScript(resource, tx, hexFrom(SCRIPT_ALWAYS_SUCCESS), "0x0102");

    const typeId = "0x000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f";
    const typeScript = createJSScript(resource, tx, hexFrom(SCRIPT_SMT), typeId);

    let smt = new CkbSmt();
    smt.update(hash_data("111"), hash_data("456"));
    smt.update(hash_data("222"), hash_data("456"));
    smt.update(hash_data("333"), hash_data("456"));
    smt.update(hash_data("444"), hash_data("456"));

    let k1 = hashCkb(hexFrom('0xaabb'));
    let v1 = hashCkb(hexFrom('0x1122'));
    console.log(`k1: ${k1}`);
    console.log(`v1: ${v1}`);

    let k2 = hashCkb(hexFrom('0xbbbb'));
    let v2 = hashCkb(hexFrom('0x3344'));
    console.log(`k2: ${k2}`);
    console.log(`v2: ${v2}`);

    smt.update(k1, v1);
    const root1 = smt.root();
    console.log(`root1: ${root1}`)

    smt.update(k2, v2);
    const root2 = smt.root();
    console.log(`root2: ${root2}`)

    let proof = smt.get_proof([k1, k2]);
    const witness = WitnessArgs.from({
        inputType: hexFrom(proof)
    });

    console.log(verify_proof(root1, proof, [[k1, v1], [k2, ""]]))
    console.log(verify_proof(root2, proof, [[k1, v1], [k2, v2]]))

    const inputCell = resource.mockCell(lockScript, typeScript, hexFrom(root1));
    const outputCell = Resource.createCellOutput(lockScript, typeScript);

    tx.inputs.push(Resource.createCellInput(inputCell));
    tx.outputs.push(outputCell);
    tx.outputsData.push(hexFrom(root2));
    tx.witnesses.push(hexFrom(witness.toBytes()));

    // verify the transaction
    const verifier = Verifier.from(resource, tx);
    verifier.verifySuccess(true);
});
