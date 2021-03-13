use crate::{
    combat::{Attack, AttackEffect, CombatEffect, CombatRequirement, GroupTarget},
    comp::{beam, CharacterState, InputKind, Ori, Pos, StateUpdate},
    event::ServerEvent,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::*;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should deal damage or heal
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// How long each beam segment persists for
    pub beam_duration: Duration,
    /// Base healing per tick
    pub heal: f32,
    /// Ticks of healing per second
    pub tick_rate: f32,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// Energy consumed per second for heal ticks
    pub energy_cost: f32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Used to specify the beam to the frontend
    pub specifier: beam::FrontendSpecifier,
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
    /// Whether or not the state should end
    pub end: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.4);
        handle_jump(data, &mut update);
        if !ability_key_is_pressed(data, self.static_data.ability_info.key) {
            handle_interrupt(data, &mut update, false);
            match update.character {
                CharacterState::HealingBeam(_) => {},
                _ => {
                    return update;
                },
            }
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::HealingBeam(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Creates beam
                    data.updater.insert(data.entity, beam::Beam {
                        hit_entities: Vec::<Uid>::new(),
                        tick_dur: Duration::from_secs_f32(1.0 / self.static_data.tick_rate),
                        timer: Duration::default(),
                    });
                    // Build up
                    update.character = CharacterState::HealingBeam(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Cast,
                        ..*self
                    });
                }
            },
            StageSection::Cast => {
                if
                /* ability_key_is_pressed(data, self.static_data.ability_info.key) */
                !self.end {
                    let speed =
                        self.static_data.range / self.static_data.beam_duration.as_secs_f32();
                    let heal = AttackEffect::new(
                        Some(GroupTarget::InGroup),
                        CombatEffect::Heal(self.static_data.heal),
                    )
                    .with_requirement(CombatRequirement::Energy(self.static_data.energy_cost))
                    .with_requirement(CombatRequirement::Combo(1));
                    let attack = Attack::default().with_effect(heal);

                    let properties = beam::Properties {
                        attack,
                        angle: self.static_data.max_angle.to_radians(),
                        speed,
                        duration: self.static_data.beam_duration,
                        owner: Some(*data.uid),
                        specifier: self.static_data.specifier,
                    };
                    // Gets offsets
                    let body_offsets = Vec3::new(
                        (data.body.radius() + 1.0) * data.inputs.look_dir.x,
                        (data.body.radius() + 1.0) * data.inputs.look_dir.y,
                        data.body.eye_height() * 0.6,
                    );
                    let pos = Pos(data.pos.0 + body_offsets);
                    // Create beam segment
                    update.server_events.push_front(ServerEvent::BeamSegment {
                        properties,
                        pos,
                        ori: Ori::from(data.inputs.look_dir),
                    });
                    update.character = CharacterState::HealingBeam(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    update.character = CharacterState::HealingBeam(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    update.character = CharacterState::HealingBeam(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                    // Make sure attack component is removed
                    data.updater.remove::<beam::Beam>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<beam::Beam>(data.entity);
            },
        }

        update
    }

    fn cancel_input(&self, data: &JoinData, input: InputKind) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.removed_inputs.push(input);

        if Some(input) == self.static_data.ability_info.input {
            if let CharacterState::HealingBeam(c) = &mut update.character {
                c.end = true;
            }
        }

        update
    }
}
