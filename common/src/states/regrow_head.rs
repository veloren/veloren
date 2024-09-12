use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    comp::{CharacterState, StateUpdate},
    event::RegrowHeadEvent,
    states::utils::{end_ability, tick_attack_or_default},
};

use super::{
    behavior::CharacterBehavior,
    utils::{AbilityInfo, StageSection},
};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FrontendSpecifier {
    Hydra,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state has until the head regrowing
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// How much energy this ability costs.
    pub energy_cost: f32,
    pub ability_info: AbilityInfo,
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
                    update.character = CharacterState::RegrowHead(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                // Buildup finished, regrow head
                } else {
                    output_events.emit_server(RegrowHeadEvent {
                        entity: data.entity,
                    });
                    update.character = CharacterState::RegrowHead(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                    });
                }
            },
            StageSection::Recover => {
                // Wait for recovery period to finish
                if self.timer < self.static_data.recover_duration {
                    update.character = CharacterState::RegrowHead(Data {
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
