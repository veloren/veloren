use crate::{
    combat,
    comp::{
        Body, CharacterState, LightEmitter, Pos, StateUpdate,
        ability::Amount,
        character_state::OutputEvents,
        object::Body::{GrenadeClay, LaserBeam, LaserBeamSmall},
        projectile::{ProjectileConstructor, aim_projectile},
    },
    event::{LocalEvent, ShootEvent},
    outcome::Outcome,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
};
use itertools::Either;
use rand::rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How much buildup is required before the attack
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// How much spread there is when more than 1 projectile is created
    pub projectile_spread: Option<ProjectileSpread>,
    /// Projectile variables
    pub projectile: ProjectileConstructor,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_speed: f32,
    /// How many projectiles are simultaneously fired
    pub num_projectiles: Amount,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Adjusts move speed during the attack per stage
    pub movement_modifier: MovementModifier,
    /// Adjusts turning rate during the attack per stage
    pub ori_modifier: OrientationModifier,
    /// Automatically aims to account for distance and elevation to target the
    /// selected pos
    pub auto_aim: bool,
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
    /// Whether the attack fired already
    pub exhausted: bool,
    /// Adjusts move speed during the attack
    pub movement_modifier: Option<f32>,
    /// How fast the entity should turn
    pub ori_modifier: Option<f32>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, self.ori_modifier.unwrap_or(1.0), None);
        handle_move(data, &mut update, self.movement_modifier.unwrap_or(0.7));
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    if let CharacterState::BasicRanged(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                    match self.static_data.projectile_body {
                        Body::Object(LaserBeam) => {
                            // Send local event used for frontend shenanigans
                            output_events.emit_local(LocalEvent::CreateOutcome(
                                Outcome::CyclopsCharge {
                                    pos: data.pos.0
                                        + *data.ori.look_dir() * (data.body.max_radius()),
                                },
                            ));
                        },
                        Body::Object(GrenadeClay) => {
                            // Send local event used for frontend shenanigans
                            output_events.emit_local(LocalEvent::CreateOutcome(
                                Outcome::FuseCharge {
                                    pos: data.pos.0
                                        + *data.ori.look_dir() * (2.5 * data.body.max_radius()),
                                },
                            ));
                        },
                        Body::Object(LaserBeamSmall) => {
                            output_events.emit_local(LocalEvent::CreateOutcome(
                                Outcome::TerracottaStatueCharge {
                                    pos: data.pos.0
                                        + *data.ori.look_dir() * (data.body.max_radius()),
                                },
                            ));
                        },
                        _ => {},
                    }
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::BasicRanged(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                        c.movement_modifier = c.static_data.movement_modifier.recover;
                        c.ori_modifier = c.static_data.ori_modifier.recover;
                    }
                }
            },
            StageSection::Recover => {
                if !self.exhausted {
                    // Fire
                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    let projectile = self.static_data.projectile.clone().create_projectile(
                        Some(*data.uid),
                        precision_mult,
                        Some(self.static_data.ability_info),
                    );
                    // Shoots all projectiles simultaneously
                    let num_projectiles = self
                        .static_data
                        .num_projectiles
                        .compute(data.heads.map_or(1, |heads| heads.amount() as u32));

                    let mut rng = rng();

                    let aim_dir = if self.static_data.ori_modifier.buildup.is_some() {
                        data.inputs.look_dir.merge_z(data.ori.look_dir())
                    } else {
                        data.inputs.look_dir
                    };

                    // Gets offsets
                    let body_offsets = data
                        .body
                        .projectile_offsets(update.ori.look_vec(), data.scale.map_or(1.0, |s| s.0));
                    let pos = Pos(data.pos.0 + body_offsets);

                    let aim_dir = if self.static_data.auto_aim
                        && let Some(sel_pos) = self
                            .static_data
                            .ability_info
                            .input_attr
                            .and_then(|ia| ia.select_pos)
                    {
                        if let Some(ideal_dir) =
                            aim_projectile(self.static_data.projectile_speed, pos.0, sel_pos, true)
                        {
                            ideal_dir.merge_z(aim_dir)
                        } else {
                            aim_dir
                        }
                    } else {
                        aim_dir
                    };

                    let dirs = if let Some(spread) = self.static_data.projectile_spread {
                        Either::Left(spread.compute_directions(
                            aim_dir,
                            *data.ori,
                            num_projectiles,
                            &mut rng,
                        ))
                    } else {
                        Either::Right((0..num_projectiles).map(|_| aim_dir))
                    };

                    for dir in dirs {
                        // Tells server to create and shoot the projectile
                        output_events.emit_server(ShootEvent {
                            entity: Some(data.entity),
                            source_vel: Some(*data.vel),
                            pos,
                            dir,
                            body: self.static_data.projectile_body,
                            projectile: projectile.clone(),
                            light: self.static_data.projectile_light,
                            speed: self.static_data.projectile_speed,
                            object: None,
                            marker: None,
                        });
                    }

                    if let CharacterState::BasicRanged(c) = &mut update.character {
                        c.exhausted = true;
                    }
                } else if self.timer < self.static_data.recover_duration {
                    // Recovers
                    if let CharacterState::BasicRanged(c) = &mut update.character {
                        c.timer = tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        );
                    }
                } else {
                    // Done
                    if input_is_pressed(data, self.static_data.ability_info.input) {
                        reset_state(self, data, output_events, &mut update);
                    } else {
                        end_ability(data, &mut update);
                    }
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

fn reset_state(
    data: &Data,
    join: &JoinData,
    output_events: &mut OutputEvents,
    update: &mut StateUpdate,
) {
    handle_input(
        join,
        output_events,
        update,
        data.static_data.ability_info.input,
    );
}
