#![feature(const_fn)]

pub extern crate plugin_derive;

pub use plugin_api as api;
pub use plugin_derive::*;

use serde::{de::DeserializeOwned, Serialize};

extern "C" {
    fn raw_emit_actions(ptr: *const u8, len: usize);
}

pub fn emit_action(action: api::Action) { emit_actions(vec![action]) }

pub fn emit_actions(actions: Vec<api::Action>) {
    let ret = bincode::serialize(&actions).expect("Can't serialize action in emit");
    unsafe {
        raw_emit_actions(ret.as_ptr(), ret.len());
    }
}

pub fn read_input<T>(ptr: i32, len: u32) -> Result<T, &'static str>
where
    T: DeserializeOwned,
{
    let slice = unsafe { ::std::slice::from_raw_parts(ptr as _, len as _) };
    bincode::deserialize(slice).map_err(|_| "Failed to deserialize function input")
}

pub fn write_output(value: impl Serialize) -> i32 {
    let ret = bincode::serialize(&value).expect("Can't serialize event output");
    let len = ret.len() as u32;
    unsafe {
        ::std::ptr::write(1 as _, len);
    }
    ret.as_ptr() as _
}

static mut BUFFERS: Vec<u8> = Vec::new();

/// Allocate buffer from wasm linear memory
/// # Safety
/// This function should never be used only intented to by used by the host
#[no_mangle]
pub unsafe fn wasm_prepare_buffer(size: i32) -> i32 {
    BUFFERS = Vec::<u8>::with_capacity(size as usize);
    BUFFERS.as_ptr() as i32
}
