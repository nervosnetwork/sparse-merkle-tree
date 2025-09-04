import { Resource, DEFAULT_SCRIPT_ALWAYS_SUCCESS, DEFAULT_SCRIPT_CKB_JS_VM } from "ckb-testtool";
import { hashCkb, hexFrom, Hex, Transaction, Script, HexLike } from "@ckb-ccc/core";
import { readFileSync } from "fs";

export const SCRIPT_ALWAYS_SUCCESS = readFileSync(DEFAULT_SCRIPT_ALWAYS_SUCCESS)

export function createJSScript(resource: Resource, tx: Transaction, jsCode: Hex, args: Hex): Script {
    const lockScript = resource.deployCell(
        hexFrom(readFileSync(DEFAULT_SCRIPT_CKB_JS_VM)),
        tx,
        false,
    );

    const cell = resource.mockCell(
        resource.createScriptUnused(), undefined,
        jsCode,
    );
    tx.cellDeps.push(resource.createCellDep(cell, "code"));

    let code_hash = hashCkb(jsCode);
    lockScript.args = hexFrom('0x0000' + code_hash.slice(2) + '04' + args.slice(2));

    return lockScript;
}

export function createScript(resource: Resource, tx: Transaction, scriptBin: Hex, args: Hex): Script {
    const lockScript = resource.deployCell(
        scriptBin,
        tx,
        false,
    );
    lockScript.args = args;
    return lockScript;
}

export function toU8a32(x: HexLike): Uint8Array {
    if (typeof x === "string") {
        let s = x.startsWith("0x") || x.startsWith("0X") ? x.slice(2) : x;
        if (s.length !== 64) {
            throw new Error(`hex length must be 64 chars (got ${s.length})`);
        }
        if (!/^[0-9a-fA-F]{64}$/.test(s)) {
            throw new Error("invalid hex string");
        }
        const arr = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            arr[i] = parseInt(s.substr(i * 2, 2), 16);
        }
        return arr;
    }

    const a = x instanceof Uint8Array ? x : new Uint8Array(x);
    if (a.length !== 32) {
        throw new Error(`Uint8Array must be length 32 (got ${a.length})`);
    }
    return a;
}

export const ZERO_HASH = new Uint8Array(32);
