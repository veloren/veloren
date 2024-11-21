use super::utils::*;
use crate::{
    comp::{character_state::OutputEvents, CharacterState, InventoryAction, StateUpdate},
    states::{
        behavior::{CharacterBehavior, JoinData},
        idle,
    },
    uid::Uid,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use vek::Vec3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    pub target_uid: Uid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub static_data: StaticData,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        let target_entity = data.id_maps.uid_entity(self.static_data.target_uid);
        let target_pos = target_entity
            .and_then(|target_entity| data.prev_phys_caches.get(target_entity))
            .and_then(|prev_phys| prev_phys.pos);

        let can_pet = target_entity.map_or(false, |target_entity| {
            target_pos.zip(data.alignments.get(target_entity)).map_or(
                false,
                |(target_position, target_alignment)| {
                    can_perform_pet(*data.pos, target_position, *target_alignment)
                },
            )
        });

        // Face target if they have a position.
        if let Some(target_pos) = target_pos {
            let ori_dir = Dir::from_unnormalized(Vec3::from((target_pos.0 - data.pos.0).xy()));
            handle_orientation(data, &mut update, 1.0, ori_dir);
        }

        leave_stance(data, output_events);
        handle_wield(data, &mut update);
        handle_jump(data, output_events, &mut update, 1.0);

        if !can_pet {
            update.character = CharacterState::Idle(idle::Data::default());
        }

        // Try to Fall/Stand up/Move
        if data.physics.on_ground.is_none() || data.inputs.move_dir.magnitude_squared() > 0.0 {
            update.character = CharacterState::Idle(idle::Data::default());
        }

        update
    }

    fn manipulate_loadout(
        &self,
        data: &JoinData,
        output_events: &mut OutputEvents,
        inv_action: InventoryAction,
    ) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_manipulate_loadout(data, output_events, &mut update, inv_action);
        update
    }

    fn wield(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_wield(data, &mut update);
        update
    }

    fn dance(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_dance(data, &mut update);
        update
    }

    fn sit(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sit(data, &mut update);
        update
    }

    fn crawl(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_crawl(data, &mut update);
        update
    }

    fn stand(&self, data: &JoinData, _: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        // Try to Fall/Stand up/Move
        update.character = CharacterState::Idle(idle::Data::default());
        update
    }
}
