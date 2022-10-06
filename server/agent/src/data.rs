use common::{
    comp::{
        buff::Buffs, group, item::MaterialStatManifest, ActiveAbilities, Alignment, Body,
        CharacterState, Combo, Energy, Health, Inventory, LightEmitter, LootOwner, Ori,
        PhysicsState, Pos, Scale, SkillSet, Stats, Vel,
    },
    link::Is,
    mounting::Mount,
    path::TraversalConfig,
    resources::{DeltaTime, Time, TimeOfDay},
    rtsim::RtSimEntity,
    terrain::TerrainGrid,
    uid::{Uid, UidAllocator},
};
use specs::{
    shred::ResourceId, Entities, Entity as EcsEntity, Read, ReadExpect, ReadStorage, SystemData,
    World,
};

// TODO: Move rtsim back into AgentData after rtsim2 when it has a separate
// crate
pub struct AgentData<'a> {
    pub entity: &'a EcsEntity,
    pub uid: &'a Uid,
    pub pos: &'a Pos,
    pub vel: &'a Vel,
    pub ori: &'a Ori,
    pub energy: &'a Energy,
    pub body: Option<&'a Body>,
    pub inventory: &'a Inventory,
    pub skill_set: &'a SkillSet,
    #[allow(dead_code)] // may be useful for pathing
    pub physics_state: &'a PhysicsState,
    pub alignment: Option<&'a Alignment>,
    pub traversal_config: TraversalConfig,
    pub scale: f32,
    pub damage: f32,
    pub light_emitter: Option<&'a LightEmitter>,
    pub glider_equipped: bool,
    pub is_gliding: bool,
    pub health: Option<&'a Health>,
    pub char_state: &'a CharacterState,
    pub active_abilities: &'a ActiveAbilities,
    pub combo: Option<&'a Combo>,
    pub cached_spatial_grid: &'a common::CachedSpatialGrid,
    pub msm: &'a MaterialStatManifest,
}

pub struct TargetData<'a> {
    pub pos: &'a Pos,
    pub body: Option<&'a Body>,
    pub scale: Option<&'a Scale>,
    pub char_state: Option<&'a CharacterState>,
    pub health: Option<&'a Health>,
}

impl<'a> TargetData<'a> {
    pub fn new(pos: &'a Pos, target: EcsEntity, read_data: &'a ReadData) -> Self {
        Self {
            pos,
            body: read_data.bodies.get(target),
            scale: read_data.scales.get(target),
            char_state: read_data.char_states.get(target),
            health: read_data.healths.get(target),
        }
    }
}

pub struct AttackData {
    pub min_attack_dist: f32,
    pub dist_sqrd: f32,
    pub angle: f32,
    pub angle_xy: f32,
}

impl AttackData {
    pub fn in_min_range(&self) -> bool { self.dist_sqrd < self.min_attack_dist.powi(2) }
}

#[derive(Eq, PartialEq)]
// When adding a new variant, first decide if it should instead fall under one
// of the pre-existing tactics
pub enum Tactic {
    // General tactics
    SimpleMelee,
    SimpleFlyingMelee,
    SimpleBackstab,
    ElevatedRanged,
    Turret,
    FixedTurret,
    RotatingTurret,
    RadialTurret,

    // Tool specific tactics
    Axe,
    Hammer,
    Sword,
    Bow,
    Staff,
    Sceptre,

    // Broad creature tactics
    CircleCharge { radius: u32, circle_time: u32 },
    QuadLowRanged,
    TailSlap,
    QuadLowQuick,
    QuadLowBasic,
    QuadLowBeam,
    QuadMedJump,
    QuadMedBasic,
    Theropod,
    BirdLargeBreathe,
    BirdLargeFire,
    BirdLargeBasic,
    ArthropodMelee,
    ArthropodRanged,
    ArthropodAmbush,

    // Specific species tactics
    Mindflayer,
    Minotaur,
    ClayGolem,
    TidalWarrior,
    Yeti,
    Harvester,
    StoneGolem,
    Deadwood,
    Mandragora,
    WoodGolem,
    GnarlingChieftain,
    OrganAura,
    Dagon,
    Cardinal,
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    pub entities: Entities<'a>,
    pub uid_allocator: Read<'a, UidAllocator>,
    pub dt: Read<'a, DeltaTime>,
    pub time: Read<'a, Time>,
    pub cached_spatial_grid: Read<'a, common::CachedSpatialGrid>,
    pub group_manager: Read<'a, group::GroupManager>,
    pub energies: ReadStorage<'a, Energy>,
    pub positions: ReadStorage<'a, Pos>,
    pub velocities: ReadStorage<'a, Vel>,
    pub orientations: ReadStorage<'a, Ori>,
    pub scales: ReadStorage<'a, Scale>,
    pub healths: ReadStorage<'a, Health>,
    pub inventories: ReadStorage<'a, Inventory>,
    pub stats: ReadStorage<'a, Stats>,
    pub skill_set: ReadStorage<'a, SkillSet>,
    pub physics_states: ReadStorage<'a, PhysicsState>,
    pub char_states: ReadStorage<'a, CharacterState>,
    pub uids: ReadStorage<'a, Uid>,
    pub groups: ReadStorage<'a, group::Group>,
    pub terrain: ReadExpect<'a, TerrainGrid>,
    pub alignments: ReadStorage<'a, Alignment>,
    pub bodies: ReadStorage<'a, Body>,
    pub is_mounts: ReadStorage<'a, Is<Mount>>,
    pub time_of_day: Read<'a, TimeOfDay>,
    pub light_emitter: ReadStorage<'a, LightEmitter>,
    #[cfg(feature = "worldgen")]
    pub world: ReadExpect<'a, Arc<world::World>>,
    pub rtsim_entities: ReadStorage<'a, RtSimEntity>,
    pub buffs: ReadStorage<'a, Buffs>,
    pub combos: ReadStorage<'a, Combo>,
    pub active_abilities: ReadStorage<'a, ActiveAbilities>,
    pub loot_owners: ReadStorage<'a, LootOwner>,
    pub msm: ReadExpect<'a, MaterialStatManifest>,
}

pub enum Path {
    Full,
    Separate,
    Partial,
}
