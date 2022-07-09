#!/bin/bash
# exports default env variables in CI
export DISABLE_GIT_LFS_CHECK=true
export VELOREN_ASSETS="assets"

# When updating RUSTFLAGS here, windows-x86_64.sh must also be updated as it sets them independently
export RUSTFLAGS="-D warnings"

export SHADERC_LIB_DIR=/shaderc/combined/
