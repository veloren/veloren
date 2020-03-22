use crate::{
    comp::{CharacterState, EnergySource, StateUpdate},
    event::LocalEvent,
    sys::{
        character_behavior::{CharacterBehavior, JoinData},
        phys::GRAVITY,
    },
    util::safe_slerp,
};
use vek::{
    vec::{Vec2, Vec3},
    Lerp,
};

const HUMANOID_CLIMB_ACCEL: f32 = 5.0;
const CLIMB_SPEED: f32 = 5.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        if let Err(_) = update.energy.try_change_by(-8, EnergySource::Climb) {
            update.character = CharacterState::Idle {};
        }

        // If no wall is in front of character ...
        if data.physics.on_wall.is_none() || data.physics.on_ground {
            if data.inputs.jump.is_pressed() {
                // They've climbed atop something, give them a boost
                update
                    .local_events
                    .push_front(LocalEvent::Jump(data.entity));
            }
            update.character = CharacterState::Idle {};
            return update;
        }

        // Move player
        update.vel.0 += Vec2::broadcast(data.dt.0)
            * data.inputs.move_dir
            * if update.vel.0.magnitude_squared() < CLIMB_SPEED.powf(2.0) {
                HUMANOID_CLIMB_ACCEL
            } else {
                0.0
            };

        // Set orientation direction based on wall direction
        let ori_dir = if let Some(wall_dir) = data.physics.on_wall {
            if Vec2::<f32>::from(wall_dir).magnitude_squared() > 0.001 {
                Vec2::from(wall_dir).normalized()
            } else {
                Vec2::from(update.vel.0)
            }
        } else {
            Vec2::from(update.vel.0)
        };

        // Smooth orientation
        update.ori.0 = safe_slerp(
            update.ori.0,
            ori_dir.into(),
            if data.physics.on_ground { 9.0 } else { 2.0 } * data.dt.0,
        );

        // Apply Vertical Climbing Movement
        if let (true, Some(_wall_dir)) = (
            (data.inputs.climb.is_pressed() | data.inputs.climb_down.is_pressed())
                && update.vel.0.z <= CLIMB_SPEED,
            data.physics.on_wall,
        ) {
            if data.inputs.climb_down.is_pressed() && !data.inputs.climb.is_pressed() {
                update.vel.0 -=
                    data.dt.0 * update.vel.0.map(|e| e.abs().powf(1.5) * e.signum() * 6.0);
            } else if data.inputs.climb.is_pressed() && !data.inputs.climb_down.is_pressed() {
                update.vel.0.z = (update.vel.0.z + data.dt.0 * GRAVITY * 1.25).min(CLIMB_SPEED);
            } else {
                update.vel.0.z = update.vel.0.z + data.dt.0 * GRAVITY * 1.5;
                update.vel.0 = Lerp::lerp(
                    update.vel.0,
                    Vec3::zero(),
                    30.0 * data.dt.0 / (1.0 - update.vel.0.z.min(0.0) * 5.0),
                );
            }
        }

        update
    }
}
