#!/bin/bash
update-alternatives --set x86_64-w64-mingw32-gcc /usr/bin/x86_64-w64-mingw32-gcc-posix
update-alternatives --set x86_64-w64-mingw32-g++ /usr/bin/x86_64-w64-mingw32-g++-posix
rm -r target/release/incremental/veloren_* || echo "all good" # TMP FIX FOR 2021-03-22-nightly
export VELOREN_USERDATA_STRATEGY=executable
time cargo build --target=x86_64-pc-windows-gnu --release --no-default-features --features default-publish