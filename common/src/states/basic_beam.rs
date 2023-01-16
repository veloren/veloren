use crate::{
    combat::{
        Attack, AttackDamage, AttackEffect, CombatEffect, CombatRequirement, Damage, DamageKind,
        DamageSource, GroupTarget,
    },
    comp::{
        beam, body::biped_large, character_state::OutputEvents, Body, CharacterState, Ori, Pos,
        StateUpdate,
    },
    event::ServerEvent,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    terrain::Block,
    uid::Uid,
    util::Dir,
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
    /// Base damage per tick
    pub damage: f32,
    /// Ticks per second
    pub tick_rate: f32,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// Adds an effect onto the main damage of the attack
    pub damage_effect: Option<CombatEffect>,
    /// Energy regenerated per tick
    pub energy_regen: f32,
    /// Energy drained per second
    pub energy_drain: f32,
    /// How fast enemy can rotate with beam
    pub ori_rate: f32,
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
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        let ori_rate = self.static_data.ori_rate;

        handle_orientation(data, &mut update, ori_rate, None);
        handle_move(data, &mut update, 0.4);
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicBeam(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
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
                    update.character = CharacterState::BasicBeam(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if input_is_pressed(data, self.static_data.ability_info.input)
                    && (self.static_data.energy_drain <= f32::EPSILON
                        || update.energy.current() > 0.0)
                {
                    let speed =
                        self.static_data.range / self.static_data.beam_duration.as_secs_f32();

                    let energy = AttackEffect::new(
                        None,
                        CombatEffect::EnergyReward(self.static_data.energy_regen),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let mut damage = AttackDamage::new(
                        Damage {
                            source: DamageSource::Energy,
                            kind: DamageKind::Energy,
                            value: self.static_data.damage,
                        },
                        Some(GroupTarget::OutOfGroup),
                        rand::random(),
                    );
                    if let Some(effect) = self.static_data.damage_effect {
                        damage = damage.with_effect(effect);
                    }
                    let (crit_chance, crit_mult) =
                        get_crit_data(data, self.static_data.ability_info);
                    let attack = Attack::default()
                        .with_damage(damage)
                        .with_crit(crit_chance, crit_mult)
                        .with_effect(energy)
                        .with_combo_increment();

                    let properties = beam::Properties {
                        attack,
                        angle: self.static_data.max_angle.to_radians(),
                        speed,
                        duration: self.static_data.beam_duration,
                        owner: Some(*data.uid),
                        specifier: self.static_data.specifier,
                    };
                    let beam_ori = {
                        // We want Beam to use Ori of owner.
                        // But we also want beam to use Z part of where owner looks.
                        // This means that we need to merge this data to one Ori.
                        //
                        // This code just gets look_dir without Z part
                        // and normalizes it. This is what `xy_dir is`.
                        //
                        // Then we find rotation between xy_dir and look_dir
                        // which gives us quaternion how of what rotation we need
                        // to do to get Z part we want.
                        //
                        // Then we construct Ori without Z part
                        // and applying `pitch` to get needed orientation.
                        let look_dir = data.inputs.look_dir;
                        let xy_dir = Dir::from_unnormalized(Vec3::new(look_dir.x, look_dir.y, 0.0))
                            .unwrap_or_default();
                        let pitch = xy_dir.rotation_between(look_dir);

                        Ori::from(Vec3::new(
                            update.ori.look_vec().x,
                            update.ori.look_vec().y,
                            0.0,
                        ))
                        .prerotated(pitch)
                    };
                    // Velocity relative to the current ground
                    let rel_vel = data.vel.0 - data.physics.ground_vel;
                    // Gets offsets
                    let body_offsets = beam_offsets(
                        data.body,
                        data.inputs.look_dir,
                        update.ori.look_vec(),
                        rel_vel,
                        data.physics.on_ground,
                    );
                    let pos = Pos(data.pos.0 + body_offsets);

                    // Create beam segment
                    output_events.emit_server(ServerEvent::BeamSegment {
                        properties,
                        pos,
                        ori: beam_ori,
                    });
                    update.character = CharacterState::BasicBeam(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });

                    // Consumes energy if there's enough left and ability key is held down
                    update
                        .energy
                        .change_by(-self.static_data.energy_drain * data.dt.0);
                } else {
                    update.character = CharacterState::BasicBeam(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    update.character = CharacterState::BasicBeam(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Done
                    end_ability(data, &mut update);
                    // Make sure attack component is removed
                    data.updater.remove::<beam::Beam>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_ability(data, &mut update);
                // Make sure attack component is removed
                data.updater.remove::<beam::Beam>(data.entity);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        handle_interrupts(data, &mut update, output_events);

        update
    }
}

fn height_offset(body: &Body, look_dir: Dir, velocity: Vec3<f32>, on_ground: Option<Block>) -> f32 {
    match body {
        // Hack to make the beam offset correspond to the animation
        Body::BirdLarge(_) => {
            body.height() * 0.3
                + if on_ground.is_none() {
                    (2.0 - velocity.xy().magnitude() * 0.25).max(-1.0)
                } else {
                    0.0
                }
        },
        Body::Golem(_) => {
            const DIR_COEFF: f32 = 2.0;
            body.height() * 0.9 + look_dir.z * DIR_COEFF
        },
        Body::BipedLarge(b) => match b.species {
            biped_large::Species::Mindflayer => body.height() * 0.6,
            _ => body.height() * 0.5,
        },
        _ => body.height() * 0.5,
    }
}

pub fn beam_offsets(
    body: &Body,
    look_dir: Dir,
    ori: Vec3<f32>,
    velocity: Vec3<f32>,
    on_ground: Option<Block>,
) -> Vec3<f32> {
    let dim = body.dimensions();
    // The width (shoulder to shoulder) and length (nose to tail)
    let (width, length) = (dim.x, dim.y);
    let body_radius = if length > width {
        // Dachshund-like
        body.max_radius()
    } else {
        // Cyclops-like
        body.min_radius()
    };
    let body_offsets_z = height_offset(body, look_dir, velocity, on_ground);
    Vec3::new(
        body_radius * ori.x * 1.1,
        body_radius * ori.y * 1.1,
        body_offsets_z,
    )
}
