use super::utils::*;
use crate::{
    comp::{
        inventory::{
            item::{ConsumableKind, ItemKind},
            slot::Slot,
        },
        CharacterState, InventoryManip, StateUpdate,
    },
    event::ServerEvent,
    states::behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// Buildup to item use
    pub buildup_duration: Duration,
    /// Duration of item use
    pub use_duration: Duration,
    /// Recovery after item use
    pub recover_duration: Duration,
    /// Inventory slot to use item from
    pub inv_slot: Slot,
    /// Kind of item used
    pub item_kind: ItemUseKind,
    /// Had weapon wielded
    pub was_wielded: bool,
    /// Was sneaking
    pub was_sneak: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        match self.static_data.item_kind {
            ItemUseKind::Consumable(ConsumableKind::Potion) => {
                handle_orientation(data, &mut update, 1.0);
                handle_move(data, &mut update, 1.0);
            },
            ItemUseKind::Consumable(ConsumableKind::Food) => {
                handle_orientation(data, &mut update, 0.0);
                handle_move(data, &mut update, 0.0);
            },
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::UseItem(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to use section of stage
                    update.character = CharacterState::UseItem(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Use,
                        ..*self
                    });
                    // Create inventory manipulation event
                    let inv_manip = InventoryManip::Use(self.static_data.inv_slot);
                    update
                        .server_events
                        .push_front(ServerEvent::InventoryManip(data.entity, inv_manip));
                }
            },
            StageSection::Use => {
                if self.timer < self.static_data.use_duration {
                    // Item use
                    update.character = CharacterState::UseItem(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::UseItem(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    update.character = CharacterState::UseItem(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Done
                    if self.static_data.was_wielded {
                        update.character = CharacterState::Wielding;
                    } else if self.static_data.was_sneak {
                        update.character = CharacterState::Sneak;
                    } else {
                        update.character = CharacterState::Idle;
                    }
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Idle;
            },
        }

        update
    }
}

/// Used to control effects based off of the type of item used
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemUseKind {
    Consumable(ConsumableKind),
}

impl From<&ItemKind> for Option<ItemUseKind> {
    fn from(item_kind: &ItemKind) -> Self {
        match item_kind {
            ItemKind::Consumable { kind, .. } => Some(ItemUseKind::Consumable(*kind)),
            _ => None,
        }
    }
}
