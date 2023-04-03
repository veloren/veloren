use crate::ai::Action;
pub use common::rtsim::{NpcId, Profession};
use common::{
    comp,
    grid::Grid,
    rtsim::{
        Actor, ChunkResource, FactionId, NpcAction, NpcActivity, Personality, SiteId, VehicleId,
    },
    store::Id,
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};
use hashbrown::HashMap;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};
use vek::*;
use world::{
    civ::Track,
    site::Site as WorldSite,
    util::{RandomPerm, LOCALITY},
};

#[derive(Copy, Clone, Debug, Default)]
pub enum SimulationMode {
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

#[derive(Default)]
pub struct Controller {
    pub actions: Vec<NpcAction>,
    pub activity: Option<NpcActivity>,
}

impl Controller {
    pub fn do_idle(&mut self) { self.activity = None; }

    pub fn do_goto(&mut self, wpos: Vec3<f32>, speed_factor: f32) {
        self.activity = Some(NpcActivity::Goto(wpos, speed_factor));
    }

    pub fn do_gather(&mut self, resources: &'static [ChunkResource]) {
        self.activity = Some(NpcActivity::Gather(resources));
    }

    pub fn do_hunt_animals(&mut self) { self.activity = Some(NpcActivity::HuntAnimals); }

    pub fn do_dance(&mut self) { self.activity = Some(NpcActivity::Dance); }

    pub fn greet(&mut self, actor: Actor) { self.actions.push(NpcAction::Greet(actor)); }
}

pub struct Brain {
    pub action: Box<dyn Action<!>>,
}

#[derive(Serialize, Deserialize)]
pub struct Npc {
    // Persisted state
    /// Represents the location of the NPC.
    pub seed: u32,
    pub wpos: Vec3<f32>,

    pub body: comp::Body,
    pub profession: Option<Profession>,
    pub home: Option<SiteId>,
    pub faction: Option<FactionId>,
    pub riding: Option<Riding>,

    pub personality: Personality,

    // Unpersisted state
    #[serde(skip)]
    pub chunk_pos: Option<Vec2<i32>>,
    #[serde(skip)]
    pub current_site: Option<SiteId>,

    #[serde(skip)]
    pub controller: Controller,

    /// Whether the NPC is in simulated or loaded mode (when rtsim is run on the
    /// server, loaded corresponds to being within a loaded chunk). When in
    /// loaded mode, the interactions of the NPC should not be simulated but
    /// should instead be derived from the game.
    #[serde(skip)]
    pub mode: SimulationMode,

    #[serde(skip)]
    pub brain: Option<Brain>,
}

impl Clone for Npc {
    fn clone(&self) -> Self {
        Self {
            seed: self.seed,
            wpos: self.wpos,
            profession: self.profession.clone(),
            home: self.home,
            faction: self.faction,
            riding: self.riding.clone(),
            body: self.body,
            personality: self.personality,
            // Not persisted
            chunk_pos: None,
            current_site: Default::default(),
            controller: Default::default(),
            mode: Default::default(),
            brain: Default::default(),
        }
    }
}

impl Npc {
    pub fn new(seed: u32, wpos: Vec3<f32>, body: comp::Body) -> Self {
        Self {
            seed,
            wpos,
            body,
            personality: Personality::default(),
            profession: None,
            home: None,
            faction: None,
            riding: None,
            chunk_pos: None,
            current_site: None,
            controller: Controller::default(),
            mode: SimulationMode::Simulated,
            brain: None,
        }
    }

    pub fn with_personality(mut self, personality: Personality) -> Self {
        self.personality = personality;
        self
    }

    pub fn with_profession(mut self, profession: impl Into<Option<Profession>>) -> Self {
        self.profession = profession.into();
        self
    }

    pub fn with_home(mut self, home: impl Into<Option<SiteId>>) -> Self {
        self.home = home.into();
        self
    }

    pub fn steering(mut self, vehicle: impl Into<Option<VehicleId>>) -> Self {
        self.riding = vehicle.into().map(|vehicle| Riding {
            vehicle,
            steering: true,
        });
        self
    }

    pub fn riding(mut self, vehicle: impl Into<Option<VehicleId>>) -> Self {
        self.riding = vehicle.into().map(|vehicle| Riding {
            vehicle,
            steering: false,
        });
        self
    }

