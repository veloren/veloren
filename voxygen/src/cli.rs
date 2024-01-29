//! NOTE: Some of these arguments are used by airshipper, so those needs to be
//! kept fairly stable (probably with some sort of migration period if we need
//! to modify the name or semantics).
//!
//! The arguments used by airshipper are:
//! * `server`
//!
//! Airshipper should only use arguments listed above! Since we will not try to
//! be careful about their stability otherwise.
//!
//! Likewise Airshipper should only use the following subcommands:
//! * `ListWgpuBackends`
use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Args {
    /// Value to auto-fill into the server field.
    ///
    /// This allows passing in server selection performed in airshipper.
    #[clap(short, long)]
    pub server: Option<String>,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available wgpu backends. This is called by Airshipper to show a
    /// dropbox of available backends
    ListWgpuBackends,
}
