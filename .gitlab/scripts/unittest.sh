#!/bin/bash
export VELOREN_ASSETS="$(pwd)/assets"
rm -r target/debug/incremental/veloren_* || echo "all good" # TMP FIX FOR 2021-03-22-nightly
time cargo test --package veloren-common-assets asset_tweak::tests --features asset_tweak --lib &&
( rm -r target/debug/incremental* || echo "all good" ) && # TMP FIX FOR 2021-03-22-nightly
time cargo test