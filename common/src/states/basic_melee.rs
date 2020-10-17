use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state is swinging for
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub base_damage: u32,
    /// Knockback
    pub knockback: f32,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the attack can deal more damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.7);
        handle_jump(data, &mut update);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicMelee(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::BasicMelee(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                        exhausted: self.exhausted,
                    });
                }
            },
            StageSection::Swing => {
                if !self.exhausted {
                    update.character = CharacterState::BasicMelee(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: self.stage_section,
                        exhausted: true,
                    });

                    // Hit attempt
                    data.updater.insert(data.entity, Attacking {
                        base_damage: self.static_data.base_damage,
                        base_heal: 0,
                        range: self.static_data.range,
                        max_angle: 180_f32.to_radians(),
                        applied: false,
                        hit_count: 0,
                        knockback: self.static_data.knockback,
                    });
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::BasicMelee(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::BasicMelee(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        exhausted: self.exhausted,
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    update.character = CharacterState::BasicMelee(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                    // Make sure attack component is removed
                    data.updater.remove::<Attacking>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<Attacking>(data.entity);
            },
        }

        // Grant energy on successful hit
        if let Some(attack) = data.attacking {
            if attack.applied && attack.hit_count > 0 {
                data.updater.remove::<Attacking>(data.entity);
                update.energy.change_by(50, EnergySource::HitEnemy);
            }
        }

        update
    }
}
