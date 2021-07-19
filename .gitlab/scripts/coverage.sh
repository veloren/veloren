#!/bin/bash
echo "modifying files in 5s, ctrl+c to abort" && sleep 5
rm -r target/debug/incremental/* || echo "all good" # TMP FIX FOR 2021-03-22-nightly
find ./* -name "Cargo.toml" -exec sed -i 's/, "simd"]/]/g' {} \;
find ./* -name "Cargo.toml" -exec sed -i 's/"simd"]/]/g' {} \;
sed -i 's/vek /#vek /g' ./Cargo.toml;
export VELOREN_ASSETS="$(pwd)/assets";
time cargo tarpaulin --skip-clean -v -- --test-threads=2;