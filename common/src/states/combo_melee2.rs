use crate::{
    comp::{
        character_state::OutputEvents, tool::Stats, CharacterState, InputKind, Melee,
        MeleeConstructor, StateUpdate, InputAttr, InventoryAction, slot::{Slot, EquipSlot},
    },
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*, idle, wielding,
    },
    uid::Uid,
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
    /// Whether or not combo melee should function as a stance (where it remains
    /// in the character state after a strike has finished)
    pub is_stance: bool,
    /// The amount of energy consumed with each swing
    pub energy_cost_per_strike: f32,
    /// What key is used to press ability
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
    pub stage_section: Option<StageSection>,
    /// Index of the strike that is currently in progress, or if not in a strike
    /// currently the next strike that will occur
    pub completed_strikes: usize,
}

pub const STANCE_ENTER_TIME: Duration = Duration::from_millis(250);
pub const STANCE_LEAVE_TIME: Duration = Duration::from_secs(3);

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        // If is a stance, use M1 to control strikes, otherwise use the input that
        // activated the ability
        let ability_input = if self.static_data.is_stance {
            InputKind::Primary
        } else {
            self.static_data.ability_info.input.unwrap_or(InputKind::Primary)
        };

        handle_orientation(data, &mut update, 1.0, None);
        let move_eff = if self.stage_section.is_some() {
            0.7
        } else {
            1.0
        };
        handle_move(data, &mut update, move_eff);
        let interrupted = handle_interrupts(data, &mut update, Some(ability_input));

        let strike_data = self.strike_data();

        match self.stage_section {
            Some(StageSection::Charge) => {
                // Adds a small duration to entering a stance to discourage spam swapping stances for ability activation benefits of matching stance
                if self.timer < STANCE_ENTER_TIME {
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else if let CharacterState::ComboMelee2(c) = &mut update.character {
                    c.timer = Duration::default();
                    c.stage_section = None;
                }
            },
            Some(StageSection::Buildup) => {
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
                        c.stage_section = Some(StageSection::Action);
                    }
                }
            },
            Some(StageSection::Action) => {
                if let Some(movement) = strike_data.movement.swing {
                    handle_forced_movement(data, &mut update, movement);
                }
                if input_is_pressed(data, ability_input) {
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        // Only have the next strike skip the recover period of this strike if not every strike in the combo is complete yet
                        c.start_next_strike = (c.completed_strikes + 1) < c.static_data.strikes.len();
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
                    let buff_strength = get_buff_strength(data, self.static_data.ability_info);

                    data.updater.insert(
                        data.entity,
                        strike_data
                            .melee_constructor
                            .create_melee(crit_data, buff_strength),
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
                    next_strike(&mut update);
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = Some(StageSection::Recover);
                    }
                }
            },
            Some(StageSection::Recover) => {
                if let Some(movement) = strike_data.movement.recover {
                    handle_forced_movement(data, &mut update, movement);
                }
                if self.timer < strike_data.recover_duration {
                    // Recovery
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // If is a stance, stay in combo melee, otherwise return to wielding
                    if self.static_data.is_stance {
                        if let CharacterState::ComboMelee2(c) = &mut update.character {
                            c.timer = Duration::default();
                            c.stage_section = None;
                            c.completed_strikes = 0;
                        }
                    } else {
                        // Return to wielding
                        end_ability(data, &mut update);
                        // Make sure attack component is removed
                        data.updater.remove::<Melee>(data.entity);
                    }
                }
            },
            Some(_) => {
                // If it somehow ends up in an incorrect stage section
                end_ability(data, &mut update);
                // Make sure attack component is removed
                data.updater.remove::<Melee>(data.entity);
            },
            None => {
                if self.timer < STANCE_LEAVE_TIME {
                    if let CharacterState::ComboMelee2(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Done
                    end_ability(data, &mut update);
                    // Make sure melee component is removed
                    data.updater.remove::<Melee>(data.entity);
                }

                handle_climb(data, &mut update);
                handle_jump(data, output_events, &mut update, 1.0);

                if input_is_pressed(data, ability_input) {
                    next_strike(&mut update)
                } else if !self.static_data.ability_info.input.map_or(false, |input| input_is_pressed(data, input)) && !interrupted {
                    attempt_input(data, output_events, &mut update);
                }
            },
        }

        update
    }

    fn start_input(
        &self,
        data: &JoinData,
        input: InputKind,
        target_entity: Option<Uid>,
        select_pos: Option<Vec3<f32>>,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        if matches!(data.character, CharacterState::ComboMelee2(data) if data.static_data.ability_info.input == Some(input) && input != InputKind::Primary && data.stage_section.is_none()) {
            end_ability(data, &mut update);
        } else {
            update.queued_inputs.insert(input, InputAttr {
                select_pos,
                target_entity,
            });
        }
        update
    }

    fn swap_equipped_weapons(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if let CharacterState::ComboMelee2(c) = data.character {
            if c.stage_section.is_none() {
                update.character =
                    CharacterState::Wielding(wielding::Data { is_sneaking: data.character.is_stealthy() });
                attempt_swap_equipped_weapons(data, &mut update);
            }
        }
        update
    }

    fn unwield(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if let CharacterState::ComboMelee2(c) = data.character {
            if c.stage_section.is_none() {
                update.character = CharacterState::Idle(idle::Data {
                    is_sneaking: data.character.is_stealthy(),
                    footwear: None,
                });
            }
        }
        update
    }

    fn manipulate_loadout(
        &self,
        data: &JoinData,
        output_events: &mut OutputEvents,
        inv_action: InventoryAction,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if let CharacterState::ComboMelee2(c) = data.character {
            if c.stage_section.is_none() {
                match inv_action {
                    InventoryAction::Drop(slot)
                    | InventoryAction::Swap(slot, _)
                    | InventoryAction::Swap(_, Slot::Equip(slot)) if matches!(slot, EquipSlot::ActiveMainhand | EquipSlot::ActiveOffhand) => {
                        update.character = CharacterState::Idle(idle::Data {
                            is_sneaking: data.character.is_stealthy(),
                            footwear: None,
                        });
                    },
                    _ => (),
                }
                handle_manipulate_loadout(data, output_events, &mut update, inv_action);
            }
        }
        update
    }

    fn glide_wield(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if let CharacterState::ComboMelee2(c) = data.character {
            if c.stage_section.is_none() {
                attempt_glide_wield(data, &mut update, output_events);
            }
        }
        update
    }

    fn sit(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if let CharacterState::ComboMelee2(c) = data.character {
            if c.stage_section.is_none() {
                attempt_sit(data, &mut update);
            }
        }
        update
    }

    fn dance(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if let CharacterState::ComboMelee2(c) = data.character {
            if c.stage_section.is_none() {
                attempt_dance(data, &mut update);
            }
        }
        update
    }

    fn sneak(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        if let CharacterState::ComboMelee2(c) = data.character {
            if c.stage_section.is_none() && data.physics.on_ground.is_some() && data.body.is_humanoid() {
                update.character = CharacterState::Wielding(wielding::Data { is_sneaking: true });
            }
        }
        update
    }
}

impl Data {
    pub fn strike_data(&self) -> &Strike<Duration> {
        &self.static_data.strikes[self.completed_strikes % self.static_data.strikes.len()]
    }
}

fn next_strike(update: &mut StateUpdate) {
    if let CharacterState::ComboMelee2(c) = &mut update.character {
        if update
            .energy
            .try_change_by(-c.static_data.energy_cost_per_strike)
            .is_ok()
        {
            c.exhausted = false;
            c.start_next_strike = false;
            c.timer = Duration::default();
            c.stage_section = Some(StageSection::Buildup);
        }
    }
}
