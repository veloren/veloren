package com.example.veloren;

public class VelorenLib {
    static { System.loadLibrary("veloren_android"); }

    // Matches the exported JNI function name in Rust:
    // Java_com_example_veloren_VelorenLib_init
    public static native void init();
}
