use crate::{
    comp::{character_state::OutputEvents, CharacterState, Melee, MeleeConstructor, StateUpdate},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
        wielding,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    /// Maximum number of consecutive strikes
    pub max_strikes: u32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// How many spins it has done
    pub current_strike: u32,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the state can deal damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 0.7);
        handle_interrupts(data, &mut update, None);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to swing section of stage
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Action;
                    }
                }
            },
            StageSection::Action => {
                if !self.exhausted {
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.exhausted = true;
                    }

                    let crit_data = get_crit_data(data, self.static_data.ability_info);
                    let buff_strength = get_buff_strength(data, self.static_data.ability_info);

                    data.updater.insert(
                        data.entity,
                        self.static_data
                            .melee_constructor
                            .create_melee(crit_data, buff_strength),
                    );
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else if self.current_strike < self.static_data.max_strikes
                    && update
                        .energy
                        .try_change_by(-self.static_data.energy_cost)
                        .is_ok()
                {
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.current_strike += 1;
                        c.exhausted = false;
                    }
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                    }
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recover
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Done
                    update.character =
                        CharacterState::Wielding(wielding::Data { is_sneaking: false });
                    // Make sure attack component is removed
                    data.updater.remove::<Melee>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding(wielding::Data { is_sneaking: false });
                // Make sure attack component is removed
                data.updater.remove::<Melee>(data.entity);
            },
        }

        update
    }
}
