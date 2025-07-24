use crate::{
    combat,
    comp::{
        Body, CharacterState, LightEmitter, Pos, StateUpdate, ability::Amount,
        character_state::OutputEvents, projectile::ProjectileConstructor,
    },
    event::ShootEvent,
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
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long the weapon needs to be prepared for
    pub buildup_duration: Duration,
    /// How long it takes to charge the weapon to max damage and knockback
    pub charge_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// How much energy is drained per second when charging
    pub energy_drain: f32,
    /// Energy per second drain after ful charge is reached
    pub idle_drain: f32,
    /// Projectile information
    pub projectile: ProjectileConstructor,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub initial_projectile_speed: f32,
    pub scaled_projectile_speed: f32,
    pub projectile_spread: Option<ProjectileSpread>,
    pub num_projectiles: Amount,
    /// Move speed efficiency
    pub move_speed: f32,
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
    /// Whether the attack fired already
    pub exhausted: bool,
}

impl Data {
    /// How complete the charge is, on a scale of 0.0 to 1.0
    pub fn charge_frac(&self) -> f32 {
        if let StageSection::Charge = self.stage_section {
            (self.timer.as_secs_f32() / self.static_data.charge_duration.as_secs_f32()).min(1.0)
        } else {
            0.0
        }
    }
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, self.static_data.move_speed);
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::ChargedRanged(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::ChargedRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Charge,
                        ..*self
                    });
                }
            },
            StageSection::Charge => {
                if !input_is_pressed(data, self.static_data.ability_info.input) && !self.exhausted {
                    let charge_frac = self.charge_frac();
                    // Fire
                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    // Gets offsets
                    let body_offsets = data
                        .body
                        .projectile_offsets(update.ori.look_vec(), data.scale.map_or(1.0, |s| s.0));
                    let pos = Pos(data.pos.0 + body_offsets);
                    let projectile = self
                        .static_data
                        .projectile
                        .handle_scaling(charge_frac)
                        .create_projectile(
                            Some(*data.uid),
                            precision_mult,
                            data.stats,
                            Some(self.static_data.ability_info),
                        );

                    dbg!(self.static_data.ability_info.input_attr);

                    let num_projectiles = self
                        .static_data
                        .num_projectiles
                        .compute(data.heads.map_or(1, |heads| heads.amount() as u32));

                    let mut rng = rng();
                    let dirs = if let Some(spread) = self.static_data.projectile_spread {
                        Either::Left(spread.compute_directions(
                            data.inputs.look_dir,
                            *data.ori,
                            num_projectiles,
                            &mut rng,
                        ))
                    } else {
                        Either::Right((0..num_projectiles).map(|_| data.inputs.look_dir))
                    };

                    for dir in dirs {
                        output_events.emit_server(ShootEvent {
                            entity: Some(data.entity),
                            pos,
                            dir,
                            body: self.static_data.projectile_body,
                            projectile: projectile.clone(),
                            light: self.static_data.projectile_light,
                            speed: self.static_data.initial_projectile_speed
                                + charge_frac * self.static_data.scaled_projectile_speed,
                            object: None,
                        });
                    }

                    update.character = CharacterState::ChargedRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        exhausted: true,
                        ..*self
                    });
                } else if self.timer < self.static_data.charge_duration
                    && input_is_pressed(data, self.static_data.ability_info.input)
                {
                    // Charges
                    update.character = CharacterState::ChargedRanged(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });

                    // Consumes energy if there's enough left and input is held down
                    update
                        .energy
                        .change_by(-self.static_data.energy_drain * data.dt.0);
                } else if input_is_pressed(data, self.static_data.ability_info.input) {
                    // Holds charge
                    update.character = CharacterState::ChargedRanged(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update
                        .energy
                        .change_by(-self.static_data.idle_drain * data.dt.0);
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::ChargedRanged(Data {
                        timer: tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        ),
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
