mod color;
mod dir;

pub const GIT_VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/githash"));

lazy_static::lazy_static! {
    pub static ref GIT_HASH: &'static str = GIT_VERSION.split("/").nth(0).expect("failed to retrieve git_hash!");
    pub static ref GIT_DATE: &'static str = GIT_VERSION.split("/").nth(1).expect("failed to retrieve git_date!");
}

pub use color::*;
pub use dir::*;
