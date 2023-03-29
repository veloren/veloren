use common::{
    comp::{
        ability::CharacterAbility,
        buff::{BuffKind, Buffs},
        character_state::AttackFilters,
        group,
        item::MaterialStatManifest,
        ActiveAbilities, Alignment, Body, CharacterState, Combo, Energy, Health, Inventory,
        LightEmitter, LootOwner, Ori, PhysicsState, Poise, Pos, Scale, SkillSet, Stance, Stats,
        Vel,
    },
    link::Is,
    mounting::Mount,
    path::TraversalConfig,
    resources::{DeltaTime, Time, TimeOfDay},
    rtsim::RtSimEntity,
    states::utils::{ForcedMovement, StageSection},
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
    pub stats: Option<&'a Stats>,
    pub poise: Option<&'a Poise>,
    pub stance: Option<&'a Stance>,
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

pub enum ActionMode {
    Reckless = 0,
    Guarded = 1,
    Fleeing = 2,
}

impl ActionMode {
    pub fn from_u8(x: u8) -> Self {
        match x {
            0 => ActionMode::Reckless,
            1 => ActionMode::Guarded,
            2 => ActionMode::Fleeing,
            _ => ActionMode::Guarded,
        }
    }
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
    // TODO: Remove tactic and ability spec
    SwordSimple,

    // Broad creature tactics
    CircleCharge { radius: u32, circle_time: u32 },
    QuadLowRanged,
    TailSlap,
    QuadLowQuick,
    QuadLowBasic,
    QuadLowBeam,
    QuadMedJump,
    QuadMedBasic,
    QuadMedHoof,
    Theropod,
    BirdLargeBreathe,
    BirdLargeFire,
    BirdLargeBasic,
    Wyvern,
    BirdMediumBasic,
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
    FrostGigas,
    BorealHammer,
}

#[derive(Copy, Clone)]
pub enum SwordTactics {
    Unskilled = 0,
    Basic = 1,
    HeavySimple = 2,
    AgileSimple = 3,
    DefensiveSimple = 4,
    CripplingSimple = 5,
    CleavingSimple = 6,
    HeavyAdvanced = 7,
    AgileAdvanced = 8,
    DefensiveAdvanced = 9,
    CripplingAdvanced = 10,
    CleavingAdvanced = 11,
}

impl SwordTactics {
    pub fn from_u8(x: u8) -> Self {
        use SwordTactics::*;
        match x {
            0 => Unskilled,
            1 => Basic,
            2 => HeavySimple,
            3 => AgileSimple,
            4 => DefensiveSimple,
            5 => CripplingSimple,
            6 => CleavingSimple,
            7 => HeavyAdvanced,
            8 => AgileAdvanced,
            9 => DefensiveAdvanced,
            10 => CripplingAdvanced,
            11 => CleavingAdvanced,
            _ => Unskilled,
        }
    }
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
    pub stances: ReadStorage<'a, Stance>,
}

pub enum Path {
    Full,
    Separate,
    Partial,
}

#[derive(Copy, Clone, Debug)]
pub enum AbilityData {
    ComboMelee {
        range: f32,
        angle: f32,
        energy_per_strike: f32,
        forced_movement: Option<ForcedMovement>,
    },
    FinisherMelee {
        range: f32,
        angle: f32,
        energy: f32,
        combo: u32,
    },
    SelfBuff {
        buff: BuffKind,
        energy: f32,
    },
    DiveMelee {
        range: f32,
        angle: f32,
        energy: f32,
    },
    DashMelee {
        range: f32,
        angle: f32,
        initial_energy: f32,
        energy_drain: f32,
        speed: f32,
        charge_dur: f32,
    },
    RapidMelee {
        range: f32,
        angle: f32,
        energy_per_strike: f32,
        strikes: u32,
        combo: u32,
    },
    ChargedMelee {
        range: f32,
        angle: f32,
        initial_energy: f32,
        energy_drain: f32,
        charge_dur: f32,
    },
    RiposteMelee {
        range: f32,
        angle: f32,
        energy: f32,
    },
    BasicBlock {
        energy: f32,
        blocked_attacks: AttackFilters,
        angle: f32,
    },
}

impl AbilityData {
    pub fn from_ability(ability: &CharacterAbility) -> Option<Self> {
        use CharacterAbility::*;
        let inner = match ability {
            ComboMelee2 {
                strikes,
                energy_cost_per_strike,
                ..
            } => {
                let (range, angle, forced_movement) = strikes
                    .iter()
                    .map(|s| {
                        (
                            s.melee_constructor.range,
                            s.melee_constructor.angle,
                            s.movement.buildup.map(|m| m * s.buildup_duration),
                        )
                    })
                    .fold(
                        (100.0, 360.0, None),
                        |(r1, a1, m1): (f32, f32, Option<ForcedMovement>),
                         (r2, a2, m2): (f32, f32, Option<ForcedMovement>)| {
                            (r1.min(r2), a1.min(a2), m1.or(m2))
                        },
                    );
                Self::ComboMelee {
                    range,
                    angle,
                    energy_per_strike: *energy_cost_per_strike,
                    forced_movement,
                }
            },
            FinisherMelee {
                energy_cost,
                melee_constructor,
                minimum_combo,
                ..
            } => Self::FinisherMelee {
                energy: *energy_cost,
                range: melee_constructor.range,
                angle: melee_constructor.angle,
                combo: *minimum_combo,
            },
            SelfBuff {
                buff_kind,
                energy_cost,
                ..
            } => Self::SelfBuff {
                buff: *buff_kind,
                energy: *energy_cost,
            },
            DiveMelee {
                energy_cost,
                melee_constructor,
                ..
            } => Self::DiveMelee {
                energy: *energy_cost,
                range: melee_constructor.range,
                angle: melee_constructor.angle,
            },
            DashMelee {
                energy_cost,
                energy_drain,
                forward_speed,
                melee_constructor,
                charge_duration,
                ..
            } => Self::DashMelee {
                initial_energy: *energy_cost,
                energy_drain: *energy_drain,
                range: melee_constructor.range,
                angle: melee_constructor.angle,
                charge_dur: *charge_duration,
                speed: *forward_speed,
            },
            RapidMelee {
                energy_cost,
                max_strikes,
                minimum_combo,
                melee_constructor,
                ..
            } => Self::RapidMelee {
                energy_per_strike: *energy_cost,
                range: melee_constructor.range,
                angle: melee_constructor.angle,
                strikes: max_strikes.unwrap_or(100),
                combo: *minimum_combo,
            },
            ChargedMelee {
                energy_cost,
                energy_drain,
                charge_duration,
                melee_constructor,
                ..
            } => Self::ChargedMelee {
                initial_energy: *energy_cost,
                energy_drain: *energy_drain,
                charge_dur: *charge_duration,
                range: melee_constructor.range,
                angle: melee_constructor.angle,
            },
            RiposteMelee {
                energy_cost,
                melee_constructor,
                ..
            } => Self::RiposteMelee {
                energy: *energy_cost,
                range: melee_constructor.range,
                angle: melee_constructor.angle,
            },
            BasicBlock {
                max_angle,
                energy_cost,
                blocked_attacks,
                ..
            } => Self::BasicBlock {
                energy: *energy_cost,
                angle: *max_angle,
                blocked_attacks: *blocked_attacks,
            },
            _ => return None,
        };
        Some(inner)
    }

    pub fn could_use(
        &self,
        attack_data: &AttackData,
        agent_data: &AgentData,
        tgt_data: &TargetData,
        desired_energy: f32,
    ) -> bool {
        let melee_check = |range: f32, angle, forced_movement: Option<ForcedMovement>| {
            let range_inc = forced_movement.map_or(0.0, |fm| match fm {
                ForcedMovement::Forward(speed) => speed * 15.0,
                ForcedMovement::Reverse(speed) => -speed,
                _ => 0.0,
            });
            let body_rad = agent_data.body.map_or(0.0, |b| b.max_radius());
            attack_data.dist_sqrd < (range + range_inc + body_rad).powi(2)
                && attack_data.angle < angle
                && attack_data.dist_sqrd > range_inc.powi(2)
        };
        let energy_check = |energy: f32| {
            agent_data.energy.current() >= energy
                && (energy < f32::EPSILON || agent_data.energy.current() >= desired_energy)
        };
        let combo_check = |combo| agent_data.combo.map_or(false, |c| c.counter() >= combo);
        let attack_kind_check = |attacks: AttackFilters| {
            tgt_data
                .char_state
                .and_then(|cs| cs.attack_kind())
                .map_or(false, |ak| attacks.applies(ak))
        };
        use AbilityData::*;
        match self {
            ComboMelee {
                range,
                angle,
                energy_per_strike,
                forced_movement,
            } => melee_check(*range, *angle, *forced_movement) && energy_check(*energy_per_strike),
            FinisherMelee {
                range,
                angle,
                energy,
                combo,
            } => melee_check(*range, *angle, None) && energy_check(*energy) && combo_check(*combo),
            SelfBuff { buff, energy } => {
                energy_check(*energy)
                    && agent_data
                        .buffs
                        .map_or(false, |buffs| !buffs.contains(*buff))
            },
            DiveMelee {
                range,
                angle,
                energy,
            } => melee_check(*range, *angle, None) && energy_check(*energy),
            DashMelee {
                range,
                angle,
                initial_energy,
                energy_drain,
                speed,
                charge_dur,
            } => {
                // TODO: Maybe figure out better way of pulling in base accel from body and
                // accounting for friction?
                const BASE_SPEED: f32 = 3.0;
                const ORI_RATE: f32 = 30.0;
                let charge_dur = ((agent_data.energy.current() - initial_energy) / energy_drain)
                    .clamp(0.0, *charge_dur);
                let charge_dist = charge_dur * speed * BASE_SPEED;
                let attack_dist = charge_dist + range;
                let ori_gap = ORI_RATE * charge_dur;
                // TODO: Replace None with actual forced movement later
                melee_check(attack_dist, angle + ori_gap, None)
                    && energy_check(*initial_energy)
                    && attack_data.dist_sqrd / charge_dist.powi(2) > 0.75_f32.powi(2)
            },
            RapidMelee {
                range,
                angle,
                energy_per_strike,
                strikes,
                combo,
            } => {
                melee_check(*range, *angle, None)
                    && energy_check(*energy_per_strike * *strikes as f32)
                    && combo_check(*combo)
            },
            ChargedMelee {
                range,
                angle,
                initial_energy,
                energy_drain,
                charge_dur,
            } => {
                melee_check(*range, *angle, None)
                    && energy_check(*initial_energy + *energy_drain * *charge_dur)
            },
            RiposteMelee {
                energy,
                range,
                angle,
            } => {
                melee_check(*range, *angle, None)
                    && energy_check(*energy)
                    && tgt_data.char_state.map_or(false, |cs| {
                        cs.is_melee_attack()
                            && matches!(
                                cs.stage_section(),
                                Some(
                                    StageSection::Buildup
                                        | StageSection::Charge
                                        | StageSection::Movement
                                )
                            )
                    })
            },
            BasicBlock {
                energy,
                angle,
                blocked_attacks,
            } => {
                melee_check(25.0, *angle, None)
                    && energy_check(*energy)
                    && attack_kind_check(*blocked_attacks)
                    && tgt_data
                        .char_state
                        .and_then(|cs| cs.stage_section())
                        .map_or(false, |ss| !matches!(ss, StageSection::Recover))
            },
        }
    }
}
