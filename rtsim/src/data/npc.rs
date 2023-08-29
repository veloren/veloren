use crate::{
    ai::Action,
    data::{Reports, Sentiments},
    gen::name,
};
pub use common::rtsim::{NpcId, Profession};
use common::{
    character::CharacterId,
    comp,
    grid::Grid,
    rtsim::{
        Actor, ChunkResource, FactionId, NpcAction, NpcActivity, NpcInput, Personality, ReportId,
        Role, SiteId, VehicleId,
    },
    store::Id,
    terrain::CoordinateConversions,
};
use hashbrown::{HashMap, HashSet};
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
    pub new_home: Option<SiteId>,
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

    pub fn say(&mut self, target: impl Into<Option<Actor>>, content: comp::Content) {
        self.actions.push(NpcAction::Say(target.into(), content));
    }

    pub fn attack(&mut self, target: impl Into<Actor>) {
        self.actions.push(NpcAction::Attack(target.into()));
    }

    pub fn set_new_home(&mut self, new_home: SiteId) { self.new_home = Some(new_home); }
}

pub struct Brain {
    pub action: Box<dyn Action<(), !>>,
}

#[derive(Serialize, Deserialize)]
pub struct Npc {
    // Persisted state
    pub seed: u32,
    /// Represents the location of the NPC.
    pub wpos: Vec3<f32>,

    pub body: comp::Body,
    pub role: Role,
    pub home: Option<SiteId>,
    pub faction: Option<FactionId>,
    pub riding: Option<Riding>,

    pub is_dead: bool,

    /// The [`Report`]s that the NPC is aware of.
    pub known_reports: HashSet<ReportId>,

    #[serde(default)]
    pub personality: Personality,
    #[serde(default)]
    pub sentiments: Sentiments,

    // Unpersisted state
    #[serde(skip)]
    pub chunk_pos: Option<Vec2<i32>>,
    #[serde(skip)]
    pub current_site: Option<SiteId>,

    #[serde(skip)]
    pub controller: Controller,
    #[serde(skip)]
    pub inbox: VecDeque<NpcInput>,

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
            role: self.role.clone(),
            home: self.home,
            faction: self.faction,
            riding: self.riding.clone(),
            is_dead: self.is_dead,
            known_reports: self.known_reports.clone(),
            body: self.body,
            personality: self.personality,
            sentiments: self.sentiments.clone(),
            // Not persisted
            chunk_pos: None,
            current_site: Default::default(),
            controller: Default::default(),
            inbox: Default::default(),
            mode: Default::default(),
            brain: Default::default(),
        }
    }
}

impl Npc {
    pub const PERM_ENTITY_CONFIG: u32 = 1;
    const PERM_NAME: u32 = 0;

    pub fn new(seed: u32, wpos: Vec3<f32>, body: comp::Body, role: Role) -> Self {
        Self {
            seed,
            wpos,
            body,
            personality: Default::default(),
            sentiments: Default::default(),
            role,
            home: None,
            faction: None,
            riding: None,
            is_dead: false,
            known_reports: Default::default(),
            chunk_pos: None,
            current_site: None,
            controller: Default::default(),
            inbox: Default::default(),
            mode: SimulationMode::Simulated,
            brain: None,
        }
    }

    // TODO: have a dedicated `NpcBuilder` type for this.
    pub fn with_personality(mut self, personality: Personality) -> Self {
        self.personality = personality;
        self
    }

    // // TODO: have a dedicated `NpcBuilder` type for this.
    // pub fn with_profession(mut self, profession: impl Into<Option<Profession>>)
    // -> Self {     if let Role::Humanoid(p) = &mut self.role {
    //         *p = profession.into();
    //     } else {
    //         panic!("Tried to assign profession {:?} to NPC, but has role {:?},
    // which cannot have a profession", profession.into(), self.role);     }
    //     self
    // }

    // TODO: have a dedicated `NpcBuilder` type for this.
    pub fn with_home(mut self, home: impl Into<Option<SiteId>>) -> Self {
        self.home = home.into();
        self
    }

    // TODO: have a dedicated `NpcBuilder` type for this.
    pub fn steering(mut self, vehicle: impl Into<Option<VehicleId>>) -> Self {
        self.riding = vehicle.into().map(|vehicle| Riding {
            vehicle,
            steering: true,
        });
        self
    }

