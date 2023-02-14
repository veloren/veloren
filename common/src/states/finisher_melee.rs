use crate::{
    combat::{CombatBuff, CombatEffect},
    comp::{character_state::OutputEvents, CharacterState, MeleeConstructor, StateUpdate},
    event::ServerEvent,
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
    /// Used to construct the Melee attack
    pub melee_constructor: MeleeConstructor,
    /// Used to determine if and how scaling of the melee attack should happen
    pub scaling: Option<Scaling>,
    /// Minimum amount of combo needed to activate ability
    pub minimum_combo: u32,
    /// Amount of combo when ability was activated
    pub combo_on_use: u32,
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
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    if let CharacterState::FinisherMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to swing section of stage
                    if let CharacterState::FinisherMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Action;
                    }
                }
            },
            StageSection::Action => {
                if !self.exhausted {
                    if let CharacterState::FinisherMelee(c) = &mut update.character {
                        c.exhausted = true;
                    }

                    // Consume combo
                    output_events.emit_server(ServerEvent::ComboChange {
                        entity: data.entity,
                        change: -(self.static_data.combo_on_use as i32),
                    });

                    let mut melee_constructor = self.static_data.melee_constructor;

                    if let Some(scaling) = self.static_data.scaling {
                        let scaled_by = match scaling.kind {
                            ScalingKind::Linear => {
                                self.static_data.combo_on_use as f32
                                    / self.static_data.minimum_combo as f32
                            },
                            ScalingKind::Sqrt => (self.static_data.combo_on_use as f32
                                / self.static_data.minimum_combo as f32)
                                .sqrt(),
                        };
                        match scaling.target {
                            ScalingTarget::Attack => {
                                melee_constructor = melee_constructor.handle_scaling(scaled_by);
                            },
                            ScalingTarget::Buff => {
                                if let Some(CombatEffect::Buff(CombatBuff { strength, .. })) =
                                    &mut melee_constructor.damage_effect
                                {
                                    *strength *= scaled_by;
                                }
                            },
                        }
                    }

                    let crit_data = get_crit_data(data, self.static_data.ability_info);
                    let tool_stats = get_tool_stats(data, self.static_data.ability_info);

                    data.updater.insert(
                        data.entity,
                        melee_constructor.create_melee(crit_data, tool_stats),
                    );
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    if let CharacterState::FinisherMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::FinisherMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover
                    }
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    if let CharacterState::FinisherMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
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

        update
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingTarget {
    Attack,
    Buff,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingKind {
    // Reaches a scaling of 1 when at minimum combo, and a scaling of 2 when at double minimum
    // combo
    Linear,
    // Reaches a scaling of 1 when at minimum combo, and a scaling of 2 when at 4x minimum combo
    Sqrt,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scaling {
    pub target: ScalingTarget,
    pub kind: ScalingKind,
}
