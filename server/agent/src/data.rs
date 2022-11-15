use common::{
    comp::{
        buff::{BuffKind, Buffs},
        group,
        item::MaterialStatManifest,
        ActiveAbilities, Alignment, Body, CharacterState, Combo, Energy, Health, Inventory,
        LightEmitter, LootOwner, Ori, PhysicsState, Poise, Pos, Scale, SkillSet, Stats, Vel,
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
    pub buffs: Option<&'a Buffs>,
    pub poise: Option<&'a Poise>,
    pub cached_spatial_grid: &'a common::CachedSpatialGrid,
    pub msm: &'a MaterialStatManifest,
}

pub struct TargetData<'a> {
    pub pos: &'a Pos,
    pub body: Option<&'a Body>,
    pub scale: Option<&'a Scale>,
    pub char_state: Option<&'a CharacterState>,
    pub health: Option<&'a Health>,
    pub buffs: Option<&'a Buffs>,
}

impl<'a> TargetData<'a> {
    pub fn new(pos: &'a Pos, target: EcsEntity, read_data: &'a ReadData) -> Self {
        Self {
            pos,
            body: read_data.bodies.get(target),
            scale: read_data.scales.get(target),
            char_state: read_data.char_states.get(target),
            health: read_data.healths.get(target),
            buffs: read_data.buffs.get(target),
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
    Roshwalr,
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
    pub poises: ReadStorage<'a, Poise>,
}

pub enum Path {
    Full,
    Separate,
    Partial,
}

pub struct ComboMeleeData {
    pub min_range: f32,
    pub max_range: f32,
    pub angle: f32,
    pub energy: f32,
}

impl ComboMeleeData {
    pub fn could_use(&self, attack_data: &AttackData, agent_data: &AgentData) -> bool {
        attack_data.dist_sqrd
            < (self.max_range + agent_data.body.map_or(0.0, |b| b.max_radius())).powi(2)
            && attack_data.dist_sqrd
                > (self.min_range + agent_data.body.map_or(0.0, |b| b.max_radius())).powi(2)
            && attack_data.angle < self.angle
            && agent_data.energy.current() >= self.energy
    }
}

pub struct FinisherMeleeData {
    pub range: f32,
    pub angle: f32,
    pub energy: f32,
    pub combo: u32,
}

impl FinisherMeleeData {
    pub fn could_use(&self, attack_data: &AttackData, agent_data: &AgentData) -> bool {
        attack_data.dist_sqrd
            < (self.range + agent_data.body.map_or(0.0, |b| b.max_radius())).powi(2)
            && attack_data.angle < self.angle
            && agent_data.energy.current() >= self.energy
            && agent_data
                .combo
                .map_or(false, |c| c.counter() >= self.combo)
    }

    pub fn use_desirable(&self, tgt_data: &TargetData, agent_data: &AgentData) -> bool {
        let combo_factor =
            agent_data.combo.map_or(0, |c| c.counter()) as f32 / self.combo as f32 * 2.0;
        let tgt_health_factor = tgt_data.health.map_or(0.0, |h| h.current()) / 50.0;
        let self_health_factor = agent_data.health.map_or(0.0, |h| h.current()) / 50.0;
        // Use becomes more desirable if either self or target is close to death
        combo_factor > tgt_health_factor.min(self_health_factor)
    }
}

pub struct SelfBuffData {
    pub buff: BuffKind,
    pub energy: f32,
}

impl SelfBuffData {
    pub fn could_use(&self, agent_data: &AgentData) -> bool {
        agent_data.energy.current() >= self.energy
    }

    pub fn use_desirable(&self, agent_data: &AgentData) -> bool {
        agent_data
            .buffs
            .map_or(false, |buffs| !buffs.contains(self.buff))
    }
}

pub struct DiveMeleeData {
    pub range: f32,
    pub angle: f32,
    pub energy: f32,
}

impl DiveMeleeData {
    // Hack here refers to agents using the mildly unintended method of roll jumping
    // to achieve the required downwards vertical speed to enter dive melee when on
    // flat ground.
    pub fn npc_should_use_hack(&self, agent_data: &AgentData, tgt_data: &TargetData) -> bool {
        let dist_sqrd_2d = agent_data.pos.0.xy().distance_squared(tgt_data.pos.0.xy());
        agent_data.energy.current() > self.energy
            && agent_data.physics_state.on_ground.is_some()
            && agent_data.pos.0.z >= tgt_data.pos.0.z
            && dist_sqrd_2d
                > ((self.range + agent_data.body.map_or(0.0, |b| b.max_radius())) / 2.0).powi(2)
            && dist_sqrd_2d
                < (self.range + agent_data.body.map_or(0.0, |b| b.max_radius()) + 5.0).powi(2)
    }
}

pub struct BlockData {
    pub angle: f32,
    // Should probably just always use 5 or so unless riposte melee
    pub range: f32,
    pub energy: f32,
}

impl BlockData {
    pub fn could_use(&self, attack_data: &AttackData, agent_data: &AgentData) -> bool {
        attack_data.dist_sqrd
            < (self.range + agent_data.body.map_or(0.0, |b| b.max_radius())).powi(2)
            && attack_data.angle < self.angle
            && agent_data.energy.current() >= self.energy
    }
}

pub struct DashMeleeData {
    pub range: f32,
    pub angle: f32,
    pub initial_energy: f32,
    pub energy_drain: f32,
    pub speed: f32,
    pub charge_dur: f32,
}

impl DashMeleeData {
    // TODO: Maybe figure out better way of pulling in base accel from body and
    // accounting for friction?
    const BASE_SPEED: f32 = 3.0;
    const ORI_RATE: f32 = 30.0;

    pub fn could_use(&self, attack_data: &AttackData, agent_data: &AgentData) -> bool {
        let charge_dur = self.charge_dur(agent_data);
        let charge_dist = charge_dur * self.speed * Self::BASE_SPEED;
        let attack_dist =
            charge_dist + self.range + agent_data.body.map_or(0.0, |b| b.max_radius());
        let ori_gap = Self::ORI_RATE * charge_dur;
        attack_data.dist_sqrd < attack_dist.powi(2)
            && attack_data.angle < self.angle + ori_gap
            && agent_data.energy.current() > self.initial_energy
    }

    pub fn use_desirable(&self, attack_data: &AttackData, agent_data: &AgentData) -> bool {
        let charge_dist = self.charge_dur(agent_data) * self.speed * Self::BASE_SPEED;
        attack_data.dist_sqrd / charge_dist.powi(2) > 0.75_f32.powi(2)
    }

    fn charge_dur(&self, agent_data: &AgentData) -> f32 {
        ((agent_data.energy.current() - self.initial_energy) / self.energy_drain)
            .clamp(0.0, self.charge_dur)
    }
}

pub struct RapidMeleeData {
    pub range: f32,
    pub angle: f32,
    pub energy: f32,
    pub strikes: u32,
}

impl RapidMeleeData {
    pub fn could_use(&self, attack_data: &AttackData, agent_data: &AgentData) -> bool {
        attack_data.dist_sqrd
            < (self.range + agent_data.body.map_or(0.0, |b| b.max_radius())).powi(2)
            && attack_data.angle < self.angle
            && agent_data.energy.current() > self.energy * self.strikes as f32
    }
}
