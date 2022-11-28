//! DB operations and schema migrations

pub(in crate::persistence) mod character;
pub mod character_loader;
pub mod character_updater;
mod diesel_to_rusqlite;
pub mod error;
mod json_models;
mod models;

use crate::persistence::character_updater::PetPersistenceData;
use common::comp;
use refinery::Report;
use rusqlite::{Connection, OpenFlags};
use std::{
    fs,
    ops::Deref,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};
use tracing::info;

/// A struct of the components that are persisted to the DB for each character
#[derive(Debug)]
pub struct PersistedComponents {
    pub body: comp::Body,
    pub stats: comp::Stats,
    pub skill_set: comp::SkillSet,
    pub inventory: comp::Inventory,
    pub waypoint: Option<comp::Waypoint>,
    pub pets: Vec<PetPersistenceData>,
    pub active_abilities: comp::ActiveAbilities,
    pub map_marker: Option<comp::MapMarker>,
}

pub type EditableComponents = (comp::Body,);

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
        if self.sql_log_mode == settings.sql_log_mode {
            return;
        }

        set_log_mode(&mut self.connection, settings.sql_log_mode);
        self.sql_log_mode = settings.sql_log_mode;

        info!(
            "SQL log mode for connection changed to {:?}",
            settings.sql_log_mode
        );
    }
}

impl Deref for VelorenConnection {
    type Target = Connection;

    fn deref(&self) -> &Connection { &self.connection }
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlLogMode {
    /// Logging is disabled
    Disabled,
    /// Records timings for each SQL statement
    Profile,
    /// Prints all executed SQL statements
    Trace,
}

impl SqlLogMode {
    pub fn variants() -> [&'static str; 3] { ["disabled", "profile", "trace"] }
}

impl Default for SqlLogMode {
    fn default() -> Self { Self::Disabled }
}

impl core::str::FromStr for SqlLogMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "disabled" => Ok(Self::Disabled),
            "profile" => Ok(Self::Profile),
            "trace" => Ok(Self::Trace),
            _ => Err("Could not parse SqlLogMode"),
        }
    }
}

impl ToString for SqlLogMode {
    fn to_string(&self) -> String {
        match self {
            SqlLogMode::Disabled => "disabled",
            SqlLogMode::Profile => "profile",
            SqlLogMode::Trace => "trace",
        }
        .into()
    }
}

/// Runs any pending database migrations. This is executed during server startup
pub fn run_migrations(settings: &DatabaseSettings) {
    let mut conn = establish_connection(settings, ConnectionMode::ReadWrite);

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

/// Runs after the migrations. In some cases, it can reclaim a significant
/// amount of space (reported 30%)
pub fn vacuum_database(settings: &DatabaseSettings) {
    let conn = establish_connection(settings, ConnectionMode::ReadWrite);

    // The params type is phony; it's required, but not meaningful.
    conn.execute::<&[u32]>("VACUUM main", &[])
        .expect("Database vacuuming failed, server startup aborted");

    info!("Database vacuumed");
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

pub(crate) fn establish_connection(
    settings: &DatabaseSettings,
    connection_mode: ConnectionMode,
) -> VelorenConnection {
    fs::create_dir_all(&settings.db_dir)
        .unwrap_or_else(|_| panic!("Failed to create saves directory: {:?}", &settings.db_dir));

    let open_flags = OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | match connection_mode {
            ConnectionMode::ReadWrite => {
                OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE
            },
            ConnectionMode::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
        };

    let connection = Connection::open_with_flags(settings.db_dir.join("db.sqlite"), open_flags)
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

    rusqlite::vtab::array::load_module(connection).expect("Failed to load sqlite array module");

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
