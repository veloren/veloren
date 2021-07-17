#!/bin/bash
rm -r target/release/incremental/veloren_* || echo "all good" # TMP FIX FOR 2021-03-22-nightly
export VELOREN_USERDATA_STRATEGY=executable
time cargo build --release --no-default-features --features default-publish
# eveluate --bin instead, last time i checked (2021-07-14) it was 2 minutes slower on release (but faster on debug)