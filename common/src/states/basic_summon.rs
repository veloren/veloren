use crate::{
    comp::{
        self,
        inventory::loadout_builder::{self, LoadoutBuilder},
        Behavior, BehaviorCapability, CharacterState, Projectile, StateUpdate,
    },
    event::{LocalEvent, ServerEvent},
    outcome::Outcome,
    skillset_builder::{self, SkillSetBuilder},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    terrain::Block,
    vol::ReadVol,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{f32::consts::PI, time::Duration};
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
    /// Range of the summons relative to the summonner
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
    fn behavior(&self, data: &JoinData) -> StateUpdate {
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
                        stage_section: StageSection::Cast,
                        ..*self
                    });
                }
            },
            StageSection::Cast => {
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
                                LoadoutBuilder::new().with_default_maintool(&body);
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

                        let stats = comp::Stats::new("Summon".to_string());

                        let health_scaling = self
                            .static_data
                            .summon_info
                            .health_scaling
                            .map(|health_scaling| comp::Health::new(body, health_scaling));

                        // Ray cast to check where summon should happen
                        let summon_frac =
                            self.summon_count as f32 / self.static_data.summon_amount as f32;

                        let length = rand::thread_rng().gen_range(
                            self.static_data.summon_distance.0..=self.static_data.summon_distance.1,
                        );

                        // Summon in a clockwise fashion
                        let ray_vector = Vec3::new(
                            (summon_frac * 2.0 * PI).sin() * length,
                            (summon_frac * 2.0 * PI).cos() * length,
                            data.body.eye_height(),
                        );

                        // Check for collision on the xy plane
                        let obstacle_xy = data
                            .terrain
                            .ray(data.pos.0, data.pos.0 + length * ray_vector)
                            .until(Block::is_solid)
                            .cast()
                            .0;

                        let collision_vector = Vec3::new(
                            data.pos.0.x + (summon_frac * 2.0 * PI).sin() * obstacle_xy,
                            data.pos.0.y + (summon_frac * 2.0 * PI).cos() * obstacle_xy,
                            data.pos.0.z,
                        );

                        // Check for collision in z up to 50 blocks
                        let obstacle_z = data
                            .terrain
                            .ray(collision_vector, collision_vector - Vec3::unit_z() * 50.0)
                            .until(Block::is_solid)
                            .cast()
                            .0;

                        // If a duration is specified, create a projectile componenent for the npc
                        let projectile = self.static_data.duration.map(|duration| Projectile {
                            hit_solid: Vec::new(),
                            hit_entity: Vec::new(),
                            time_left: duration,
                            owner: Some(*data.uid),
                            ignore_group: true,
                            is_sticky: false,
                            is_point: false,
                        });

                        // Send server event to create npc
                        update.server_events.push_front(ServerEvent::CreateNpc {
                            pos: comp::Pos(collision_vector - Vec3::unit_z() * obstacle_z),
                            stats,
                            skill_set,
                            health: health_scaling,
                            poise: comp::Poise::new(body),
                            loadout,
                            body,
                            agent: Some(comp::Agent::new(
                                None,
                                &body,
                                Behavior::from(BehaviorCapability::SPEAK),
                                true,
                            )),
                            alignment: comp::Alignment::Owned(*data.uid),
                            scale: self
                                .static_data
                                .summon_info
                                .scale
                                .unwrap_or(comp::Scale(1.0)),
                            home_chunk: None,
                            drop_item: None,
                            rtsim_entity: None,
                            projectile,
                        });

                        // Send local event used for frontend shenanigans
                        update.local_events.push_front(LocalEvent::CreateOutcome(
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
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
            },
        }

        update
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SummonInfo {
    body: comp::Body,
    scale: Option<comp::Scale>,
    health_scaling: Option<u16>,
    // TODO: use assets for specifying skills and loadout?
    loadout_config: Option<loadout_builder::Preset>,
    skillset_config: Option<skillset_builder::Preset>,
}
