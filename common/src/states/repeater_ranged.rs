use crate::{
    comp::{Body, CharacterState, Gravity, LightEmitter, Projectile, StateUpdate},
    event::ServerEvent,
    states::utils::{StageSection, *},
    sys::character_behavior::*,
    util::dir::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
    /// How long the state is in movement
    pub movement_duration: Duration,
    /// How long we've readied the weapon
    pub buildup_duration: Duration,
    /// How long the state is shooting
    pub shoot_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Whether there should be a jump and how strong the leap is
    pub leap: Option<f32>,
    /// Projectile options
    pub projectile: Projectile,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_gravity: Option<Gravity>,
    pub projectile_speed: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// How many repetitions remaining
    pub reps_remaining: u32,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 1.0);
        handle_jump(data, &mut update);

        match self.stage_section {
            StageSection::Movement => {
                // Jumping
                if let Some(leap_strength) = self.static_data.leap {
                    update.vel.0 = Vec3::new(data.vel.0.x, data.vel.0.y, leap_strength);
                }
                if self.timer < self.static_data.movement_duration {
                    // Do movement
                    update.character = CharacterState::RepeaterRanged(Data {
                        static_data: self.static_data.clone(),
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        reps_remaining: self.reps_remaining,
                    });
                } else {
                    // Transition to buildup
                    update.character = CharacterState::RepeaterRanged(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Buildup,
                        reps_remaining: self.reps_remaining,
                    });
                }
            },
            StageSection::Buildup => {
                // Aim gliding
                if self.static_data.leap.is_some() {
                    update.vel.0 = Vec3::new(data.vel.0.x, data.vel.0.y, 0.0);
                }
                if self.timer < self.static_data.buildup_duration {
                    // Buildup to attack
                    update.character = CharacterState::RepeaterRanged(Data {
                        static_data: self.static_data.clone(),
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        reps_remaining: self.reps_remaining,
                    });
                } else {
                    // Transition to shoot
                    update.character = CharacterState::RepeaterRanged(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Buildup,
                        reps_remaining: self.reps_remaining,
                    });
                }
            },
            StageSection::Shoot => {
                // Aim gliding
                if self.static_data.leap.is_some() {
                    update.vel.0 = Vec3::new(data.vel.0.x, data.vel.0.y, 0.0);
                }
                if self.reps_remaining > 0 {
                    // Fire
                    let mut projectile = self.static_data.projectile.clone();
                    projectile.owner = Some(*data.uid);
                    update.server_events.push_front(ServerEvent::Shoot {
                        entity: data.entity,
                        // Provides slight variation to projectile direction
                        dir: Dir::from_unnormalized(Vec3::new(
                            data.inputs.look_dir[0]
                                + (if self.reps_remaining % 2 == 0 {
                                    self.reps_remaining as f32 / 400.0
                                } else {
                                    -1.0 * self.reps_remaining as f32 / 400.0
                                }),
                            data.inputs.look_dir[1]
                                + (if self.reps_remaining % 2 == 0 {
                                    -1.0 * self.reps_remaining as f32 / 400.0
                                } else {
                                    self.reps_remaining as f32 / 400.0
                                }),
                            data.inputs.look_dir[2],
                        ))
                        .unwrap_or(data.inputs.look_dir),
                        body: self.static_data.projectile_body,
                        projectile,
                        light: self.static_data.projectile_light,
                        gravity: self.static_data.projectile_gravity,
                        speed: self.static_data.projectile_speed,
                    });

                    // Shoot projectiles
                    update.character = CharacterState::RepeaterRanged(Data {
                        static_data: self.static_data.clone(),
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        reps_remaining: self.reps_remaining - 1,
                    });
                } else if self.timer < self.static_data.shoot_duration {
                    // Finish shooting
                    update.character = CharacterState::RepeaterRanged(Data {
                        static_data: self.static_data.clone(),
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        reps_remaining: self.reps_remaining,
                    });
                } else {
                    // Transition to recover
                    update.character = CharacterState::RepeaterRanged(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Buildup,
                        reps_remaining: self.reps_remaining,
                    });
                }
            },
            StageSection::Recover => {
                if !data.physics.on_ground {
                    // Lands
                    update.character = CharacterState::RepeaterRanged(Data {
                        static_data: self.static_data.clone(),
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        reps_remaining: self.reps_remaining,
                    });
                } else if self.timer < self.static_data.recover_duration {
                    // Recovers from attack
                    update.character = CharacterState::RepeaterRanged(Data {
                        static_data: self.static_data.clone(),
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        reps_remaining: self.reps_remaining,
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
