#!/usr/bin/env bash
set -euo pipefail
# Simple build script that uses cargo-ndk to produce .so for arm64.
# Ensure ANDROID_NDK_HOME is set and cargo-ndk is installed: cargo install cargo-ndk
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  echo "Set ANDROID_NDK_HOME to your Android NDK path (e.g. /path/to/android-ndk)"
  exit 1
fi

# Output directory inside an Android project where Android Studio expects .so files
OUT_DIR="./android/app/src/main/jniLibs"
mkdir -p "${OUT_DIR}/arm64-v8a"

# Build and place .so in jniLibs/arm64-v8a
cargo ndk -t arm64-v8a --platform 21 --output "${OUT_DIR}" build --release --manifest-path android/Cargo.toml

echo "Built .so files in ${OUT_DIR}/arm64-v8a"
