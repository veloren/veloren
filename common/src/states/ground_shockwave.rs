use crate::{
    comp::{shockwave, Attacking, CharacterState, StateUpdate},
    event::ServerEvent,
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Whether the attack can deal more damage
    pub exhausted: bool,
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub damage: u32,
    /// Knockback
    pub knockback: f32,
    /// Angle of the shockwave
    pub shockwave_angle: f32,
    /// Speed of the shockwave
    pub shockwave_speed: f32,
    /// How long the shockwave travels for
    pub shockwave_duration: Duration,
    /// Whether the shockwave requires the target to be on the ground
    pub requires_ground: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.05);

        if self.buildup_duration != Duration::default() {
            // Build up
            update.character = CharacterState::GroundShockwave(Data {
                exhausted: self.exhausted,
                buildup_duration: self
                    .buildup_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                recover_duration: self.recover_duration,
                damage: self.damage,
                knockback: self.knockback,
                shockwave_angle: self.shockwave_angle,
                shockwave_speed: self.shockwave_speed,
                shockwave_duration: self.shockwave_duration,
                requires_ground: self.requires_ground,
            });
        } else if !self.exhausted {
            // Attack
            let properties = shockwave::Properties {
                angle: self.shockwave_angle,
                speed: self.shockwave_speed,
                duration: self.shockwave_duration,
                damage: self.damage,
                knockback: self.knockback,
                requires_ground: self.requires_ground,
                owner: Some(*data.uid),
            };
            update.server_events.push_front(ServerEvent::Shockwave {
                properties,
                pos: *data.pos,
                ori: *data.ori,
            });

            update.character = CharacterState::GroundShockwave(Data {
                exhausted: true,
                buildup_duration: self.buildup_duration,
                recover_duration: self.recover_duration,
                damage: self.damage,
                knockback: self.knockback,
                shockwave_angle: self.shockwave_angle,
                shockwave_speed: self.shockwave_speed,
                shockwave_duration: self.shockwave_duration,
                requires_ground: self.requires_ground,
            });
        } else if self.recover_duration != Duration::default() {
            // Recovery
            update.character = CharacterState::GroundShockwave(Data {
                exhausted: self.exhausted,
                buildup_duration: self.buildup_duration,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                damage: self.damage,
                knockback: self.knockback,
                shockwave_angle: self.shockwave_angle,
                shockwave_speed: self.shockwave_speed,
                shockwave_duration: self.shockwave_duration,
                requires_ground: self.requires_ground,
            });
        } else {
            // Done
            update.character = CharacterState::Wielding;
            // Make sure attack component is removed
            data.updater.remove::<Attacking>(data.entity);
        }

        update
    }
}
