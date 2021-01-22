use crate::comp::skills::SkillSet;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum StatChangeError {
    Underflow,
    Overflow,
}

impl fmt::Display for StatChangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Underflow => "insufficient stat quantity",
            Self::Overflow => "stat quantity would overflow",
        })
    }
}
impl Error for StatChangeError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub name: String,
    pub skill_set: SkillSet,
}

impl Stats {
    pub fn new(name: String) -> Self {
        Self {
            name,
            skill_set: SkillSet::default(),
        }
    }

    /// Creates an empty `Stats` instance - used during character loading from
    /// the database
    pub fn empty() -> Self {
        Self {
            name: "".to_owned(),
            skill_set: SkillSet::default(),
        }
    }
}

impl Component for Stats {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
