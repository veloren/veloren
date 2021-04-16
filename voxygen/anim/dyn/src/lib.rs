//! This crate hacks around the inability to dynamically specify the
//! `crate-type` for cargo to build.
//!
//! For more details on the issue this is a decent starting point: https://github.com/rust-lang/cargo/pull/8789
//!
//! This crate avoids use building the dynamic lib when it isn't needed and the
//! same with the non dynamic build. Additionally, this allows compilation to
//! start earlier since a cdylib doesn't pipeline with it's dependencies.
//!
//! NOTE: the `be-dyn-lib` feature must be used for this crate to be useful, it
//! is not on by default becaue this causes cargo to switch the feature on in
//! the anim crate when compiling the static lib into voxygen.
#[cfg(feature = "be-dyn-lib")]
pub use veloren_voxygen_anim::*;
