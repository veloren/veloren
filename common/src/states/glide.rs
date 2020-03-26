use crate::{
    comp::{CharacterState, StateUpdate},
    sys::character_behavior::{CharacterBehavior, JoinData},
    util::safe_slerp,
};
use vek::Vec2;

// Gravity is 9.81 * 4, so this makes gravity equal to .15
const GLIDE_ANTIGRAV: f32 = crate::sys::phys::GRAVITY * 0.90;
const GLIDE_ACCEL: f32 = 12.0;
const GLIDE_SPEED: f32 = 45.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        // If glide button isn't held or player is on ground, end glide
        if !data.inputs.glide.is_pressed() || data.physics.on_ground {
            update.character = CharacterState::Idle {};
        }

        // If there is a wall in front of character go to climb
        if let Some(_) = data.physics.on_wall {
            update.character = CharacterState::Climb {};
        }

        // Move player according to movement direction vector
        update.vel.0 += Vec2::broadcast(data.dt.0)
            * data.inputs.move_dir
            * if data.vel.0.magnitude_squared() < GLIDE_SPEED.powf(2.0) {
                GLIDE_ACCEL
            } else {
                0.0
            };

        // Determine orientation vector from movement direction vector
        let ori_dir = Vec2::from(update.vel.0);
        update.ori.0 = safe_slerp(update.ori.0, ori_dir.into(), 2.0 * data.dt.0);

        // Apply Glide antigrav lift
        if Vec2::<f32>::from(update.vel.0).magnitude_squared() < GLIDE_SPEED.powf(2.0)
            && update.vel.0.z < 0.0
        {
            let lift = GLIDE_ANTIGRAV + update.vel.0.z.abs().powf(2.0) * 0.15;
            update.vel.0.z += data.dt.0
                * lift
                * (Vec2::<f32>::from(update.vel.0).magnitude() * 0.075)
                    .min(1.0)
                    .max(0.2);
        }

        // Otherwise keep gliding
        update
    }
}
