pub mod character;
mod error;
mod models;
mod schema;

extern crate diesel;

use diesel::prelude::*;
use diesel_migrations::embed_migrations;
use std::{env, fs, path::Path};

// See: https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/macro.embed_migrations.html
// This macro is called at build-time, and produces the necessary migration info
// for the `embedded_migrations` call below.
embed_migrations!();

pub fn run_migrations() -> Result<(), diesel_migrations::RunMigrationsError> {
    let _ = fs::create_dir(format!("{}/saves/", binary_absolute_path()));
    embedded_migrations::run_with_output(&establish_connection(), &mut std::io::stdout())
}

fn establish_connection() -> SqliteConnection {
    let database_url = format!("{}/saves/db.sqlite", binary_absolute_path());
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

// Get the absolute path of the binary so that the database is always stored
// beside it, no matter where the binary is run from
fn binary_absolute_path() -> String {
    let binary_path;
    match env::current_exe() {
        Ok(exe_path) => binary_path = exe_path,
        Err(e) => panic!("Failed to get current exe path: {}", e),
    };

    match Path::new(&binary_path.display().to_string()).parent() {
        Some(path) => return path.display().to_string(),
        None => panic!("Failed to get current exe parent path"),
    };
}
