#!/bin/bash
export VELOREN_USERDATA_STRATEGY=executable
export PKG_CONFIG="/usr/bin/aarch64-linux-gnu-pkg-config"
time cargo build --target=aarch64-unknown-linux-gnu --release --no-default-features --features default-publish
# evaluate --bin instead, last time i checked (2021-07-14) it was 2 minutes slower on release (but faster on debug)
