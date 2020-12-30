//! Consolidates Diesel and validation errors under a common error type

extern crate diesel;

use std::fmt;

#[derive(Debug)]
pub enum Error {
    // An invalid asset was returned from the database
    AssetError(String),
    // The player has already reached the max character limit
    CharacterLimitReached,
    // An error occurred while establish a db connection
    DatabaseConnectionError(diesel::ConnectionError),
    // An error occurred while running migrations
    DatabaseMigrationError(diesel_migrations::RunMigrationsError),
    // An error occurred when performing a database action
    DatabaseError(diesel::result::Error),
    // Unable to load body or stats for a character
    CharacterDataError,
    SerializationError(serde_json::Error),
    ConversionError(String),
    OtherError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::AssetError(error) => error.to_string(),
            Self::CharacterLimitReached => String::from("Character limit exceeded"),
            Self::DatabaseError(error) => error.to_string(),
            Self::DatabaseConnectionError(error) => error.to_string(),
            Self::DatabaseMigrationError(error) => error.to_string(),
            Self::CharacterDataError => String::from("Error while loading character data"),
            Self::SerializationError(error) => error.to_string(),
            Self::ConversionError(error) => error.to_string(),
            Self::OtherError(error) => error.to_string(),
        })
    }
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Error { Error::DatabaseError(error) }
}

impl From<diesel::ConnectionError> for Error {
    fn from(error: diesel::ConnectionError) -> Error { Error::DatabaseConnectionError(error) }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Error { Error::SerializationError(error) }
}

impl From<diesel_migrations::RunMigrationsError> for Error {
    fn from(error: diesel_migrations::RunMigrationsError) -> Error {
        Error::DatabaseMigrationError(error)
    }
}
