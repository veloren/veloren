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

use common::{
    assets,
    comp::{
        item::{ItemKind, ToolKind},
        object, Body, CharacterAbilityType, InventoryUpdateEvent,
    },
    event::EventBus,
    outcome::Outcome,
    state::State,
    terrain::TerrainChunk,
};
use event_mapper::SfxEventMapper;
use hashbrown::HashMap;
use rand::prelude::*;
use serde::Deserialize;
use specs::WorldExt;
use tracing::{debug, warn};
use vek::*;

/// We watch the states of nearby entities in order to emit SFX at their
/// position based on their state. This constant limits the radius that we
/// observe to prevent tracking distant entities. It approximates the distance
/// at which the volume of the sfx emitted is too quiet to be meaningful for the
/// player.
const SFX_DIST_LIMIT_SQR: f32 = 20000.0;

pub struct SfxEventItem {
    pub sfx: SfxEvent,
    pub pos: Option<Vec3<f32>>,
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
    CollectFailed,
    Consumed(String),
    Debug,
    Dropped,
    Given,
    Swapped,
}

impl From<&InventoryUpdateEvent> for SfxEvent {
    fn from(value: &InventoryUpdateEvent) -> Self {
        match value {
            InventoryUpdateEvent::Collected(item) => {
                // Handle sound effects for types of collected items, falling back to the
                // default Collected event
                match &item.kind() {
                    ItemKind::Tool(tool) => {
                        SfxEvent::Inventory(SfxInventoryEvent::CollectedTool(tool.kind.clone()))
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
    pub files: Vec<String>,
    pub threshold: f64,
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
    triggers: SfxTriggers,
    event_mapper: SfxEventMapper,
}

impl SfxMgr {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            triggers: Self::load_sfx_items(),
            event_mapper: SfxEventMapper::new(),
        }
    }

    pub fn maintain(
        &mut self,
        audio: &mut AudioFrontend,
        state: &State,
        player_entity: specs::Entity,
        camera: &Camera,
        terrain: &Terrain<TerrainChunk>,
    ) {
        if !audio.sfx_enabled() {
            return;
        }

        let ecs = state.ecs();
        let focus_off = camera.get_focus_pos().map(f32::trunc);
        let cam_pos = camera.dependents().cam_pos + focus_off;

        audio.set_listener_pos(cam_pos, camera.dependents().cam_dir);

        // TODO: replace; deprecated in favor of outcomes
        self.event_mapper
            .maintain(state, player_entity, camera, &self.triggers, terrain);

        // TODO: replace; deprecated in favor of outcomes
        let events = ecs.read_resource::<EventBus<SfxEventItem>>().recv_all();

        for event in events {
            let position = match event.pos {
                Some(pos) => pos,
                _ => cam_pos,
            };

            if let Some(item) = self.triggers.get_trigger(&event.sfx) {
                let sfx_file = match item.files.len() {
                    0 => {
                        debug!("Sfx event {:?} is missing audio file.", event.sfx);
                        "voxygen.audio.sfx.placeholder"
                    },
                    1 => item
                        .files
                        .last()
                        .expect("Failed to determine sound file for this trigger item."),
                    _ => {
                        let rand_step = rand::random::<usize>() % item.files.len();
                        &item.files[rand_step]
                    },
                };

                audio.play_sfx(sfx_file, position, event.vol);
            } else {
                debug!("Missing sfx trigger config for sfx event. {:?}", event.sfx);
            }
        }
    }

    pub fn handle_outcome(&mut self, outcome: &Outcome, audio: &mut AudioFrontend) {
        if !audio.sfx_enabled() {
            return;
        }

        match outcome {
            Outcome::Explosion { pos, power, .. } => {
                audio.play_sfx(
                    // TODO: from sfx config?
                    "voxygen.audio.sfx.explosion",
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
                    Body::Object(object::Body::BoltFire | object::Body::BoltFireBig) => {
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
        }
    }

    fn load_sfx_items() -> SfxTriggers {
        match assets::load_file("voxygen.audio.sfx", &["ron"]) {
            Ok(file) => match ron::de::from_reader(file) {
                Ok(config) => config,
                Err(error) => {
                    warn!(
                        "Error parsing sfx config file, sfx will not be available: {}",
                        format!("{:#?}", error)
                    );

                    SfxTriggers::default()
                },
            },
            Err(error) => {
                warn!(
                    "Error reading sfx config file, sfx will not be available: {}",
                    format!("{:#?}", error)
                );

                SfxTriggers::default()
            },
        }
    }
}
