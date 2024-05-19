out/hades.js: out/hades.wasm src/lib.rs
	wasm-bindgen --out-dir out out/hades2_bindings.wasm --typescript --target bundler
	wasm-opt out/hades2_bindings_bg.wasm -o out/hades2_bindings_bg.wasm -O2


out/hades.wasm: src/lib.rs Cargo.toml
	cargo build -Zunstable-options --out-dir out --release

.PHONY: clean
clean:
	rm -rf out