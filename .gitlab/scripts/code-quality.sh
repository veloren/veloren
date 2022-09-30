#!/bin/bash
# cargo clippy is a superset of cargo check, so we don't check manually

time cargo clippy \
    --all-targets \
    --locked \
    --features="bin_cmd_doc_gen,bin_compression,bin_csv,bin_graphviz,bin_bot,bin_asset_migrate,asset_tweak,bin,stat" \
    -- -D warnings &&

# Ensure that the veloren-voxygen default-publish feature builds as it excludes some default features
time cargo clippy -p \
    veloren-voxygen --locked \
    --no-default-features \
    --features="default-publish" \
    -- -D warnings &&
time cargo fmt --all -- --check
