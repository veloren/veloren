use super::utils::*;
use crate::{
    comp::{
        character_state::OutputEvents, controller::InputKind, item::ItemDefinitionIdOwned,
        slot::InvSlotId, CharacterState, InventoryManip, StateUpdate,
    },
    event::{InventoryManipEvent, LocalEvent, ToggleSpriteLightEvent},
    outcome::Outcome,
    states::behavior::{CharacterBehavior, JoinData},
    terrain::SpriteKind,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// Buildup to sprite interaction
    pub buildup_duration: Duration,
    /// Duration of sprite interaction
    pub use_duration: Duration,
    /// Recovery after sprite interaction
    pub recover_duration: Duration,
    /// Position sprite is located at
    pub sprite_pos: Vec3<i32>,
    /// Kind of sprite interacted with
    pub sprite_kind: SpriteInteractKind,
    /// Had weapon wielded
    pub was_wielded: bool,
    /// Was sneaking
    pub was_sneak: bool,
    /// The item required to interact with the sprite, if one was required
    ///
    /// The second field is the slot that the required item was in when this
    /// state was created. If it isn't in this slot anymore the interaction will
    /// fail.
    ///
    /// If third field is true, item should be consumed on collection
    pub required_item: Option<(ItemDefinitionIdOwned, InvSlotId, bool)>,
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
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        let ori_dir = Dir::from_unnormalized(Vec3::from(
            (self.static_data.sprite_pos.map(|x| x as f32 + 0.5) - data.pos.0).xy(),
        ));
        handle_orientation(data, &mut update, 1.0, ori_dir);
        handle_move(data, &mut update, 0.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    if let CharacterState::SpriteInteract(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to use section of stage
                    if let CharacterState::SpriteInteract(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Action;
                    }
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.use_duration {
                    // sprite interaction
                    if let CharacterState::SpriteInteract(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::SpriteInteract(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                    }
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    if let CharacterState::SpriteInteract(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Create inventory manipulation event
                    let (has_required_item, inv_slot) = self
                        .static_data
                        .required_item
                        .as_ref()
                        .map_or((true, None), |&(ref item_def_id, slot, consume)| {
                            // Check that required item is still in expected slot
                            let has_item = data
                                .inventory
                                .and_then(|inv| inv.get(slot))
                                .map_or(false, |item| item.item_definition_id() == *item_def_id);

                            (has_item, has_item.then_some((slot, consume)))
                        });
                    if has_required_item {
                        let inv_manip = InventoryManip::Collect {
                            sprite_pos: self.static_data.sprite_pos,
                            required_item: inv_slot,
                        };
                        match self.static_data.sprite_kind {
                            SpriteInteractKind::ToggleLight(enable) => {
                                output_events.emit_server(ToggleSpriteLightEvent {
                                    entity: data.entity,
                                    pos: self.static_data.sprite_pos,
                                    enable,
                                })
                            },
                            _ => output_events
                                .emit_server(InventoryManipEvent(data.entity, inv_manip)),
                        }

                        if matches!(self.static_data.sprite_kind, SpriteInteractKind::Unlock) {
                            output_events.emit_local(LocalEvent::CreateOutcome(
                                Outcome::SpriteUnlocked {
                                    pos: self.static_data.sprite_pos,
                                },
                            ));
                        }
                    }
                    // Done
                    end_ability(data, &mut update);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_ability(data, &mut update);
            },
        }

        // Allow attacks and abilities to interrupt
        handle_wield(data, &mut update);

        // At end of state logic so an interrupt isn't overwritten
        if input_is_pressed(data, InputKind::Roll) {
            handle_input(data, output_events, &mut update, InputKind::Roll);
        }

        update
    }
}

/// Used to control effects based off of the type of sprite interacted with
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpriteInteractKind {
    Chest,
    Harvestable,
    Collectible,
    Unlock,
    Fallback,
    ToggleLight(bool),
}

impl From<SpriteKind> for Option<SpriteInteractKind> {
    fn from(sprite_kind: SpriteKind) -> Self {
        match sprite_kind {
            SpriteKind::Apple
            | SpriteKind::Mushroom
            | SpriteKind::RedFlower
            | SpriteKind::Sunflower
            | SpriteKind::Coconut
            | SpriteKind::Beehive
            | SpriteKind::Cotton
            | SpriteKind::Moonbell
            | SpriteKind::Pyrebloom
            | SpriteKind::WildFlax
            | SpriteKind::RoundCactus
            | SpriteKind::ShortFlatCactus
            | SpriteKind::MedFlatCactus
            | SpriteKind::Wood
            | SpriteKind::Bamboo
            | SpriteKind::Hardwood
            | SpriteKind::Ironwood
            | SpriteKind::Frostwood
            | SpriteKind::Eldwood => Some(SpriteInteractKind::Harvestable),
            SpriteKind::Stones
            | SpriteKind::Twigs
            | SpriteKind::VialEmpty
            | SpriteKind::Bowl
            | SpriteKind::PotionMinor
            | SpriteKind::Seashells
            | SpriteKind::Bomb => Some(SpriteInteractKind::Collectible),
            SpriteKind::Keyhole
            | SpriteKind::BoneKeyhole
            | SpriteKind::HaniwaKeyhole
            | SpriteKind::GlassKeyhole
            | SpriteKind::KeyholeBars => Some(SpriteInteractKind::Unlock),
            // Collectible checked in addition to container for case that sprite requires a tool to
            // collect and cannot be collected by hand, yet still meets the container check
            _ if sprite_kind.is_container() && sprite_kind.is_collectible() => {
                Some(SpriteInteractKind::Chest)
            },
            _ if sprite_kind.is_collectible() => Some(SpriteInteractKind::Fallback),
            _ => None,
        }
    }
}

impl SpriteInteractKind {
    /// Returns (buildup, use, recover)
    pub fn durations(&self) -> (Duration, Duration, Duration) {
        match self {
            Self::Chest => (
                Duration::from_secs_f32(0.5),
                Duration::from_secs_f32(2.0),
                Duration::from_secs_f32(0.5),
            ),
            Self::Collectible => (
                Duration::from_secs_f32(0.1),
                Duration::from_secs_f32(0.2),
                Duration::from_secs_f32(0.1),
            ),
            Self::Harvestable => (
                Duration::from_secs_f32(0.3),
                Duration::from_secs_f32(0.3),
                Duration::from_secs_f32(0.2),
            ),
            Self::Fallback => (
                Duration::from_secs_f32(5.0),
                Duration::from_secs_f32(5.0),
                Duration::from_secs_f32(5.0),
            ),
            Self::Unlock => (
                Duration::from_secs_f32(0.8),
                Duration::from_secs_f32(1.0),
                Duration::from_secs_f32(0.3),
            ),
            Self::ToggleLight(_) => (
                Duration::from_secs_f32(0.1),
                Duration::from_secs_f32(0.2),
                Duration::from_secs_f32(0.1),
            ),
        }
    }
}
