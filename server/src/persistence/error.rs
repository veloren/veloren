extern crate diesel;

use std::fmt;

#[derive(Debug)]
pub enum Error {
    // The player has already reached the max character limit
    CharacterLimitReached,
    // An error occured when performing a database action
    DatabaseError(diesel::result::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::DatabaseError(diesel_error) => diesel_error.to_string(),
            Self::CharacterLimitReached => String::from("Character limit exceeded"),
        })
    }
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Error { Error::DatabaseError(error) }
}
