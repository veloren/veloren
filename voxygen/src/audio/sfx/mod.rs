//! Manages individual sfx event system, listens for sfx events, and requests
//! playback at the requested position and volume
//!
//! Veloren's sfx are managed through a configuration which lives in the
//! codebase under `/assets/voxygen/audio/sfx.ron`.
//!
//! If there are errors while reading or deserialising the configuration file, a
//! warning is logged and sfx will be disabled.
//!
//! Each entry in the configuration consists of an
//! [SfxEvent](../../../veloren_common/event/enum.SfxEvent.html) item, with some
//! additional information to allow playback:
//! - `files` - the paths to the `.wav` files to be played for the sfx. minus
//!   the file extension. This can be a single item if the same sound can be
//!   played each time, or a list of files from which one is chosen at random to
//!   be played.
//! - `threshold` - the time that the system should wait between successive
//!   plays. This avoids playing the sound with very fast successive repetition
//!   when the character can maintain a state over a long period, such as
//!   running or climbing.
//!
//! The following snippet details some entries in the configuration and how they
//! map to the sound files:
//! ```ignore
//! Run: (
//!    files: [
//!        "voxygen.audio.sfx.footsteps.stepgrass_1",
//!        "voxygen.audio.sfx.footsteps.stepgrass_2",
//!        "voxygen.audio.sfx.footsteps.stepgrass_3",
//!        "voxygen.audio.sfx.footsteps.stepgrass_4",
//!        "voxygen.audio.sfx.footsteps.stepgrass_5",
//!        "voxygen.audio.sfx.footsteps.stepgrass_6",
//!    ],
//!    threshold: 0.25, // wait 0.25s between plays
//! ),
//! Wield(Sword): ( // depends on the player's weapon
//!    files: [
//!        "voxygen.audio.sfx.weapon.sword_out",
//!    ],
//!    threshold: 0.5,
//! ),
//! ...
//! ```
//!
//! These items (for example, the `Wield(Sword)` occasionally depend on some
//! property which varies in game. The
//! [SfxEvent](../../../veloren_common/event/enum.SfxEvent.html) documentation
//! provides links to those variables, some examples are provided her for longer
//! items:
//!
//! ```ignore
//! // An inventory action
//! Inventory(Dropped): (
//!     files: [
//!        "voxygen.audio.sfx.footsteps.stepgrass_4",
//!    ],
//!    threshold: 0.5,
//! ),
//! // An inventory action which depends upon the item
//! Inventory(Consumed(Apple)): (
//!    files: [
//!        "voxygen.audio.sfx.inventory.consumable.apple",
//!    ],
//!    threshold: 0.5
//! ),
//! // An attack ability which depends on the weapon
//! Attack(DashMelee, Sword): (
//!     files: [
//!         "voxygen.audio.sfx.weapon.sword_dash_01",
//!         "voxygen.audio.sfx.weapon.sword_dash_02",
//!     ],
//!     threshold: 1.2,
//! ),
//! // A multi-stage attack ability which depends on the weapon
//! Attack(ComboMelee(Swing, 1), Sword): (
//!     files: [
//!         "voxygen.audio.sfx.abilities.swing_sword",
//!     ],
//!     threshold: 0.5,
//! ),
//! ```

mod event_mapper;

use crate::{
    audio::AudioFrontend,
    scene::{Camera, Terrain},
};

use client::Client;
use common::{
    assets::{self, AssetExt, AssetHandle},
    comp::{
        item::{ItemKind, ToolKind},
        object, Body, CharacterAbilityType, InventoryUpdateEvent,
    },
    outcome::Outcome,
    terrain::TerrainChunk,
};
use common_sys::state::State;
use event_mapper::SfxEventMapper;
use hashbrown::HashMap;
use rand::prelude::*;
use serde::Deserialize;
use tracing::warn;
use vek::*;

/// We watch the states of nearby entities in order to emit SFX at their
/// position based on their state. This constant limits the radius that we
/// observe to prevent tracking distant entities. It approximates the distance
/// at which the volume of the sfx emitted is too quiet to be meaningful for the
/// player.
const SFX_DIST_LIMIT_SQR: f32 = 20000.0;

