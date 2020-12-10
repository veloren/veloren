mod color;
pub mod dir;
pub mod find_dist;
mod option;
pub mod userdata_dir;

pub const GIT_VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/githash"));
pub const GIT_TAG: &str = include_str!(concat!(env!("OUT_DIR"), "/gittag"));
pub const VELOREN_VERSION_STAGE: &str = "Pre-Alpha";

lazy_static::lazy_static! {
    pub static ref GIT_HASH: &'static str = GIT_VERSION.split('/').next().expect("failed to retrieve git_hash!");
    static ref GIT_DATETIME: &'static str = GIT_VERSION.split('/').nth(1).expect("failed to retrieve git_datetime!");
    pub static ref GIT_DATE: String = GIT_DATETIME.split('-').take(3).collect::<Vec<&str>>().join("-");
    pub static ref GIT_TIME: &'static str = GIT_DATETIME.split('-').nth(3).expect("failed to retrieve git_time!");
    pub static ref DISPLAY_VERSION: String = if GIT_TAG.is_empty() {
        format!("{}-{}", VELOREN_VERSION_STAGE, GIT_DATE.to_string())
    } else {
        format!("{}-{}", VELOREN_VERSION_STAGE, GIT_TAG.to_string())
    };
    pub static ref DISPLAY_VERSION_LONG: String = if GIT_TAG.is_empty() {
        format!("{} ({})", DISPLAY_VERSION.as_str(), GIT_HASH.to_string())
    } else {
        format!("{} ({})", DISPLAY_VERSION.as_str(), GIT_VERSION.to_string())
    };
}

pub use color::*;
pub use dir::*;
pub use option::*;

#[cfg(feature = "tracy")] pub use tracy_client;

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
        #[cfg(not(feature = "tracy"))]
        let span = tracing::span!(tracing::Level::TRACE, $name);
        #[cfg(not(feature = "tracy"))]
        let $guard_name = span.enter();
        // Directly use `tracy_client` to decrease overhead for better timing
        #[cfg(feature = "tracy")]
        let $guard_name = $crate::util::tracy_client::Span::new(
            $name,
            "",
            module_path!(),
            line!(),
            // No callstack since this has significant overhead
            0,
        );
    };
    ($guard_name:tt, $no_tracy_name:expr, $tracy_name:expr) => {
        #[cfg(not(feature = "tracy"))]
        $crate::span!($guard_name, $no_tracy_name);
        #[cfg(feature = "tracy")]
        $crate::span!($guard_name, $tracy_name);
    };
}

/// There's no guard, but really this is actually the guard
pub struct GuardlessSpan {
    span: tracing::Span,
    subscriber: tracing::Dispatch,
}

impl GuardlessSpan {
    pub fn new(span: tracing::Span) -> Self {
        let subscriber = tracing::dispatcher::get_default(|d| d.clone());
        span.id().map(|id| subscriber.enter(&id));
        Self { span, subscriber }
    }
}

impl Drop for GuardlessSpan {
    fn drop(&mut self) { self.span.id().map(|id| self.subscriber.exit(&id)); }
}

#[macro_export]
macro_rules! no_guard_span {
    ($level:ident, $name:expr, $($fields:tt)*) => {
        GuardlessSpan::new(
            tracing::span!(tracing::Level::$level, $name, $($fields)*)
        )
    };
    ($level:ident, $name:expr) => {
        GuardlessSpan::new(
            tracing::span!(tracing::Level::$level, $name)
        )
    };
    ($name:expr) => {
        GuardlessSpan::new(
            tracing::span!(tracing::Level::TRACE, $name)
        )
    };
}
