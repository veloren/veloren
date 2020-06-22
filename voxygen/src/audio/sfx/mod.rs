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
//! Attack(TripleStrike(First), Sword): (
//!     files: [
//!         "voxygen.audio.sfx.weapon.sword_03",
//!         "voxygen.audio.sfx.weapon.sword_04",
//!     ],
//!     threshold: 0.5,
//! ),
//! ```

mod event_mapper;

use crate::audio::AudioFrontend;
use common::{
    assets,
    comp::{Ori, Pos},
    event::{EventBus, SfxEvent, SfxEventItem},
    state::State,
};
use event_mapper::SfxEventMapper;
use hashbrown::HashMap;
use serde::Deserialize;
use specs::WorldExt;
use tracing::warn;
use vek::*;

/// We watch the states of nearby entities in order to emit SFX at their
/// position based on their state. This constant limits the radius that we
/// observe to prevent tracking distant entities. It approximates the distance
/// at which the volume of the sfx emitted is too quiet to be meaningful for the
/// player.
const SFX_DIST_LIMIT_SQR: f32 = 20000.0;

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
    ) {
        if !audio.sfx_enabled() {
            return;
        }

        self.event_mapper
            .maintain(state, player_entity, &self.triggers);

        let ecs = state.ecs();

        let player_position = ecs
            .read_storage::<Pos>()
            .get(player_entity)
            .map_or(Vec3::zero(), |pos| pos.0);

        let player_ori = *ecs
            .read_storage::<Ori>()
            .get(player_entity)
            .copied()
            .unwrap_or_default()
            .0;

        audio.set_listener_pos(&player_position, &player_ori);

        let events = ecs.read_resource::<EventBus<SfxEventItem>>().recv_all();

        for event in events {
            let position = match event.pos {
                Some(pos) => pos,
                _ => player_position,
            };

            if let Some(item) = self.triggers.get_trigger(&event.sfx) {
                let sfx_file = match item.files.len() {
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
            }
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
