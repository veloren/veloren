use crate::{
    comp::StateUpdate,
    sys::{character_behavior::JoinData, phys::GRAVITY},
};
use std::time::Duration;
use vek::{Vec2, Vec3};

const HUMANOID_WATER_ACCEL: f32 = 70.0;
const HUMANOID_WATER_SPEED: f32 = 120.0;

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

    // Update velocity
    update.vel.0 += Vec2::broadcast(data.dt.0)
        * data.inputs.move_dir
        * if update.vel.0.magnitude_squared() < HUMANOID_WATER_SPEED.powf(2.0) {
            HUMANOID_WATER_ACCEL
        } else {
            0.0
        };

    // Set direction based on move direction when on the ground
    let ori_dir = if update.character.is_attack() || update.character.is_block() {
        Vec2::from(data.inputs.look_dir).normalized()
    } else {
        Vec2::from(update.vel.0)
    };

    if ori_dir.magnitude_squared() > 0.0001
        && (update.ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
            > 0.001
    {
        update.ori.0 = vek::ops::Slerp::slerp(
            update.ori.0,
            ori_dir.into(),
            if data.physics.on_ground { 9.0 } else { 2.0 } * data.dt.0,
        );
    }

    // Force players to pulse jump button to swim up
    if data.inputs.jump.is_pressed() && !data.inputs.jump.is_long_press(Duration::from_millis(600))
    {
        update.vel.0.z = (update.vel.0.z + data.dt.0 * GRAVITY * 1.25).min(HUMANOID_WATER_SPEED);
    }

    update
}
