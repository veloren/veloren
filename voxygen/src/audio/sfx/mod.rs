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
    audio::{
        AudioFrontend,
        channel::{SFX_DIST_LIMIT_SQR, UiChannelTag},
    },
    scene::{Camera, Terrain},
};

use client::Client;
use common::{
    DamageSource,
    assets::{self, AssetExt, AssetHandle},
    comp::{
        Body, CharacterAbilityType, Health, InventoryUpdateEvent, UtteranceKind, beam, biped_large,
        biped_small, bird_large, bird_medium, crustacean, humanoid,
        item::{AbilitySpec, ItemDefinitionId, ItemDesc, ItemKind, ToolKind, item_key::ItemKey},
        object,
        poise::PoiseState,
        quadruped_low, quadruped_medium, quadruped_small,
    },
    outcome::Outcome,
    terrain::{BlockKind, SpriteKind, TerrainChunk},
    uid::Uid,
    vol::ReadVol,
};
use common_state::State;
use event_mapper::SfxEventMapper;
use hashbrown::HashMap;
use rand::prelude::*;
use serde::Deserialize;
use tracing::{debug, error, warn};
use vek::*;

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
    SplashSmall,
    SplashMedium,
    SplashBig,
    Run(BlockKind),
    QuadRun(BlockKind),
    Roll,
    RollCancel,
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
    PickaxeDamage,
    PickaxeDamageStrong,
    PickaxeBreakBlock,
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
    TerracottaStatueCharge,
    LaserBeam,
    Steam,
    FuseCharge,
    Music(ToolKind, AbilitySpec),
    Yeet,
    Hiss,
    LongHiss,
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
    SurpriseEgg,
    Transformation,
    Bleep,
    Charge,
    StrigoiHead,
    BloodmoonHeiressSummon,
    TrainChugg,
    TrainChuggSteam,
    TrainAmbience,
    TrainClack,
    TrainSpeed,
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
    VampireBat,
    Legoom,
}

