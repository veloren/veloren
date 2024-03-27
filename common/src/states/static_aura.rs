use crate::{
    combat::GroupTarget,
    comp::{
        aura::{AuraBuffConstructor, AuraTarget, Auras},
        character_state::OutputEvents,
        CharacterState, StateUpdate,
    },
    event::CreateAuraEntityEvent,
    resources::Secs,
    states::{
        behavior::{CharacterBehavior, JoinData},
        sprite_summon::create_sprites,
        utils::*,
    },
    terrain::SpriteKind,
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
    /// Information about sprites if the state should create sprites
    pub sprite_info: Option<SpriteInfo>,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
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
    /// If creates sprites, what radius has been achieved so far
    pub achieved_radius: Option<i32>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::StaticAura(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Build up
                    update.character = CharacterState::StaticAura(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        achieved_radius: self.achieved_radius,
                    });
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.cast_duration {
                    // If creates sprites, create sprites
                    let achieved_radius = if let Some(sprite_info) = self.static_data.sprite_info {
                        let timer_frac =
                            self.timer.as_secs_f32() / self.static_data.cast_duration.as_secs_f32();

                        let achieved_radius = create_sprites(
                            data,
                            output_events,
                            sprite_info.sprite,
                            timer_frac,
                            sprite_info.summon_distance,
                            self.achieved_radius.unwrap_or(0),
                            360.0,
                            sprite_info.sparseness,
                            data.pos.0,
                            false,
                            sprite_info.del_timeout,
                        );
                        Some(achieved_radius)
                    } else {
                        None
                    };
                    // Cast
                    update.character = CharacterState::StaticAura(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, None),
                        achieved_radius,
                        ..*self
                    });
                } else {
                    // Creates aura
                    let targets =
                        AuraTarget::from((Some(self.static_data.targets), Some(data.uid)));
                    let mut auras = Vec::new();
                    for aura_data in &self.static_data.auras {
                        let aura = aura_data.to_aura(
                            data.uid,
                            self.static_data.range,
                            self.static_data.aura_duration,
                            targets,
                            *data.time,
                        );
                        auras.push(aura);
                    }
                    output_events.emit_server(CreateAuraEntityEvent {
                        auras: Auras::new(auras),
                        pos: *data.pos,
                        creator_uid: *data.uid,
                    });
                    update.character = CharacterState::StaticAura(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        achieved_radius: self.achieved_radius,
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    update.character = CharacterState::StaticAura(Data {
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

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpriteInfo {
    pub sprite: SpriteKind,
    pub del_timeout: Option<(f32, f32)>,
    pub summon_distance: (f32, f32),
    pub sparseness: f64,
}
