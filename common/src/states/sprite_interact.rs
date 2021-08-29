use super::utils::*;
use crate::{
    comp::{CharacterState, InventoryManip, StateUpdate},
    event::ServerEvent,
    states::behavior::{CharacterBehavior, JoinData},
    terrain::SpriteKind,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// Buildup to item use
    pub buildup_duration: Duration,
    /// Duration of item use
    pub use_duration: Duration,
    /// Recovery after item use
    pub recover_duration: Duration,
    /// Position sprite is located at
    pub sprite_pos: Vec3<i32>,
    /// Kind of sprite interacted with
    pub sprite_kind: SpriteInteractKind,
    /// Had weapon wielded
    pub was_wielded: bool,
    /// Was sneaking
    pub was_sneak: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

        let ori_dir = Dir::from_unnormalized(Vec3::from(
            (self.static_data.sprite_pos.map(|x| x as f32) - data.pos.0).xy(),
        ));
        handle_orientation(data, &mut update, 1.0, ori_dir);
        handle_move(data, &mut update, 0.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::SpriteInteract(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to use section of stage
                    update.character = CharacterState::SpriteInteract(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.use_duration {
                    // Item use
                    update.character = CharacterState::SpriteInteract(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::SpriteInteract(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovery
                    update.character = CharacterState::SpriteInteract(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Create inventory manipulation event
                    let inv_manip = InventoryManip::Collect(self.static_data.sprite_pos);
                    update
                        .server_events
                        .push_front(ServerEvent::InventoryManip(data.entity, inv_manip));
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

        update
    }
}

/// Used to control effects based off of the type of item used
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SpriteInteractKind {
    Chest,
    Harvestable,
    Collectible,
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
            | SpriteKind::MedFlatCactus => Some(SpriteInteractKind::Harvestable),
            SpriteKind::Stones
            | SpriteKind::Twigs
            | SpriteKind::VialEmpty
            | SpriteKind::Bowl
            | SpriteKind::PotionMinor
            | SpriteKind::Seashells => Some(SpriteInteractKind::Collectible),
            SpriteKind::DungeonChest0
            | SpriteKind::DungeonChest1
            | SpriteKind::DungeonChest2
            | SpriteKind::DungeonChest3
            | SpriteKind::DungeonChest4
            | SpriteKind::DungeonChest5
            | SpriteKind::Chest
            | SpriteKind::ChestBuried
            | SpriteKind::Mud
            | SpriteKind::Crate => Some(SpriteInteractKind::Chest),
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
                Duration::from_secs_f32(0.3),
                Duration::from_secs_f32(0.1),
            ),
            Self::Harvestable => (
                Duration::from_secs_f32(0.3),
                Duration::from_secs_f32(0.5),
                Duration::from_secs_f32(0.2),
            ),
        }
    }
}
