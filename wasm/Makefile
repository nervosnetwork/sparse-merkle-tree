default: build-nodejs

build-nodejs:
	wasm-pack build --target nodejs --out-dir pkg-nodejs

build-web:
	wasm-pack build --target web --out-dir pkg-web

build-test: build-nodejs
	cd tests && pnpm install
	cd tests/ckb-contract && pnpm install
	cd tests && pnpm add ../pkg-nodejs
	cd tests/ckb-contract && pnpm build 

test: build-test
	cd tests && pnpm jest

run-example: build-web
	cp -r pkg-web/ examples/
	pnpm dlx http-server ./examples/ -p 8000

clean:
	cargo clean
	rm -rf pkg-nodejs pkg-web
