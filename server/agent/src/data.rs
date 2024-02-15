use crate::util::*;
use common::{
    comp::{
        ability::{CharacterAbility, BASE_ABILITY_LIMIT},
        buff::{BuffKind, Buffs},
        character_state::AttackFilters,
        group,
        inventory::{
            item::{
                tool::{AbilityMap, ToolKind},
                ItemKind, MaterialStatManifest,
            },
            slot::EquipSlot,
        },
        ActiveAbilities, Alignment, Body, CharacterState, Combo, Energy, Health, Inventory,
        LightEmitter, LootOwner, Ori, PhysicsState, Poise, Pos, Presence, Scale, SkillSet, Stance,
        Stats, Vel,
    },
    consts::GRAVITY,
    event, event_emitters,
    link::Is,
    mounting::{Mount, Rider, VolumeRider},
    path::TraversalConfig,
    resources::{DeltaTime, Time, TimeOfDay},
    rtsim::{Actor, RtSimEntity},
    states::utils::{ForcedMovement, StageSection},
    terrain::TerrainGrid,
    uid::{IdMaps, Uid},
};
use specs::{shred, Entities, Entity as EcsEntity, Read, ReadExpect, ReadStorage, SystemData};

event_emitters! {
    pub struct AgentEvents[AgentEmitters] {
        chat: event::ChatEvent,
        sound: event::SoundEvent,
        process_trade_action: event::ProcessTradeActionEvent,
    }
}

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
    pub rtsim_entity: Option<&'a RtSimEntity>,
}

pub struct TargetData<'a> {
    pub pos: &'a Pos,
    pub body: Option<&'a Body>,
    pub scale: Option<&'a Scale>,
    pub char_state: Option<&'a CharacterState>,
    pub health: Option<&'a Health>,
    pub buffs: Option<&'a Buffs>,
    pub drawn_weapons: (Option<ToolKind>, Option<ToolKind>),
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
            drawn_weapons: {
                let slotted_tool = |inv: &Inventory, slot| {
                    if let Some(ItemKind::Tool(tool)) =
                        inv.equipped(slot).map(|i| i.kind()).as_deref()
                    {
                        Some(tool.kind)
                    } else {
                        None
                    }
                };
                read_data
                    .inventories
                    .get(target)
                    .map_or((None, None), |inv| {
                        (
                            slotted_tool(inv, EquipSlot::ActiveMainhand),
                            slotted_tool(inv, EquipSlot::ActiveOffhand),
                        )
                    })
            },
        }
    }

    pub fn considered_ranged(&self) -> bool {
        let is_ranged_tool = |tool| match tool {
            Some(
                ToolKind::Sword
                | ToolKind::Axe
                | ToolKind::Hammer
                | ToolKind::Dagger
                | ToolKind::Shield
                | ToolKind::Spear
                | ToolKind::Farming
                | ToolKind::Pick
                | ToolKind::Shovel
                | ToolKind::Natural
                | ToolKind::Empty,
            )
            | None => false,
            Some(
                ToolKind::Bow
                | ToolKind::Staff
                | ToolKind::Sceptre
                | ToolKind::Blowgun
                | ToolKind::Debug
                | ToolKind::Instrument,
            ) => true,
        };
        is_ranged_tool(self.drawn_weapons.0) || is_ranged_tool(self.drawn_weapons.1)
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
    FieryTornado,
    SimpleDouble,
    ClayGolem,
    ClaySteed,
    AncientEffigy,
    // u8s are weights that each ability gets used, if it can be used
    RandomAbilities {
        primary: u8,
        secondary: u8,
        abilities: [u8; BASE_ABILITY_LIMIT],
    },

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
    CircleCharge {
        radius: u32,
        circle_time: u32,
    },
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
    GraveWarden,
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
    HermitAlligator,
    Cardinal,
    SeaBishop,
    Roshwalr,
    FrostGigas,
    BorealHammer,
    Dullahan,
    Cyclops,
    IceDrake,
    Flamekeeper,

    // Adlets
    AdletHunter,
    AdletIcepicker,
    AdletTracker,
    AdletElder,

    // Haniwa
    HaniwaSoldier,
    HaniwaGuard,
    HaniwaArcher,
    // Terracotta
    TerracottaStatue,
    Cursekeeper,
    CursekeeperFake,
    ShamanicSpirit,
    Jiangshi,
}

