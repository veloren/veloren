#!/bin/bash
update-alternatives --set x86_64-w64-mingw32-gcc /usr/bin/x86_64-w64-mingw32-gcc-posix
update-alternatives --set x86_64-w64-mingw32-g++ /usr/bin/x86_64-w64-mingw32-g++-posix
export VELOREN_USERDATA_STRATEGY=executable
time cargo build --target=x86_64-pc-windows-gnu --release --no-default-features --features default-publish &&