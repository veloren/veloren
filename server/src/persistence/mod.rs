//! DB operations and schema migrations
//!
//! This code uses several [`Diesel ORM`](http://diesel.rs/) tools for DB operations:
//! - [`diesel-migrations`](https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/)
//!   for managing table migrations
//! - [`diesel-cli`](https://github.com/diesel-rs/diesel/tree/master/diesel_cli/)
//!   for generating and testing migrations

pub(in crate::persistence) mod character;
pub mod character_loader;
pub mod character_updater;
mod error;
mod json_models;
mod models;
mod schema;

use common::comp;
use diesel::{connection::SimpleConnection, prelude::*};
use diesel_migrations::embed_migrations;
use std::{fs, path::Path};
use tracing::info;

/// A tuple of the components that are persisted to the DB for each character
pub type PersistedComponents = (comp::Body, comp::Stats, comp::Inventory, comp::Loadout);

// See: https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/macro.embed_migrations.html
// This macro is called at build-time, and produces the necessary migration info
// for the `embedded_migrations` call below.
//
// NOTE: Adding a useless comment to trigger the migrations being run. Alter
// when needed.
embed_migrations!();

struct TracingOut;

impl std::io::Write for TracingOut {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        info!("{}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

/// Runs any pending database migrations. This is executed during server startup
pub fn run_migrations(db_dir: &Path) -> Result<(), diesel_migrations::RunMigrationsError> {
    let _ = fs::create_dir(format!("{}/", db_dir.display()));

    embedded_migrations::run_with_output(
        &establish_connection(db_dir)
            .expect(
                "If we cannot execute migrations, we should not be allowed to launch the server, \
                 so we don't populate it with bad data.",
            )
            .0,
        &mut std::io::LineWriter::new(TracingOut),
    )
}

/// A database connection blessed by Veloren.
pub struct VelorenConnection(SqliteConnection);

/// A transaction blessed by Veloren.
#[derive(Clone, Copy)]
pub struct VelorenTransaction<'a>(&'a SqliteConnection);

impl VelorenConnection {
    /// Open a transaction in order to be able to run a set of queries against
    /// the database. We require the use of a transaction, rather than
    /// allowing direct session access, so that (1) we can control things
    /// like the retry process (at a future date), and (2) to avoid
    /// accidentally forgetting to open or reuse a transaction.
    ///
    /// We could make things even more foolproof, but we restrict ourselves to
    /// this for now.
    pub fn transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: for<'a> FnOnce(VelorenTransaction<'a>) -> Result<T, E>,
        E: From<diesel::result::Error>,
    {
        self.0.transaction(|| f(VelorenTransaction(&self.0)))
    }
}

impl<'a> core::ops::Deref for VelorenTransaction<'a> {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target { &self.0 }
}

pub fn establish_connection(db_dir: &Path) -> QueryResult<VelorenConnection> {
    let database_url = format!("{}/db.sqlite", db_dir.display());

    let connection = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    // Use Write-Ahead-Logging for improved concurrency: https://sqlite.org/wal.html
    // Set a busy timeout (in ms): https://sqlite.org/c3ref/busy_timeout.html
    connection
        .batch_execute(
            "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA busy_timeout = 250;
        ",
        )
        .expect(
            "Failed adding PRAGMA statements while establishing sqlite connection, including \
             enabling foreign key constraints.  We will not allow connecting to the server under \
             these conditions.",
        );

    Ok(VelorenConnection(connection))
}
