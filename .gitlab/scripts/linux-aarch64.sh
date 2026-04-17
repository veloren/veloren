#!/bin/bash
export NOVA_FORGE_USERDATA_STRATEGY=executable;
export PKG_CONFIG="/usr/bin/aarch64-linux-gnu-pkg-config";
time cargo build --target=aarch64-unknown-linux-gnu --release --no-default-features --features default-publish;
# evaluate --bin instead, last time i checked (2021-07-14) it was 2 minutes slower on release (but faster on debug)

# compress debuginfos via zlib, which reduces the size by 50% while still beeing supported by most tools
aarch64-linux-gnu-objcopy --compress-debug-sections=zlib target/aarch64-unknown-linux-gnu/release/nova-forge-server-cli target/aarch64-unknown-linux-gnu/release/nova-forge-server-cli-compressed
aarch64-linux-gnu-objcopy --compress-debug-sections=zlib target/aarch64-unknown-linux-gnu/release/nova-forge-voxygen target/aarch64-unknown-linux-gnu/release/nova-forge-voxygen-compressed
mv target/aarch64-unknown-linux-gnu/release/nova-forge-server-cli-compressed target/aarch64-unknown-linux-gnu/release/nova-forge-server-cli
mv target/aarch64-unknown-linux-gnu/release/nova-forge-voxygen-compressed target/aarch64-unknown-linux-gnu/release/nova-forge-voxygen
