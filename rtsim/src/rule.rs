pub mod migrate;
pub mod npc_ai;
pub mod replenish_resources;
pub mod simulate_npcs;
pub mod sync_npcs;

use super::RtState;
use std::fmt;

#[derive(Debug)]
pub enum RuleError {
    NoSuchRule(&'static str),
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NoSuchRule(r) => {
                write!(f, "tried to fetch rule state '{}' but it does not exist", r)
            },
        }
    }
}

pub trait Rule: Sized + Send + Sync + 'static {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError>;
}
