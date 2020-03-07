use crate::{
    comp::{CharacterState, StateUpdate},
    sys::character_state::JoinData,
};
use std::{collections::VecDeque, time::Duration};
use vek::Vec3;

const ROLL_SPEED: f32 = 17.0;

pub fn behavior(data: &JoinData) -> StateUpdate {
    let mut update = StateUpdate {
        character: *data.character,
        pos: *data.pos,
        vel: *data.vel,
        ori: *data.ori,
        energy: *data.energy,
        local_events: VecDeque::new(),
        server_events: VecDeque::new(),
    };

    if let CharacterState::Roll { remaining_duration } = data.character {
        // Update velocity
        update.vel.0 = Vec3::new(0.0, 0.0, update.vel.0.z)
            + (update.vel.0 * Vec3::new(1.0, 1.0, 0.0)
                + 1.5 * data.inputs.move_dir.try_normalized().unwrap_or_default())
            .try_normalized()
            .unwrap_or_default()
                * ROLL_SPEED;

        // Smooth orientation
        if update.vel.0.magnitude_squared() > 0.0001
            && (update.ori.0.normalized() - Vec3::from(update.vel.0).normalized())
                .magnitude_squared()
                > 0.001
        {
            update.ori.0 =
                vek::ops::Slerp::slerp(update.ori.0, update.vel.0.into(), 9.0 * data.dt.0);
        }

        if *remaining_duration == Duration::default() {
            // Roll duration has expired
            update.character = CharacterState::Idle {};
        } else {
            // Otherwise, tick down remaining_duration
            update.character = CharacterState::Roll {
                remaining_duration: remaining_duration
                    .checked_sub(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
            };
        }
    }

    update
}
