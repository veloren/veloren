all:
	(cd ..; cargo component build --release)
	cp ../target/wasm32-wasi/release/anim.wasm anim.wasm 
	tar cvf ../../../../assets/plugins/lizard.plugin.tar anim.wasm plugin.toml common voxygen
