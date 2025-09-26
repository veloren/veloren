use crate::{
    combat,
    comp::{
        Body, CharacterState, LightEmitter, Pos, ProjectileConstructor, StateUpdate,
        character_state::OutputEvents,
    },
    event::{EnergyChangeEvent, ShootEvent},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
    },
    util::Dir,
};
use rand::{Rng, rng};
use serde::{Deserialize, Serialize};
use std::{f32::consts::TAU, time::Duration};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
    /// How long we've readied the weapon
    pub buildup_duration: Duration,
    /// How long the state is shooting
    pub shoot_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Energy cost per projectile
    pub energy_cost: f32,
    #[serde(default)]
    pub options: Options,
    /// Projectile options
    pub projectile: ProjectileConstructor,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_speed: f32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Used to specify the attack to the frontend
    pub specifier: Option<FrontendSpecifier>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Options {
    pub speed_ramp: Option<RampOptions>,
    pub max_projectiles: Option<u32>,
    pub offset: Option<OffsetOptions>,
    #[serde(default)]
    pub fire_all: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RampOptions {
    /// Max bonus to speed that can be reached
    pub max_bonus: f32,
    /// Projectiles required to reach half of max speed
    pub half_speed_at: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OffsetOptions {
    pub radius: f32,
    pub height: f32,
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
    /// Speed of the state while in shoot section
    pub speed: f32,
    /// Number of projectiles fired so far
    pub projectiles_fired: u32,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 0.3);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Buildup to attack
                    if let CharacterState::RapidRanged(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transition to shoot
                    if let CharacterState::RapidRanged(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Action;
                    }
                }
            },
            StageSection::Action => {
                // We want to ensure that we only "fire all" if there is a finite amount to fire
                let fire_all = self.static_data.options.fire_all
                    && self.static_data.options.max_projectiles.is_some();
                if self.timer < self.static_data.shoot_duration {
                    // Draw projectile
                    if let CharacterState::RapidRanged(c) = &mut update.character {
                        c.timer = self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0 * self.speed))
                            .unwrap_or_default();
                    }
                } else if (input_is_pressed(data, self.static_data.ability_info.input) || fire_all)
                    && update.energy.current() >= self.static_data.energy_cost
                    && self
                        .static_data
                        .options
                        .max_projectiles
                        .is_none_or(|max| self.projectiles_fired < max)
                {
                    // Fire if input is pressed still
                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    // Gets offsets
                    let offset = if let Some(offset) = self.static_data.options.offset {
                        let mut rng = rng();
                        let theta = rng.random::<f32>() * TAU;
                        let radius = offset.radius * rng.random::<f32>().sqrt();
                        let x = radius * theta.sin();
                        let y = radius * theta.cos();
                        let z = offset.height;
                        vek::Vec3::new(x, y, z)
                    } else {
                        data.body.projectile_offsets(
                            update.ori.look_vec(),
                            data.scale.map_or(1.0, |s| s.0),
                        )
                    };
                    let pos = Pos(data.pos.0 + offset);

                    let direction: Dir = if self.static_data.projectile_speed < 1.0 {
                        Dir::down()
                    } else {
                        data.inputs.look_dir
                    };

                    let projectile = self.static_data.projectile.clone().create_projectile(
                        Some(*data.uid),
                        precision_mult,
                        data.stats,
                        Some(self.static_data.ability_info),
                    );
                    output_events.emit_server(ShootEvent {
                        entity: Some(data.entity),
                        source_vel: Some(*data.vel),
                        pos,
                        dir: direction,
                        body: self.static_data.projectile_body,
                        projectile,
                        light: self.static_data.projectile_light,
                        speed: self.static_data.projectile_speed,
                        object: None,
                        marker: None,
                    });

                    // Removes energy from character when arrow is fired
                    output_events.emit_server(EnergyChangeEvent {
                        entity: data.entity,
                        change: -self.static_data.energy_cost,
                        reset_rate: false,
                    });

                    // Sets new speed of shoot. Scales based off of the number of projectiles fired
                    // if there is a speed ramp.
                    let new_speed = if let Some(speed_ramp) = self.static_data.options.speed_ramp {
                        1.0 + self.projectiles_fired as f32
                            / (speed_ramp.half_speed_at as f32 + self.projectiles_fired as f32)
                            * speed_ramp.max_bonus
                    } else {
                        1.0
                    };

                    if let CharacterState::RapidRanged(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.speed = new_speed;
                        c.projectiles_fired = self.projectiles_fired + 1;
                    }
                } else {
                    // Transition to recover
                    if let CharacterState::RapidRanged(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                    }
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recover from attack
                    if let CharacterState::RapidRanged(c) = &mut update.character {
                        c.timer = tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        );
                    }
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    FireRainPhoenix,
}
