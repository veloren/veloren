use crate::{
    comp::{character_state::OutputEvents, CharacterState, Melee, MeleeConstructor, StateUpdate},
    consts::GRAVITY,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until the state attacks
    pub buildup_duration: Duration,
    /// How long the state is in the swing duration
    pub swing_duration: Duration,
    /// How long until state ends
    pub recover_duration: Duration,
    /// Used to construct the Melee attack
    pub melee_constructor: MeleeConstructor,
    /// Energy cost per attack
    pub energy_cost: f32,
    /// Whether spin state is infinite
    pub is_infinite: bool,
    /// Used to dictate how movement functions in this state
    pub movement_behavior: MovementBehavior,
    /// Used for forced forward movement
    pub forward_speed: f32,
    /// Number of spins
    pub num_spins: u32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Used to specify the melee attack to the frontend
    pub specifier: Option<FrontendSpecifier>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// How many spins it has done
    pub consecutive_spins: u32,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the state can deal damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        match self.static_data.movement_behavior {
            MovementBehavior::ForwardGround | MovementBehavior::Stationary => {},
            MovementBehavior::AxeHover => {
                let new_vel_z = update.vel.0.z + GRAVITY * data.dt.0 * 0.5;
                update.vel.0 = Vec3::new(0.0, 0.0, new_vel_z) + data.inputs.move_dir * 5.0;
            },
            MovementBehavior::Walking => {
                handle_move(data, &mut update, 0.2);
            },
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::SpinMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::SpinMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if !self.exhausted {
                    update.character = CharacterState::SpinMelee(Data {
                        timer: Duration::default(),
                        exhausted: true,
                        ..*self
                    });

                    let crit_data = get_crit_data(data, self.static_data.ability_info);
                    let tool_stats = get_tool_stats(data, self.static_data.ability_info);

                    data.updater.insert(
                        data.entity,
                        self.static_data
                            .melee_constructor
                            .create_melee(crit_data, tool_stats),
                    );
                } else if self.timer < self.static_data.swing_duration {
                    if matches!(
                        self.static_data.movement_behavior,
                        MovementBehavior::ForwardGround
                    ) {
                        handle_forced_movement(
                            data,
                            &mut update,
                            ForcedMovement::Forward(self.static_data.forward_speed),
                        );
                    }

                    // Swings
                    update.character = CharacterState::SpinMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else if update.energy.current() >= self.static_data.energy_cost
                    && (self.consecutive_spins < self.static_data.num_spins
                        || (self.static_data.is_infinite
                            && input_is_pressed(data, self.static_data.ability_info.input)))
                {
                    update.character = CharacterState::SpinMelee(Data {
                        timer: Duration::default(),
                        consecutive_spins: self.consecutive_spins + 1,
                        exhausted: false,
                        ..*self
                    });
                    // Consumes energy if there's enough left and RMB is held down
                    update.energy.change_by(-self.static_data.energy_cost);
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::SpinMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                    // Remove melee attack component
                    data.updater.remove::<Melee>(data.entity);
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recover
                    update.character = CharacterState::SpinMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Done
                    end_melee_ability(data, &mut update);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_melee_ability(data, &mut update);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        handle_interrupts(data, &mut update, output_events);

        update
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementBehavior {
    Stationary,
    ForwardGround,
    AxeHover,
    Walking,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    CultistVortex,
}
