use crate::{
    comp::{CharacterState, Climb, EnergySource, StateUpdate},
    event::LocalEvent,
    sys::{
        character_behavior::{CharacterBehavior, JoinData},
        phys::GRAVITY,
    },
    util::Dir,
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

        // If no wall is in front of character or we stopped climbing;
        let (wall_dir, climb) = if let (Some(wall_dir), Some(climb), false) = (
            data.physics.on_wall,
            data.inputs.climb,
            data.physics.on_ground,
        ) {
            (wall_dir, climb)
        } else {
            if data.inputs.jump.is_pressed() {
                // They've climbed atop something, give them a boost
                update
                    .local_events
                    .push_front(LocalEvent::Jump(data.entity));
            }
            update.character = CharacterState::Idle {};
            return update;
        };

        // Move player
        update.vel.0 += Vec2::broadcast(data.dt.0)
            * data.inputs.move_dir
            * if update.vel.0.magnitude_squared() < CLIMB_SPEED.powf(2.0) {
                HUMANOID_CLIMB_ACCEL
            } else {
                0.0
            };

        // Expend energy if climbing
        let energy_use = match climb {
            Climb::Up | Climb::Down => 8,
            Climb::Hold => 1,
        };
        if let Err(_) = update
            .energy
            .try_change_by(-energy_use, EnergySource::Climb)
        {
            update.character = CharacterState::Idle {};
        }

        // Set orientation direction based on wall direction
        let ori_dir = Vec2::from(wall_dir);

        // Smooth orientation
        update.ori.0 = Dir::slerp_to_vec3(
            update.ori.0,
            ori_dir.into(),
            if data.physics.on_ground { 9.0 } else { 2.0 } * data.dt.0,
        );

        // Apply Vertical Climbing Movement
        if update.vel.0.z <= CLIMB_SPEED {
            match climb {
                Climb::Down => {
                    update.vel.0 -=
                        data.dt.0 * update.vel.0.map(|e| e.abs().powf(1.5) * e.signum() * 6.0);
                },
                Climb::Up => {
                    update.vel.0.z = (update.vel.0.z + data.dt.0 * GRAVITY * 1.25).min(CLIMB_SPEED);
                },
                Climb::Hold => {
                    // Antigrav
                    update.vel.0.z = (update.vel.0.z + data.dt.0 * GRAVITY * 1.5).min(CLIMB_SPEED);
                    update.vel.0 = Lerp::lerp(
                        update.vel.0,
                        Vec3::zero(),
                        30.0 * data.dt.0 / (1.0 - update.vel.0.z.min(0.0) * 5.0),
                    );
                },
            }
        }

        update
    }
}
