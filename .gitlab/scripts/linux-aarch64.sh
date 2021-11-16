#!/bin/bash
export VELOREN_USERDATA_STRATEGY=executable
time PKG_CONFIG=/usr/bin/aarch64-linux-gnu-pkg-config cargo build --release --no-default-features --features default-publish --release --target aarch64-unknown-linux-gnu
# evaluate --bin instead, last time i checked (2021-07-14) it was 2 minutes slower on release (but faster on debug)
