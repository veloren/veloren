#!/bin/bash
update-alternatives --set x86_64-w64-mingw32-gcc /usr/bin/x86_64-w64-mingw32-gcc-posix
update-alternatives --set x86_64-w64-mingw32-g++ /usr/bin/x86_64-w64-mingw32-g++-posix
export VELOREN_USERDATA_STRATEGY=executable

# RUSTFLAGS is set here in addition to env.sh (which is used for all targets not just windows) due to
# https://github.com/rust-lang/cargo/issues/5376 which prevents the windows-specific rustflags set in
# .cargo/config from being applied
export RUSTFLAGS="-D warnings -C link-arg=-lpsapi"

time cargo build --target=x86_64-pc-windows-gnu --release --no-default-features --features default-publish
