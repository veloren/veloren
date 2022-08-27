use hashbrown::HashMap;
use serde::{Serialize, Deserialize};
use slotmap::HopSlotMap;
use vek::*;
use rand::prelude::*;
use std::{ops::{Deref, DerefMut}, collections::VecDeque};
use common::{
    uid::Uid,
    store::Id,
    rtsim::{SiteId, FactionId, RtSimController},
    comp,
};
use world::{util::RandomPerm, civ::Track};
use world::site::Site as WorldSite;
pub use common::rtsim::{NpcId, Profession};

#[derive(Copy, Clone, Default)]
pub enum NpcMode {
    /// The NPC is unloaded and is being simulated via rtsim.
    #[default]
    Simulated,
    /// The NPC has been loaded into the game world as an ECS entity.
    Loaded,
}

#[derive(Clone)]
pub struct PathData<P, N> {
    pub end: N,
    pub path: VecDeque<P>,
    pub repoll: bool,
}

#[derive(Clone, Default)]
pub struct PathingMemory {
    pub intrasite_path: Option<(PathData<Vec2<i32>, Vec2<i32>>, Id<WorldSite>)>,
    pub intersite_path: Option<(PathData<(Id<Track>, bool), SiteId>, usize)>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Npc {
    // Persisted state

    /// Represents the location of the NPC.
    pub seed: u32,
    pub wpos: Vec3<f32>,

    pub profession: Option<Profession>,
    pub home: Option<SiteId>,
    pub faction: Option<FactionId>,

    // Unpersisted state
    #[serde(skip_serializing, skip_deserializing)]
    pub pathing: PathingMemory,

    #[serde(skip_serializing, skip_deserializing)]
    pub current_site: Option<SiteId>,

    /// (wpos, speed_factor)
    #[serde(skip_serializing, skip_deserializing)]
    pub goto: Option<(Vec3<f32>, f32)>,
    
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
            profession: None,
            home: None,
            faction: None,
            pathing: Default::default(),
            current_site: None,
            goto: None,
            mode: NpcMode::Simulated,
        }
    }

    pub fn with_profession(mut self, profession: impl Into<Option<Profession>>) -> Self {
        self.profession = profession.into();
        self
    }

    pub fn with_home(mut self, home: impl Into<Option<SiteId>>) -> Self {
        self.home = home.into();
        self
    }

    pub fn with_faction(mut self, faction: impl Into<Option<FactionId>>) -> Self {
        self.faction = faction.into();
        self
    }

    pub fn rng(&self, perm: u32) -> impl Rng { RandomPerm::new(self.seed.wrapping_add(perm)) }

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
