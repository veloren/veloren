#![feature(fundamental)]

pub mod userdata_dir;

pub use userdata_dir::userdata_dir;

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
        let $guard_name = $crate::tracy_client::Span::new(
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
        if let Some(id) = span.id() {
            subscriber.enter(&id)
        }
        Self { span, subscriber }
    }
}

impl Drop for GuardlessSpan {
    fn drop(&mut self) {
        if let Some(id) = self.span.id() {
            self.subscriber.exit(&id)
        }
    }
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
