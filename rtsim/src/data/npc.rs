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
        Role, SiteId,
    },
    store::Id,
    terrain::CoordinateConversions,
    util::Dir,
};
use hashbrown::{HashMap, HashSet};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};
use tracing::error;
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
    pub look_dir: Option<Dir>,
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

    pub fn do_dance(&mut self, dir: Option<Dir>) { self.activity = Some(NpcActivity::Dance(dir)); }

    pub fn do_cheer(&mut self, dir: Option<Dir>) { self.activity = Some(NpcActivity::Cheer(dir)); }

    pub fn do_sit(&mut self, dir: Option<Dir>, pos: Option<Vec3<i32>>) {
        self.activity = Some(NpcActivity::Sit(dir, pos));
    }

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
    pub uid: u64,
    // Persisted state
    pub seed: u32,
    /// Represents the location of the NPC.
    pub wpos: Vec3<f32>,
    pub dir: Vec2<f32>,

    pub body: comp::Body,
    pub role: Role,
    pub home: Option<SiteId>,
    pub faction: Option<FactionId>,
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
            uid: self.uid,
            seed: self.seed,
            wpos: self.wpos,
            dir: self.dir,
            role: self.role.clone(),
            home: self.home,
            faction: self.faction,
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
            // To be assigned later
            uid: 0,
            seed,
            wpos,
            dir: Vec2::unit_x(),
            body,
            personality: Default::default(),
            sentiments: Default::default(),
            role,
            home: None,
            faction: None,
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
            Role::Monster | Role::Wild | Role::Vehicle => None,
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

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GridCell {
    pub npcs: Vec<NpcId>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NpcLink {
    pub mount: NpcId,
    pub rider: Actor,
    pub is_steering: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Riders {
    steerer: Option<MountId>,
    riders: Vec<MountId>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(
    from = "HopSlotMap<MountId, NpcLink>",
    into = "HopSlotMap<MountId, NpcLink>"
)]
pub struct NpcLinks {
    links: HopSlotMap<MountId, NpcLink>,
    mount_map: slotmap::SecondaryMap<NpcId, Riders>,
    rider_map: HashMap<Actor, MountId>,
}

impl NpcLinks {
    pub fn remove_mount(&mut self, mount: NpcId) {
        if let Some(riders) = self.mount_map.remove(mount) {
            for link in riders
                .riders
                .into_iter()
                .chain(riders.steerer)
                .filter_map(|link_id| self.links.get(link_id))
            {
                self.rider_map.remove(&link.rider);
            }
        }
    }

    /// Internal function, only removes from `mount_map`.
    fn remove_rider(&mut self, id: MountId, link: &NpcLink) {
        if let Some(riders) = self.mount_map.get_mut(link.mount) {
            if link.is_steering && riders.steerer == Some(id) {
                riders.steerer = None;
            } else if let Some((i, _)) = riders.riders.iter().enumerate().find(|(_, i)| **i == id) {
                riders.riders.remove(i);
            }

            if riders.steerer.is_none() && riders.riders.is_empty() {
                self.mount_map.remove(link.mount);
            }
        }
    }

    pub fn remove_link(&mut self, link_id: MountId) {
        if let Some(link) = self.links.remove(link_id) {
            self.rider_map.remove(&link.rider);
            self.remove_rider(link_id, &link);
        }
    }

    pub fn dismount(&mut self, rider: impl Into<Actor>) {
        if let Some(id) = self.rider_map.remove(&rider.into()) {
            if let Some(link) = self.links.remove(id) {
                self.remove_rider(id, &link);
            }
        }
    }

    // This is the only function to actually add a mount link.
    // And it ensures that there isn't link chaining
    pub fn add_mounting(
        &mut self,
        mount: NpcId,
        rider: impl Into<Actor>,
        steering: bool,
    ) -> Result<MountId, MountingError> {
        let rider = rider.into();
        if Actor::Npc(mount) == rider {
            return Err(MountingError::MountSelf);
        }
        if let Actor::Npc(rider) = rider
            && self.mount_map.contains_key(rider)
        {
            return Err(MountingError::RiderIsMounted);
        }
        if self.rider_map.contains_key(&Actor::Npc(mount)) {
            return Err(MountingError::MountIsRiding);
        }
        if let Some(mount_entry) = self.mount_map.entry(mount) {
            if let hashbrown::hash_map::Entry::Vacant(rider_entry) = self.rider_map.entry(rider) {
                let riders = mount_entry.or_insert(Riders::default());

                if steering {
                    if riders.steerer.is_none() {
                        let id = self.links.insert(NpcLink {
                            mount,
                            rider,
                            is_steering: true,
                        });
                        riders.steerer = Some(id);
                        rider_entry.insert(id);
                        Ok(id)
                    } else {
                        Err(MountingError::HasSteerer)
                    }
                } else {
                    // TODO: Maybe have some limit on the number of riders depending on the mount?
                    let id = self.links.insert(NpcLink {
                        mount,
                        rider,
                        is_steering: false,
                    });
                    riders.riders.push(id);
                    rider_entry.insert(id);
                    Ok(id)
                }
            } else {
                Err(MountingError::AlreadyRiding)
            }
        } else {
            Err(MountingError::MountDead)
        }
    }

    pub fn steer(
        &mut self,
        mount: NpcId,
        rider: impl Into<Actor>,
    ) -> Result<MountId, MountingError> {
        self.add_mounting(mount, rider, true)
    }

    pub fn ride(
        &mut self,
        mount: NpcId,
        rider: impl Into<Actor>,
    ) -> Result<MountId, MountingError> {
        self.add_mounting(mount, rider, false)
    }

    pub fn get_mount_link(&self, rider: impl Into<Actor>) -> Option<&NpcLink> {
        self.rider_map
            .get(&rider.into())
            .and_then(|link| self.links.get(*link))
    }

    pub fn get_steerer_link(&self, mount: NpcId) -> Option<&NpcLink> {
        self.mount_map
            .get(mount)
            .and_then(|mount| self.links.get(mount.steerer?))
    }

    pub fn get(&self, id: MountId) -> Option<&NpcLink> { self.links.get(id) }

    pub fn ids(&self) -> impl Iterator<Item = MountId> + '_ { self.links.keys() }

    pub fn iter(&self) -> impl Iterator<Item = &NpcLink> + '_ { self.links.values() }

    pub fn iter_mounts(&self) -> impl Iterator<Item = NpcId> + '_ { self.mount_map.keys() }
}

impl From<HopSlotMap<MountId, NpcLink>> for NpcLinks {
    fn from(mut value: HopSlotMap<MountId, NpcLink>) -> Self {
        let mut from_map = slotmap::SecondaryMap::new();
        let mut to_map = HashMap::with_capacity(value.len());
        let mut delete = Vec::new();
        for (id, link) in value.iter() {
            if let Some(entry) = from_map.entry(link.mount) {
                let riders = entry.or_insert(Riders::default());
                if link.is_steering {
                    if let Some(old) = riders.steerer.replace(id) {
                        error!("Replaced steerer {old:?} with {id:?}");
                    }
                } else {
                    riders.riders.push(id);
                }
            } else {
                delete.push(id);
            }
            to_map.insert(link.rider, id);
        }
        for id in delete {
            value.remove(id);
        }
        Self {
            links: value,
            mount_map: from_map,
            rider_map: to_map,
        }
    }
}

impl From<NpcLinks> for HopSlotMap<MountId, NpcLink> {
    fn from(other: NpcLinks) -> Self { other.links }
}
slotmap::new_key_type! {
    pub struct MountId;
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MountData {
    is_steering: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Npcs {
    pub uid_counter: u64,
    pub npcs: HopSlotMap<NpcId, Npc>,
    pub mounts: NpcLinks,
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
            uid_counter: 0,
            npcs: Default::default(),
            mounts: Default::default(),
            npc_grid: construct_npc_grid(),
            character_map: Default::default(),
        }
    }
}

fn construct_npc_grid() -> Grid<GridCell> { Grid::new(Vec2::zero(), Default::default()) }

#[derive(Debug)]
pub enum MountingError {
    MountDead,
    RiderDead,
    HasSteerer,
    AlreadyRiding,
    MountIsRiding,
    RiderIsMounted,
    MountSelf,
}

impl Npcs {
    pub fn create_npc(&mut self, mut npc: Npc) -> NpcId {
        npc.uid = self.uid_counter;
        self.uid_counter += 1;
        self.npcs.insert(npc)
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
