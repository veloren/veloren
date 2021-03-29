use crate::{
    comp::{
        self,
        inventory::loadout_builder::{LoadoutBuilder, LoadoutConfig},
        Behavior, CharacterState, StateUpdate,
    },
    event::{LocalEvent, ServerEvent},
    outcome::Outcome,
    skillset_builder::{SkillSetBuilder, SkillSetConfig},
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
    /// How long the state builds up for
    pub buildup_duration: Duration,
    /// How long the state is casting for
    pub cast_duration: Duration,
    /// How long the state recovers for
    pub recover_duration: Duration,
    /// How many creatures the state should summon
    pub summon_amount: u32,
    /// Information about the summoned creature
    pub summon_info: SummonInfo,
    /// Miscellaneous information about the ability
    pub ability_info: AbilityInfo,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// How many creatures have been summoned
    pub summon_count: u32,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicSummon(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::BasicSummon(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Cast,
                        ..*self
                    });
                }
            },
            StageSection::Cast => {
                if self.timer < self.static_data.cast_duration
                    || self.summon_count < self.static_data.summon_amount
                {
                    if self.timer
                        > self.static_data.cast_duration * self.summon_count
                            / self.static_data.summon_amount
                    {
                        let body = self.static_data.summon_info.body;

                        let loadout = LoadoutBuilder::build_loadout(
                            body,
                            None,
                            self.static_data.summon_info.loadout_config,
                            None,
                        )
                        .build();
                        let mut stats = comp::Stats::new("Summon".to_string());
                        stats.skill_set = SkillSetBuilder::build_skillset(
                            &None,
                            self.static_data.summon_info.skillset_config,
                        )
                        .build();

                        // Send server event to create npc
                        update.server_events.push_front(ServerEvent::CreateNpc {
                            pos: *data.pos,
                            stats,
                            health: comp::Health::new(
                                body,
                                self.static_data.summon_info.health_scaling,
                            ),
                            poise: comp::Poise::new(body),
                            loadout,
                            body,
                            agent: Some(comp::Agent::new(None, None, &body, true)),
                            behavior: Some(Behavior::new(true, false)),
                            alignment: comp::Alignment::Owned(*data.uid),
                            scale: self
                                .static_data
                                .summon_info
                                .scale
                                .unwrap_or(comp::Scale(1.0)),
                            home_chunk: None,
                            drop_item: None,
                            rtsim_entity: None,
                        });

                        // Send local event used for frontend shenanigans
                        update.local_events.push_front(LocalEvent::CreateOutcome(
                            Outcome::SummonedCreature {
                                pos: data.pos.0,
                                body,
                            },
                        ));

                        update.character = CharacterState::BasicSummon(Data {
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(data.dt.0))
                                .unwrap_or_default(),
                            summon_count: self.summon_count + 1,
                            ..*self
                        });
                    } else {
                        // Cast
                        update.character = CharacterState::BasicSummon(Data {
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(data.dt.0))
                                .unwrap_or_default(),
                            ..*self
                        });
                    }
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::BasicSummon(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    update.character = CharacterState::BasicSummon(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
            },
        }

        update
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SummonInfo {
    body: comp::Body,
    scale: Option<comp::Scale>,
    health_scaling: u16,
    loadout_config: Option<LoadoutConfig>,
    skillset_config: Option<SkillSetConfig>,
}
