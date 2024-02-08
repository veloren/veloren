use crate::{
    combat::{self, CombatEffect},
    comp::{
        character_state::OutputEvents, Body, CharacterState, LightEmitter, Pos,
        ProjectileConstructor, StateUpdate,
    },
    event::{EnergyChangeEvent, ShootEvent},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
    },
    util::Dir,
};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{f32::consts::TAU, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Max speed that can be reached
    pub max_speed: f32,
    /// Projectiles required to reach half of max speed
    pub half_speed_at: u32,
    /// Projectile options
    pub projectile: ProjectileConstructor,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_speed: f32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Adds an effect onto the main damage of the attack
    pub damage_effect: Option<CombatEffect>,
    /// Whether ablity should be casted from above as aoe or shoot projectiles
    /// as normal
    pub properties_of_aoe: Option<ProjectileOffset>,
    /// Used to specify the attack to the frontend
    pub specifier: Option<FrontendSpecifier>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectileOffset {
    /// Radius of AOE
    pub radius: f32,
    /// Height of shooting point for AOE's projectiles
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
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transition to shoot
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.shoot_duration {
                    // Draw projectile
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0 * self.speed))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else if input_is_pressed(data, self.static_data.ability_info.input)
                    && update.energy.current() >= self.static_data.energy_cost
                {
                    // Fire if input is pressed still
                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    let tool_stats = get_tool_stats(data, self.static_data.ability_info);
                    // Gets offsets
                    let pos: Pos = self.static_data.properties_of_aoe.as_ref().map_or_else(
                        || {
                            // Default position
                            let body_offsets = data.body.projectile_offsets(
                                update.ori.look_vec(),
                                data.scale.map_or(1.0, |s| s.0),
                            );
                            Pos(data.pos.0 + body_offsets)
                        },
                        |aoe_data| {
                            // Position calculated from aoe_data
                            let rand_pos = {
                                let mut rng = thread_rng();
                                let theta = rng.gen::<f32>() * TAU;
                                let radius = aoe_data.radius * rng.gen::<f32>().sqrt();
                                let x = radius * theta.sin();
                                let y = radius * theta.cos();
                                vek::Vec2::new(x, y)
                            };
                            Pos(data.pos.0 + rand_pos.with_z(aoe_data.height))
                        },
                    );

                    let direction: Dir = if self.static_data.properties_of_aoe.is_some() {
                        Dir::down()
                    } else {
                        data.inputs.look_dir
                    };

                    let projectile = self.static_data.projectile.create_projectile(
                        Some(*data.uid),
                        precision_mult,
                        tool_stats,
                        self.static_data.damage_effect,
                    );
                    output_events.emit_server(ShootEvent {
                        entity: data.entity,
                        pos,
                        dir: direction,
                        body: self.static_data.projectile_body,
                        projectile,
                        light: self.static_data.projectile_light,
                        speed: self.static_data.projectile_speed,
                        object: None,
                    });

                    // Removes energy from character when arrow is fired
                    output_events.emit_server(EnergyChangeEvent {
                        entity: data.entity,
                        change: -self.static_data.energy_cost,
                    });

                    // Sets new speed of shoot. Scales based off of the number of projectiles fired.
                    let new_speed = 1.0
                        + self.projectiles_fired as f32
                            / (self.static_data.half_speed_at as f32
                                + self.projectiles_fired as f32)
                            * self.static_data.max_speed;

                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: Duration::default(),
                        speed: new_speed,
                        projectiles_fired: self.projectiles_fired + 1,
                        ..*self
                    });
                } else {
                    // Transition to recover
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recover from attack
                    update.character = CharacterState::RepeaterRanged(Data {
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    FireRain,
}
