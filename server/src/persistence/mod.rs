//! DB operations and schema migrations

pub(in crate::persistence) mod character;
pub mod character_loader;
pub mod character_updater;
mod diesel_to_rusqlite;
pub mod error;
mod json_models;
mod models;

use common::comp;
use refinery::Report;
use rusqlite::{Connection, OpenFlags};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};
use tracing::info;

/// A tuple of the components that are persisted to the DB for each character
pub type PersistedComponents = (
    comp::Body,
    comp::Stats,
    comp::Inventory,
    Option<comp::Waypoint>,
);

// See: https://docs.rs/refinery/0.5.0/refinery/macro.embed_migrations.html
// This macro is called at build-time, and produces the necessary migration info
// for the `run_migrations` call below.
mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./src/migrations");
}

/// A database connection blessed by Veloren.
pub(crate) struct VelorenConnection {
    connection: Connection,
    sql_log_mode: SqlLogMode,
}

impl VelorenConnection {
    fn new(connection: Connection) -> Self {
        Self {
            connection,
            sql_log_mode: SqlLogMode::Disabled,
        }
    }

    /// Updates the SQLite log mode if DatabaseSetting.sql_log_mode has changed
    pub fn update_log_mode(&mut self, database_settings: &Arc<RwLock<DatabaseSettings>>) {
        let settings = database_settings
            .read()
            .expect("DatabaseSettings RwLock was poisoned");
        if self.sql_log_mode == (*settings).sql_log_mode {
            return;
        }

        set_log_mode(&mut self.connection, (*settings).sql_log_mode);
        self.sql_log_mode = (*settings).sql_log_mode;

        info!(
            "SQL log mode for connection changed to {:?}",
            settings.sql_log_mode
        );
    }
}

fn set_log_mode(connection: &mut Connection, sql_log_mode: SqlLogMode) {
    // Rusqlite's trace and profile logging are mutually exclusive and cannot be
    // used together
    match sql_log_mode {
        SqlLogMode::Trace => {
            connection.trace(Some(rusqlite_trace_callback));
        },
        SqlLogMode::Profile => {
            connection.profile(Some(rusqlite_profile_callback));
        },
        SqlLogMode::Disabled => {
            connection.trace(None);
            connection.profile(None);
        },
    };
}

#[derive(Clone)]
pub struct DatabaseSettings {
    pub db_dir: PathBuf,
    pub sql_log_mode: SqlLogMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SqlLogMode {
    /// Logging is disabled
    Disabled,
    /// Records timings for each SQL statement
    Profile,
    /// Prints all executed SQL statements
    Trace,
}

/// Runs any pending database migrations. This is executed during server startup
pub fn run_migrations(settings: &DatabaseSettings) {
    let mut conn = establish_connection(settings);

    diesel_to_rusqlite::migrate_from_diesel(&mut conn)
        .expect("One-time migration from Diesel to Refinery failed");

    // If migrations fail to run, the server cannot start since the database will
    // not be in the required state.
    let report: Report = embedded::migrations::runner()
        .set_abort_divergent(false)
        .run(&mut conn.connection)
        .expect("Database migrations failed, server startup aborted");

    let applied_migrations = report.applied_migrations().len();
    info!("Applied {} database migrations", applied_migrations);
}

// These callbacks use info logging because they are never enabled by default,
// only when explicitly turned on via CLI arguments or interactive CLI commands.
// Setting them to anything other than info would remove the ability to get SQL
// logging from a running server that wasn't started at higher than info.
fn rusqlite_trace_callback(log_message: &str) {
    info!("{}", log_message);
}
fn rusqlite_profile_callback(log_message: &str, dur: Duration) {
    info!("{} Duration: {:?}", log_message, dur);
}

pub(crate) fn establish_connection(settings: &DatabaseSettings) -> VelorenConnection {
    fs::create_dir_all(&settings.db_dir).expect(&*format!(
        "Failed to create saves directory: {:?}",
        &settings.db_dir
    ));
    let connection = Connection::open_with_flags(
        &settings.db_dir.join("db.sqlite"),
        OpenFlags::SQLITE_OPEN_PRIVATE_CACHE | OpenFlags::default(),
    )
    .unwrap_or_else(|err| {
        panic!(
            "Error connecting to {}, Error: {:?}",
            settings.db_dir.join("db.sqlite").display(),
            err
        )
    });

    let mut veloren_connection = VelorenConnection::new(connection);

    let connection = &mut veloren_connection.connection;

    set_log_mode(connection, settings.sql_log_mode);
    veloren_connection.sql_log_mode = settings.sql_log_mode;

    rusqlite::vtab::array::load_module(&connection).expect("Failed to load sqlite array module");

    connection.set_prepared_statement_cache_capacity(100);

    // Use Write-Ahead-Logging for improved concurrency: https://sqlite.org/wal.html
    // Set a busy timeout (in ms): https://sqlite.org/c3ref/busy_timeout.html
    connection
        .pragma_update(None, "foreign_keys", &"ON")
        .expect("Failed to set foreign_keys PRAGMA");
    connection
        .pragma_update(None, "journal_mode", &"WAL")
        .expect("Failed to set journal_mode PRAGMA");
    connection
        .pragma_update(None, "busy_timeout", &"250")
        .expect("Failed to set busy_timeout PRAGMA");

    veloren_connection
}
