use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, NullStorage};
use specs_idvs::IdvStorage;
use uuid::Uuid;

use crate::resources::BattleMode;

const MAX_ALIAS_LEN: usize = 32;

#[derive(Debug)]
pub enum DisconnectReason {
    Kicked,
    NewerLogin,
    NetworkError,
    Timeout,
    ClientRequested,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub alias: String,
    pub battle_mode: BattleMode,
    uuid: Uuid,
}

impl Player {
    pub fn new(alias: String, battle_mode: BattleMode, uuid: Uuid) -> Self {
        Self {
            alias,
            battle_mode,
            uuid,
        }
    }

    /// Currently we allow attacking only if both players are opt-in to PvP.
    ///
    /// Simple as tea, if they don't want the tea, don't make them drink the tea.
    pub fn allow_harm(&self, other: &Player) -> bool {
        // TODO: discuss if we want to keep self-harm
        matches!(
            (self.battle_mode, other.battle_mode),
            (BattleMode::PvP, BattleMode::PvP)
        )
    }

    /// Inverse of `allow_harm`. Read its doc to learn more.
    pub fn disallow_harm(&self, other: &Player) -> bool {
        !self.allow_harm(other)
    }

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
