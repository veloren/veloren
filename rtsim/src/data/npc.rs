use hashbrown::HashMap;
use serde::{Serialize, Deserialize};
use slotmap::HopSlotMap;
use vek::*;
use std::ops::{Deref, DerefMut};
use common::{
    uid::Uid,
    store::Id,
    rtsim::SiteId,
};
pub use common::rtsim::NpcId;

#[derive(Clone, Serialize, Deserialize)]
pub struct Npc {
    // Persisted state

    /// Represents the location of the NPC.
    pub loc: NpcLoc,

    // Unpersisted state

    /// The position of the NPC in the world. Note that this is derived from [`Npc::loc`] and cannot be updated manually
    #[serde(skip_serializing, skip_deserializing)]
    wpos: Vec3<f32>,
    /// Whether the NPC is in simulated or loaded mode (when rtsim is run on the server, loaded corresponds to being
    /// within a loaded chunk). When in loaded mode, the interactions of the NPC should not be simulated but should
    /// instead be derived from the game.
    #[serde(skip_serializing, skip_deserializing)]
    pub mode: NpcMode,
}

impl Npc {
    pub fn new(loc: NpcLoc) -> Self {
        Self {
            loc,
            wpos: Vec3::zero(),
            mode: NpcMode::Simulated,
        }
    }

    pub fn wpos(&self) -> Vec3<f32> { self.wpos }

    /// You almost certainly *DO NOT* want to use this method.
    ///
    /// Update the NPC's wpos as a result of routine NPC simulation derived from its location.
    pub(crate) fn tick_wpos(&mut self, wpos: Vec3<f32>) { self.wpos = wpos; }
}

#[derive(Copy, Clone, Default)]
pub enum NpcMode {
    /// The NPC is unloaded and is being simulated via rtsim.
    #[default]
    Simulated,
    /// The NPC has been loaded into the game world as an ECS entity.
    Loaded,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NpcLoc {
    Wild { wpos: Vec3<f32> },
    Site { site: SiteId, wpos: Vec3<f32> },
    Travelling {
        a: SiteId,
        b: SiteId,
        frac: f32,
    },
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