    // TODO: have a dedicated `NpcBuilder` type for this.
    pub fn riding(mut self, vehicle: impl Into<Option<VehicleId>>) -> Self {
        self.riding = vehicle.into().map(|vehicle| Riding {
            vehicle,
            steering: false,
        });
        self
    }

    // TODO: have a dedicated `NpcBuilder` type for this.
    pub fn with_faction(mut self, faction: impl Into<Option<FactionId>>) -> Self {
        self.faction = faction.into();
        self
    }

    pub fn rng(&self, perm: u32) -> impl Rng { RandomPerm::new(self.seed.wrapping_add(perm)) }

    // TODO: Don't make this depend on deterministic RNG, actually persist names
    // once we've decided that we want to
    pub fn get_name(&self) -> String { name::generate(&mut self.rng(Self::PERM_NAME)) }

    pub fn profession(&self) -> Option<Profession> {
        match &self.role {
            Role::Civilised(profession) => profession.clone(),
            Role::Monster | Role::Wild => None,
        }
    }

    pub fn cleanup(&mut self, reports: &Reports) {
        // Clear old or superfluous sentiments
        // TODO: It might be worth giving more important NPCs a higher sentiment
        // 'budget' than less important ones.
        self.sentiments
            .cleanup(crate::data::sentiment::NPC_MAX_SENTIMENTS);
        // Clear reports that have been forgotten
        self.known_reports
            .retain(|report| reports.contains_key(*report));
        // TODO: Limit number of reports
        // TODO: Clear old inbox items
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Riding {
    pub vehicle: VehicleId,
    pub steering: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum VehicleKind {
    Airship,
    Boat,
}

// TODO: Merge into `Npc`?
#[derive(Clone, Serialize, Deserialize)]
pub struct Vehicle {
    pub wpos: Vec3<f32>,
    pub dir: Vec2<f32>,

    pub body: comp::ship::Body,

    #[serde(skip)]
    pub chunk_pos: Option<Vec2<i32>>,

    #[serde(skip)]
    pub driver: Option<Actor>,

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
            dir: Vec2::unit_y(),
            body,
            chunk_pos: None,
            driver: None,
            mode: SimulationMode::Simulated,
        }
    }

    pub fn get_body(&self) -> comp::Body { comp::Body::Ship(self.body) }

    /// Max speed in block/s
    pub fn get_speed(&self) -> f32 {
        match self.body {
            comp::ship::Body::DefaultAirship => 7.0,
            comp::ship::Body::AirBalloon => 8.0,
            comp::ship::Body::SailBoat => 5.0,
            comp::ship::Body::Galleon => 6.0,
            comp::ship::Body::Skiff => 6.0,
            comp::ship::Body::Submarine => 4.0,
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
    // TODO: Consider switching to `common::util::SpatialGrid` instead
    #[serde(skip, default = "construct_npc_grid")]
    pub npc_grid: Grid<GridCell>,
    #[serde(skip)]
    pub character_map: HashMap<Vec2<i32>, Vec<(CharacterId, Vec3<f32>)>>,
}

impl Default for Npcs {
    fn default() -> Self {
        Self {
            npcs: Default::default(),
            vehicles: Default::default(),
            npc_grid: construct_npc_grid(),
            character_map: Default::default(),
        }
    }
}

fn construct_npc_grid() -> Grid<GridCell> { Grid::new(Vec2::zero(), Default::default()) }

impl Npcs {
    pub fn create_npc(&mut self, npc: Npc) -> NpcId { self.npcs.insert(npc) }

    pub fn create_vehicle(&mut self, vehicle: Vehicle) -> VehicleId {
        self.vehicles.insert(vehicle)
    }

    /// Queries nearby npcs, not garantueed to work if radius > 32.0
    // TODO: Find a more efficient way to implement this, it's currently
    // (theoretically) O(n^2).
    pub fn nearby(
        &self,
        this_npc: Option<NpcId>,
        wpos: Vec3<f32>,
        radius: f32,
    ) -> impl Iterator<Item = Actor> + '_ {
        let chunk_pos = wpos.xy().as_().wpos_to_cpos();
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
                                .map_or(false, |npc| npc.wpos.distance_squared(wpos) < r_sqr)
                                && Some(*npc) != this_npc
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
                            if c_wpos.distance_squared(wpos) < r_sqr {
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
