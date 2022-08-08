use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, NullStorage};
use uuid::Uuid;

use crate::resources::{BattleMode, Time};

pub const MAX_ALIAS_LEN: usize = 32;

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
    pub last_battlemode_change: Option<Time>,
    uuid: Uuid,
}

impl BattleMode {
    pub fn may_harm(self, other: Self) -> bool {
        matches!((self, other), (BattleMode::PvP, BattleMode::PvP))
    }
}

impl Player {
    pub fn new(
        alias: String,
        battle_mode: BattleMode,
        uuid: Uuid,
        last_battlemode_change: Option<Time>,
    ) -> Self {
        Self {
            alias,
            battle_mode,
            last_battlemode_change,
            uuid,
        }
    }

    /// Currently we allow attacking only if both players are opt-in to PvP.
    ///
    /// Simple as tea, if they don't want the tea, don't make them drink the
    /// tea.
    /// You can make tea for yourself though.
    pub fn may_harm(&self, other: &Player) -> bool { self.battle_mode.may_harm(other.battle_mode) }

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
    type Storage = DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}

#[derive(Clone, Debug, Default)]
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
