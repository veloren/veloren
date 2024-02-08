use super::utils::*;
use crate::{
    comp::{
        buff::{BuffChange, BuffKind},
        character_state::OutputEvents,
        controller::InputKind,
        inventory::{
            item::{ConsumableKind, ItemKind},
            slot::{InvSlotId, Slot},
        },
        CharacterState, InventoryManip, StateUpdate,
    },
    event::{BuffEvent, InventoryManipEvent},
    states::behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticData {
    /// Buildup to item use
    pub buildup_duration: Duration,
    /// Duration of item use
    pub use_duration: Duration,
    /// Recovery after item use
    pub recover_duration: Duration,
    /// Inventory slot to use item from
    pub inv_slot: InvSlotId,
    /// Item hash, used to verify that slot still has the correct item
    pub item_hash: u64,
    /// Kind of item used
    pub item_kind: ItemUseKind,
    /// Had weapon wielded
    pub was_wielded: bool,
    /// Was sneaking
    pub was_sneak: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        match self.static_data.item_kind {
            ItemUseKind::Consumable(ConsumableKind::Drink | ConsumableKind::Charm) => {
                handle_orientation(data, &mut update, 1.0, None);
                handle_move(data, &mut update, 1.0);
            },
            ItemUseKind::Consumable(ConsumableKind::Food | ConsumableKind::ComplexFood) => {
                handle_orientation(data, &mut update, 0.0, None);
                handle_move(data, &mut update, 0.0);
            },
        }

        let use_point = match self.static_data.item_kind {
            ItemUseKind::Consumable(ConsumableKind::Drink | ConsumableKind::Food) => {
                UsePoint::BuildupUse
            },
            ItemUseKind::Consumable(ConsumableKind::ComplexFood | ConsumableKind::Charm) => {
                UsePoint::UseRecover
            },
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
                        stage_section: StageSection::Action,
                    });
                    if let UsePoint::BuildupUse = use_point {
                        // Create inventory manipulation event
                        use_item(data, output_events, self);
                    }
                }
            },
            StageSection::Action => {
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
                        use_item(data, output_events, self);
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
                    end_ability(data, &mut update);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_ability(data, &mut update);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        if input_is_pressed(data, InputKind::Roll) {
            handle_input(data, output_events, &mut update, InputKind::Roll);
        }

        if matches!(update.character, CharacterState::Roll(_)) {
            // Remove potion/saturation effect if left the use item state early by rolling
            output_events.emit_server(BuffEvent {
                entity: data.entity,
                buff_change: BuffChange::RemoveByKind(BuffKind::Potion),
            });
            output_events.emit_server(BuffEvent {
                entity: data.entity,
                buff_change: BuffChange::RemoveByKind(BuffKind::Saturation),
            });
        }

        update
    }
}

/// Used to control effects based off of the type of item used
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
                Duration::from_secs_f32(4.0),
                Duration::from_secs_f32(0.5),
            ),
            Self::Consumable(ConsumableKind::ComplexFood) => (
                Duration::from_secs_f32(1.0),
                Duration::from_secs_f32(4.5),
                Duration::from_secs_f32(0.5),
            ),
            Self::Consumable(ConsumableKind::Charm) => (
                Duration::from_secs_f32(0.1),
                Duration::from_secs_f32(0.8),
                Duration::from_secs_f32(0.1),
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

fn use_item(data: &JoinData, output_events: &mut OutputEvents, state: &Data) {
    // Check if the same item is in the slot
    let item_is_same = data
        .inventory
        .and_then(|inv| inv.get(state.static_data.inv_slot))
        .map_or(false, |item| {
            item.item_hash() == state.static_data.item_hash
        });
    if item_is_same {
        // Create inventory manipulation event
        let inv_manip = InventoryManip::Use(Slot::Inventory(state.static_data.inv_slot));
        output_events.emit_server(InventoryManipEvent(data.entity, inv_manip));
    }
}
