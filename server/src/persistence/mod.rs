pub mod character;
pub mod stats;

mod error;
mod models;
mod schema;

extern crate diesel;

use diesel::prelude::*;
use diesel_migrations::embed_migrations;
use std::fs;

// See: https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/macro.embed_migrations.html
// This macro is called at build-time, and produces the necessary migration info
// for the `embedded_migrations` call below.
embed_migrations!();

pub fn run_migrations(db_dir: &str) -> Result<(), diesel_migrations::RunMigrationsError> {
    let _ = fs::create_dir(format!("{}/", db_dir));
    embedded_migrations::run_with_output(&establish_connection(db_dir), &mut std::io::stdout())
}

fn establish_connection(db_dir: &str) -> SqliteConnection {
    let database_url = format!("{}/db.sqlite", db_dir);
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
