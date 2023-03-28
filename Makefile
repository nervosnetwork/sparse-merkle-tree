default: fmt clippy test bench-test check test-c-impl test-cxx-build

test:
	cargo test --all --features std,smtc

bench-test:
	cargo bench -- --test

clippy:
	cargo clippy  --all --features std,smtc --all-targets

fmt:
	cargo fmt --all -- --check

check:
	cargo check --no-default-features

test-c-impl:
	git submodule update --init --recursive
	cd c/rust-tests && cargo test

test-cxx-build:
	g++ -c src/ckb_smt.c -I c -o smt.o && rm -rf smt.o
