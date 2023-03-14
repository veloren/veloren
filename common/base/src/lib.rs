pub mod userdata_dir;

pub use userdata_dir::userdata_dir;

// Panic in debug or tests, warn in release
#[macro_export]
macro_rules! dev_panic {
    ($msg:expr) => {
        if cfg!(any(debug_assertions, test)) {
            panic!("{}", $msg);
        } else {
            tracing::error!("{}", $msg);
        }
    };

    ($msg:expr, or return $result:expr) => {
        if cfg!(any(debug_assertions, test)) {
            panic!("{}", $msg);
        } else {
            tracing::warn!("{}", $msg);
            return $result;
        }
    };
}

#[cfg(feature = "tracy")] pub use tracy_client;

/// Allows downstream crates to conditionally do things based on whether tracy
/// is enabled without having to expose a cargo feature themselves.
pub const TRACY_ENABLED: bool = cfg!(feature = "tracy");

#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! plot {
    ($name:expr, $value:expr) => {
        // type check
        let _: f64 = $value;
    };
}

#[cfg(feature = "tracy")]
pub use tracy_client::plot;

// https://discordapp.com/channels/676678179678715904/676685797524766720/723358438943621151
#[cfg(not(feature = "tracy"))]
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
        let span = tracing::span!(tracing::Level::TRACE, $name);
        let $guard_name = span.enter();
    };
    ($guard_name:tt, $no_tracy_name:expr, $tracy_name:expr) => {
        $crate::span!($guard_name, $no_tracy_name);
    };
}

#[cfg(feature = "tracy")]
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
        // Directly use `tracy_client` to decrease overhead for better timing
        $crate::prof_span_alloc!($guard_name, $name);
    };
    ($guard_name:tt, $no_tracy_name:expr, $tracy_name:expr) => {
        $crate::span!($guard_name, $tracy_name);
    };
}

#[cfg(not(feature = "tracy"))]
pub struct ProfSpan;

/// Just implemented so that we dont need to have
/// #[allow(clippy::drop_non_drop)] everywhere
#[cfg(not(feature = "tracy"))]
impl Drop for ProfSpan {
    fn drop(&mut self) {}
}

#[cfg(feature = "tracy")]
pub struct ProfSpan(pub tracy_client::Span);

/// Like the span macro but only used when profiling and not in regular tracing
/// operations
#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! prof_span {
    ($guard_name:tt, $name:expr) => {
        let $guard_name = $crate::ProfSpan;
    };
    // Shorthand for when you want the guard to just be dropped at the end of the scope instead
    // of controlling it manually
    ($name:expr) => {
        $crate::prof_span!(_guard, $name);
    };
}

/// Like the span macro but only used when profiling and not in regular tracing
/// operations
#[cfg(feature = "tracy")]
#[macro_export]
macro_rules! prof_span {
    ($guard_name:tt, $name:expr) => {
        let $guard_name = $crate::ProfSpan(
            // No callstack since this has significant overhead
            $crate::tracy_client::span!($name, 0),
        );
    };
    // Shorthand for when you want the guard to just be dropped at the end of the scope instead
    // of controlling it manually
    ($name:expr) => {
        $crate::prof_span!(_guard, $name);
    };
}

/// Like the prof_span macro but this one allocates so it can use strings only
/// known at runtime.
#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! prof_span_alloc {
    ($guard_name:tt, $name:expr) => {
        let $guard_name = $crate::ProfSpan;
    };
    // Shorthand for when you want the guard to just be dropped at the end of the scope instead
    // of controlling it manually
    ($name:expr) => {
        $crate::prof_span!(_guard, $name);
    };
}

/// Like the prof_span macro but this one allocates so it can use strings only
/// known at runtime.
#[cfg(feature = "tracy")]
#[macro_export]
macro_rules! prof_span_alloc {
    ($guard_name:tt, $name:expr) => {
        let $guard_name = $crate::ProfSpan({
            struct S;
            let type_name = core::any::type_name::<S>();
            let function_name = &type_name[..type_name.len() - 3];
            $crate::tracy_client::Client::running()
                .expect("prof_span_alloc! without a running tracy_client::Client")
                // No callstack since this has significant overhead
                .span_alloc($name, function_name, file!(), line!(), 0)
        });
    };
    // Shorthand for when you want the guard to just be dropped at the end of the scope instead
    // of controlling it manually
    ($name:expr) => {
        $crate::prof_span!(_guard, $name);
    };
}
