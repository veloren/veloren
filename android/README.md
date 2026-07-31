Quick steps to build and test the android/ veloren_android skeleton

Prereqs:
- Android NDK installed (r21+ recommended). Set ANDROID_NDK_HOME to the NDK path.
- Rust toolchain installed; add target: rustup target add aarch64-linux-android
- cargo-ndk installed: cargo install cargo-ndk
- (Optional) Android Studio if you plan to build the APK; otherwise you can copy the built .so into an existing Android project jniLibs/

Build:
1. From repo root:
   ./android/scripts/build_android.sh

2. Copy app/src/main/jniLibs/arm64-v8a/libveloren_android.so into an Android project's app/src/main/jniLibs/arm64-v8a/.
3. Add VelorenLib.java into the app package and call VelorenLib.init() from MainActivity.onCreate().

Notes:
- This creates a minimal shared library that initializes android_logger and logs a message.
- Many Veloren crates are desktop-only — expecting compile/link errors the first time is normal. We'll iterate on fixing those.
