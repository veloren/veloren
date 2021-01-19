use crate::{
    comp,
    comp::{skills::SkillSet, Body},
};
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
    pub body_type: Body,
}

impl Stats {
    pub fn new(name: String, body: Body) -> Self {
        Self {
            name,
            skill_set: SkillSet::default(),
            body_type: body,
        }
    }

    /// Creates an empty `Stats` instance - used during character loading from
    /// the database
    pub fn empty() -> Self {
        Self {
            name: "".to_owned(),
            skill_set: SkillSet::default(),
            body_type: comp::Body::Humanoid(comp::body::humanoid::Body::random()),
        }
    }
}

impl Component for Stats {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
