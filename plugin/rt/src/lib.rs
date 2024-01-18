pub extern crate plugin_derive;

pub mod retrieve;

use api::RetrieveError;
pub use retrieve::*;

use std::convert::TryInto;

pub use plugin_api as api;
pub use plugin_derive::*;

use serde::{de::DeserializeOwned, Serialize};

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn raw_emit_actions(ptr: i64, len: i64);
    fn raw_retrieve_action(ptr: i64, len: i64) -> i64;
    pub fn dbg(i: i32);
}

pub fn retrieve_action<T: DeserializeOwned>(_actions: &api::Retrieve) -> Result<T, RetrieveError> {
    #[cfg(target_arch = "wasm32")]
    {
        let ret = bincode::serialize(&_actions).expect("Can't serialize action in emit");
        unsafe {
            let ptr = raw_retrieve_action(to_i64(ret.as_ptr() as _), to_i64(ret.len() as _));
            let ptr = from_i64(ptr);
            let len =
                u64::from_le_bytes(std::slice::from_raw_parts(ptr as _, 8).try_into().unwrap());
            let a = ::std::slice::from_raw_parts((ptr + 8) as _, len as _);
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
            raw_emit_actions(to_i64(ret.as_ptr() as _), to_i64(ret.len() as _));
        }
    }
}

pub fn read_input<T>(ptr: i64, len: i64) -> Result<T, &'static str>
where
    T: DeserializeOwned,
{
    let slice = unsafe { std::slice::from_raw_parts(from_i64(ptr) as _, from_i64(len) as _) };
    bincode::deserialize(slice).map_err(|_| "Failed to deserialize function input")
}

/// This function split a u128 in two u64 encoding them as le bytes
pub fn from_u128(i: u128) -> (u64, u64) {
    let i = i.to_le_bytes();
    (
        u64::from_le_bytes(i[0..8].try_into().unwrap()),
        u64::from_le_bytes(i[8..16].try_into().unwrap()),
    )
}

/// This function merge two u64 encoded as le in one u128
pub fn to_u128(a: u64, b: u64) -> u128 {
    let a = a.to_le_bytes();
    let b = b.to_le_bytes();
    u128::from_le_bytes([a, b].concat().try_into().unwrap())
}

/// This function encode a u64 into a i64 using le bytes
pub fn to_i64(i: u64) -> i64 { i64::from_le_bytes(i.to_le_bytes()) }

/// This function decode a i64 into a u64 using le bytes
pub fn from_i64(i: i64) -> u64 { u64::from_le_bytes(i.to_le_bytes()) }

static mut VEC: Vec<u8> = vec![];
static mut DATA: Vec<u8> = vec![];

pub fn write_output(value: impl Serialize) -> i64 {
    unsafe {
        VEC = bincode::serialize(&value).expect("Can't serialize event output");
        DATA = [
            (VEC.as_ptr() as u64).to_le_bytes(),
            (VEC.len() as u64).to_le_bytes(),
        ]
        .concat();
        to_i64(DATA.as_ptr() as u64)
    }
}

static mut BUFFERS: Vec<u8> = Vec::new();

/// Allocate buffer from wasm linear memory
/// # Safety
/// This function should never be used only intended to by used by the host
#[no_mangle]
pub unsafe fn wasm_prepare_buffer(size: i64) -> i64 {
    BUFFERS = vec![0u8; size as usize];
    BUFFERS.as_ptr() as i64
}
