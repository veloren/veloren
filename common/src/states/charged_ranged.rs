use crate::{
    comp::{
        projectile, Body, CharacterState, EnergySource, Gravity, LightEmitter, Projectile,
        StateUpdate,
    },
    event::ServerEvent,
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
    Damage, Damages,
};
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
    pub energy_drain: u32,
    /// How much damage is dealt with no charge
    pub initial_damage: u32,
    /// How much damage is dealt with max charge
    pub max_damage: u32,
    /// How much knockback there is with no charge
    pub initial_knockback: f32,
    /// How much knockback there is at max charge
    pub max_knockback: f32,
    /// Projectile information
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_gravity: Option<Gravity>,
    pub initial_projectile_speed: f32,
    pub max_projectile_speed: f32,
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

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.3);
        handle_jump(data, &mut update);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::ChargedRanged(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::ChargedRanged(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Charge,
                        exhausted: self.exhausted,
                    });
                }
            },
            StageSection::Charge => {
                if !data.inputs.secondary.is_pressed() && !self.exhausted {
                    let charge_frac = (self.timer.as_secs_f32()
                        / self.static_data.charge_duration.as_secs_f32())
                    .min(1.0);
                    let damage = self.static_data.initial_damage as f32
                        + (charge_frac
                            * (self.static_data.max_damage - self.static_data.initial_damage)
                                as f32);
                    let knockback = self.static_data.initial_knockback as f32
                        + (charge_frac
                            * (self.static_data.max_knockback - self.static_data.initial_knockback)
                                as f32);
                    // Fire
                    let mut projectile = Projectile {
                        hit_solid: vec![projectile::Effect::Stick],
                        hit_entity: vec![
                            projectile::Effect::Damages(Damages::new(
                                Some(Damage::Projectile(damage)),
                                None,
                            )),
                            projectile::Effect::Knockback(knockback),
                            projectile::Effect::Vanish,
                        ],
                        time_left: Duration::from_secs(15),
                        owner: None,
                        ignore_group: true,
                    };
                    projectile.owner = Some(*data.uid);
                    update.server_events.push_front(ServerEvent::Shoot {
                        entity: data.entity,
                        dir: data.inputs.look_dir,
                        body: self.static_data.projectile_body,
                        projectile,
                        light: self.static_data.projectile_light,
                        gravity: self.static_data.projectile_gravity,
                        speed: self.static_data.initial_projectile_speed
                            + charge_frac
                                * (self.static_data.max_projectile_speed
                                    - self.static_data.initial_projectile_speed),
                    });

                    update.character = CharacterState::ChargedRanged(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        exhausted: true,
                    });
                } else if self.timer < self.static_data.charge_duration
                    && data.inputs.secondary.is_pressed()
                {
                    // Charges
                    update.character = CharacterState::ChargedRanged(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update.energy.change_by(
                        -(self.static_data.energy_drain as f32 * data.dt.0) as i32,
                        EnergySource::Ability,
                    );
                } else if data.inputs.secondary.is_pressed() {
                    // Holds charge
                    update.character = CharacterState::ChargedRanged(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update.energy.change_by(
                        -(self.static_data.energy_drain as f32 * data.dt.0 / 5.0) as i32,
                        EnergySource::Ability,
                    );
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::ChargedRanged(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
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
