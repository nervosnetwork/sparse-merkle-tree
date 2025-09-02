default: fmt clippy clippy-trie test test-trie bench-test bench-test-trie check test-c-impl test-cxx-build test-blake2b-ref

test:
	cargo test --all --features std,smtc

test-trie:
	cargo test --all --all-features

bench-test:
	cargo bench -- --test

bench-test-trie:
	cargo bench --features trie -- --test

clippy:
	cargo clippy  --all --features std,smtc --all-targets

clippy-trie:
	cargo clippy  --all --all-features --all-targets

fmt:
	cargo fmt --all -- --check

check:
	cargo check --no-default-features

test-c-impl:
	git submodule update --init --recursive
	cd c/rust-tests && cargo test

test-cxx-build:
	g++ -c src/ckb_smt.c -I c -o smt.o && rm -rf smt.o

test-blake2b-ref:
	cargo test --no-default-features --features="std"
