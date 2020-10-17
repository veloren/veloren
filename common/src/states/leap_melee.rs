use crate::{
    comp::{Attacking, CharacterState, StateUpdate},
    states::utils::{StageSection, *},
    sys::character_behavior::{CharacterBehavior, JoinData},
    Damage, Damages,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long the state is moving
    pub movement_duration: Duration,
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the weapon swings
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
    /// Affects how far forward the player leaps
    pub forward_leap_strength: f32,
    /// Affects how high the player leaps
    pub vertical_leap_strength: f32,
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

        handle_move(data, &mut update, 0.3);
        handle_jump(data, &mut update);

        match self.stage_section {
            // Delay before leaping into the air
            StageSection::Buildup => {
                // Wait for `buildup_duration` to expire
                if self.timer < self.static_data.buildup_duration {
                    update.character = CharacterState::LeapMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to leap portion of state after buildup delay
                    update.character = CharacterState::LeapMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Movement,
                        ..*self
                    });
                }
            },
            StageSection::Movement => {
                if self.timer < self.static_data.movement_duration {
                    // Apply jumping force while in Movement portion of state
                    update.vel.0 = Vec3::new(
                        data.inputs.look_dir.x,
                        data.inputs.look_dir.y,
                        self.static_data.vertical_leap_strength,
                    ) * 2.0
                        // Multiply decreasing amount linearly over time of
                        // movement duration
                        * (1.0
                            - self.timer.as_secs_f32()
                                / self.static_data.movement_duration.as_secs_f32())
                        // Apply inputted movement directions at 0.25 strength
                        + (update.vel.0 * Vec3::new(2.0, 2.0, 0.0)
                            + 0.25 * data.inputs.move_dir.try_normalized().unwrap_or_default())
                        .try_normalized()
                        .unwrap_or_default()
                        // Multiply by forward leap strength
                            * self.static_data.forward_leap_strength
                        // Control forward movement based on look direction.
                        // This allows players to stop moving forward when they
                        // look downward at target
                            * (1.0 - data.inputs.look_dir.z.abs());

                    // Increment duration
                    // If we were to set a timeout for state, this would be
                    // outside if block and have else check for > movement
                    // duration * some multiplier
                    update.character = CharacterState::LeapMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else if data.physics.on_ground {
                    // Transitions to swing portion of state upon hitting ground
                    update.character = CharacterState::LeapMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                        ..*self
                    });
                }
            },
            StageSection::Swing => {
                if self.timer < self.static_data.swing_duration {
                    // Swings weapons
                    update.character = CharacterState::LeapMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to recover portion
                    update.character = CharacterState::LeapMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if !self.exhausted {
                    // Hit attempt, when animation plays
                    data.updater.insert(data.entity, Attacking {
                        damages: Damages::new(
                            Some(Damage::Melee(self.static_data.base_damage as f32)),
                            None,
                        ),
                        range: self.static_data.range,
                        max_angle: self.static_data.max_angle.to_radians(),
                        applied: false,
                        hit_count: 0,
                        knockback: self.static_data.knockback,
                    });

                    update.character = CharacterState::LeapMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        exhausted: true,
                        ..*self
                    });
                } else if self.timer < self.static_data.recover_duration {
                    // Complete recovery delay before finishing state
                    update.character = CharacterState::LeapMelee(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
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

        update
    }
}
