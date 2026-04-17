#!/bin/bash
export NOVA_FORGE_USERDATA_STRATEGY=executable;
time cargo build --release --no-default-features --features default-publish;
# evaluate --bin instead, last time i checked (2021-07-14) it was 2 minutes slower on release (but faster on debug)

# compress debuginfos via zlib, which reduces the size by 50% while still beeing supported by most tools
objcopy --compress-debug-sections=zlib target/release/nova-forge-server-cli target/release/nova-forge-server-cli-compressed
objcopy --compress-debug-sections=zlib target/release/nova-forge-voxygen target/release/nova-forge-voxygen-compressed
mv target/release/nova-forge-server-cli-compressed target/release/nova-forge-server-cli
mv target/release/nova-forge-voxygen-compressed target/release/nova-forge-voxygen
