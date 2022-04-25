#!/bin/bash
echo "modifying files in 5s, ctrl+c to abort" && sleep 5
find ./* -name "Cargo.toml" -exec sed -i 's/, "simd"]/]/g' {} \;
find ./* -name "Cargo.toml" -exec sed -i 's/"simd"]/]/g' {} \;
export VELOREN_ASSETS="$(pwd)/assets";
time cargo tarpaulin --skip-clean -v -- --test-threads=2;
