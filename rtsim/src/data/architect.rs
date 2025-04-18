use std::collections::VecDeque;

use common::{
    comp,
    resources::TimeOfDay,
    rtsim::{FactionId, Role},
};
use serde::{Deserialize, Serialize};

use super::Npc;

#[derive(Clone, Serialize, Deserialize)]
pub struct Death {
    pub time: TimeOfDay,
    pub body: comp::Body,
    pub role: Role,
    pub faction: Option<FactionId>,
}

/// The architect has the responsibility of making sure the game keeps working.
/// Which means keeping the simulation in check, and making sure interesting
/// stuff keeps happening.
///
/// Currently it handles:
/// - Keeping track of all deaths that happen, and respawn something similar to
///   keep the world from dying out.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Architect {
    pub deaths: VecDeque<Death>,
}

impl Architect {
    pub fn on_death(&mut self, npc: &Npc, time: TimeOfDay) {
        self.deaths.push_back(Death {
            time,
            body: npc.body,
            role: npc.role.clone(),
            faction: npc.faction,
        })
    }
}
