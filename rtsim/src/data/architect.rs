use std::collections::VecDeque;

use common::{
    comp,
    resources::TimeOfDay,
    rtsim::{FactionId, Role},
};
use enum_map::EnumMap;
use serde::{Deserialize, Serialize};

use super::Npc;

#[derive(Clone, Serialize, Deserialize)]
pub struct Death {
    pub time: TimeOfDay,
    pub body: comp::Body,
    pub role: Role,
    pub faction: Option<FactionId>,
}

#[derive(enum_map::Enum)]
pub enum TrackedPopulation {
    Adventurers,
    Merchants,
    Guards,
    Captains,
    OtherTownNpcs,

    PirateCaptains,
    Pirates,
    Cultists,

    GigasFrost,
    GigasFire,
    OtherMonsters,

    CloudWyvern,
    FrostWyvern,
    SeaWyvern,
    FlameWyvern,
    WealdWyvern,
    Phoenix,
    Roc,
    Cockatrice,

    Other,
}

impl TrackedPopulation {
    pub fn from_body_and_role(body: &comp::Body, role: &Role) -> Self {
        match (body, role) {
            (_, Role::Civilised(role)) => match role {
                Some(role) => match role {
                    common::rtsim::Profession::Farmer
                    | common::rtsim::Profession::Herbalist
                    | common::rtsim::Profession::Blacksmith
                    | common::rtsim::Profession::Chef
                    | common::rtsim::Profession::Alchemist
                    | common::rtsim::Profession::Hunter => Self::OtherTownNpcs,
                    common::rtsim::Profession::Merchant => Self::Merchants,
                    common::rtsim::Profession::Guard => Self::Guards,
                    common::rtsim::Profession::Adventurer(_) => Self::Adventurers,
                    common::rtsim::Profession::Pirate(false) => Self::Pirates,
                    common::rtsim::Profession::Pirate(true) => Self::PirateCaptains,
                    common::rtsim::Profession::Cultist => Self::Cultists,
                    common::rtsim::Profession::Captain => Self::Captains,
                },
                None => Self::OtherTownNpcs,
            },
            (comp::Body::BirdLarge(bird), Role::Wild) => match bird.species {
                comp::bird_large::Species::Phoenix => Self::Phoenix,
                comp::bird_large::Species::Cockatrice => Self::Cockatrice,
                comp::bird_large::Species::Roc => Self::Roc,
                comp::bird_large::Species::FlameWyvern => Self::FlameWyvern,
                comp::bird_large::Species::CloudWyvern => Self::CloudWyvern,
                comp::bird_large::Species::FrostWyvern => Self::FrostWyvern,
                comp::bird_large::Species::SeaWyvern => Self::SeaWyvern,
                comp::bird_large::Species::WealdWyvern => Self::WealdWyvern,
            },
            (comp::Body::BipedLarge(biped), Role::Monster) => match biped.species {
                comp::biped_large::Species::Gigasfrost => Self::GigasFrost,
                comp::biped_large::Species::Gigasfire => Self::GigasFire,
                _ => Self::OtherMonsters,
            },

            _ => Self::Other,
        }
    }
}

#[derive(Default, Clone)]
pub struct Population {
    populations: EnumMap<TrackedPopulation, u32>,
}

impl Population {
    pub fn total(&self) -> u32 { self.populations.values().sum::<u32>() }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TrackedPopulation, u32)> + use<'_> {
        self.populations.iter().map(|(k, v)| (k, *v))
    }

    pub fn of_death(&self, death: &Death) -> u32 {
        let pop = TrackedPopulation::from_body_and_role(&death.body, &death.role);
        self.populations[pop]
    }

    fn of_death_mut(&mut self, death: &Death) -> &mut u32 {
        let pop = TrackedPopulation::from_body_and_role(&death.body, &death.role);
        &mut self.populations[pop]
    }

    pub fn on_death(&mut self, death: &Death) {
        let n = self.of_death_mut(death);

        *n = n.saturating_sub(1);
    }

    pub fn on_spawn(&mut self, death: &Death) {
        let n = self.of_death_mut(death);

        *n = n.saturating_add(1);
    }

    pub fn add(&mut self, pop: TrackedPopulation, amount: u32) { self.populations[pop] += amount; }
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

    /// This is calculated on startup. And includes both dead and alive.
    #[serde(skip)]
    pub population: Population,
    /// This is calculated on startup, based on world size and what sites there
    /// are.
    #[serde(skip)]
    pub wanted_population: Population,
}

impl Architect {
    pub fn on_death(&mut self, npc: &Npc, time: TimeOfDay) {
        let death = Death {
            time,
            body: npc.body,
            role: npc.role.clone(),
            faction: npc.faction,
        };
        self.deaths.push_back(death)
    }
}
