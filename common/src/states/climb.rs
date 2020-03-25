use crate::{
    comp::{CharacterState, Climb, EnergySource, StateUpdate},
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

        // If no wall is in front of character or we stopped climbing;
        if data.physics.on_wall.is_none() || data.physics.on_ground || data.inputs.climb.is_none() {
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

        // Expend energy if climbing
        let energy_use = match data.inputs.climb {
            Some(Climb::Up) | Some(Climb::Down) => 8,
            Some(Climb::Hold) => 1,
            // Note: this is currently unreachable
            None => 0,
        };
        if let Err(_) = update
            .energy
            .try_change_by(-energy_use, EnergySource::Climb)
        {
            update.character = CharacterState::Idle {};
        }

        // Set orientation direction based on wall direction
        let ori_dir = if let Some(wall_dir) = data.physics.on_wall {
            Vec2::from(wall_dir)
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
        if let (Some(climb), true, Some(_wall_dir)) = (
            data.inputs.climb,
            update.vel.0.z <= CLIMB_SPEED,
            data.physics.on_wall,
        ) {
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
