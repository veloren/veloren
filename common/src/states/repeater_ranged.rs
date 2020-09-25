use crate::{
    comp::{Body, CharacterState, Gravity, LightEmitter, Projectile, StateUpdate},
    event::ServerEvent,
    states::utils::*,
    sys::character_behavior::*,
    util::dir::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// How long the state is moving
    pub movement_duration: Duration,
    /// Can you hold the ability beyond the prepare duration
    pub holdable: bool,
    /// How long we have to prepare the weapon
    pub prepare_duration: Duration,
    /// How long we prepared the weapon already
    pub prepare_timer: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    pub projectile: Projectile,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_gravity: Option<Gravity>,
    pub projectile_speed: f32,
    /// How many repetitions remaining
    pub reps_remaining: u32,
    /// Whether there should be a jump
    pub leap: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 1.0);
        handle_jump(data, &mut update);

        if self.reps_remaining > 0
            && if self.holdable {
                data.inputs.holding_ability_key() || self.prepare_timer < self.prepare_duration
            } else {
                self.prepare_timer < self.prepare_duration
            }
        {
            // Prepare (draw the bow)
            update.character = CharacterState::RepeaterRanged(Data {
                movement_duration: self.movement_duration,
                prepare_timer: self.prepare_timer + Duration::from_secs_f32(data.dt.0),
                holdable: self.holdable,
                prepare_duration: self.prepare_duration,
                recover_duration: self.recover_duration,
                projectile: self.projectile.clone(),
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
                projectile_gravity: self.projectile_gravity,
                projectile_speed: self.projectile_speed,
                reps_remaining: self.reps_remaining,
                leap: self.leap,
            });
        } else if self.movement_duration != Duration::default() {
            // Jumping
            if self.leap {
                update.vel.0 = Vec3::new(data.vel.0[0], data.vel.0[1], 10.0);
            }

            update.character = CharacterState::RepeaterRanged(Data {
                movement_duration: self
                    .movement_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                prepare_timer: self.prepare_timer,
                holdable: self.holdable,
                prepare_duration: self.prepare_duration,
                recover_duration: self.recover_duration,
                projectile: self.projectile.clone(),
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
                projectile_gravity: self.projectile_gravity,
                projectile_speed: self.projectile_speed,
                reps_remaining: self.reps_remaining,
                leap: self.leap,
            });
        } else if self.recover_duration != Duration::default() {
            // Hover
            update.vel.0 = Vec3::new(data.vel.0[0], data.vel.0[1], 0.0);

            // Recovery
            update.character = CharacterState::RepeaterRanged(Data {
                movement_duration: Duration::default(),
                prepare_timer: self.prepare_timer,
                holdable: self.holdable,
                prepare_duration: self.prepare_duration,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                projectile: self.projectile.clone(),
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
                projectile_gravity: self.projectile_gravity,
                projectile_speed: self.projectile_speed,
                reps_remaining: self.reps_remaining,
                leap: self.leap,
            });
        } else if self.reps_remaining > 0 {
            // Hover
            update.vel.0 = Vec3::new(data.vel.0[0], data.vel.0[1], 0.0);

            // Fire
            let mut projectile = self.projectile.clone();
            projectile.owner = Some(*data.uid);
            update.server_events.push_front(ServerEvent::Shoot {
                entity: data.entity,
                dir: Dir::from_unnormalized(Vec3::new(
                    data.inputs.look_dir[0]
                        + (if self.reps_remaining % 2 == 0 {
                            self.reps_remaining as f32 / 400.0
                        } else {
                            -1.0 * self.reps_remaining as f32 / 400.0
                        }),
                    data.inputs.look_dir[1]
                        + (if self.reps_remaining % 2 == 0 {
                            -1.0 * self.reps_remaining as f32 / 400.0
                        } else {
                            self.reps_remaining as f32 / 400.0
                        }),
                    data.inputs.look_dir[2],
                ))
                .expect("That didn't work"),
                body: self.projectile_body,
                projectile,
                light: self.projectile_light,
                gravity: self.projectile_gravity,
                speed: self.projectile_speed,
            });

            update.character = CharacterState::RepeaterRanged(Data {
                movement_duration: self.movement_duration,
                prepare_timer: self.prepare_timer,
                holdable: self.holdable,
                prepare_duration: self.prepare_duration,
                //recover_duration: self.recover_duration,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                projectile: self.projectile.clone(),
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
                projectile_gravity: self.projectile_gravity,
                projectile_speed: self.projectile_speed,
                reps_remaining: self.reps_remaining - 1,
                leap: self.leap,
            });
            return update;
        } else {
            // Done
            update.character = CharacterState::Wielding;
        }

        update
    }
}
