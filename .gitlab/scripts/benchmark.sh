#!/bin/bash
rm -r target/release/incremental/veloren_* || echo "all good" # TMP FIX FOR 2021-03-22-nightly
rm -r target/release/incremental/* || echo "all good" # TMP FIX FOR 2021-03-22-nightly
time cargo bench