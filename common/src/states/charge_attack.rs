use super::utils::*;
use crate::{
    comp::{CharacterState::*, HealthChange, HealthSource, StateUpdate},
    event::ServerEvent,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use std::{collections::VecDeque, time::Duration};
use vek::Vec3;

const CHARGE_SPEED: f32 = 20.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// How long the state has until exiting
    pub remaining_duration: Duration,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *data.pos,
            vel: *data.vel,
            ori: *data.ori,
            character: *data.character,
            energy: *data.energy,
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };
        // Move player
        update.vel.0 = Vec3::new(0.0, 0.0, update.vel.0.z)
            + (update.vel.0 * Vec3::new(1.0, 1.0, 0.0)
                + 1.5 * data.inputs.move_dir.try_normalized().unwrap_or_default())
            .try_normalized()
            .unwrap_or_default()
                * CHARGE_SPEED;

        // Check if hitting another entity
        if let Some(uid_b) = data.physics.touch_entity {
            // Send Damage event
            update.server_events.push_front(ServerEvent::Damage {
                uid: uid_b,
                change: HealthChange {
                    amount: -20,
                    cause: HealthSource::Attack { by: *data.uid },
                },
            });

            // Go back to wielding or idling
            attempt_wield(data, &mut update);
            return update;
        }

        // Check if charge timed out or can't keep moving forward
        if self.remaining_duration == Duration::default() || update.vel.0.magnitude_squared() < 10.0
        {
            attempt_wield(data, &mut update);
            return update;
        }

        // Tick remaining-duration and keep charging
        update.character = ChargeAttack(Data {
            remaining_duration: self
                .remaining_duration
                .checked_sub(Duration::from_secs_f32(data.dt.0))
                .unwrap_or_default(),
        });

        update
    }
}
