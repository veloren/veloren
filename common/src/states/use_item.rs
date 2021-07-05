use super::utils::*;
use crate::{
    comp::{
        buff::{BuffChange, BuffKind},
        inventory::{
            item::{ConsumableKind, ItemKind},
            slot::{InvSlotId, Slot},
        },
        CharacterState, InventoryManip, StateUpdate,
    },
    event::ServerEvent,
    states::behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// Buildup to item use
    pub buildup_duration: Duration,
    /// Duration of item use
    pub use_duration: Duration,
    /// Recovery after item use
    pub recover_duration: Duration,
    /// Inventory slot to use item from
    pub inv_slot: InvSlotId,
    /// Item definition id, used to verify that slot still has the correct item
    pub item_definition_id: String,
    /// Kind of item used
    pub item_kind: ItemUseKind,
    /// Had weapon wielded
    pub was_wielded: bool,
    /// Was sneaking
    pub was_sneak: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
            ItemUseKind::Consumable(ConsumableKind::Drink) => {
                handle_orientation(data, &mut update, 1.0);
                handle_move(data, &mut update, 1.0);
            },
            ItemUseKind::Consumable(ConsumableKind::Food | ConsumableKind::ComplexFood) => {
                handle_orientation(data, &mut update, 0.0);
                handle_move(data, &mut update, 0.0);
            },
        }

        let use_point = match self.static_data.item_kind {
            ItemUseKind::Consumable(ConsumableKind::Drink | ConsumableKind::Food) => {
                UsePoint::BuildupUse
            },
            ItemUseKind::Consumable(ConsumableKind::ComplexFood) => UsePoint::UseRecover,
        };

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::UseItem(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to use section of stage
                    update.character = CharacterState::UseItem(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Use,
                    });
                    if let UsePoint::BuildupUse = use_point {
                        // Create inventory manipulation event
                        use_item(data, &mut update, self);
                    }
                }
            },
            StageSection::Use => {
                if self.timer < self.static_data.use_duration {
                    // Item use
                    update.character = CharacterState::UseItem(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::UseItem(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                    });
                    if let UsePoint::UseRecover = use_point {
                        // Create inventory manipulation event
                        use_item(data, &mut update, self);
                    }
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    update.character = CharacterState::UseItem(Data {
                        static_data: self.static_data.clone(),
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

        // At end of state logic so an interrupt isn't overwritten
        handle_state_interrupt(data, &mut update, false);

        if matches!(update.character, CharacterState::Roll(_)) {
            // Remove potion/saturation effect if left the use item state early by rolling
            update.server_events.push_front(ServerEvent::Buff {
                entity: data.entity,
                buff_change: BuffChange::RemoveByKind(BuffKind::Potion),
            });
            update.server_events.push_front(ServerEvent::Buff {
                entity: data.entity,
                buff_change: BuffChange::RemoveByKind(BuffKind::Saturation),
            });
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

impl ItemUseKind {
    /// Returns (buildup, use, recover)
    pub fn durations(&self) -> (Duration, Duration, Duration) {
        match self {
            Self::Consumable(ConsumableKind::Drink) => (
                Duration::from_secs_f32(0.1),
                Duration::from_secs_f32(1.1),
                Duration::from_secs_f32(0.1),
            ),
            Self::Consumable(ConsumableKind::Food) => (
                Duration::from_secs_f32(1.0),
                Duration::from_secs_f32(5.0),
                Duration::from_secs_f32(0.5),
            ),
            Self::Consumable(ConsumableKind::ComplexFood) => (
                Duration::from_secs_f32(1.0),
                Duration::from_secs_f32(5.0),
                Duration::from_secs_f32(0.5),
            ),
        }
    }
}

/// Used to control when the item is used in the state
enum UsePoint {
    /// Between buildup and use
    BuildupUse,
    /// Between use and recover
    UseRecover,
}

fn use_item(data: &JoinData, update: &mut StateUpdate, state: &Data) {
    // Check if the same item is in the slot
    let item_is_same = data
        .inventory
        .get(state.static_data.inv_slot)
        .map_or(false, |item| {
            item.item_definition_id() == state.static_data.item_definition_id
        });
    if item_is_same {
        // Create inventory manipulation event
        let inv_manip = InventoryManip::Use(Slot::Inventory(state.static_data.inv_slot));
        update
            .server_events
            .push_front(ServerEvent::InventoryManip(data.entity, inv_manip));
    }
}
