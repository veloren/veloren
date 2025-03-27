pub mod userdata_dir;

pub use userdata_dir::userdata_dir;

/// Panic in debug or tests, log error/warn in release
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

#[cfg(feature = "tracy")]
pub use profiling::tracy_client;

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
/// #[expect(clippy::drop_non_drop)] everywhere
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
                .span_alloc(Some($name), function_name, file!(), line!(), 0)
        });
    };
    // Shorthand for when you want the guard to just be dropped at the end of the scope instead
    // of controlling it manually
    ($name:expr) => {
        $crate::prof_span!(_guard, $name);
    };
}

/// strum::EnumIter alternative that supports nested enums
#[macro_export]
macro_rules! enum_iter {
    (
        $( #[ $enum_attr:meta ] )*
        $vis:vis enum $enum_name:ident {
            $(
                $( #[ $variant_attr:meta ] )*
                $variant:ident $(($nested_enum:ty))?
            ),* $(,)?
        }
    ) => {
        $( #[ $enum_attr ] )*
        $vis enum $enum_name {
            $(
                $( #[ $variant_attr ] )*
                $variant $(($nested_enum))?
            ),*
        }

        impl $enum_name {
            $vis fn all_variants() -> Vec<$enum_name> {
                let mut buff = vec![];
                $(
                    #[allow(unused_variables)]
                    let is_nested = false;
                    $(
                        // fake capture on $nested_enum to trigger
                        // macro expansion and switch `is_nested` to `true`
                        let _fake_capture: Option<$nested_enum> = None;
                        let is_nested = true;
                    )?

                    if is_nested {
                        $(
                            buff.extend(
                                <$nested_enum>::iter().map($enum_name::$variant)
                            );
                        )?
                    } else {
                        #[allow(unreachable_code)]
                        buff.push(
                            // if we have variant with nested enum, we need to
                            // return smth like Color::Red(Shade::Light)
                            //
                            // the problem is that we don't know what to return
                            // and frankly we don't need to, because we won't
                            // hit this branch
                            //
                            // for that we use $nested_enum to create fake
                            // capture and return unreachable!() which will
                            // give us `!` type and pass the typecheck
                            $enum_name::$variant $(
                                ({
                                    let _fake_capture: Option<$nested_enum> = None;
                                    unreachable!();
                                })
                            )?
                        );
                    }
                )*

                buff
            }

            $vis fn iter() -> impl Iterator<Item=Self> {
                Self::all_variants().into_iter()
            }
        }
    }
}

#[test]
fn test_enum_iter() {
    enum_iter! {
        #[derive(Eq, PartialEq, Debug)]
        enum Shade {
            Good,
            Meh,
            Bad,
        }
    }

    enum_iter! {
        #[derive(Debug, Eq, PartialEq)]
        enum Color {
            Green,
            Red(Shade),
            Blue,
        }
    }

    let results: Vec<_> = Shade::iter().collect();
    assert_eq!(results, vec![Shade::Good, Shade::Meh, Shade::Bad,]);

    let results: Vec<_> = Color::iter().collect();
    assert_eq!(results, vec![
        Color::Green,
        Color::Red(Shade::Good),
        Color::Red(Shade::Meh),
        Color::Red(Shade::Bad),
        Color::Blue,
    ]);
}

#[macro_export]
macro_rules! struct_iter {
    (
        $( #[ $type_attr:meta ] )*
        $vis:vis struct $struct_name:ident {
            $(
                $( #[ $field_attr:meta ] )*
                $field_vis:vis $field:ident: $field_type:ty
            ),* $(,)?
        }
    ) => {
        $( #[ $type_attr ] )*
        $vis struct $struct_name {
            $(
                $( #[ $field_attr ] )*
                $field_vis $field: $field_type
            ),*
        }

        impl $struct_name {
            fn all_variants() -> Vec<$struct_name> {
                #[derive(Default, Clone)]
                pub struct Builder {
                    $(
                        $( #[ $field_attr ] )*
                        $field: Option<$field_type>
                    ),*
                }

                impl Builder {
                    $(
                        pub fn $field(mut self, val: $field_type) -> Self {
                            self.$field = Some(val);
                            self
                        }
                    )*

                    pub fn build_expect(self) -> $struct_name {
                        $struct_name {
                            $(
                                $field: self.$field.unwrap()
                            ),*
                        }
                    }
                }

                let mut builder_buff = vec![Builder::default()];
                // launch build spiral
                $(
                    let mut next_buff = vec![];
                    for step in builder_buff {
                        for kind in <$field_type>::iter() {
                            next_buff.push(step.clone().$field(kind));
                        }
                    }
                    builder_buff = next_buff;
                )*

                let mut result_buff = vec![];
                for builder in builder_buff {
                    result_buff.push(builder.build_expect())
                }
                return result_buff;
            }

            $vis fn iter() -> impl Iterator<Item=Self> {
                Self::all_variants().into_iter()
            }
        }
    }
}

#[test]
fn test_struct_iter() {
    enum_iter! {
        #[derive(Eq, PartialEq, Debug, Clone)]
        enum Species {
            BlueDragon,
            RedDragon,
        }
    }

    enum_iter! {
        #[derive(Eq, PartialEq, Debug, Clone)]
        enum BodyType {
            Male,
            Female,
        }
    }

    struct_iter! {
        #[derive(Eq, PartialEq, Debug)]
        struct Body {
            species: Species,
            body_type: BodyType,
        }
    }

    let results: Vec<_> = Body::iter().collect();
    assert_eq!(results, vec![
        Body {
            species: Species::BlueDragon,
            body_type: BodyType::Male
        },
        Body {
            species: Species::BlueDragon,
            body_type: BodyType::Female
        },
        Body {
            species: Species::RedDragon,
            body_type: BodyType::Male
        },
        Body {
            species: Species::RedDragon,
            body_type: BodyType::Female
        },
    ])
}
