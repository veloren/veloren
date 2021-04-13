//! Consolidates rusqlite and validation errors under a common error type

extern crate rusqlite;

use std::fmt;

#[derive(Debug)]
pub enum PersistenceError {
    // An invalid asset was returned from the database
    AssetError(String),
    // The player has already reached the max character limit
    CharacterLimitReached,
    // An error occurred while establish a db connection
    DatabaseConnectionError(rusqlite::Error),
    // An error occurred when performing a database action
    DatabaseError(rusqlite::Error),
    // Unable to load body or stats for a character
    CharacterDataError,
    SerializationError(serde_json::Error),
    ConversionError(String),
    OtherError(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::AssetError(error) => error.to_string(),
            Self::CharacterLimitReached => String::from("Character limit exceeded"),
            Self::DatabaseError(error) => error.to_string(),
            Self::DatabaseConnectionError(error) => error.to_string(),
            Self::CharacterDataError => String::from("Error while loading character data"),
            Self::SerializationError(error) => error.to_string(),
            Self::ConversionError(error) => error.to_string(),
            Self::OtherError(error) => error.to_string(),
        })
    }
}

impl From<rusqlite::Error> for PersistenceError {
    fn from(error: rusqlite::Error) -> PersistenceError { PersistenceError::DatabaseError(error) }
}

impl From<serde_json::Error> for PersistenceError {
    fn from(error: serde_json::Error) -> PersistenceError {
        PersistenceError::SerializationError(error)
    }
}
