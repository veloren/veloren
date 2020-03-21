use crate::{
    comp::{Body, CharacterState, Gravity, Projectile, StateUpdate},
    event::ServerEvent,
    states::utils::*,
    sys::character_behavior::*,
};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// How long we prepared the weapon already
    pub prepare_timer: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Projectile
    pub projectile: Projectile,
    /// Projectile
    pub projectile_body: Body,
    /// Whether the attack fired already
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update);
        handle_jump(data, &mut update);

        if !self.exhausted
            && (data.inputs.primary.is_pressed() | data.inputs.secondary.is_pressed())
        {
            // Prepare (draw the bow)
            update.character = CharacterState::BasicRanged(Data {
                prepare_timer: self.prepare_timer + Duration::from_secs_f32(data.dt.0),
                recover_duration: self.recover_duration,
                projectile: self.projectile.clone(),
                projectile_body: self.projectile_body,
                exhausted: false,
            });
        } else if !self.exhausted {
            // Fire
            let mut projectile = self.projectile.clone();
            projectile.set_owner(*data.uid);
            update.server_events.push_front(ServerEvent::Shoot {
                entity: data.entity,
                dir: data.inputs.look_dir,
                body: self.projectile_body,
                light: None,
                projectile,
                gravity: Some(Gravity(0.1)),
            });

            update.character = CharacterState::BasicRanged(Data {
                prepare_timer: self.prepare_timer,
                recover_duration: self.recover_duration,
                projectile: self.projectile.clone(),
                projectile_body: self.projectile_body,
                exhausted: true,
            });
        } else if self.recover_duration != Duration::default() {
            // Recovery
            update.character = CharacterState::BasicRanged(Data {
                prepare_timer: self.prepare_timer,
                recover_duration: self
                    .recover_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                projectile: self.projectile.clone(),
                projectile_body: self.projectile_body,
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
