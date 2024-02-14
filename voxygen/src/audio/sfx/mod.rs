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
//! Run(Grass): ( // depends on underfoot block
//!    files: [
//!        "voxygen.audio.sfx.footsteps.stepgrass_1",
//!        "voxygen.audio.sfx.footsteps.stepgrass_2",
//!        "voxygen.audio.sfx.footsteps.stepgrass_3",
//!        "voxygen.audio.sfx.footsteps.stepgrass_4",
//!        "voxygen.audio.sfx.footsteps.stepgrass_5",
//!        "voxygen.audio.sfx.footsteps.stepgrass_6",
//!    ],
//!    threshold: 1.6, // travelled distance before next play
//! ),
//! Wield(Sword): ( // depends on the player's weapon
//!    files: [
//!        "voxygen.audio.sfx.weapon.sword_out",
//!    ],
//!    threshold: 0.5, // wait 0.5s between plays
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
//! ```

mod event_mapper;

use specs::WorldExt;

use crate::{
    audio::AudioFrontend,
    scene::{Camera, Terrain},
};

use client::Client;
use common::{
    assets::{self, AssetExt, AssetHandle},
    comp::{
        beam, biped_large, biped_small, bird_large, humanoid,
        item::{item_key::ItemKey, AbilitySpec, ItemDefinitionId, ItemKind, ToolKind},
        object,
        poise::PoiseState,
        quadruped_low, quadruped_medium, quadruped_small, Body, CharacterAbilityType, Health,
        InventoryUpdateEvent, UtteranceKind,
    },
    outcome::Outcome,
    terrain::{BlockKind, SpriteKind, TerrainChunk},
    uid::Uid,
    DamageSource,
};
use common_state::State;
use event_mapper::SfxEventMapper;
use hashbrown::HashMap;
use rand::prelude::*;
use serde::Deserialize;
use tracing::{debug, warn};
use vek::*;

/// We watch the states of nearby entities in order to emit SFX at their
/// position based on their state. This constant limits the radius that we
/// observe to prevent tracking distant entities. It approximates the distance
/// at which the volume of the sfx emitted is too quiet to be meaningful for the
/// player.
const SFX_DIST_LIMIT_SQR: f32 = 20000.0;

#[derive(Clone, Debug, PartialEq, Deserialize, Hash, Eq)]
pub enum SfxEvent {
    Campfire,
    Embers,
    Birdcall,
    Owl,
    Cricket1,
    Cricket2,
    Cricket3,
    Frog,
    Bees,
    RunningWaterSlow,
    RunningWaterFast,
    Lavapool,
    Idle,
    Swim,
    Run(BlockKind),
    QuadRun(BlockKind),
    Roll,
    Sneak,
    Climb,
    GliderOpen,
    Glide,
    GliderClose,
    CatchAir,
    Jump,
    Fall,
    Attack(CharacterAbilityType, ToolKind),
    Wield(ToolKind),
    Unwield(ToolKind),
    Inventory(SfxInventoryEvent),
    Explosion,
    Damage,
    Death,
    Parry,
    Block,
    BreakBlock,
    SceptreBeam,
    SkillPointGain,
    ArrowHit,
    ArrowMiss,
    ArrowShot,
    FireShot,
    FlameThrower,
    PoiseChange(PoiseState),
    GroundSlam,
    FlashFreeze,
    GigaRoar,
    IceSpikes,
    IceCrack,
    Utterance(UtteranceKind, VoiceKind),
    Lightning,
    CyclopsCharge,
    LaserBeam,
    Steam,
    FuseCharge,
    Music(ToolKind, AbilitySpec),
    Yeet,
    Klonk,
    SmashKlonk,
    FireShockwave,
    DeepLaugh,
    Whoosh,
    Swoosh,
    GroundDig,
    PortalActivated,
    TeleportedByPortal,
    FromTheAshes,
}

