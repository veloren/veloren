use crate::{
    combat::GroupTarget,
    comp::{
        aura::{AuraBuffConstructor, AuraChange, AuraKind, AuraTarget, Specifier},
        character_state::OutputEvents,
        CharacterState, StateUpdate,
    },
    event::{AuraEvent, ComboChangeEvent},
    resources::Secs,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should create the aura
    pub buildup_duration: Duration,
    /// How long the state is creating an aura
    pub cast_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Determines how the aura selects its targets
    pub targets: GroupTarget,
    /// Has information used to construct the auras
    pub auras: Vec<AuraBuffConstructor>,
    /// How long aura lasts
    pub aura_duration: Option<Secs>,
    /// Radius of aura
    pub range: f32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Whether the aura's effect scales with the user's current combo
    pub scales_with_combo: bool,
    /// Combo at the time the aura is first cast
    pub combo_at_cast: u32,
    /// Used to specify aura to the frontend
    pub specifier: Option<Specifier>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 0.8);
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicAura(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Creates aura
                    let targets =
                        AuraTarget::from((Some(self.static_data.targets), Some(data.uid)));
                    for aura_data in &self.static_data.auras {
                        let mut aura = aura_data.to_aura(
                            data.uid,
                            self.static_data.range,
                            // check for indefinite aura
                            self.static_data.aura_duration,
                            targets,
                            *data.time,
                        );
                        if self.static_data.scales_with_combo {
                            match aura.aura_kind {
                                AuraKind::Buff {
                                    kind: _,
                                    ref mut data,
                                    category: _,
                                    source: _,
                                } => {
                                    data.strength *=
                                        (self.static_data.combo_at_cast.max(1) as f32).sqrt();
                                },
                            }
                            output_events.emit_server(ComboChangeEvent {
                                entity: data.entity,
                                change: -(self.static_data.combo_at_cast as i32),
                            });
                        }
                        output_events.emit_server(AuraEvent {
                            entity: data.entity,
                            aura_change: AuraChange::Add(aura),
                        });
                    }
                    // Build up
                    update.character = CharacterState::BasicAura(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                    });
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.cast_duration {
                    // Cast
                    update.character = CharacterState::BasicAura(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    update.character = CharacterState::BasicAura(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    update.character = CharacterState::BasicAura(Data {
                        static_data: self.static_data.clone(),
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
