import * as bindings from "@ckb-js-std/bindings";
import { HighLevel, hashCkb } from "@ckb-js-std/core";

function main() {
    // HighLevel.checkTypeId(35);

    let proof = HighLevel.loadWitnessArgs(0, bindings.SOURCE_GROUP_INPUT).inputType;
    if (proof == undefined) { return 1; }

    let k1 = hashCkb(bindings.hex.decode('aabb'));
    let v1 = hashCkb(bindings.hex.decode('1122'));
    let k2 = hashCkb(bindings.hex.decode('bbbb'));
    let v2 = hashCkb(bindings.hex.decode('3344'));
    {
        let root_hash = HighLevel.loadCellData(0, bindings.SOURCE_GROUP_INPUT);
        let smt = new bindings.Smt();
        smt.insert(k1, v1);
        smt.insert(k2, new ArrayBuffer(32));

        if (!smt.verify(root_hash, proof)) {
            console.error("verify smt1 failed");
            return 4;
        }
    }
    {
        let root_hash = HighLevel.loadCellData(0, bindings.SOURCE_GROUP_OUTPUT);
        let smt = new bindings.Smt();
        smt.insert(k1, v1);
        smt.insert(k2, v2);

        if (!smt.verify(root_hash, proof)) {
            console.error("verify smt2 failed");
            return 5;
        }
    }
    console.log("Checked");
    return 0;
}

bindings.exit(main());