fn body_to_voice(body: &Body) -> Option<VoiceKind> {
    Some(match body {
        Body::Humanoid(body) => match &body.body_type {
            humanoid::BodyType::Female => VoiceKind::HumanFemale,
            humanoid::BodyType::Male => VoiceKind::HumanMale,
        },
        Body::QuadrupedLow(body) => match body.species {
            quadruped_low::Species::Maneater => VoiceKind::Maneater,
            quadruped_low::Species::Alligator | quadruped_low::Species::Snaretongue => {
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
        Body::BirdMedium(body) => match body.species {
            bird_medium::Species::BloodmoonBat | bird_medium::Species::VampireBat => {
                VoiceKind::VampireBat
            },
            _ => VoiceKind::Bird,
        },
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
            biped_small::Species::GreenLegoom
            | biped_small::Species::OchreLegoom
            | biped_small::Species::PurpleLegoom
            | biped_small::Species::RedLegoom => VoiceKind::Legoom,
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

#[derive(Deserialize, Debug)]
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

        let cam_pos = camera.get_pos_with_focus();

        // Sets the listener position to the camera position facing the
        // same direction as the camera
        audio.set_listener_pos(cam_pos, camera.dependents().cam_dir);

        let triggers = self.triggers.read();

        let underwater = state
            .terrain()
            .get(cam_pos.map(|e| e.floor() as i32))
            .map(|b| b.is_liquid())
            .unwrap_or(false);

        if underwater {
            audio.set_sfx_master_filter(888);
        } else {
            audio.set_sfx_master_filter(20000);
        }

        // Update continuing sounds with player position
        if let Some(inner) = audio.inner.as_mut() {
            let player_pos = client.position().unwrap_or_default();
            inner.channels.sfx.iter_mut().for_each(|c| {
                if !c.is_done() {
                    c.update(player_pos)
                }
            })
        }

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

    #[expect(clippy::single_match)]
    pub fn handle_outcome(
        &mut self,
        outcome: &Outcome,
        audio: &mut AudioFrontend,
        client: &Client,
    ) {
        if !audio.sfx_enabled() && !audio.subtitles_enabled {
            return;
        }
        let triggers = self.triggers.read();
        let uids = client.state().ecs().read_storage::<Uid>();
        let player_pos = client.position().unwrap_or_default();
        if audio.get_listener().is_none() {
            return;
        }
        match outcome {
            Outcome::Explosion { pos, power, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Explosion);
                audio.emit_sfx(
                    sfx_trigger_item,
                    *pos,
                    Some((power.abs() / 2.5).min(1.5)),
                    player_pos,
                );
            },
            Outcome::Lightning { pos } => {
                let distance = pos.distance(audio.get_listener_pos());
                let power = (1.0 - distance / 6_000.0).max(0.0).powi(7);
                if power > 0.0 {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Lightning);
                    let volume = (power * 3.0).min(2.9);
                    // Delayed based on distance / speed of sound (approxmately 340 m/s)
                    audio.play_ambience_oneshot(
                        super::channel::AmbienceChannelTag::Thunder,
                        sfx_trigger_item,
                        Some(volume),
                        Some(distance / 340.0),
                    );
                }
            },
            Outcome::GroundSlam { pos, .. } | Outcome::ClayGolemDash { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GroundSlam);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::SurpriseEgg { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SurpriseEgg);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::Transformation { pos, .. } => {
                // TODO: Give this a sound
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Transformation);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::LaserBeam { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::LaserBeam);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::CyclopsCharge { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::CyclopsCharge);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::FlamethrowerCharge { pos, .. }
            | Outcome::TerracottaStatueCharge { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::CyclopsCharge);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::FuseCharge { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FuseCharge);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::Charge { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::CyclopsCharge);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::FlashFreeze { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FlashFreeze);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::SummonedCreature { pos, body, .. } => {
                match body {
                    Body::BipedSmall(body) => match body.species {
                        biped_small::Species::IronDwarf => {
                            let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Bleep);
                            audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                        },
                        biped_small::Species::Boreal | biped_small::Species::Ashen => {
                            let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GigaRoar);
                            audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                        },
                        biped_small::Species::ShamanicSpirit | biped_small::Species::Jiangshi => {
                            let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Klonk);
                            audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                        },
                        _ => {},
                    },
                    Body::BipedLarge(body) => match body.species {
                        biped_large::Species::TerracottaBesieger
                        | biped_large::Species::TerracottaPursuer => {
                            let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Klonk);
                            audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                        },
                        _ => {},
                    },
                    Body::BirdMedium(body) => match body.species {
                        bird_medium::Species::Bat => {
                            let sfx_trigger_item =
                                triggers.get_key_value(&SfxEvent::BloodmoonHeiressSummon);
                            audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                        },
                        _ => {},
                    },
                    Body::Crustacean(body) => match body.species {
                        crustacean::Species::SoldierCrab => {
                            let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Hiss);
                            audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                        },
                        _ => {},
                    },
                    Body::Object(object::Body::Lavathrower) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::DeepLaugh);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                    },
                    Body::Object(object::Body::SeaLantern) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::LongHiss);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                    },
                    Body::Object(object::Body::Tornado)
                    | Body::Object(object::Body::FieryTornado) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Swoosh);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                    },
                    _ => { // not mapped to sfx file
                    },
                }
            },
            Outcome::GroundDig { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GroundDig);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::PortalActivated { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::PortalActivated);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::TeleportedByPortal { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::TeleportedByPortal);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::IceSpikes { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::IceSpikes);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::IceCrack { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::IceCrack);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::Steam { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Steam);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::FireShockwave { pos, .. } | Outcome::FireLowShockwave { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FlameThrower);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::FromTheAshes { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FromTheAshes);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
            },
            Outcome::ProjectileShot { pos, body, .. } => {
                match body {
                    Body::Object(
                        object::Body::Arrow
                        | object::Body::MultiArrow
                        | object::Body::ArrowSnake
                        | object::Body::ArrowTurret
                        | object::Body::ArrowClay
                        | object::Body::BoltBesieger
                        | object::Body::HarlequinDagger
                        | object::Body::SpectralSwordSmall
                        | object::Body::SpectralSwordLarge,
                    ) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::ArrowShot);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, player_pos);
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
                        audio.emit_sfx(sfx_trigger_item, *pos, None, player_pos);
                    },
                    Body::Object(
                        object::Body::IronPikeBomb
                        | object::Body::BubbleBomb
                        | object::Body::MinotaurAxe
                        | object::Body::Pebble,
                    ) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Whoosh);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, player_pos);
                    },
                    Body::Object(
                        object::Body::LaserBeam
                        | object::Body::LaserBeamSmall
                        | object::Body::LightningBolt,
                    ) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::LaserBeam);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, player_pos);
                    },
                    Body::Object(
                        object::Body::AdletTrap | object::Body::BorealTrap | object::Body::Mine,
                    ) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Yeet);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, player_pos);
                    },
                    Body::Object(object::Body::StrigoiHead) => {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::StrigoiHead);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, player_pos);
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
                    | object::Body::BoltBesieger
                    | object::Body::HarlequinDagger
                    | object::Body::SpectralSwordSmall
                    | object::Body::SpectralSwordLarge
                    | object::Body::Pebble,
                ) => {
                    if target.is_none() {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::ArrowMiss);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                    } else if *source == client.uid() {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::ArrowHit);
                        audio.emit_sfx(
                            sfx_trigger_item,
                            client.position().unwrap_or(*pos),
                            Some(2.0),
                            player_pos,
                        );
                    } else {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::ArrowHit);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                    }
                },
                Body::Object(
                    object::Body::AdletTrap
                    | object::Body::BorealTrap
                    | object::Body::Mine
                    | object::Body::StrigoiHead,
                ) => {
                    if target.is_none() {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Klonk);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                    } else if *source == client.uid() {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SmashKlonk);
                        audio.emit_sfx(
                            sfx_trigger_item,
                            client.position().unwrap_or(*pos),
                            Some(2.0),
                            player_pos,
                        );
                    } else {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SmashKlonk);
                        audio.emit_sfx(sfx_trigger_item, *pos, Some(2.0), player_pos);
                    }
                },
                _ => {},
            },
            Outcome::SkillPointGain { uid, .. } => {
                if let Some(client_uid) = uids.get(client.entity()) {
                    if uid == client_uid {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SkillPointGain);
                        audio.emit_ui_sfx(sfx_trigger_item, Some(0.4), Some(UiChannelTag::LevelUp));
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
                    if rand::rng().random_bool(0.5) {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SceptreBeam);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, player_pos);
                    };
                },
                beam::FrontendSpecifier::Flamethrower
                | beam::FrontendSpecifier::Cultist
                | beam::FrontendSpecifier::PhoenixLaser
                | beam::FrontendSpecifier::FireGigasOverheat
                | beam::FrontendSpecifier::FirePillar => {
                    if rand::rng().random_bool(0.5) {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FlameThrower);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, player_pos);
                    }
                },
                beam::FrontendSpecifier::FlameWallPillar => {
                    if rand::rng().random_bool(0.02) {
                        let sfx_trigger_item = triggers.get_key_value(&SfxEvent::FlameThrower);
                        audio.emit_sfx(sfx_trigger_item, *pos, None, player_pos);
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
                    player_pos,
                );
            },
            Outcome::FailedSpriteUnlock { pos } => {
                // TODO: Dedicated sound effect!
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::BreakBlock);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e as f32 + 0.5),
                    Some(2.0),
                    player_pos,
                );
            },
            Outcome::BreakBlock { pos, tool, .. } => {
                let sfx_trigger_item =
                    triggers.get_key_value(&if matches!(tool, Some(ToolKind::Pick)) {
                        SfxEvent::PickaxeBreakBlock
                    } else {
                        SfxEvent::BreakBlock
                    });
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e as f32 + 0.5),
                    Some(3.0),
                    player_pos,
                );
            },
            Outcome::DamagedBlock {
                pos,
                stage_changed,
                tool,
                ..
            } => {
                let sfx_trigger_item = triggers.get_key_value(&match (stage_changed, tool) {
                    (false, Some(ToolKind::Pick)) => SfxEvent::PickaxeDamage,
                    (true, Some(ToolKind::Pick)) => SfxEvent::PickaxeDamageStrong,
                    // SFX already emitted by ability
                    (_, Some(ToolKind::Shovel)) => return,
                    (_, _) => SfxEvent::BreakBlock,
                });

                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e as f32 + 0.5),
                    Some(if *stage_changed { 3.0 } else { 2.0 }),
                    player_pos,
                );
            },
            Outcome::HealthChange { pos, info, .. } => {
                // Ignore positive damage (healing) and buffs for now
                if info.amount < Health::HEALTH_EPSILON
                    && !matches!(info.cause, Some(DamageSource::Buff(_)))
                {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Damage);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), player_pos);
                }
            },
            Outcome::Death { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Death);
                audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), player_pos);
            },
            Outcome::Block { pos, parry, .. } => {
                if *parry {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Parry);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), player_pos);
                } else {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Block);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), player_pos);
                }
            },
            Outcome::PoiseChange {
                pos,
                state: poise_state,
                ..
            } => match poise_state {
                PoiseState::Normal => {},
                PoiseState::Interrupted => {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::PoiseChange(PoiseState::Interrupted));
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), player_pos);
                },
                PoiseState::Stunned => {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::PoiseChange(PoiseState::Stunned));
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), player_pos);
                },
                PoiseState::Dazed => {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::PoiseChange(PoiseState::Dazed));
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), player_pos);
                },
                PoiseState::KnockedDown => {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::PoiseChange(PoiseState::KnockedDown));
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.5), player_pos);
                },
            },
            Outcome::Utterance { pos, kind, body } => {
                if let Some(voice) = body_to_voice(body) {
                    let sfx_trigger_item =
                        triggers.get_key_value(&SfxEvent::Utterance(*kind, voice));
                    if let Some(sfx_trigger_item) = sfx_trigger_item {
                        audio.emit_sfx(Some(sfx_trigger_item), *pos, Some(1.5), player_pos);
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
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.0), player_pos);
                } else {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::GliderClose);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(1.0), player_pos);
                }
            },
            Outcome::SpriteDelete {
                pos,
                sprite: SpriteKind::SeaUrchin,
            } => {
                let pos = pos.map(|e| e as f32 + 0.5);
                let power = (0.6 - pos.distance(audio.get_listener_pos()) / 5_000.0)
                    .max(0.0)
                    .powi(7);
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Explosion);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos,
                    Some((power.abs() / 2.5).min(0.3)),
                    player_pos,
                );
            },
            Outcome::Whoosh { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Whoosh);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e + 0.5),
                    Some(3.0),
                    player_pos,
                );
            },
            Outcome::Swoosh { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Swoosh);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e + 0.5),
                    Some(3.0),
                    player_pos,
                );
            },
            Outcome::Slash { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::SmashKlonk);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e + 0.5),
                    Some(3.0),
                    player_pos,
                );
            },
            Outcome::Bleep { pos, .. } => {
                let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Bleep);
                audio.emit_sfx(
                    sfx_trigger_item,
                    pos.map(|e| e + 0.5),
                    Some(3.0),
                    player_pos,
                );
            },
            Outcome::HeadLost { uid, .. } => {
                let positions = client.state().ecs().read_storage::<common::comp::Pos>();
                if let Some(pos) = client
                    .state()
                    .ecs()
                    .read_resource::<common::uid::IdMaps>()
                    .uid_entity(*uid)
                    .and_then(|entity| positions.get(entity))
                {
                    let sfx_trigger_item = triggers.get_key_value(&SfxEvent::Death);
                    audio.emit_sfx(sfx_trigger_item, pos.0, Some(2.0), player_pos);
                } else {
                    error!("Couldn't get position of entity that lost head");
                }
            },
            Outcome::Splash { vel, pos, mass, .. } => {
                let magnitude = (-vel.z).max(0.0);
                let energy = mass * magnitude;

                if energy > 0.0 {
                    let (sfx, volume) = if energy < 10.0 {
                        (SfxEvent::SplashSmall, energy / 20.0)
                    } else if energy < 100.0 {
                        (SfxEvent::SplashMedium, (energy - 10.0) / 90.0 + 0.5)
                    } else {
                        (SfxEvent::SplashBig, (energy / 100.0).sqrt() + 0.5)
                    };
                    let sfx_trigger_item = triggers.get_key_value(&sfx);
                    audio.emit_sfx(sfx_trigger_item, *pos, Some(volume.min(2.0)), player_pos);
                }
            },
            Outcome::ExpChange { .. } | Outcome::ComboChange { .. } => {},
            _ => {},
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
