use crate::raw_api;

pub fn print(s: &str) {
    unsafe {
        raw_api::print(s.as_bytes().as_ptr(), s.len());
    }
}
