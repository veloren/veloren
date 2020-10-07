use super::utils::*;
use crate::{
    comp::{CharacterState, StateUpdate},
    sys::character_behavior::{CharacterBehavior, JoinData},
};

pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(&data, &mut update, 1.0);
        handle_jump(&data, &mut update);
        handle_dodge_input(data, &mut update);
        handle_wield(data, &mut update);

        // If not on the ground while wielding glider enter gliding state
        if !data.physics.on_ground {
            update.character = CharacterState::Glide;
        }
        if data
            .physics
            .in_fluid
            .map(|depth| depth > 0.5)
            .unwrap_or(false)
        {
            update.character = CharacterState::Idle;
        }
        if data.loadout.glider.is_none() {
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
}
