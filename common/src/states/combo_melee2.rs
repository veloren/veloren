use crate::{
    comp::{
        character_state::OutputEvents, tool::Stats, CharacterState, MeleeConstructor, StateUpdate,
    },
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::*;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strike<T> {
    /// Used to construct the Melee attack
    pub melee_constructor: MeleeConstructor,
    /// Initial buildup duration of stage (how long until state can deal damage)
    pub buildup_duration: T,
    /// Duration of stage spent in swing (controls animation stuff, and can also
    /// be used to handle movement separately to buildup)
    pub swing_duration: T,
    /// At what fraction of the swing duration to apply the melee "hit"
    pub hit_timing: f32,
    /// Initial recover duration of stage (how long until character exits state)
    pub recover_duration: T,
    /// How much forward movement there is in the swing portion of the stage
    #[serde(default)]
    pub movement: StrikeMovement,
    /// Adjusts turning rate during the attack
    pub ori_modifier: f32,
}

impl Strike<f32> {
    pub fn to_duration(self) -> Strike<Duration> {
        Strike::<Duration> {
            melee_constructor: self.melee_constructor,
            buildup_duration: Duration::from_secs_f32(self.buildup_duration),
            swing_duration: Duration::from_secs_f32(self.swing_duration),
            hit_timing: self.hit_timing,
            recover_duration: Duration::from_secs_f32(self.recover_duration),
            movement: self.movement,
            ori_modifier: self.ori_modifier,
        }
    }

    #[must_use]
    pub fn adjusted_by_stats(self, stats: Stats) -> Self {
        Self {
            melee_constructor: self.melee_constructor.adjusted_by_stats(stats),
            buildup_duration: self.buildup_duration / stats.speed,
            swing_duration: self.swing_duration / stats.speed,
            hit_timing: self.hit_timing,
            recover_duration: self.recover_duration / stats.speed,
            movement: self.movement,
            ori_modifier: self.ori_modifier,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct StrikeMovement {
    pub buildup: Option<ForcedMovement>,
    pub swing: Option<ForcedMovement>,
    pub recover: Option<ForcedMovement>,
}

// TODO: Completely rewrite this with skill tree rework. Don't bother converting
// to melee constructor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
    /// Data for each stage
    pub strikes: Vec<Strike<Duration>>,
    /// The amount of energy consumed with each swing
    pub energy_cost_per_strike: f32,
    /// Whether or not the state should progress through all strikes
    /// automatically once the state is entered
    pub auto_progress: bool,
    pub ability_info: AbilityInfo,
}
/// A sequence of attacks that can incrementally become faster and more
/// damaging.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Whether the attack was executed already
    pub exhausted: bool,
    /// Whether the strike should skip recover
    pub start_next_strike: bool,
    /// Timer for each stage
    pub timer: Duration,
    /// Checks what section a strike is in, if a strike is currently being
    /// performed
    pub stage_section: StageSection,
    /// Index of the strike that is currently in progress, or if not in a strike
    /// currently the next strike that will occur
    pub completed_strikes: usize,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 0.7);
        handle_interrupts(data, &mut update, output_events);

        let strike_data = self.strike_data();

        match self.stage_section {
            StageSection::Buildup => {
                if let Some(movement) = strike_data.movement.buildup {
                    handle_forced_movement(data, &mut update, movement);
                }
                if self.timer < strike_data.buildup_duration {
                    // Build up
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to swing section of stage
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Action;
                    }
                }
            },
            StageSection::Action => {
                if let Some(movement) = strike_data.movement.swing {
                    handle_forced_movement(data, &mut update, movement);
                }
                if input_is_pressed(data, self.static_data.ability_info.input)
                    || self.static_data.auto_progress
                {
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        // Only have the next strike skip the recover period of this strike if not
                        // every strike in the combo is complete yet
                        c.start_next_strike =
                            (c.completed_strikes + 1) < c.static_data.strikes.len();
                    }
                }
                if self.timer.as_secs_f32()
                    > strike_data.hit_timing * strike_data.swing_duration.as_secs_f32()
                    && !self.exhausted
                {
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                        c.exhausted = true;
                    }

                    let crit_data = get_crit_data(data, self.static_data.ability_info);
                    let tool_stats = get_tool_stats(data, self.static_data.ability_info);

                    data.updater.insert(
                        data.entity,
                        strike_data
                            .melee_constructor
                            .create_melee(crit_data, tool_stats),
                    );
                } else if self.timer < strike_data.swing_duration {
                    // Swings
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else if self.start_next_strike {
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.completed_strikes += 1;
                    }
                    next_strike(data, &mut update);
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                    }
                }
            },
            StageSection::Recover => {
                if let Some(movement) = strike_data.movement.recover {
                    handle_forced_movement(data, &mut update, movement);
                }
                if self.timer < strike_data.recover_duration {
                    // Recovery
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Return to wielding
                    end_melee_ability(data, &mut update);
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

impl Data {
    pub fn strike_data(&self) -> &Strike<Duration> {
        &self.static_data.strikes[self.completed_strikes % self.static_data.strikes.len()]
    }
}

fn next_strike(data: &JoinData, update: &mut StateUpdate) {
    let revert_to_wield = if let CharacterState::ComboMelee2(c) = &mut update.character {
        if update
            .energy
            .try_change_by(-c.static_data.energy_cost_per_strike)
            .is_ok()
        {
            c.exhausted = false;
            c.start_next_strike = false;
            c.timer = Duration::default();
            c.stage_section = StageSection::Buildup;
            false
        } else {
            true
        }
    } else {
        false
    };
    if revert_to_wield {
        end_melee_ability(data, update)
    }
}
