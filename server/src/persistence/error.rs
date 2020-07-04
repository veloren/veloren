//! Consolidates Diesel and validation errors under a common error type

extern crate diesel;

use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Error {
    // The player has already reached the max character limit
    CharacterLimitReached,
    // An error occured while establish a db connection
    DatabaseConnectionError(diesel::ConnectionError),
    // An error occured while running migrations
    DatabaseMigrationError(diesel_migrations::RunMigrationsError),
    // An error occured when performing a database action
    DatabaseError(diesel::result::Error),
    // Unable to load body or stats for a character
    CharacterDataError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::CharacterLimitReached => String::from("Character limit exceeded"),
            Self::DatabaseError(error) => error.to_string(),
            Self::DatabaseConnectionError(error) => error.to_string(),
            Self::DatabaseMigrationError(error) => error.to_string(),
            Self::CharacterDataError => String::from("Error while loading character data"),
        })
    }
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Error { Error::DatabaseError(error) }
}

impl From<diesel::ConnectionError> for Error {
    fn from(error: diesel::ConnectionError) -> Error { Error::DatabaseConnectionError(error) }
}

impl From<diesel_migrations::RunMigrationsError> for Error {
    fn from(error: diesel_migrations::RunMigrationsError) -> Error {
        Error::DatabaseMigrationError(error)
    }
}
