use crate::{
    combat,
    comp::{
        CharacterState, MeleeConstructor, StateUpdate, character_state::OutputEvents,
        tool::ToolKind,
    },
    event::LocalEvent,
    outcome::Outcome,
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
    /// At what fraction of swing_duration to make the hit
    pub hit_timing: f32,
    /// Used to construct the Melee attack
    pub melee_constructor: MeleeConstructor,
    /// Adjusts move speed during the attack per stage
    #[serde(default)]
    pub movement_modifier: MovementModifier,
    /// Adjusts turning rate during the attack per stage
    #[serde(default)]
    pub ori_modifier: OrientationModifier,
    /// Used to indicate to the frontend what ability this is for any special
    /// effects
    pub frontend_specifier: Option<FrontendSpecifier>,
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
    /// Adjusts move speed during the attack
    pub movement_modifier: Option<f32>,
    /// How fast the entity should turn
    pub ori_modifier: Option<f32>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, self.ori_modifier.unwrap_or(1.0), None);
        handle_move(data, &mut update, self.movement_modifier.unwrap_or(0.7));
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::BasicMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        movement_modifier: self.static_data.movement_modifier.swing,
                        ori_modifier: self.static_data.ori_modifier.swing,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if !self.exhausted
                    && self.timer.as_secs_f32()
                        >= self.static_data.swing_duration.as_secs_f32()
                            * self.static_data.hit_timing
                {
                    update.character = CharacterState::BasicMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        exhausted: true,
                        ..*self
                    });

                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    let tool_stats = get_tool_stats(data, self.static_data.ability_info);

                    data.updater.insert(
                        data.entity,
                        self.static_data
                            .melee_constructor
                            .create_melee(
                                precision_mult,
                                tool_stats,
                                data.stats,
                                self.static_data.ability_info,
                            )
                            .with_block_breaking(
                                data.inputs
                                    .break_block_pos
                                    .map(|p| {
                                        (
                                            p.map(|e| e.floor() as i32),
                                            self.static_data.ability_info.tool,
                                        )
                                    })
                                    .filter(|(_, tool)| {
                                        matches!(tool, Some(ToolKind::Pick | ToolKind::Shovel))
                                    }),
                            ),
                    );
                    // Send local event used for frontend shenanigans
                    if self.static_data.ability_info.tool == Some(ToolKind::Shovel) {
                        output_events.emit_local(LocalEvent::CreateOutcome(Outcome::GroundDig {
                            pos: data.pos.0 + *data.ori.look_dir() * (data.body.max_radius()),
                        }));
                    }
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::BasicMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::BasicMelee(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        movement_modifier: self.static_data.movement_modifier.recover,
                        ori_modifier: self.static_data.ori_modifier.recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    update.character = CharacterState::BasicMelee(Data {
                        timer: tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        ),
                        movement_modifier: self.static_data.movement_modifier.recover,
                        ori_modifier: self.static_data.ori_modifier.recover,
                        ..*self
                    });
                } else {
                    // Done
                    if input_is_pressed(data, self.static_data.ability_info.input) {
                        reset_state(self, data, output_events, &mut update);
                    } else {
                        end_melee_ability(data, &mut update);
                    }
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

fn reset_state(
    data: &Data,
    join: &JoinData,
    output_events: &mut OutputEvents,
    update: &mut StateUpdate,
) {
    handle_input(
        join,
        output_events,
        update,
        data.static_data.ability_info.input,
    );
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    FlameTornado,
    FireGigasWhirlwind,
}
