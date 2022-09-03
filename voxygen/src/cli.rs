//! NOTE: Some of these arguments are used by airshipper, so those needs to be
//! kept fairly stable (probably with some sort of migration period if we need
//! to modify the name or semantics).
//!
//! The arguments used by airshipper are:
//! * `server`
//!
//! Airshipper should only use arguments listed above! Since we will not try to
//! be careful about their stability otherwise.
use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Value to auto-fill into the server field.
    ///
    /// This allows passing in server selection performed in airshipper.
    #[clap(short, long)]
    pub server: Option<String>,
}
