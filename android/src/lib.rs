// Minimal JNI-exported functions to verify we can build a shared library,
// initialize Android logging, and be called from Java/Kotlin.
//
// This file intentionally keeps logic minimal — the goal is to surface compile/link issues.

use jni::objects::JClass;
use jni::JNIEnv;
use android_logger::Config;
use log::info;

/// Called from Java as: VelorenLib.init()
#[no_mangle]
pub extern "C" fn Java_com_example_veloren_VelorenLib_init(_env: JNIEnv, _class: JClass) {
    // Initialize android_logger once
    android_logger::init_once(Config::default());
    info!("veloren_android: init called");
    // TODO: start a thread for engine loop or initialize subsystems.
}
