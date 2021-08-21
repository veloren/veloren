#!/bin/bash
rm -r target/debug/incremental/* || echo "all good" # TMP FIX FOR 2021-03-22-nightly
time cargo clippy --all-targets --locked --features="bin_cmd_doc_gen,bin_compression,bin_csv,bin_graphviz,bin_bot,asset_tweak" -- -D warnings &&
# Ensure that the veloren-voxygen default-publish feature builds as it excludes some default features
time cargo clippy -p veloren-voxygen --locked --no-default-features --features="default-publish" -- -D warnings &&
time cargo fmt --all -- --check