    pub fn with_faction(mut self, faction: impl Into<Option<FactionId>>) -> Self {
        self.faction = faction.into();
        self
    }

    pub fn rng(&self, perm: u32) -> impl Rng { RandomPerm::new(self.seed.wrapping_add(perm)) }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Riding {
    pub vehicle: VehicleId,
    pub steering: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum VehicleKind {
    Airship,
    Boat,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Vehicle {
    pub wpos: Vec3<f32>,

    pub body: comp::ship::Body,

    #[serde(skip)]
    pub chunk_pos: Option<Vec2<i32>>,

    #[serde(skip)]
    pub driver: Option<Actor>,

    #[serde(skip)]
    // TODO: Find a way to detect riders when the vehicle is loaded
    pub riders: Vec<Actor>,

    /// Whether the Vehicle is in simulated or loaded mode (when rtsim is run on
    /// the server, loaded corresponds to being within a loaded chunk). When
    /// in loaded mode, the interactions of the Vehicle should not be
    /// simulated but should instead be derived from the game.
    #[serde(skip)]
    pub mode: SimulationMode,
}

impl Vehicle {
    pub fn new(wpos: Vec3<f32>, body: comp::ship::Body) -> Self {
        Self {
            wpos,
            body,
            chunk_pos: None,
            driver: None,
            riders: Vec::new(),
            mode: SimulationMode::Simulated,
        }
    }

    pub fn get_body(&self) -> comp::Body { comp::Body::Ship(self.body) }

    /// Max speed in block/s
    pub fn get_speed(&self) -> f32 {
        match self.body {
            comp::ship::Body::DefaultAirship => 15.0,
            comp::ship::Body::AirBalloon => 16.0,
            comp::ship::Body::SailBoat => 12.0,
            comp::ship::Body::Galleon => 13.0,
            _ => 10.0,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GridCell {
    pub npcs: Vec<NpcId>,
    pub vehicles: Vec<VehicleId>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Npcs {
    pub npcs: HopSlotMap<NpcId, Npc>,
    pub vehicles: HopSlotMap<VehicleId, Vehicle>,
    // TODO: This feels like it should be its own rtsim resource
    #[serde(skip, default = "construct_npc_grid")]
    pub npc_grid: Grid<GridCell>,
    #[serde(skip)]
    pub character_map: HashMap<Vec2<i32>, Vec<(common::character::CharacterId, Vec3<f32>)>>,
}

fn construct_npc_grid() -> Grid<GridCell> { Grid::new(Vec2::zero(), Default::default()) }

impl Npcs {
    pub fn create_npc(&mut self, npc: Npc) -> NpcId { self.npcs.insert(npc) }

    pub fn create_vehicle(&mut self, vehicle: Vehicle) -> VehicleId {
        self.vehicles.insert(vehicle)
    }

    /// Queries nearby npcs, not garantueed to work if radius > 32.0
    pub fn nearby(&self, wpos: Vec2<f32>, radius: f32) -> impl Iterator<Item = Actor> + '_ {
        let chunk_pos = wpos
            .as_::<i32>()
            .map2(TerrainChunkSize::RECT_SIZE.as_::<i32>(), |e, sz| {
                e.div_euclid(sz)
            });
        let r_sqr = radius * radius;
        LOCALITY
            .into_iter()
            .flat_map(move |neighbor| {
                self.npc_grid.get(chunk_pos + neighbor).map(move |cell| {
                    cell.npcs
                        .iter()
                        .copied()
                        .filter(move |npc| {
                            self.npcs
                                .get(*npc)
                                .map_or(false, |npc| npc.wpos.xy().distance_squared(wpos) < r_sqr)
                        })
                        .map(Actor::Npc)
                })
            })
            .flatten()
            .chain(
                self.character_map
                    .get(&chunk_pos)
                    .map(|characters| {
                        characters.iter().filter_map(move |(character, c_wpos)| {
                            if c_wpos.xy().distance_squared(wpos) < r_sqr {
                                Some(Actor::Character(*character))
                            } else {
                                None
                            }
                        })
                    })
                    .into_iter()
                    .flatten(),
            )
    }
}

impl Deref for Npcs {
    type Target = HopSlotMap<NpcId, Npc>;

    fn deref(&self) -> &Self::Target { &self.npcs }
}

impl DerefMut for Npcs {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.npcs }
}
