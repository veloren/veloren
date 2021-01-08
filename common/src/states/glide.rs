use super::utils::handle_climb;
use crate::{
    comp::{inventory::slot::EquipSlot, CharacterState, EnergySource, StateUpdate},
    states::behavior::{CharacterBehavior, JoinData},
    util::Dir,
};
use serde::{Deserialize, Serialize};
use vek::Vec2;

// Gravity is 9.81 * 4, so this makes gravity equal to .15
const GLIDE_ANTIGRAV: f32 = crate::consts::GRAVITY * 0.90;
const GLIDE_ACCEL: f32 = 12.0;
const GLIDE_SPEED: f32 = 45.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        // If player is on ground, end glide
        if data.physics.on_ground {
            update.character = CharacterState::GlideWield;
            return update;
        }
        if data
            .physics
            .in_liquid
            .map(|depth| depth > 0.5)
            .unwrap_or(false)
        {
            update.character = CharacterState::Idle;
        }
        if data.inventory.equipped(EquipSlot::Glider).is_none() {
            update.character = CharacterState::Idle
        };
        // If there is a wall in front of character and they are trying to climb go to
        // climb
        handle_climb(&data, &mut update);

        // Move player according to movement direction vector
        update.vel.0 += Vec2::broadcast(data.dt.0)
            * data.inputs.move_dir
            * if data.vel.0.magnitude_squared() < GLIDE_SPEED.powi(2) {
                GLIDE_ACCEL
            } else {
                0.0
            };

        // Determine orientation vector from movement direction vector
        let horiz_vel = Vec2::from(update.vel.0);
        update.ori.0 = Dir::slerp_to_vec3(update.ori.0, horiz_vel.into(), 2.0 * data.dt.0);

        // Apply Glide antigrav lift
        let horiz_speed_sq = horiz_vel.magnitude_squared();
        if horiz_speed_sq < GLIDE_SPEED.powi(2) && update.vel.0.z < 0.0 {
            let lift = (GLIDE_ANTIGRAV + update.vel.0.z.powi(2) * 0.15)
                * (horiz_speed_sq * f32::powf(0.075, 2.0)).clamp(0.2, 1.0)
                * data.dt.0;

            update.vel.0.z += lift;

            // Expend energy during strenuous maneuvers.
            // Cost increases with lift exceeding that of calmly gliding.
            let energy_cost = (10.0 * (lift - GLIDE_ANTIGRAV * data.dt.0)).max(0.0) as i32;
            if update
                .energy
                .try_change_by(-energy_cost, EnergySource::Glide)
                .is_err()
            {
                update.character = CharacterState::Idle {};
            }
        }

        update
    }

    fn unwield(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle;
        update
    }
}
