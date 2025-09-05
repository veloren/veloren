use std::time::Duration;

use common_assets::{AssetExt, Ron};
use rand::rng;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{
    comp::{CharacterState, StateUpdate, item::Reagent},
    event::TransformEvent,
    generation::{EntityConfig, EntityInfo},
    states::utils::{end_ability, tick_attack_or_default},
};

use super::{
    behavior::CharacterBehavior,
    utils::{AbilityInfo, StageSection},
};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FrontendSpecifier {
    Evolve,
    Cursekeeper,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state has until transformation
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// The entity configuration you will be transformed into
    pub target: String,
    pub ability_info: AbilityInfo,
    /// Whether players are allowed to transform
    pub allow_players: bool,
    /// Used to specify the transformation to the frontend
    pub specifier: Option<FrontendSpecifier>,
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
    fn behavior(
        &self,
        data: &super::behavior::JoinData,
        output_events: &mut crate::comp::character_state::OutputEvents,
    ) -> crate::comp::StateUpdate {
        let mut update = StateUpdate::from(data);
        match self.stage_section {
            StageSection::Buildup => {
                // Tick the timer as long as buildup hasn't finihsed
                if self.timer < self.static_data.buildup_duration {
                    update.character = CharacterState::Transform(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                // Buildup finished, start transformation
                } else {
                    let Ok(entity_config) = Ron::<EntityConfig>::load(&self.static_data.target)
                    else {
                        error!(?self.static_data.target, "Failed to load entity configuration");
                        end_ability(data, &mut update);
                        return update;
                    };

                    let entity_info = EntityInfo::at(data.pos.0).with_entity_config(
                        entity_config.read().clone().into_inner(),
                        Some(&self.static_data.target),
                        &mut rng(),
                        None,
                    );

                    // Handle frontend events
                    if let Some(specifier) = self.static_data.specifier {
                        match specifier {
                            FrontendSpecifier::Evolve => {
                                output_events.emit_local(crate::event::LocalEvent::CreateOutcome(
                                    crate::outcome::Outcome::Explosion {
                                        pos: data.pos.0,
                                        power: 5.0,
                                        radius: 2.0,
                                        is_attack: false,
                                        reagent: Some(Reagent::White),
                                    },
                                ))
                            },
                            FrontendSpecifier::Cursekeeper => {
                                output_events.emit_local(crate::event::LocalEvent::CreateOutcome(
                                    crate::outcome::Outcome::Explosion {
                                        pos: data.pos.0,
                                        power: 5.0,
                                        radius: 2.0,
                                        is_attack: false,
                                        reagent: Some(Reagent::Purple),
                                    },
                                ))
                            },
                        }
                    }

                    output_events.emit_server(TransformEvent {
                        target_entity: *data.uid,
                        entity_info,
                        allow_players: self.static_data.allow_players,
                        delete_on_failure: false,
                    });
                    update.character = CharacterState::Transform(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                    });
                }
            },
            StageSection::Recover => {
                // Wait for recovery period to finish
                if self.timer < self.static_data.recover_duration {
                    update.character = CharacterState::Transform(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        ),
                        ..*self
                    });
                } else {
                    // End the ability after recovery is done
                    end_ability(data, &mut update);
                }
            },
            _ => {
                // If we somehow ended up in an incorrect character state, end the ability
                end_ability(data, &mut update);
            },
        }

        update
    }
}
