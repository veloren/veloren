use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, NullStorage};
use specs_idvs::IdvStorage;
use uuid::Uuid;

const MAX_ALIAS_LEN: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub alias: String,
    uuid: Uuid,
}

impl Player {
    pub fn new(alias: String, uuid: Uuid) -> Self { Self { alias, uuid } }

    pub fn is_valid(&self) -> bool { Self::alias_validate(&self.alias).is_ok() }

    pub fn alias_validate(alias: &str) -> Result<(), AliasError> {
        // TODO: Expose auth name validation and use it here.
        // See https://gitlab.com/veloren/auth/-/blob/master/server/src/web.rs#L20
        if !alias
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            Err(AliasError::ForbiddenCharacters)
        } else if alias.len() > MAX_ALIAS_LEN {
            Err(AliasError::TooLong)
        } else {
            Ok(())
        }
    }

    /// Not to be confused with uid
    pub fn uuid(&self) -> Uuid { self.uuid }
}

impl Component for Player {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Respawn;
impl Component for Respawn {
    type Storage = NullStorage<Self>;
}

pub enum AliasError {
    ForbiddenCharacters,
    TooLong,
}

impl ToString for AliasError {
    fn to_string(&self) -> String {
        match *self {
            AliasError::ForbiddenCharacters => "Alias contains illegal characters.",
            AliasError::TooLong => "Alias is too long.",
        }
        .to_string()
    }
}
