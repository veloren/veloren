use crate::{
    comp::{Body, CharacterState, Gravity, LightEmitter, Projectile, StateUpdate},
    event::ServerEvent,
    states::utils::*,
    sys::character_behavior::*,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Can you hold the abilty beyond the prepare duration
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
    /// Whether the attack fired already
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.3);
        handle_jump(data, &mut update);

        if !self.exhausted
            && if self.holdable {
                data.inputs.holding_ability_key() || self.prepare_timer < self.prepare_duration
            } else {
                self.prepare_timer < self.prepare_duration
            }
        {
            // Prepare (draw the bow)
            update.character = CharacterState::BasicRanged(Data {
                prepare_timer: self.prepare_timer + Duration::from_secs_f32(data.dt.0),
                holdable: self.holdable,
                prepare_duration: self.prepare_duration,
                recover_duration: self.recover_duration,
                projectile: self.projectile.clone(),
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
                projectile_gravity: self.projectile_gravity,
                exhausted: false,
            });
        } else if !self.exhausted {
            // Fire
            let mut projectile = self.projectile.clone();
            projectile.owner = Some(*data.uid);
            update.server_events.push_front(ServerEvent::Shoot {
                entity: data.entity,
                dir: data.inputs.look_dir,
                body: self.projectile_body,
                projectile,
                light: self.projectile_light,
                gravity: self.projectile_gravity,
            });

            update.character = CharacterState::BasicRanged(Data {
                prepare_timer: self.prepare_timer,
                holdable: self.holdable,
                prepare_duration: self.prepare_duration,
                recover_duration: self.recover_duration,
                projectile: self.projectile.clone(),
                projectile_body: self.projectile_body,
                projectile_light: self.projectile_light,
                projectile_gravity: self.projectile_gravity,
                exhausted: true,
            });
        } else if self.recover_duration != Duration::default() {
            // Recovery
            update.character = CharacterState::BasicRanged(Data {
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
                exhausted: true,
            });
            return update;
        } else {
            // Done
            update.character = CharacterState::Wielding;
        }

        update
    }
}
