use crate::{
    combat,
    comp::{CharacterState, MeleeConstructor, StateUpdate, character_state::OutputEvents},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
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
    /// How long the state has until exiting if the ability missed
    pub whiffed_recover_duration: Duration,
    /// Base value that incoming damage is reduced by and converted to poise
    /// damage
    pub block_strength: f32,
    /// Used to construct the Melee attack
    pub melee_constructor: MeleeConstructor,
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
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the attack can deal more damage
    pub exhausted: bool,
    /// Whether the riposte whiffed
    pub whiffed: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 0.7);
        handle_jump(data, output_events, &mut update, 1.0);
        handle_interrupts(data, &mut update, output_events);

        match self.stage_section {
            StageSection::Buildup => {
                if let CharacterState::RiposteMelee(c) = &mut update.character {
                    if self.timer < self.static_data.buildup_duration {
                        // Build up
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    } else {
                        // If duration finishes with no parry occurring transition to recover
                        // Transition to action happens in parry hook server event
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                    }
                }
            },
            StageSection::Action => {
                if !self.exhausted {
                    if let CharacterState::RiposteMelee(c) = &mut update.character {
                        c.exhausted = true;
                    }

                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    let tool_stats = get_tool_stats(data, self.static_data.ability_info);

                    data.updater.insert(
                        data.entity,
                        self.static_data.melee_constructor.create_melee(
                            precision_mult,
                            tool_stats,
                            data.stats,
                            self.static_data.ability_info,
                        ),
                    );
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    if let CharacterState::RiposteMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::RiposteMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover
                    }
                }
            },
            StageSection::Recover => {
                if let CharacterState::RiposteMelee(c) = &mut update.character {
                    let recover_duration = if c.whiffed {
                        self.static_data.whiffed_recover_duration
                    } else {
                        self.static_data.recover_duration
                    };
                    if self.timer < recover_duration {
                        // Recovery
                        c.timer = tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        );
                    } else {
                        // Done
                        end_melee_ability(data, &mut update);
                    }
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_melee_ability(data, &mut update);
            },
        }

        update
    }
}