#[derive(Copy, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
pub enum AxeTactics {
    Unskilled = 0,
    SavageSimple = 1,
    MercilessSimple = 2,
    RivingSimple = 3,
    SavageIntermediate = 4,
    MercilessIntermediate = 5,
    RivingIntermediate = 6,
    SavageAdvanced = 7,
    MercilessAdvanced = 8,
    RivingAdvanced = 9,
}

impl AxeTactics {
    pub fn from_u8(x: u8) -> Self {
        use AxeTactics::*;
        match x {
            0 => Unskilled,
            1 => SavageSimple,
            2 => MercilessSimple,
            3 => RivingSimple,
            4 => SavageIntermediate,
            5 => MercilessIntermediate,
            6 => RivingIntermediate,
            7 => SavageAdvanced,
            8 => MercilessAdvanced,
            9 => RivingAdvanced,
            _ => Unskilled,
        }
    }
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    pub entities: Entities<'a>,
    pub id_maps: Read<'a, IdMaps>,
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
    pub is_riders: ReadStorage<'a, Is<Rider>>,
    pub is_volume_riders: ReadStorage<'a, Is<VolumeRider>>,
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
    pub presences: ReadStorage<'a, Presence>,
    pub ability_map: ReadExpect<'a, AbilityMap>,
}

impl<'a> ReadData<'a> {
    pub fn lookup_actor(&self, actor: Actor) -> Option<EcsEntity> {
        match actor {
            Actor::Character(character_id) => self.id_maps.character_entity(character_id),
            Actor::Npc(npc_id) => self.id_maps.rtsim_entity(RtSimEntity(npc_id)),
        }
    }
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
        combo_scales: bool,
    },
    SelfBuff {
        buff: BuffKind,
        energy: f32,
        combo: u32,
        combo_scales: bool,
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
    BasicRanged {
        energy: f32,
        projectile_speed: f32,
    },
    BasicMelee {
        energy: f32,
        range: f32,
        angle: f32,
    },
    LeapMelee {
        energy: f32,
        range: f32,
        angle: f32,
        forward_leap: f32,
        vertical_leap: f32,
        leap_dur: f32,
    },
    BasicBeam {
        energy_drain: f32,
        range: f32,
        angle: f32,
        ori_rate: f32,
    },
}