#[derive(Copy, Clone, Debug, PartialEq, Deserialize, Hash, Eq)]
pub enum VoiceKind {
    HumanFemale,
    HumanMale,
    BipedLarge,
    Wendigo,
    Reptile,
    Bird,
    Critter,
    Sheep,
    Pig,
    Cow,
    Canine,
    Dagon,
    Lion,
    Mindflayer,
    Marlin,
    Maneater,
    Adlet,
    Antelope,
    Alligator,
    SeaCrocodile,
    Saurok,
    Cat,
    Goat,
    Mandragora,
    Asp,
    Fungome,
    Truffler,
    Wolf,
    Wyvern,
    Phoenix,
}

fn body_to_voice(body: &Body) -> Option<VoiceKind> {
    Some(match body {
        Body::Humanoid(body) => match &body.body_type {
            humanoid::BodyType::Female => VoiceKind::HumanFemale,
            humanoid::BodyType::Male => VoiceKind::HumanMale,
        },
        Body::QuadrupedLow(body) => match body.species {
            quadruped_low::Species::Maneater => VoiceKind::Maneater,
            quadruped_low::Species::Alligator | quadruped_low::Species::HermitAlligator => {
                VoiceKind::Alligator
            },
            quadruped_low::Species::SeaCrocodile => VoiceKind::SeaCrocodile,
            quadruped_low::Species::Dagon => VoiceKind::Dagon,
            quadruped_low::Species::Asp => VoiceKind::Asp,
            _ => return None,
        },
        Body::QuadrupedSmall(body) => match body.species {
            quadruped_small::Species::Truffler => VoiceKind::Truffler,
            quadruped_small::Species::Fungome => VoiceKind::Fungome,
            quadruped_small::Species::Sheep => VoiceKind::Sheep,
            quadruped_small::Species::Pig | quadruped_small::Species::Boar => VoiceKind::Pig,
            quadruped_small::Species::Cat => VoiceKind::Cat,
            quadruped_small::Species::Goat => VoiceKind::Goat,
            _ => VoiceKind::Critter,
        },
        Body::QuadrupedMedium(body) => match body.species {
            quadruped_medium::Species::Saber
            | quadruped_medium::Species::Tiger
            | quadruped_medium::Species::Lion
            | quadruped_medium::Species::Frostfang
            | quadruped_medium::Species::Snowleopard => VoiceKind::Lion,
            quadruped_medium::Species::Wolf => VoiceKind::Wolf,
            quadruped_medium::Species::Roshwalr
            | quadruped_medium::Species::Tarasque
            | quadruped_medium::Species::Darkhound
            | quadruped_medium::Species::Bonerattler
            | quadruped_medium::Species::Grolgar => VoiceKind::Canine,
            quadruped_medium::Species::Cattle
            | quadruped_medium::Species::Catoblepas
            | quadruped_medium::Species::Highland
            | quadruped_medium::Species::Yak
            | quadruped_medium::Species::Moose
            | quadruped_medium::Species::Dreadhorn => VoiceKind::Cow,
            quadruped_medium::Species::Antelope => VoiceKind::Antelope,
            _ => return None,
        },
        Body::BirdMedium(_) => VoiceKind::Bird,
        Body::BirdLarge(body) => match body.species {
            bird_large::Species::CloudWyvern
            | bird_large::Species::FlameWyvern
            | bird_large::Species::FrostWyvern
            | bird_large::Species::SeaWyvern
            | bird_large::Species::WealdWyvern => VoiceKind::Wyvern,
            bird_large::Species::Phoenix => VoiceKind::Phoenix,
            _ => VoiceKind::Bird,
        },
        Body::BipedSmall(body) => match body.species {
            biped_small::Species::Adlet => VoiceKind::Adlet,
            biped_small::Species::Mandragora => VoiceKind::Mandragora,
            biped_small::Species::Flamekeeper => VoiceKind::BipedLarge,
            _ => return None,
        },
        Body::BipedLarge(body) => match body.species {
            biped_large::Species::Wendigo => VoiceKind::Wendigo,
            biped_large::Species::Occultsaurok
            | biped_large::Species::Mightysaurok
            | biped_large::Species::Slysaurok => VoiceKind::Saurok,
            biped_large::Species::Mindflayer => VoiceKind::Mindflayer,
            _ => VoiceKind::BipedLarge,
        },
        Body::Theropod(_) | Body::Dragon(_) => VoiceKind::Reptile,
        Body::FishSmall(_) | Body::FishMedium(_) => VoiceKind::Marlin,
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq, Deserialize, Hash, Eq)]
pub enum SfxInventoryEvent {
    Collected,
    CollectedTool(ToolKind),
    CollectedItem(String),
    CollectFailed,
    Consumed(ItemKey),
    Debug,
    Dropped,
    Given,
    Swapped,
    Craft,
}

// TODO Move to a separate event mapper?
impl From<&InventoryUpdateEvent> for SfxEvent {
    fn from(value: &InventoryUpdateEvent) -> Self {
        match value {
            InventoryUpdateEvent::Collected(item) => {
                // Handle sound effects for types of collected items, falling
                // back to the default Collected event
                match &*item.kind() {
                    ItemKind::Tool(tool) => {
                        SfxEvent::Inventory(SfxInventoryEvent::CollectedTool(tool.kind))
                    },
                    ItemKind::Ingredient { .. }
                        if matches!(
                            item.item_definition_id(),
                            ItemDefinitionId::Simple(id) if id.contains("mineral.gem.")
                        ) =>
                    {
                        SfxEvent::Inventory(SfxInventoryEvent::CollectedItem(String::from(
                            "Gemstone",
                        )))
                    },
                    _ => SfxEvent::Inventory(SfxInventoryEvent::Collected),
                }
            },
            InventoryUpdateEvent::BlockCollectFailed { .. }
            | InventoryUpdateEvent::EntityCollectFailed { .. } => {
                SfxEvent::Inventory(SfxInventoryEvent::CollectFailed)
            },
            InventoryUpdateEvent::Consumed(consumable) => {
                SfxEvent::Inventory(SfxInventoryEvent::Consumed(consumable.clone()))
            },
            InventoryUpdateEvent::Debug => SfxEvent::Inventory(SfxInventoryEvent::Debug),
            InventoryUpdateEvent::Dropped => SfxEvent::Inventory(SfxInventoryEvent::Dropped),
            InventoryUpdateEvent::Given => SfxEvent::Inventory(SfxInventoryEvent::Given),
            InventoryUpdateEvent::Swapped => SfxEvent::Inventory(SfxInventoryEvent::Swapped),
            InventoryUpdateEvent::Craft => SfxEvent::Inventory(SfxInventoryEvent::Craft),
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

    #[serde(default)]
    pub subtitle: Option<String>,
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
        if !audio.sfx_enabled() && !audio.subtitles_enabled {
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

    #[allow(clippy::single_match)]
    pub fn handle_outcome(
        &mut self,
        outcome: &Outcome,
        audio: &mut AudioFrontend,
        client: &Client,
        underwater: bool,
    ) {
        if !audio.sfx_enabled() && !audio.subtitles_enabled {
            return;
        }
        let triggers = self.triggers.read();
        let uids = client.state().ecs().read_storage::<Uid>();

        // TODO handle underwater
        match outcome {
            Outcome::Explosion { pos, power, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Explosion);
                audio.emit_sfx(
                    sfx_trigger_item,
                    *pos,
                    Some((power.abs() / 2.5).min(1.5)),
                    underwater,
                );
            },
            Outcome::Lightning { pos } => {
                let power = (1.0 - pos.distance(audio.listener.pos) / 5_000.0)
                    .max(0.0)
                    .powi(7);
                if power > 0.0 {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Lightning);
                    // TODO: Don't use UI sfx, add a way to control position falloff
                    audio.emit_ui_sfx(sfx_trigger_item, Some((power * 3.0).min(2.9)));
                }
            },
            Outcome::GroundSlam { pos, .. } | Outcome::ClayGolemDash { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GroundSlam);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::LaserBeam { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::LaserBeam);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::CyclopsCharge { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::CyclopsCharge);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::FlamethrowerCharge { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::CyclopsCharge);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::FuseCharge { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FuseCharge);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::FlashFreeze { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FlashFreeze);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::SummonedCreature { pos, body, .. } => {
                match body {
                    Body::BipedSmall(body) => match body.species {
                        biped_small::Species::Clockwork => {
                            let sfx_trigger_item = triggers.get_key_value(&SfxEvent::DeepLaugh);
                            audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
                        },
                        biped_small::Species::Boreal => {
                            let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GigaRoar);
                            audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
                        },
                        _ => {},
                    },
                    Body::Object(object::Body::Flamethrower) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::DeepLaugh);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
                    },
                    Body::Object(object::Body::Tornado)
                    | Body::Object(object::Body::FieryTornado) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Swoosh);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
                    },
                    _ => { // not mapped to sfx file
                    },
                }
            },
            Outcome::GroundDig { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GroundDig);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::PortalActivated { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::PortalActivated);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::TeleportedByPortal { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::TeleportedByPortal);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::IceSpikes { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::IceSpikes);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::IceCrack { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::IceCrack);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::Steam { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Steam);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::FireShockwave { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FlameThrower);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::FromTheAshes { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FromTheAshes);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
            },
            Outcome::ProjectileShot { pos, body, .. } => {
                match body {
                    Body::Object(
                        object::Body::Arrow
                        | object::Body::MultiArrow
                        | object::Body::ArrowSnake
                        | object::Body::ArrowTurret
                        | object::Body::ArrowClay
                        | object::Body::SpectralSwordSmall
                        | object::Body::SpectralSwordLarge,
                    ) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::ArrowShot);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, underwater);
                    },
                    Body::Object(
                        object::Body::BoltFire
                        | object::Body::BoltFireBig
                        | object::Body::BoltNature
                        | object::Body::BoltIcicle
                        | object::Body::SpearIcicle
                        | object::Body::GrenadeClay
                        | object::Body::SpitPoison,
                    ) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FireShot);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, underwater);
                    },
                    Body::Object(object::Body::LaserBeam)
                    | Body::Object(object::Body::LightningBolt) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::LaserBeam);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, underwater);
                    },
                    Body::Object(object::Body::AdletTrap | object::Body::Mine) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Yeet);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, underwater);
                    },
                    _ => {
                        // not mapped to sfx file
                    },
                }
            },
            Outcome::ProjectileHit {
                pos,
                body,
                source,
                target,
                ..
            } => match body {
                Body::Object(
                    object::Body::Arrow
                    | object::Body::MultiArrow
                    | object::Body::ArrowSnake
                    | object::Body::ArrowTurret
                    | object::Body::ArrowClay
                    | object::Body::SpectralSwordSmall
                    | object::Body::SpectralSwordLarge,
                ) => {
                    if target.is_none() {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::ArrowMiss);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
                    } else if *source == client.uid() {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::ArrowHit);
                        audio.emit_sfx(
                            sfx_trigger_item,
                            client.position().unwrap_or(*pos),
                            Some(2.0),
                            underwater,
                        );
                    } else {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::ArrowHit);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
                    }
                },
                Body::Object(
                    object::Body::AdletTrap | object::Body::Mine | object::Body::Pebble,
                ) => {
                    if target.is_none() {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Klonk);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
                    } else if *source == client.uid() {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SmashKlonk);
                        audio.emit_sfx(
                            sfx_trigger_item,
                            client.position().unwrap_or(*pos),
                            Some(2.0),
                            underwater,
                        );
                    } else {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SmashKlonk);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), underwater);
                    }
                },
                _ => {},
            },
            Outcome::SkillPointGain { uid, .. } => {
                if let Some(client_uid) = uids.get(client.entity()) {
                    if uid == client_uid {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SkillPointGain);
                        audio.emit_ui_sfx(sfx_trigger_item, Some(0.4));
                    }
                }
            },
            Outcome::Beam { pos, specifier } => match specifier {
                beam::FrontendSpecifier::LifestealBeam
                | beam::FrontendSpecifier::Steam
                | beam::FrontendSpecifier::Poison
                | beam::FrontendSpecifier::Ink
                | beam::FrontendSpecifier::Lightning
                | beam::FrontendSpecifier::Frost
                | beam::FrontendSpecifier::Bubbles => {
                    if thread_rng().gen_bool(0.5) {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SceptreBeam);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, underwater);
                    };
                },
                beam::FrontendSpecifier::Flamethrower
                | beam::FrontendSpecifier::Cultist
                | beam::FrontendSpecifier::PhoenixLaser => {
                    if thread_rng().gen_bool(0.5) {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FlameThrower);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, underwater);
                    }
                },
                beam::FrontendSpecifier::Gravewarden | beam::FrontendSpecifier::WebStrand => {},
            },
            Outcome::SpriteUnlocked { pos } => {
                // TODO: Dedicated sound effect!
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GliderOpen);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e as f32 + 0.5),
                    Some(2.0),
                    underwater,
                );
            },
            Outcome::FailedSpriteUnlock { pos } => {
                // TODO: Dedicated sound effect!
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::BreakBlock);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e as f32 + 0.5),
                    Some(2.0),
                    underwater,
                );
            },
            Outcome::BreakBlock { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::BreakBlock);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e as f32 + 0.5),
                    Some(3.0),
                    underwater,
                );
            },
            Outcome::HealthChange { pos, info, .. } => {
                // Ignore positive damage (healing) and buffs for now
                if info.amount < Health::HEALTH_EPSILON
                    && !matches!(info.cause, Some(DamageSource::Buff(_)))
                {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Damage);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), underwater);
                }
            },
            Outcome::Death { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Death);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), underwater);
            },
            Outcome::Block { pos, parry, .. } => {
                if *parry {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Parry);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), underwater);
                } else {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Block);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), underwater);
                }
            },
            Outcome::PoiseChange { pos, state, .. } => match state {
                PoiseState::Normal => {},
                PoiseState::Interrupted => {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::PoiseChange(PoiseState::Interrupted));
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), underwater);
                },
                PoiseState::Stunned => {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::PoiseChange(PoiseState::Stunned));
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), underwater);
                },
                PoiseState::Dazed => {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::PoiseChange(PoiseState::Dazed));
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), underwater);
                },
                PoiseState::KnockedDown => {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::PoiseChange(PoiseState::KnockedDown));
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), underwater);
                },
            },
            Outcome::Utterance { pos, kind, body } => {
                if let Some(voice) = body_to_voice(body) {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::Utterance(*kind, voice));
                    if let Some(sfx_trigger_item) = sfx_trigger_item {
                        audio.emit_sfx(Some(sfx_trigger_item), *pos, Some(1.5), underwater);
                    } else {
                        debug!(
                            "No utterance sound effect exists for ({:?}, {:?})",
                            kind, voice
                        );
                    }
                }
            },
            Outcome::Glider { pos, wielded } => {
                if *wielded {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GliderOpen);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.0), underwater);
                } else {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GliderClose);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.0), underwater);
                }
            },
            Outcome::SpriteDelete { pos, sprite } => {
                match sprite {
                    SpriteKind::SeaUrchin => {
                        let pos = pos.map(|e| e as f32 + 0.5);
                        let power = (0.6 - pos.distance(audio.listener.pos) / 5_000.0)
                            .max(0.0)
                            .powi(7);
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Explosion);
                        audio.emit_sfx(
                            sfx_trigger_item,
                            pos,
                            Some((power.abs() / 2.5).min(0.3)),
                            underwater,
                        );
                    },
                    _ => {},
                };
            },
            Outcome::Whoosh { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Whoosh);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e + 0.5),
                    Some(3.0),
                    underwater,
                );
            },
            Outcome::Swoosh { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Swoosh);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e + 0.5),
                    Some(3.0),
                    underwater,
                );
            },
            Outcome::Slash { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SmashKlonk);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e + 0.5),
                    Some(3.0),
                    underwater,
                );
            },
            Outcome::ExpChange { .. } | Outcome::ComboChange { .. } => {},
        }
    }

    fn load_sfx_items() -> AssetHandle<SfxTriggers> {
        SfxTriggers::load_or_insert_with("voxygen.audio.sfx", |error| {
            warn!(
                "Error reading sfx config file, sfx will not be available: {:#?}",
                error
            );

            SfxTriggers::default()
        })
    }
}

impl assets::Asset for SfxTriggers {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_sfx_triggers() { let _ = SfxTriggers::load_expect("voxygen.audio.sfx"); }
}