pub struct SfxEventItem {
    /// The SFX event that triggers this sound
    pub sfx: SfxEvent,
    /// The position at which the sound should play
    pub pos: Option<Vec3<f32>>,
    /// The volume to play the sound at
    pub vol: Option<f32>,
}

impl SfxEventItem {
    pub fn new(sfx: SfxEvent, pos: Option<Vec3<f32>>, vol: Option<f32>) -> Self {
        Self { sfx, pos, vol }
    }

    pub fn at_player_position(sfx: SfxEvent) -> Self {
        Self {
            sfx,
            pos: None,
            vol: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Hash, Eq)]
pub enum SfxEvent {
    Campfire,
    Embers,
    Birdcall,
    Owl,
    Cricket,
    Frog,
    Bees,
    RunningWater,
    Idle,
    Swim,
    Run,
    QuadRun,
    SnowRun,
    QuadSnowRun,
    Roll,
    Sneak,
    Climb,
    GliderOpen,
    Glide,
    GliderClose,
    Jump,
    Fall,
    ExperienceGained,
    LevelUp,
    Attack(CharacterAbilityType, ToolKind),
    Wield(ToolKind),
    Unwield(ToolKind),
    Inventory(SfxInventoryEvent),
    Explosion,
    ProjectileShot,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Hash, Eq)]
pub enum SfxInventoryEvent {
    Collected,
    CollectedTool(ToolKind),
    CollectedItem(String),
    CollectFailed,
    Consumed(String),
    Debug,
    Dropped,
    Given,
    Swapped,
}

// TODO Move to a separate event mapper?
impl From<&InventoryUpdateEvent> for SfxEvent {
    fn from(value: &InventoryUpdateEvent) -> Self {
        match value {
            InventoryUpdateEvent::Collected(item) => {
                // Handle sound effects for types of collected items, falling
                // back to the default Collected event
                match &item.kind() {
                    ItemKind::Tool(tool) => {
                        SfxEvent::Inventory(SfxInventoryEvent::CollectedTool(tool.kind))
                    },
                    ItemKind::Ingredient { kind } => match &kind[..] {
                        "ShinyGem" => {
                            SfxEvent::Inventory(SfxInventoryEvent::CollectedItem(kind.clone()))
                        },
                        _ => SfxEvent::Inventory(SfxInventoryEvent::Collected),
                    },
                    _ => SfxEvent::Inventory(SfxInventoryEvent::Collected),
                }
            },
            InventoryUpdateEvent::CollectFailed => {
                SfxEvent::Inventory(SfxInventoryEvent::CollectFailed)
            },
            InventoryUpdateEvent::Consumed(consumable) => {
                SfxEvent::Inventory(SfxInventoryEvent::Consumed(consumable.clone()))
            },
            InventoryUpdateEvent::Debug => SfxEvent::Inventory(SfxInventoryEvent::Debug),
            InventoryUpdateEvent::Dropped => SfxEvent::Inventory(SfxInventoryEvent::Dropped),
            InventoryUpdateEvent::Given => SfxEvent::Inventory(SfxInventoryEvent::Given),
            InventoryUpdateEvent::Swapped => SfxEvent::Inventory(SfxInventoryEvent::Swapped),
            _ => SfxEvent::Inventory(SfxInventoryEvent::Swapped),
        }
    }
}

#[derive(Deserialize)]
pub struct SfxTriggerItem {
    /// A list of SFX filepaths for this event
    pub files: Vec<String>,
    /// The time to wait before repeating this SfxEvent
    pub threshold: f32,
}

#[derive(Deserialize, Default)]
pub struct SfxTriggers(HashMap<SfxEvent, SfxTriggerItem>);

impl SfxTriggers {
    pub fn get_trigger(&self, trigger: &SfxEvent) -> Option<&SfxTriggerItem> { self.0.get(trigger) }

    pub fn get_key_value(&self, trigger: &SfxEvent) -> Option<(&SfxEvent, &SfxTriggerItem)> {
        self.0.get_key_value(trigger)
    }
}

