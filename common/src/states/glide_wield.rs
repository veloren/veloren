use super::utils::*;
use crate::{
    comp::{slot::EquipSlot, CharacterState, EnergySource, InventoryAction, StateUpdate},
    states::behavior::{CharacterBehavior, JoinData},
};

pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 1.0);
        handle_jump(data, &mut update, 1.0);
        handle_dodge_input(data, &mut update);
        handle_wield(data, &mut update);

        // If not on the ground while wielding glider enter gliding state
        if !data.physics.on_ground {
            // Expend energy to slow a fall
            let energy_cost = (0.5 * (data.vel.0.z + 15.0).min(0.0).powi(2)) as i32;
            if update
                .energy
                .try_change_by(-energy_cost, EnergySource::Glide)
                .is_ok()
            {
                update.character = CharacterState::Glide;
            } else {
                update.energy.set_to(0, EnergySource::Glide);
                update.character = CharacterState::Idle;
            }
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

        update
    }

    fn sit(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sit(data, &mut update);
        update
    }

    fn dance(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_dance(data, &mut update);
        update
    }

    fn sneak(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sneak(data, &mut update);
        update
    }

    fn unwield(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        update.character = CharacterState::Idle;
        update
    }

    fn manipulate_loadout(&self, data: &JoinData, inv_action: InventoryAction) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_manipulate_loadout(&data, &mut update, inv_action);
        update
    }
}
