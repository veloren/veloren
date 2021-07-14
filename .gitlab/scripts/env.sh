#!/bin/bash
# exports default env variables in CI
export DISABLE_GIT_LFS_CHECK=true
export VELOREN_ASSETS="assets"
export RUSTFLAGS="-D warnings"
export SHADERC_LIB_DIR=/shaderc/combined/