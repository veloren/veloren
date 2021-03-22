use crate::comp::skills::SkillSet;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::{error::Error, fmt};

#[derive(Debug)]
#[allow(dead_code)] // TODO: remove once trade sim hits master
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
    // TODO: Make skillset a separate component, probably too heavy for something that will
    // potentially be updated every tick (especially as more buffs are added)
    pub skill_set: SkillSet,
    pub damage_reduction: f32,
}

impl Stats {
    pub fn new(name: String) -> Self {
        Self {
            name,
            skill_set: SkillSet::default(),
            damage_reduction: 0.0,
        }
    }

    /// Creates an empty `Stats` instance - used during character loading from
    /// the database
    pub fn empty() -> Self {
        Self {
            name: "".to_owned(),
            skill_set: SkillSet::default(),
            damage_reduction: 0.0,
        }
    }
}

impl Component for Stats {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
