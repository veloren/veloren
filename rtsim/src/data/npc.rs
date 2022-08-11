use hashbrown::HashMap;
use serde::{Serialize, Deserialize};
use slotmap::HopSlotMap;
use vek::*;
use rand::prelude::*;
use std::ops::{Deref, DerefMut};
use common::{
    uid::Uid,
    store::Id,
    rtsim::{SiteId, RtSimController},
    comp,
};
use world::util::RandomPerm;
pub use common::rtsim::NpcId;

#[derive(Copy, Clone, Default)]
pub enum NpcMode {
    /// The NPC is unloaded and is being simulated via rtsim.
    #[default]
    Simulated,
    /// The NPC has been loaded into the game world as an ECS entity.
    Loaded,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Npc {
    // Persisted state

    /// Represents the location of the NPC.
    pub seed: u32,
    pub wpos: Vec3<f32>,

    // Unpersisted state

    /// (wpos, speed_factor)
    #[serde(skip_serializing, skip_deserializing)]
    pub target: Option<(Vec3<f32>, f32)>,
    /// Whether the NPC is in simulated or loaded mode (when rtsim is run on the server, loaded corresponds to being
    /// within a loaded chunk). When in loaded mode, the interactions of the NPC should not be simulated but should
    /// instead be derived from the game.
    #[serde(skip_serializing, skip_deserializing)]
    pub mode: NpcMode,
}

impl Npc {
    const PERM_SPECIES: u32 = 0;
    const PERM_BODY: u32 = 1;

    pub fn new(seed: u32, wpos: Vec3<f32>) -> Self {
        Self {
            seed,
            wpos,
            target: None,
            mode: NpcMode::Simulated,
        }
    }

    pub fn rng(&self, perm: u32) -> impl Rng { RandomPerm::new(self.seed + perm) }

    pub fn get_body(&self) -> comp::Body {
        let species = *(&comp::humanoid::ALL_SPECIES)
            .choose(&mut self.rng(Self::PERM_SPECIES))
            .unwrap();
        comp::humanoid::Body::random_with(&mut self.rng(Self::PERM_BODY), &species).into()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Npcs {
    pub npcs: HopSlotMap<NpcId, Npc>,
}

impl Npcs {
    pub fn create(&mut self, npc: Npc) -> NpcId {
        self.npcs.insert(npc)
    }
}

impl Deref for Npcs {
    type Target = HopSlotMap<NpcId, Npc>;
    fn deref(&self) -> &Self::Target { &self.npcs }
}

impl DerefMut for Npcs {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.npcs }
}
