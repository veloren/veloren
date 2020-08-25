mod color;
mod dir;

pub const GIT_VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/githash"));

lazy_static::lazy_static! {
    pub static ref GIT_HASH: &'static str = GIT_VERSION.split('/').next().expect("failed to retrieve git_hash!");
    pub static ref GIT_DATE: &'static str = GIT_VERSION.split('/').nth(1).expect("failed to retrieve git_date!");
}

pub use color::*;
pub use dir::*;

// https://discordapp.com/channels/676678179678715904/676685797524766720/723358438943621151
#[macro_export]
macro_rules! span {
    ($guard_name:tt, $level:ident, $name:expr, $($fields:tt)*) => {
        let span = tracing::span!(tracing::Level::$level, $name, $($fields)*);
        let $guard_name = span.enter();
    };
    ($guard_name:tt, $level:ident, $name:expr) => {
        let span = tracing::span!(tracing::Level::$level, $name);
        let $guard_name = span.enter();
    };
    ($guard_name:tt, $name:expr) => {
        let span = tracing::span!(tracing::Level::INFO, $name);
        let $guard_name = span.enter();
    };
}
