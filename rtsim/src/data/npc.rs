use hashbrown::HashMap;
use serde::{Serialize, Deserialize};
use slotmap::HopSlotMap;
use vek::*;
use std::ops::{Deref, DerefMut};
use common::uid::Uid;

slotmap::new_key_type! { pub struct NpcId; }

#[derive(Clone, Serialize, Deserialize)]
pub struct Npc {
    pub wpos: Vec3<f32>,
    #[serde(skip_serializing, skip_deserializing)]
    pub mode: NpcMode,
}

impl Npc {
    pub fn at(wpos: Vec3<f32>) -> Self {
        Self { wpos, mode: NpcMode::Simulated }
    }
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
pub struct Npcs {
    pub npcs: HopSlotMap<NpcId, Npc>,
}

impl Npcs {
    pub fn spawn(&mut self, npc: Npc) -> NpcId {
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