#[derive(Copy, Clone, Debug, Default)]
pub struct AbilityPreferences {
    pub desired_energy: f32,
    pub combo_scaling_buildup: u32,
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
                scaling,
                ..
            } => Self::FinisherMelee {
                energy: *energy_cost,
                range: melee_constructor.range,
                angle: melee_constructor.angle,
                combo: *minimum_combo,
                combo_scales: scaling.is_some(),
            },
            SelfBuff {
                buff_kind,
                energy_cost,
                combo_cost,
                combo_scaling,
                ..
            } => Self::SelfBuff {
                buff: *buff_kind,
                energy: *energy_cost,
                combo: *combo_cost,
                combo_scales: combo_scaling.is_some(),
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
            BasicRanged {
                energy_cost,
                projectile_speed,
                ..
            } => Self::BasicRanged {
                energy: *energy_cost,
                projectile_speed: *projectile_speed,
            },
            BasicMelee {
                energy_cost,
                melee_constructor,
                ..
            } => Self::BasicMelee {
                energy: *energy_cost,
                range: melee_constructor.range,
                angle: melee_constructor.angle,
            },
            LeapMelee {
                energy_cost,
                movement_duration,
                melee_constructor,
                forward_leap_strength,
                vertical_leap_strength,
                ..
            } => Self::LeapMelee {
                energy: *energy_cost,
                leap_dur: *movement_duration,
                range: melee_constructor.range,
                angle: melee_constructor.angle,
                forward_leap: *forward_leap_strength,
                vertical_leap: *vertical_leap_strength,
            },
            BasicBeam {
                range,
                max_angle,
                ori_rate,
                energy_drain,
                ..
            } => Self::BasicBeam {
                range: *range,
                angle: *max_angle,
                ori_rate: *ori_rate,
                energy_drain: *energy_drain,
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
        read_data: &ReadData,
        ability_preferences: AbilityPreferences,
    ) -> bool {
        let melee_check = |range: f32, angle, forced_movement: Option<ForcedMovement>| {
            let (range_inc, min_mult) = forced_movement.map_or((0.0, 0.0), |fm| match fm {
                ForcedMovement::Forward(speed) => (speed * 15.0, 1.0),
                ForcedMovement::Reverse(speed) => (-speed, 1.0),
                ForcedMovement::Leap {
                    vertical, forward, ..
                } => (
                    {
                        let dur = vertical * 2.0 / GRAVITY;
                        // 0.75 factor to allow for fact that agent looks down as they approach, so
                        // won't go as far
                        forward * dur * 0.75
                    },
                    0.0,
                ),
                _ => (0.0, 0.0),
            });
            let body_rad = agent_data.body.map_or(0.0, |b| b.max_radius());
            attack_data.dist_sqrd < (range + range_inc + body_rad).powi(2)
                && attack_data.angle < angle
                && attack_data.dist_sqrd > (range_inc * min_mult).powi(2)
        };
        let energy_check = |energy: f32| {
            agent_data.energy.current() >= energy
                && (energy < f32::EPSILON
                    || agent_data.energy.current() >= ability_preferences.desired_energy)
        };
        let combo_check = |combo, scales| {
            let additional_combo = if scales {
                ability_preferences.combo_scaling_buildup
            } else {
                0
            };
            agent_data
                .combo
                .map_or(false, |c| c.counter() >= combo + additional_combo)
        };
        let attack_kind_check = |attacks: AttackFilters| {
            tgt_data
                .char_state
                .and_then(|cs| cs.attack_kind())
                .map_or(false, |ak| attacks.applies(ak))
        };
        let ranged_check = |proj_speed| {
            let max_horiz_dist: f32 = {
                let flight_time = proj_speed * 2_f32.sqrt() / GRAVITY;
                proj_speed * 2_f32.sqrt() / 2.0 * flight_time
            };
            attack_data.dist_sqrd < max_horiz_dist.powi(2)
                && entities_have_line_of_sight(
                    agent_data.pos,
                    agent_data.body,
                    agent_data.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                )
        };
        let beam_check = |range: f32, angle, ori_rate: f32| {
            let angle_inc = ori_rate.to_degrees();
            attack_data.dist_sqrd < range.powi(2)
                && attack_data.angle < angle + angle_inc
                && entities_have_line_of_sight(
                    agent_data.pos,
                    agent_data.body,
                    agent_data.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                )
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
                combo_scales,
            } => {
                melee_check(*range, *angle, None)
                    && energy_check(*energy)
                    && combo_check(*combo, *combo_scales)
            },
            SelfBuff {
                buff,
                energy,
                combo,
                combo_scales,
            } => {
                energy_check(*energy)
                    && combo_check(*combo, *combo_scales)
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
                    && combo_check(*combo, false)
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
            BasicRanged {
                energy,
                projectile_speed,
            } => ranged_check(*projectile_speed) && energy_check(*energy),
            BasicMelee {
                energy,
                range,
                angle,
            } => melee_check(*range, *angle, None) && energy_check(*energy),
            LeapMelee {
                energy,
                range,
                angle,
                leap_dur,
                forward_leap,
                vertical_leap,
            } => {
                use common::states::utils::MovementDirection;
                let forced_move = Some(ForcedMovement::Leap {
                    vertical: *vertical_leap * *leap_dur * 2.0,
                    forward: *forward_leap,
                    progress: 0.0,
                    direction: MovementDirection::Look,
                });
                melee_check(*range, *angle, forced_move) && energy_check(*energy)
            },
            BasicBeam {
                energy_drain,
                range,
                angle,
                ori_rate,
            } => beam_check(*range, *angle, *ori_rate) && energy_check(*energy_drain * 3.0),
        }
    }
}
