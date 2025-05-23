use crate::{
    comp::{
        self, Behavior, BehaviorCapability,
        Body::Object,
        CharacterState, Projectile, StateUpdate,
        character_state::OutputEvents,
        inventory::loadout_builder::{self, LoadoutBuilder},
        object::Body::FieryTornado,
    },
    event::{CreateNpcEvent, LocalEvent, NpcBuilder},
    npc::NPC_NAMES,
    outcome::Outcome,
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
use std::{f32::consts::PI, ops::Sub, time::Duration};
use vek::*;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long the state builds up for
    pub buildup_duration: Duration,
    /// How long the state is casting for
    pub cast_duration: Duration,
    /// How long the state recovers for
    pub recover_duration: Duration,
    /// How many creatures the state should summon
    pub summon_amount: u32,
    /// Range of the summons relative to the summoner
    pub summon_distance: (f32, f32),
    /// Information about the summoned creature
    pub summon_info: SummonInfo,
    /// Miscellaneous information about the ability
    pub ability_info: AbilityInfo,
    /// Duration of the summoned entity
    pub duration: Option<Duration>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// How many creatures have been summoned
    pub summon_count: u32,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicSummon(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::BasicSummon(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.cast_duration
                    || self.summon_count < self.static_data.summon_amount
                {
                    if self.timer
                        > self.static_data.cast_duration * self.summon_count
                            / self.static_data.summon_amount
                    {
                        let SummonInfo {
                            body,
                            loadout_config,
                            skillset_config,
                            ..
                        } = self.static_data.summon_info;

                        let loadout = {
                            let loadout_builder =
                                LoadoutBuilder::empty().with_default_maintool(&body);
                            // If preset is none, use default equipment
                            if let Some(preset) = loadout_config {
                                loadout_builder.with_preset(preset).build()
                            } else {
                                loadout_builder.with_default_equipment(&body).build()
                            }
                        };

                        let skill_set = {
                            let skillset_builder = SkillSetBuilder::default();
                            if let Some(preset) = skillset_config {
                                skillset_builder.with_preset(preset).build()
                            } else {
                                skillset_builder.build()
                            }
                        };

                        let stats = comp::Stats::new(
                            self.static_data
                                .summon_info
                                .use_npc_name
                                .then(|| {
                                    let all_names = NPC_NAMES.read();
                                    all_names.get_default_name(&self.static_data.summon_info.body)
                                })
                                .flatten()
                                .unwrap_or_else(|| {
                                    Content::with_attr(
                                        "name-custom-fallback-summon",
                                        body.gender_attr(),
                                    )
                                }),
                            body,
                        );

                        let health = self
                            .static_data
                            .summon_info
                            .has_health
                            .then(|| comp::Health::new(body));

                        // Ray cast to check where summon should happen
                        let summon_frac =
                            self.summon_count as f32 / self.static_data.summon_amount as f32;

                        let length = rand::thread_rng().gen_range(
                            self.static_data.summon_distance.0..=self.static_data.summon_distance.1,
                        );
                        let extra_height =
                            if self.static_data.summon_info.body == Object(FieryTornado) {
                                15.0
                            } else {
                                0.0
                            };
                        let position =
                            Vec3::new(data.pos.0.x, data.pos.0.y, data.pos.0.z + extra_height);
                        // Summon in a clockwise fashion
                        let ray_vector = Vec3::new(
                            (summon_frac * 2.0 * PI).sin() * length,
                            (summon_frac * 2.0 * PI).cos() * length,
                            0.0,
                        );

                        // Check for collision on the xy plane, subtract 1 to get point before block
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
                            position.z + data.body.eye_height(data.scale.map_or(1.0, |s| s.0)),
                        );

                        // Check for collision in z up to 50 blocks
                        let obstacle_z = data
                            .terrain
                            .ray(collision_vector, collision_vector - Vec3::unit_z() * 50.0)
                            .until(Block::is_solid)
                            .cast()
                            .0;

                        // If a duration is specified, create a projectile component for the npc
                        let projectile = self.static_data.duration.map(|duration| Projectile {
                            hit_solid: Vec::new(),
                            hit_entity: Vec::new(),
                            timeout: Vec::new(),
                            time_left: duration,
                            owner: Some(*data.uid),
                            ignore_group: true,
                            is_sticky: false,
                            is_point: false,
                        });

                        let mut rng = rand::thread_rng();
                        // Send server event to create npc
                        output_events.emit_server(CreateNpcEvent {
                            pos: comp::Pos(collision_vector - Vec3::unit_z() * obstacle_z),
                            ori: comp::Ori::from(Dir::random_2d(&mut rng)),
                            npc: NpcBuilder::new(stats, body, comp::Alignment::Owned(*data.uid))
                                .with_skill_set(skill_set)
                                .with_health(health)
                                .with_inventory(comp::Inventory::with_loadout(loadout, body))
                                .with_agent(
                                    comp::Agent::from_body(&body)
                                        .with_behavior(Behavior::from(BehaviorCapability::SPEAK))
                                        .with_no_flee_if(true),
                                )
                                .with_scale(
                                    self.static_data
                                        .summon_info
                                        .scale
                                        .unwrap_or(comp::Scale(1.0)),
                                )
                                .with_projectile(projectile),
                        });

                        // Send local event used for frontend shenanigans
                        output_events.emit_local(LocalEvent::CreateOutcome(
                            Outcome::SummonedCreature {
                                pos: data.pos.0,
                                body,
                            },
                        ));

                        update.character = CharacterState::BasicSummon(Data {
                            timer: tick_attack_or_default(data, self.timer, None),
                            summon_count: self.summon_count + 1,
                            ..*self
                        });
                    } else {
                        // Cast
                        update.character = CharacterState::BasicSummon(Data {
                            timer: tick_attack_or_default(data, self.timer, None),
                            ..*self
                        });
                    }
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::BasicSummon(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    update.character = CharacterState::BasicSummon(Data {
                        timer: tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        ),
                        ..*self
                    });
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
pub struct SummonInfo {
    body: comp::Body,
    scale: Option<comp::Scale>,
    has_health: bool,
    use_npc_name: bool,
    // TODO: use assets for specifying skills and loadout?
    loadout_config: Option<loadout_builder::Preset>,
    skillset_config: Option<skillset_builder::Preset>,
}
