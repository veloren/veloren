use crate::{
    comp::{
        buff::{Buff, BuffChange, BuffData, BuffKind, BuffSource},
        character_state::OutputEvents,
        CharacterState, StateUpdate,
    },
    event::ServerEvent,
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
                    // Creates buff
                    let buff = Buff::new(
                        self.static_data.buff_kind,
                        BuffData {
                            strength: self.static_data.buff_strength,
                            duration: self.static_data.buff_duration,
                            delay: None,
                        },
                        Vec::new(),
                        BuffSource::Character { by: *data.uid },
                        *data.time,
                        Some(data.stats),
                        data.health,
                    );
                    output_events.emit_server(ServerEvent::Buff {
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
