use crate::{
    comp::{beam, Body, CharacterState, EnergyChange, EnergySource, Ori, Pos, StateUpdate},
    event::ServerEvent,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    uid::Uid,
    Damage, DamageSource, GroupTarget,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should deal damage or heal
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// How long each beam segment persists for
    pub beam_duration: Duration,
    /// Base healing per second
    pub base_hps: u32,
    /// Base damage per second
    pub base_dps: u32,
    /// Ticks of damage/healing per second
    pub tick_rate: f32,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// Lifesteal efficiency (0 gives 0% conversion of damage to health, 1 gives
    /// 100% conversion of damage to health)
    pub lifesteal_eff: f32,
    /// Energy regened per second for damage ticks
    pub energy_regen: u32,
    /// Energy consumed per second for heal ticks
    pub energy_cost: u32,
    /// Energy drained per
    pub energy_drain: u32,
    /// What key is used to press ability
    pub ability_key: AbilityKey,
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
    /// Used for particle stuffs
    pub particle_ori: Option<Vec3<f32>>,
    /// Used to offset beam and particles
    pub offset: Vec3<f32>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.4);
        handle_jump(data, &mut update);
        if !ability_key_is_pressed(data, self.static_data.ability_key) {
            handle_interrupt(data, &mut update, false);
            match update.character {
                CharacterState::BasicBeam(_) => {},
                _ => {
                    return update;
                },
            }
        }

        if unwrap_tool_data(data).is_none() {
            update.character = CharacterState::Idle;
            return update;
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicBeam(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        particle_ori: Some(*data.inputs.look_dir),
                        ..*self
                    });
                } else {
                    // Creates beam
                    data.updater.insert(data.entity, beam::Beam {
                        hit_entities: Vec::<Uid>::new(),
                        tick_dur: Duration::from_secs_f32(1.0 / self.static_data.tick_rate),
                        timer: Duration::default(),
                    });
                    // Gets offsets
                    let body_offsets = Vec3::new(
                        (data.body.radius() + 1.0) * data.inputs.look_dir.x,
                        (data.body.radius() + 1.0) * data.inputs.look_dir.y,
                        data.body.eye_height(),
                    ) * 0.55;
                    // Build up
                    update.character = CharacterState::BasicBeam(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Cast,
                        particle_ori: Some(*data.inputs.look_dir),
                        offset: body_offsets,
                        ..*self
                    });
                }
            },
            StageSection::Cast => {
                if ability_key_is_pressed(data, self.static_data.ability_key)
                    && (self.static_data.energy_drain == 0 || update.energy.current() > 0)
                {
                    let damage = Damage {
                        source: DamageSource::Energy,
                        value: self.static_data.base_dps as f32 / self.static_data.tick_rate,
                    };
                    let heal = Damage {
                        source: DamageSource::Healing,
                        value: self.static_data.base_hps as f32 / self.static_data.tick_rate,
                    };
                    let speed =
                        self.static_data.range / self.static_data.beam_duration.as_secs_f32();
                    let properties = beam::Properties {
                        angle: self.static_data.max_angle.to_radians(),
                        speed,
                        damages: vec![
                            (Some(GroupTarget::OutOfGroup), damage),
                            (Some(GroupTarget::InGroup), heal),
                        ],
                        lifesteal_eff: self.static_data.lifesteal_eff,
                        energy_regen: self.static_data.energy_regen,
                        energy_cost: self.static_data.energy_cost,
                        duration: self.static_data.beam_duration,
                        owner: Some(*data.uid),
                    };
                    // Gets offsets
                    let body_offsets = match data.body {
                        Body::Humanoid(_) => {
                            Vec3::new(
                                data.body.radius() + 2.0 * data.inputs.look_dir.x,
                                data.body.radius() + 2.0 * data.inputs.look_dir.y,
                                data.body.eye_height(),
                            ) * 0.55
                        },
                        _ => {
                            Vec3::new(
                                data.body.radius() * 3.0 * data.inputs.look_dir.x,
                                data.body.radius() * 3.0 * data.inputs.look_dir.y,
                                data.body.eye_height(),
                            ) * 0.55
                        },
                    };
                    let pos = Pos(data.pos.0 + body_offsets);
                    // Create beam segment
                    update.server_events.push_front(ServerEvent::BeamSegment {
                        properties,
                        pos,
                        ori: Ori(data.inputs.look_dir),
                    });
                    update.character = CharacterState::BasicBeam(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        particle_ori: Some(*data.inputs.look_dir),
                        offset: body_offsets,
                        ..*self
                    });

                    // Consumes energy if there's enough left and ability key is held down
                    update.energy.change_by(EnergyChange {
                        amount: -(self.static_data.energy_drain as f32 * data.dt.0) as i32,
                        source: EnergySource::Ability,
                    });
                } else {
                    update.character = CharacterState::BasicBeam(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        particle_ori: Some(*data.inputs.look_dir),
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    update.character = CharacterState::BasicBeam(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        particle_ori: Some(*data.inputs.look_dir),
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
}
