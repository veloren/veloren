use hashbrown::HashMap;
use std::fmt;
use vek::*;

#[derive(Debug)]
pub enum LocationError<'a> {
    InvalidName(String),
    DuplicateName(String),
    DoesNotExist(&'a str),
}

impl<'a> fmt::Display for LocationError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(
                f,
                "Location name '{}' is invalid. Names may only contain lowercase ASCII and \
                 underscores",
                name
            ),
            Self::DuplicateName(name) => write!(
                f,
                "Location '{}' already exists, consider deleting it first",
                name
            ),
            Self::DoesNotExist(name) => write!(f, "Location '{}' does not exist", name),
        }
    }
}

/// Locations are moderator-defined positions that can be teleported between by
/// players. They currently do not persist between server sessions.
#[derive(Default)]
pub struct Locations {
    locations: HashMap<String, Vec3<f32>>,
}

impl Locations {
    pub fn insert(&mut self, name: String, pos: Vec3<f32>) -> Result<(), LocationError<'static>> {
        if name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            self.locations
                .try_insert(name, pos)
                .map(|_| ())
                .map_err(|o| LocationError::DuplicateName(o.entry.key().clone()))
        } else {
            Err(LocationError::InvalidName(name))
        }
    }

    pub fn get<'a>(&self, name: &'a str) -> Result<Vec3<f32>, LocationError<'a>> {
        self.locations
            .get(name)
            .copied()
            .ok_or(LocationError::DoesNotExist(name))
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> { self.locations.keys() }

    pub fn remove<'a>(&mut self, name: &'a str) -> Result<(), LocationError<'a>> {
        self.locations
            .remove(name)
            .map(|_| ())
            .ok_or(LocationError::DoesNotExist(name))
    }
}
