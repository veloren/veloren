all:
	(cd ..; cargo component build --release)
	cp ../target/wasm32-wasi/release/hello.wasm hello.wasm 
	tar cvf ../../../../assets/plugins/hello.plugin.tar hello.wasm plugin.toml
