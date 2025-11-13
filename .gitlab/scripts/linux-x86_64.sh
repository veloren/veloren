#!/bin/bash
export VELOREN_USERDATA_STRATEGY=executable;
time cargo build --release --no-default-features --features default-publish;
# evaluate --bin instead, last time i checked (2021-07-14) it was 2 minutes slower on release (but faster on debug)

# compress debuginfos via zlib, which reduces the size by 50% while still beeing supported by most tools
objcopy --compress-debug-sections=zlib target/release/veloren-server-cli target/release/veloren-server-cli-compressed
objcopy --compress-debug-sections=zlib target/release/veloren-voxygen target/release/veloren-voxygen-compressed
mv target/release/veloren-server-cli-compressed target/release/veloren-server-cli
mv target/release/veloren-voxygen-compressed target/release/veloren-voxygen
