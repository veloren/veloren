use crate::{
    combat::{AttackTarget, CombatEffect},
    comp::{
        self, Behavior, BehaviorCapability, Body, CharacterState, Object, Ori, PidController, Pos,
        Projectile, StateUpdate, Stats, Vel,
        ability::Dodgeable,
        agent, beam,
        character_state::OutputEvents,
        inventory::loadout_builder::{self, LoadoutBuilder},
        object::{self, Body::FieryTornado},
    },
    event::{CreateNpcEvent, CreateObjectEvent, LocalEvent, NpcBuilder, SummonBeamPillarsEvent},
    npc::NPC_NAMES,
    outcome::Outcome,
    resources::Secs,
    skillset_builder::{self, SkillSetBuilder},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    terrain::Block,
    util::Dir,
    vol::ReadVol,
};
use common_i18n::Content;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    f32::consts::{PI, TAU},
    ops::Sub,
    time::Duration,
};
use vek::*;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long the state builds up for
    pub buildup_duration: Duration,
    /// How long the state is casting for
    pub cast_duration: Duration,
    /// How long the state recovers for
    pub recover_duration: Duration,
    /// Information about the summoned entities
    pub summon_info: SummonInfo,
    /// Adjusts move speed during the attack per stage
    pub movement_modifier: MovementModifier,
    /// Adjusts turning rate during the attack per stage
    pub ori_modifier: OrientationModifier,
    /// Miscellaneous information about the ability
    pub ability_info: AbilityInfo,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// How many entities have been summoned
    pub summon_count: u32,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Adjusts move speed during the attack
    pub movement_modifier: Option<f32>,
    /// How fast the entity should turn
    pub ori_modifier: Option<f32>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        let target_uid = || {
            data.controller
                .queued_inputs
                .get(&self.static_data.ability_info.input)
                .or(self.static_data.ability_info.input_attr.as_ref())
                .and_then(|input| input.target_entity)
        };

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    if let CharacterState::BasicSummon(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::BasicSummon(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Action;
                        c.movement_modifier = self.static_data.movement_modifier.swing;
                        c.ori_modifier = self.static_data.ori_modifier.swing;
                    }
                }
            },
            StageSection::Action => {
                let summon_amount = self.static_data.summon_info.summon_amount();
                if self.timer < self.static_data.cast_duration || self.summon_count < summon_amount
                {
                    if self.timer
                        > self.static_data.cast_duration * self.summon_count / summon_amount
                    {
                        match &self.static_data.summon_info {
                            SummonInfo::Npc {
                                summoned_amount: _,
                                summon_distance,
                                body,
                                loadout_config,
                                skillset_config,
                                scale,
                                has_health,
                                use_npc_name,
                                duration,
                            } => {
                                let loadout = {
                                    let loadout_builder =
                                        LoadoutBuilder::empty().with_default_maintool(body);
                                    // If preset is none, use default equipment
                                    if let Some(preset) = loadout_config {
                                        loadout_builder.with_preset(*preset).build()
                                    } else {
                                        loadout_builder.with_default_equipment(body).build()
                                    }
                                };

                                let skill_set = {
                                    let skillset_builder = SkillSetBuilder::default();
                                    if let Some(preset) = skillset_config {
                                        skillset_builder.with_preset(*preset).build()
                                    } else {
                                        skillset_builder.build()
                                    }
                                };

                                let stats = comp::Stats::new(
                                    use_npc_name
                                        .then(|| {
                                            let all_names = NPC_NAMES.read();
                                            all_names.get_default_name(body)
                                        })
                                        .flatten()
                                        .unwrap_or_else(|| {
                                            Content::with_attr(
                                                "name-custom-fallback-summon",
                                                body.gender_attr(),
                                            )
                                        }),
                                    *body,
                                );

                                let health = has_health.then(|| comp::Health::new(*body));

                                // Ray cast to check where summon should happen
                                let summon_frac = self.summon_count as f32 / summon_amount as f32;

                                let length =
                                    rand::rng().random_range(summon_distance.0..=summon_distance.1);
                                let extra_height = if *body == Body::Object(FieryTornado) {
                                    15.0
                                } else {
                                    0.0
                                };
                                let position = Vec3::new(
                                    data.pos.0.x,
                                    data.pos.0.y,
                                    data.pos.0.z + extra_height,
                                );
                                // Summon in a clockwise fashion
                                let ray_vector = Vec3::new(
                                    (summon_frac * 2.0 * PI).sin() * length,
                                    (summon_frac * 2.0 * PI).cos() * length,
                                    0.0,
                                );

                                // Check for collision on the xy plane, subtract 1 to get point
                                // before block
                                let obstacle_xy = data
                                    .terrain
                                    .ray(position, position + length * ray_vector)
                                    .until(Block::is_solid)
                                    .cast()
                                    .0
                                    .sub(1.0);

                                let collision_vector = Vec3::new(
                                    position.x + (summon_frac * 2.0 * PI).sin() * obstacle_xy,
                                    position.y + (summon_frac * 2.0 * PI).cos() * obstacle_xy,
                                    position.z
                                        + data.body.eye_height(data.scale.map_or(1.0, |s| s.0)),
                                );

                                // Check for collision in z up to 50 blocks
                                let obstacle_z = data
                                    .terrain
                                    .ray(collision_vector, collision_vector - Vec3::unit_z() * 50.0)
                                    .until(Block::is_solid)
                                    .cast()
                                    .0;

                                // If a duration is specified, create a projectile component for the
                                // npc
                                let projectile = duration.map(|duration| Projectile {
                                    hit_solid: Vec::new(),
                                    hit_entity: Vec::new(),
                                    timeout: Vec::new(),
                                    time_left: duration,
                                    init_time: Secs(duration.as_secs_f64()),
                                    owner: Some(*data.uid),
                                    ignore_group: true,
                                    is_sticky: false,
                                    is_point: false,
                                    homing: None,
                                });

                                let mut rng = rand::rng();
                                // Send server event to create npc
                                output_events.emit_server(CreateNpcEvent {
                                    pos: comp::Pos(collision_vector - Vec3::unit_z() * obstacle_z),
                                    ori: comp::Ori::from(Dir::random_2d(&mut rng)),
                                    npc: NpcBuilder::new(
                                        stats,
                                        *body,
                                        comp::Alignment::Owned(*data.uid),
                                    )
                                    .with_skill_set(skill_set)
                                    .with_health(health)
                                    .with_inventory(comp::Inventory::with_loadout(loadout, *body))
                                    .with_agent(
                                        comp::Agent::from_body(body)
                                            .with_behavior(Behavior::from(
                                                BehaviorCapability::SPEAK,
                                            ))
                                            .with_no_flee_if(true),
                                    )
                                    .with_scale(scale.unwrap_or(comp::Scale(1.0)))
                                    .with_projectile(projectile),
                                });

                                // Send local event used for frontend shenanigans
                                output_events.emit_local(LocalEvent::CreateOutcome(
                                    Outcome::SummonedCreature {
                                        pos: data.pos.0,
                                        body: *body,
                                    },
                                ));
                            },
                            SummonInfo::BeamPillar {
                                buildup_duration,
                                attack_duration,
                                beam_duration,
                                target,
                                radius,
                                height,
                                damage,
                                damage_effect,
                                dodgeable,
                                tick_rate,
                                specifier,
                                indicator_specifier,
                            } => {
                                let target = match target {
                                    BeamPillarTarget::Single => target_uid()
                                        .and_then(|target_uid| data.id_maps.uid_entity(target_uid))
                                        .map(AttackTarget::Entity),
                                    BeamPillarTarget::AllInRange(range) => {
                                        Some(AttackTarget::AllInRange(*range))
                                    },
                                };

                                if let Some(target) = target {
                                    output_events.emit_server(SummonBeamPillarsEvent {
                                        summoner: data.entity,
                                        target,
                                        buildup_duration: Duration::from_secs_f32(
                                            *buildup_duration,
                                        ),
                                        attack_duration: Duration::from_secs_f32(*attack_duration),
                                        beam_duration: Duration::from_secs_f32(*beam_duration),
                                        radius: *radius,
                                        height: *height,
                                        damage: *damage,
                                        damage_effect: damage_effect.clone(),
                                        dodgeable: *dodgeable,
                                        tick_rate: *tick_rate,
                                        specifier: *specifier,
                                        indicator_specifier: *indicator_specifier,
                                    });
                                }
                            },
                            SummonInfo::BeamWall {
                                buildup_duration,
                                attack_duration,
                                beam_duration,
                                pillar_count,
                                wall_radius,
                                pillar_radius,
                                height,
                                damage,
                                damage_effect,
                                dodgeable,
                                tick_rate,
                                specifier,
                                indicator_specifier,
                            } => {
                                let xy_angle = data
                                    .ori
                                    .to_horizontal()
                                    .angle_between(Ori::from(Dir::right()));

                                let phi = TAU / *pillar_count as f32;

                                output_events.emit_server(SummonBeamPillarsEvent {
                                    summoner: data.entity,
                                    target: AttackTarget::Pos(Vec3::new(
                                        data.pos.0.x
                                            + (wall_radius
                                                * (self.summon_count as f32 * phi + xy_angle)
                                                    .cos()),
                                        data.pos.0.y
                                            + (wall_radius
                                                * (self.summon_count as f32 * phi + xy_angle)
                                                    .sin()),
                                        data.pos.0.z,
                                    )),
                                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                                    attack_duration: Duration::from_secs_f32(*attack_duration),
                                    beam_duration: Duration::from_secs_f32(*beam_duration),
                                    radius: *pillar_radius,
                                    height: *height,
                                    damage: *damage,
                                    damage_effect: damage_effect.clone(),
                                    dodgeable: *dodgeable,
                                    tick_rate: *tick_rate,
                                    specifier: *specifier,
                                    indicator_specifier: *indicator_specifier,
                                });
                            },
                            SummonInfo::Crux {
                                max_height,
                                scale,
                                range,
                                strength,
                                duration,
                            } => {
                                let body = object::Body::Crux;
                                if let Some((kp, ki, kd)) =
                                    agent::pid_coefficients(&Body::Object(body))
                                {
                                    let initial_pos = data.pos.0 + 2.0 * Vec3::<f32>::unit_z();

                                    output_events.emit_server(CreateObjectEvent {
                                        pos: Pos(initial_pos),
                                        vel: Vel(Vec3::zero()),
                                        body: object::Body::Crux,
                                        object: Some(Object::Crux {
                                            owner: *data.uid,
                                            scale: *scale,
                                            range: *range,
                                            strength: *strength,
                                            duration: Secs(*duration),
                                            pid_controller: Some(PidController::new(
                                                kp,
                                                ki,
                                                kd,
                                                initial_pos.z + max_height,
                                                0.0,
                                                |sp, pv| sp - pv,
                                            )),
                                        }),
                                        item: None,
                                        light_emitter: None,
                                        stats: Some(Stats::new(
                                            Content::Key(String::from("lantern-crux")),
                                            Body::Object(object::Body::Crux),
                                        )),
                                    });
                                }
                            },
                        }

                        if let CharacterState::BasicSummon(c) = &mut update.character {
                            c.timer = tick_attack_or_default(data, self.timer, None);
                            c.summon_count = self.summon_count + 1;
                        }
                    } else {
                        // Cast
                        if let CharacterState::BasicSummon(c) = &mut update.character {
                            c.timer = tick_attack_or_default(data, self.timer, None);
                        }
                    }
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::BasicSummon(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                        c.movement_modifier = self.static_data.movement_modifier.recover;
                        c.ori_modifier = self.static_data.ori_modifier.recover;
                    }
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    if let CharacterState::BasicSummon(c) = &mut update.character {
                        c.timer = tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        );
                    }
                } else {
                    // Done
                    end_ability(data, &mut update);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_ability(data, &mut update);
            },
        }

        update
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BeamPillarTarget {
    Single,
    AllInRange(f32),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SummonInfo {
    Npc {
        summoned_amount: u32,
        summon_distance: (f32, f32),
        body: comp::Body,
        scale: Option<comp::Scale>,
        has_health: bool,
        #[serde(default)]
        use_npc_name: bool,
        // TODO: use assets for specifying skills and loadout?
        loadout_config: Option<loadout_builder::Preset>,
        skillset_config: Option<skillset_builder::Preset>,
        duration: Option<Duration>,
    },
    BeamPillar {
        buildup_duration: f32,
        attack_duration: f32,
        beam_duration: f32,
        target: BeamPillarTarget,
        radius: f32,
        height: f32,
        damage: f32,
        #[serde(default)]
        damage_effect: Option<CombatEffect>,
        #[serde(default)]
        dodgeable: Dodgeable,
        tick_rate: f32,
        specifier: beam::FrontendSpecifier,
        indicator_specifier: BeamPillarIndicatorSpecifier,
    },
    BeamWall {
        buildup_duration: f32,
        attack_duration: f32,
        beam_duration: f32,
        pillar_count: u32,
        wall_radius: f32,
        pillar_radius: f32,
        height: f32,
        damage: f32,
        #[serde(default)]
        damage_effect: Option<CombatEffect>,
        #[serde(default)]
        dodgeable: Dodgeable,
        tick_rate: f32,
        specifier: beam::FrontendSpecifier,
        indicator_specifier: BeamPillarIndicatorSpecifier,
    },
    Crux {
        max_height: f32,
        scale: f32,
        range: f32,
        strength: f32,
        duration: f64,
    },
}

impl SummonInfo {
    fn summon_amount(&self) -> u32 {
        match self {
            SummonInfo::Npc {
                summoned_amount, ..
            } => *summoned_amount,
            SummonInfo::BeamPillar { .. } => 1, // Fire pillars are summoned simultaneously
            SummonInfo::BeamWall { pillar_count, .. } => *pillar_count,
            SummonInfo::Crux { .. } => 1,
        }
    }

    pub fn scale_range(&mut self, scale: f32) {
        match self {
            SummonInfo::Npc {
                summon_distance, ..
            } => {
                summon_distance.0 *= scale;
                summon_distance.1 *= scale;
            },
            SummonInfo::BeamPillar { target, .. } => {
                if let BeamPillarTarget::AllInRange(range) = target {
                    *range *= scale;
                }
            },
            SummonInfo::BeamWall { wall_radius, .. } => {
                *wall_radius *= scale;
            },
            SummonInfo::Crux { .. } => {},
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, strum::EnumString)]
pub enum BeamPillarIndicatorSpecifier {
    FirePillar,
}
