use crate::{
    comp::{
        buff::{Buff, BuffCategory, BuffChange, BuffData, BuffKind, BuffSource},
        character_state::OutputEvents,
        CharacterState, StateUpdate,
    },
    event::{BuffEvent, ComboChangeEvent, LocalEvent},
    outcome::Outcome,
    resources::Secs,
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
    /// How long until state should create the aura
    pub buildup_duration: Duration,
    /// How long the state is creating an aura
    pub cast_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// What kind of buff is created
    pub buff_kind: BuffKind,
    /// Strength of the created buff
    pub buff_strength: f32,
    /// How long buff lasts
    pub buff_duration: Option<Secs>,
    /// This is the minimum amount of combo required to enter this character
    /// state
    pub combo_cost: u32,
    pub combo_scaling: Option<ScalingKind>,
    /// This is the amount of combo held by the entity when this character state
    /// was entered
    pub combo_on_use: u32,
    /// Controls whether `SelfBuff`s that were previously applied should be
    /// removed
    pub enforced_limit: bool,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Used to specify an outcome for the buff
    pub specifier: Option<FrontendSpecifier>,
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
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.8);
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::SelfBuff(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Consume combo
                    let combo_consumption = if self.static_data.combo_scaling.is_some() {
                        self.static_data.combo_on_use
                    } else {
                        self.static_data.combo_cost
                    };
                    output_events.emit_server(ComboChangeEvent {
                        entity: data.entity,
                        change: -(combo_consumption as i32),
                    });

                    let scaling_factor = self.static_data.combo_scaling.map_or(1.0, |cs| {
                        cs.factor(
                            self.static_data.combo_on_use as f32,
                            self.static_data.combo_cost as f32,
                        )
                    });

                    let mut buff_cat_ids = if self
                        .static_data
                        .ability_info
                        .ability
                        .map_or(false, |a| a.ability.is_from_tool())
                    {
                        vec![BuffCategory::RemoveOnLoadoutChange]
                    } else {
                        Vec::new()
                    };

                    // Remove previous selfbuffs if we should
                    if self.static_data.enforced_limit {
                        buff_cat_ids.push(BuffCategory::SelfBuff);

                        output_events.emit_server(BuffEvent {
                            entity: data.entity,
                            buff_change: BuffChange::RemoveByCategory {
                                all_required: vec![BuffCategory::SelfBuff],
                                any_required: vec![],
                                none_required: vec![],
                            },
                        });
                    }

                    // Creates buff
                    let buff = Buff::new(
                        self.static_data.buff_kind,
                        BuffData::new(
                            self.static_data.buff_strength * scaling_factor,
                            self.static_data.buff_duration,
                        ),
                        buff_cat_ids,
                        BuffSource::Character { by: *data.uid },
                        *data.time,
                        Some(data.stats),
                    );
                    output_events.emit_server(BuffEvent {
                        entity: data.entity,
                        buff_change: BuffChange::Add(buff),
                    });
                    // Build up
                    update.character = CharacterState::SelfBuff(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.cast_duration {
                    // Cast
                    update.character = CharacterState::SelfBuff(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                    if let Some(FrontendSpecifier::FromTheAshes) = self.static_data.specifier {
                        // Send local event used for frontend shenanigans
                        output_events.emit_local(LocalEvent::CreateOutcome(
                            Outcome::FromTheAshes { pos: data.pos.0 },
                        ));
                    }
                } else {
                    update.character = CharacterState::SelfBuff(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    update.character = CharacterState::SelfBuff(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Done
                    end_ability(data, &mut update);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_ability(data, &mut update);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        handle_interrupts(data, &mut update, output_events);

        update
    }
}
/// Used to specify a particular effect for frontend purposes
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    FromTheAshes,
}
