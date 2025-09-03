# Sparse Merkle Tree WASM

This project compiles the Rust implementation of a Sparse Merkle Tree into WebAssembly (WASM), making it accessible for JavaScript/TypeScript applications.

## Build

Currently, both Node.js and web environments are supported. Developers can choose the appropriate build target:

```sh
make build-nodejs
```

or

```sh
make build-web
```

## Run Web Example

```sh
make run-example
```

The demo page does not include a UI. Instead, it automatically runs SMT-related scripts after loading.

## Usage

* **Contract test example**: See line 48 in `tests/tests/smt-ckb.test.ts`
* **Web example**: See line 12 in `examples/index.html`