pub struct SfxMgr {
    /// This is an `AssetHandle` so it is reloaded automatically
    /// when the manifest is edited.
    pub triggers: AssetHandle<SfxTriggers>,
    event_mapper: SfxEventMapper,
}

impl Default for SfxMgr {
    fn default() -> Self {
        Self {
            triggers: Self::load_sfx_items(),
            event_mapper: SfxEventMapper::new(),
        }
    }
}

impl SfxMgr {
    pub fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        player_entity: specs::Entity,
        camera: &Camera,
        terrain: &Terrain<TerrainChunk>,
        client: &Client,
    ) {
        // Checks if the SFX volume is set to zero or audio is disabled
        // This prevents us from running all the following code unnecessarily
        if !audio.sfx_enabled() {
            return;
        }

        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;

        // Sets the listener position to the camera position facing the
        // same direction as the camera
        audio.set_listener_pos(cam_pos, camera.dependents().cam_dir);

        let triggers = self.triggers.read();

        self.event_mapper.maintain(
            audio,
            state,
            player_entity,
            camera,
            &triggers,
            terrain,
            client,
        );
    }

    pub fn handle_outcome(&mut self, outcome: &Outcome, audio: &mut AudioFrontend) {
        if !audio.sfx_enabled() {
            return;
        }

        match outcome {
            Outcome::Explosion {
                pos,
                power,
                is_attack,
                ..
            } => {
                let file_ref = if *is_attack && *power < 0.0 {
                    "voxygen.audio.sfx.abilities.heal_bomb"
                } else {
                    "voxygen.audio.sfx.explosion"
                };

                audio.play_sfx(
                    // TODO: from sfx config?
                    file_ref,
                    *pos,
                    Some((power.abs() / 2.5).min(1.5)),
                );
            },
            Outcome::ProjectileShot { pos, body, .. } => {
                // TODO: from sfx config?
                match body {
                    Body::Object(
                        object::Body::Arrow | object::Body::MultiArrow | object::Body::ArrowSnake,
                    ) => {
                        let file_ref = vec![
                            "voxygen.audio.sfx.abilities.arrow_shot_1",
                            "voxygen.audio.sfx.abilities.arrow_shot_2",
                            "voxygen.audio.sfx.abilities.arrow_shot_3",
                            "voxygen.audio.sfx.abilities.arrow_shot_4",
                        ][rand::thread_rng().gen_range(1, 4)];

                        audio.play_sfx(file_ref, *pos, None);
                    },
                    Body::Object(
                        object::Body::BoltFire
                        | object::Body::BoltFireBig
                        | object::Body::BoltNature,
                    ) => {
                        let file_ref = vec![
                            "voxygen.audio.sfx.abilities.fire_shot_1",
                            "voxygen.audio.sfx.abilities.fire_shot_2",
                        ][rand::thread_rng().gen_range(1, 2)];

                        audio.play_sfx(file_ref, *pos, None);
                    },
                    _ => {
                        // not mapped to sfx file
                    },
                }
            },
            Outcome::SkillPointGain { pos, .. } => {
                let file_ref = "voxygen.audio.sfx.character.level_up_sound_-_shorter_wind_up";
                audio.play_sfx(file_ref, *pos, None);
            },
            Outcome::Beam { pos, heal } => {
                if *heal {
                    let file_ref = "voxygen.audio.sfx.abilities.staff_channeling";
                    audio.play_sfx(file_ref, *pos, None);
                } else {
                    let file_ref = "voxygen.audio.sfx.abilities.flame_thrower";
                    audio.play_sfx(file_ref, *pos, None);
                }
            },
            _ => {},
        }
    }

    fn load_sfx_items() -> AssetHandle<SfxTriggers> {
        // Cannot fail: A default value is always provided
        SfxTriggers::load_expect("voxygen.audio.sfx")
    }
}

impl assets::Asset for SfxTriggers {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";

    fn default_value(_: &str, error: assets::Error) -> Result<Self, assets::Error> {
        warn!(
            "Error reading sfx config file, sfx will not be available: {:#?}",
            error
        );

        Ok(SfxTriggers::default())
    }
}
