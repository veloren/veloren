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
        #[cfg(not(feature = "tracy"))]
        let span = tracing::span!(tracing::Level::TRACE, $name);
        #[cfg(not(feature = "tracy"))]
        let $guard_name = span.enter();
        // Directly use `tracy_client` to decrease overhead for better timing
        #[cfg(feature = "tracy")]
        let $guard_name = tracy_client::Span::new(
            $name,
            "",
            module_path!(),
            line!(),
            // No callstack since this has significant overhead
            0,
        );
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
