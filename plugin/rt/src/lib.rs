#![feature(const_fn)]

pub extern crate plugin_derive;

pub mod retrieve;

use api::RetrieveError;
pub use retrieve::*;

use std::convert::TryInto;

pub use retrieve::*;

pub use plugin_api as api;
pub use plugin_derive::*;

use serde::{de::DeserializeOwned, Serialize};

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn raw_emit_actions(ptr: *const u8, len: usize);
    fn raw_retrieve_action(ptr: *const u8, len: usize) -> i64;
    pub fn dbg(i: i32);
}

pub fn retrieve_action<T: DeserializeOwned>(_actions: &api::Retrieve) -> Result<T, RetrieveError> {
    #[cfg(target_arch = "wasm32")]
    {
        let ret = bincode::serialize(&_actions).expect("Can't serialize action in emit");
        unsafe {
            let (ptr, len) = from_i64(raw_retrieve_action(ret.as_ptr(), ret.len()));
            let a = ::std::slice::from_raw_parts(ptr as _, len as _);
            bincode::deserialize::<Result<T, RetrieveError>>(&a)
                .map_err(|x| RetrieveError::BincodeError(x.to_string()))?
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    unreachable!()
}

pub fn emit_action(action: api::Action) { emit_actions(vec![action]) }

pub fn emit_actions(_actions: Vec<api::Action>) {
    #[cfg(target_arch = "wasm32")]
    {
        let ret = bincode::serialize(&_actions).expect("Can't serialize action in emit");
        unsafe {
            raw_emit_actions(ret.as_ptr(), ret.len());
        }
    }
}

pub fn read_input<T>(ptr: i32, len: u32) -> Result<T, &'static str>
where
    T: DeserializeOwned,
{
    let slice = unsafe { ::std::slice::from_raw_parts(ptr as _, len as _) };
    bincode::deserialize(slice).map_err(|_| "Failed to deserialize function input")
}

pub fn from_i64(i: i64) -> (i32, i32) {
    let i = i.to_le_bytes();
    (
        i32::from_le_bytes(i[0..4].try_into().unwrap()),
        i32::from_le_bytes(i[4..8].try_into().unwrap()),
    )
}

pub fn to_i64(a: i32, b: i32) -> i64 {
    let a = a.to_le_bytes();
    let b = b.to_le_bytes();
    i64::from_le_bytes([a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]])
}

pub fn write_output(value: impl Serialize) -> i64 {
    let ret = bincode::serialize(&value).expect("Can't serialize event output");
    to_i64(ret.as_ptr() as _, ret.len() as _)
}

static mut BUFFERS: Vec<u8> = Vec::new();

/// Allocate buffer from wasm linear memory
/// # Safety
/// This function should never be used only intented to by used by the host
#[no_mangle]
pub unsafe fn wasm_prepare_buffer(size: i32) -> i32 {
    BUFFERS = vec![0u8; size as usize];
    BUFFERS.as_ptr() as i32
}
