use crate::{
    comp::{
        projectile, Body, CharacterState, EnergySource, Gravity, LightEmitter, Projectile,
        StateUpdate,
    },
    event::ServerEvent,
    states::utils::*,
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MAX_GRAVITY: f32 = 0.2;
const MIN_GRAVITY: f32 = 0.05;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Whether the attack fired already
    pub exhausted: bool,
    /// How much energy is drained per second when charging
    pub energy_drain: u32,
    /// How much damage is dealt with no charge
    pub initial_damage: u32,
    /// How much damage is dealt with max charge
    pub max_damage: u32,
    /// How much knockback there is with no chage
    pub initial_knockback: f32,
    /// How much knockback there is at max charge
    pub max_knockback: f32,
    /// How long the weapon needs to be prepared for
    pub prepare_duration: Duration,
    /// How long it takes to charge the weapon to max damage and knockback
    pub charge_duration: Duration,
    /// How long the state has been charging
    pub charge_timer: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Projectile information
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.3);
        handle_jump(data, &mut update);

        if self.prepare_duration != Duration::default() {
            // Prepare (draw the bow)
            update.character = CharacterState::ChargedRanged(Data {
                exhausted: self.exhausted,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self
                    .prepare_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                charge_duration: self.charge_duration,
                charge_timer: self.charge_timer,
                recover_duration: self.recover_duration,
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
            });
        } else if data.inputs.secondary.is_pressed()
            && self.charge_timer < self.charge_duration
            && update.energy.current() > 0
        {
            // Charge the bow
            update.character = CharacterState::ChargedRanged(Data {
                exhausted: self.exhausted,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self.prepare_duration,
                charge_timer: self
                    .charge_timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                charge_duration: self.charge_duration,
                recover_duration: self.recover_duration,
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
            });

            // Consumes energy if there's enough left and RMB is held down
            update.energy.change_by(
                -(self.energy_drain as f32 * data.dt.0) as i32,
                EnergySource::Ability,
            );
        } else if data.inputs.secondary.is_pressed() {
            // Charge the bow
            update.character = CharacterState::ChargedRanged(Data {
                exhausted: self.exhausted,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self.prepare_duration,
                charge_timer: self.charge_timer,
                charge_duration: self.charge_duration,
                recover_duration: self.recover_duration,
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
            });

            // Consumes energy if there's enough left and RMB is held down
            update.energy.change_by(
                -(self.energy_drain as f32 * data.dt.0 / 5.0) as i32,
                EnergySource::Ability,
            );
        } else if !self.exhausted {
            let charge_amount =
                (self.charge_timer.as_secs_f32() / self.charge_duration.as_secs_f32()).min(1.0);
            // Fire
            let mut projectile = Projectile {
                hit_solid: vec![projectile::Effect::Stick],
                hit_entity: vec![
                    projectile::Effect::Damage(
                        -(self.initial_damage as i32
                            + (charge_amount * (self.max_damage - self.initial_damage) as f32)
                                as i32),
                    ),
                    projectile::Effect::Knockback(
                        self.initial_knockback
                            + charge_amount * (self.max_knockback - self.initial_knockback),
                    ),
                    projectile::Effect::Vanish,
                ],
                time_left: Duration::from_secs(15),
                owner: None,
            };
            projectile.owner = Some(*data.uid);
            update.server_events.push_front(ServerEvent::Shoot {
                entity: data.entity,
                dir: data.inputs.look_dir,
                body: self.projectile_body,
                projectile,
                light: self.projectile_light,
                gravity: Some(Gravity(
                    MAX_GRAVITY - charge_amount * (MAX_GRAVITY - MIN_GRAVITY),
                )),
            });

            update.character = CharacterState::ChargedRanged(Data {
                exhausted: true,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self.prepare_duration,
                charge_timer: self.charge_timer,
                charge_duration: self.charge_duration,
                recover_duration: self.recover_duration,
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
            });
        } else if self.recover_duration != Duration::default() {
            // Recovery
            update.character = CharacterState::ChargedRanged(Data {
                exhausted: self.exhausted,
                energy_drain: self.energy_drain,
                initial_damage: self.initial_damage,
                max_damage: self.max_damage,
                initial_knockback: self.initial_knockback,
                max_knockback: self.max_knockback,
                prepare_duration: self.prepare_duration,
                charge_timer: self.charge_timer,
                charge_duration: self.charge_duration,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
            });
        } else {
            // Done
            update.character = CharacterState::Wielding;
        }

        update
    }
}
