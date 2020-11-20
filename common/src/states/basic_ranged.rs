use crate::{
    comp::{Body, CharacterState, Gravity, LightEmitter, ProjectileConstructor, StateUpdate},
    event::ServerEvent,
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How much buildup is required before the attack
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Projectile variables
    pub projectile: ProjectileConstructor,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_gravity: Option<Gravity>,
    pub projectile_speed: f32,
    /// What key is used to press ability
    pub ability_key: AbilityKey,
    /// Whether or not the ability can auto continue
    pub can_continue: bool,
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
    /// Whether the attack fired already
    pub exhausted: bool,
    /// If in buildup, whether the attack has continued form previous attack; if
    /// in recover, whether the attack will continue to a new attack
    pub continue_next: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.3);
        handle_jump(data, &mut update);
        if !ability_key_is_pressed(data, self.static_data.ability_key) {
            handle_interrupt(data, &mut update, false);
            match update.character {
                CharacterState::BasicRanged(_) => {},
                _ => {
                    return update;
                },
            }
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicRanged(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::BasicRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if !self.exhausted {
                    // Fire
                    let projectile = self
                        .static_data
                        .projectile
                        .create_projectile(Some(*data.uid));
                    update.server_events.push_front(ServerEvent::Shoot {
                        entity: data.entity,
                        dir: data.inputs.look_dir,
                        body: self.static_data.projectile_body,
                        projectile,
                        light: self.static_data.projectile_light,
                        gravity: self.static_data.projectile_gravity,
                        speed: self.static_data.projectile_speed,
                    });

                    update.character = CharacterState::BasicRanged(Data {
                        exhausted: true,
                        continue_next: false,
                        ..*self
                    });
                } else if self.timer < self.static_data.recover_duration {
                    if ability_key_is_pressed(data, self.static_data.ability_key) {
                        // Recovers
                        update.character = CharacterState::BasicRanged(Data {
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(data.dt.0))
                                .unwrap_or_default(),
                            continue_next: self.static_data.can_continue,
                            ..*self
                        });
                    } else {
                        // Recovers
                        update.character = CharacterState::BasicRanged(Data {
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(data.dt.0))
                                .unwrap_or_default(),
                            ..*self
                        });
                    }
                } else if self.continue_next {
                    // Restarts character state
                    update.character = CharacterState::BasicRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Buildup,
                        exhausted: false,
                        ..*self
                    })
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
