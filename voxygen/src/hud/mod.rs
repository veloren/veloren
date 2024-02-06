mod animation;
mod bag;
mod buffs;
mod buttons;
mod change_notification;
mod chat;
mod crafting;
mod diary;
mod esc_menu;
mod group;
mod hotbar;
mod loot_scroller;
mod map;
mod minimap;
mod overhead;
mod overitem;
mod popup;
mod prompt_dialog;
mod quest;
mod settings_window;
mod skillbar;
mod slots;
mod social;
mod subtitles;
mod trade;

pub mod img_ids;
pub mod item_imgs;
pub mod util;

pub use crafting::CraftingTab;
pub use hotbar::{SlotContents as HotbarSlotContents, State as HotbarState};
pub use item_imgs::animate_by_pulse;
pub use loot_scroller::LootMessage;
pub use settings_window::ScaleChange;
pub use subtitles::Subtitle;

use bag::Bag;
use buffs::BuffsBar;
use buttons::Buttons;
use change_notification::{ChangeNotification, NotificationReason};
use chat::Chat;
use chrono::NaiveTime;
use crafting::Crafting;
use diary::{Diary, SelectedSkillTree};
use esc_menu::EscMenu;
use group::Group;
use img_ids::Imgs;
use item_imgs::ItemImgs;
use loot_scroller::LootScroller;
use map::Map;
use minimap::{MiniMap, VoxelMinimap};
use popup::Popup;
use prompt_dialog::PromptDialog;
use quest::Quest;
use serde::{Deserialize, Serialize};
use settings_window::{SettingsTab, SettingsWindow};
use skillbar::Skillbar;
use social::Social;
use subtitles::Subtitles;
use trade::Trade;

use crate::{
    cmd::get_player_uuid,
    ecs::{
        comp as vcomp,
        comp::{HpFloater, HpFloaterList},
    },
    game_input::GameInput,
    hud::{img_ids::ImgsRot, prompt_dialog::DialogOutcomeEvent},
    render::UiDrawer,
    scene::camera::{self, Camera},
    session::{
        interactable::{BlockInteraction, Interactable},
        settings_change::{
            Audio, Chat as ChatChange, Interface as InterfaceChange, SettingsChange,
        },
    },
    settings::chat::ChatFilter,
    ui::{
        fonts::Fonts, img_ids::Rotations, slot, slot::SlotKey, Graphic, Ingameable, ScaleMode, Ui,
    },
    window::Event as WinEvent,
    GlobalState,
};
use client::Client;
use common::{
    combat,
    comp::{
        self,
        ability::{AuxiliaryAbility, Stance},
        fluid_dynamics,
        inventory::{
            slot::{InvSlotId, Slot},
            trade_pricing::TradePricing,
            CollectFailedReason,
        },
        item::{
            tool::{AbilityContext, ToolKind},
            ItemDefinitionIdOwned, ItemDesc, ItemI18n, MaterialStatManifest, Quality,
        },
        loot_owner::LootOwnerKind,
        pet::is_mountable,
        skillset::{skills::Skill, SkillGroupKind, SkillsPersistenceError},
        BuffData, BuffKind, Health, Item, MapMarkerChange, PresenceKind,
    },
    consts::MAX_PICKUP_RANGE,
    link::Is,
    mounting::{Mount, Rider, VolumePos},
    outcome::Outcome,
    resources::{Secs, Time},
    slowjob::SlowJobPool,
    terrain::{SpriteKind, TerrainChunk, UnlockKind},
    trade::{ReducedInventory, TradeAction},
    uid::Uid,
    util::{srgba_to_linear, Dir},
    vol::RectRasterableVol,
};
use common_base::{prof_span, span};
use common_net::{
    msg::{world_msg::SiteId, Notification},
    sync::WorldSyncExt,
};
use conrod_core::{
    text::cursor::Index,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use hashbrown::{HashMap, HashSet};
use i18n::Localization;
use rand::Rng;
use specs::{Entity as EcsEntity, Join, LendJoin, WorldExt};
use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::warn;
use vek::*;

const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
const TEXT_VELORITE: Color = Color::Rgba(0.0, 0.66, 0.66, 1.0);
const TEXT_BLUE_COLOR: Color = Color::Rgba(0.8, 0.9, 1.0, 1.0);
const TEXT_GRAY_COLOR: Color = Color::Rgba(0.5, 0.5, 0.5, 1.0);
const TEXT_DULL_RED_COLOR: Color = Color::Rgba(0.56, 0.2, 0.2, 1.0);
const TEXT_BG: Color = Color::Rgba(0.0, 0.0, 0.0, 1.0);
const TEXT_COLOR_GREY: Color = Color::Rgba(1.0, 1.0, 1.0, 0.5);
//const TEXT_COLOR_2: Color = Color::Rgba(0.0, 0.0, 0.0, 1.0);
const TEXT_COLOR_3: Color = Color::Rgba(1.0, 1.0, 1.0, 0.1);
const TEXT_BIND_CONFLICT_COLOR: Color = Color::Rgba(1.0, 0.0, 0.0, 1.0);
const BLACK: Color = Color::Rgba(0.0, 0.0, 0.0, 1.0);
//const BG_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 0.8);
const HP_COLOR: Color = Color::Rgba(0.33, 0.63, 0.0, 1.0);
const LOW_HP_COLOR: Color = Color::Rgba(0.93, 0.59, 0.03, 1.0);
const CRITICAL_HP_COLOR: Color = Color::Rgba(0.79, 0.19, 0.17, 1.0);
const STAMINA_COLOR: Color = Color::Rgba(0.29, 0.62, 0.75, 0.9);
const ENEMY_HP_COLOR: Color = Color::Rgba(0.93, 0.1, 0.29, 1.0);
const XP_COLOR: Color = Color::Rgba(0.59, 0.41, 0.67, 1.0);
const POISE_COLOR: Color = Color::Rgba(0.70, 0.0, 0.60, 1.0);
const POISEBAR_TICK_COLOR: Color = Color::Rgba(0.70, 0.90, 0.0, 1.0);
//const TRANSPARENT: Color = Color::Rgba(0.0, 0.0, 0.0, 0.0);
//const FOCUS_COLOR: Color = Color::Rgba(1.0, 0.56, 0.04, 1.0);
//const RAGE_COLOR: Color = Color::Rgba(0.5, 0.04, 0.13, 1.0);
const BUFF_COLOR: Color = Color::Rgba(0.06, 0.69, 0.12, 1.0);
const DEBUFF_COLOR: Color = Color::Rgba(0.79, 0.19, 0.17, 1.0);

// Item Quality Colors
const QUALITY_LOW: Color = Color::Rgba(0.60, 0.60, 0.60, 1.0); // Grey - Trash, can be sold to vendors
const QUALITY_COMMON: Color = Color::Rgba(0.79, 1.00, 1.00, 1.0); // Light blue - Crafting mats, food, starting equipment, quest items (like keys), rewards for easy quests
const QUALITY_MODERATE: Color = Color::Rgba(0.06, 0.69, 0.12, 1.0); // Green - Quest Rewards, commonly looted items from NPCs
const QUALITY_HIGH: Color = Color::Rgba(0.18, 0.32, 0.9, 1.0); // Blue - Dungeon rewards, boss loot, rewards for hard quests
const QUALITY_EPIC: Color = Color::Rgba(0.58, 0.29, 0.93, 1.0); // Purple - Rewards for epic quests and very hard bosses
const QUALITY_LEGENDARY: Color = Color::Rgba(0.92, 0.76, 0.0, 1.0); // Gold - Legendary items that require a big effort to acquire
const QUALITY_ARTIFACT: Color = Color::Rgba(0.74, 0.24, 0.11, 1.0); // Orange - Not obtainable by normal means, "artifacts"
const QUALITY_DEBUG: Color = Color::Rgba(0.79, 0.19, 0.17, 1.0); // Red - Admin and debug items

// Chat Colors
/// Color for chat command errors (yellow !)
const ERROR_COLOR: Color = Color::Rgba(1.0, 1.0, 0.0, 1.0);
/// Color for chat command info (blue i)
const INFO_COLOR: Color = Color::Rgba(0.28, 0.83, 0.71, 1.0);
/// Online color
const ONLINE_COLOR: Color = Color::Rgba(0.3, 1.0, 0.3, 1.0);
/// Offline color
const OFFLINE_COLOR: Color = Color::Rgba(1.0, 0.3, 0.3, 1.0);
/// Color for a private message from another player
const TELL_COLOR: Color = Color::Rgba(0.98, 0.71, 1.0, 1.0);
/// Color for local chat
const SAY_COLOR: Color = Color::Rgba(1.0, 0.8, 0.8, 1.0);
/// Color for group chat
const GROUP_COLOR: Color = Color::Rgba(0.47, 0.84, 1.0, 1.0);
/// Color for factional chat
const FACTION_COLOR: Color = Color::Rgba(0.24, 1.0, 0.48, 1.0);
/// Color for regional chat
const REGION_COLOR: Color = Color::Rgba(0.8, 1.0, 0.8, 1.0);
/// Color for death messagesw
const KILL_COLOR: Color = Color::Rgba(1.0, 0.17, 0.17, 1.0);
/// Color for global messages
const WORLD_COLOR: Color = Color::Rgba(0.95, 1.0, 0.95, 1.0);

//Nametags
const GROUP_MEMBER: Color = Color::Rgba(0.47, 0.84, 1.0, 1.0);
const DEFAULT_NPC: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);

// UI Color-Theme
const UI_MAIN: Color = Color::Rgba(0.61, 0.70, 0.70, 1.0); // Greenish Blue
const UI_SUBTLE: Color = Color::Rgba(0.2, 0.24, 0.24, 1.0); // Dark Greenish Blue
//const UI_MAIN: Color = Color::Rgba(0.1, 0.1, 0.1, 0.97); // Dark
const UI_HIGHLIGHT_0: Color = Color::Rgba(0.79, 1.09, 1.09, 1.0);
// Pull-Down menu BG color
const MENU_BG: Color = Color::Rgba(0.1, 0.12, 0.12, 1.0);
//const UI_DARK_0: Color = Color::Rgba(0.25, 0.37, 0.37, 1.0);

/// Distance at which nametags are visible for group members
const NAMETAG_GROUP_RANGE: f32 = 1000.0;
/// Distance at which nametags are visible
const NAMETAG_RANGE: f32 = 40.0;
/// Time nametags stay visible after doing damage even if they are out of range
/// in seconds
const NAMETAG_DMG_TIME: f32 = 60.0;
/// Range damaged triggered nametags can be seen
const NAMETAG_DMG_RANGE: f32 = 120.0;
/// Range to display speech-bubbles at
const SPEECH_BUBBLE_RANGE: f32 = NAMETAG_RANGE;
const EXP_FLOATER_LIFETIME: f32 = 2.0;
const EXP_ACCUMULATION_DURATION: f32 = 0.5;

// TODO: Don't hard code this
pub fn default_water_color() -> Rgba<f32> { srgba_to_linear(Rgba::new(0.0, 0.18, 0.37, 1.0)) }

widget_ids! {
    struct Ids {
        // Crosshair
        crosshair_inner,
        crosshair_outer,

        // SCT
        player_scts[],
        player_sct_bgs[],
        player_rank_up,
        player_rank_up_txt_number,
        player_rank_up_txt_0,
        player_rank_up_txt_0_bg,
        player_rank_up_txt_1,
        player_rank_up_txt_1_bg,
        player_rank_up_icon,
        sct_exp_bgs[],
        sct_exps[],
        sct_exp_icons[],
        sct_lvl_bg,
        sct_lvl,
        hurt_bg,
        death_bg,
        sct_bgs[],
        scts[],

        overheads[],
        overitems[],

        // Alpha Disclaimer
        alpha_text,

        // Debug
        debug_bg,
        fps_counter,
        ping,
        coordinates,
        velocity,
        glide_ratio,
        glide_aoe,
        orientation,
        look_direction,
        loaded_distance,
        time,
        entity_count,
        num_chunks,
        num_lights,
        num_figures,
        num_particles,
        current_biome,
        current_site,
        graphics_backend,
        gpu_timings[],
        weather,
        song_info,

        // Game Version
        version,

        // Help
        help,
        help_info,
        debug_info,
        lantern_info,

        // Window Frames
        window_frame_0,
        window_frame_1,
        window_frame_2,
        window_frame_3,
        window_frame_4,
        window_frame_5,

        button_help2,
        button_help3,

        // External
        chat,
        loot_scroller,
        map,
        world_map,
        character_window,
        popup,
        minimap,
        prompt_dialog,
        bag,
        trade,
        social,
        quest,
        diary,
        skillbar,
        buttons,
        buffs,
        esc_menu,
        small_window,
        social_window,
        quest_window,
        crafting_window,
        settings_window,
        group_window,
        item_info,
        subtitles,

        // Free look indicator
        free_look_txt,
        free_look_bg,

        // Auto walk indicator
        auto_walk_txt,
        auto_walk_bg,

        // Walking speed indicator
        walking_speed_txt,
        walking_speed_bg,

        // Temporal (fading) camera zoom lock indicator
        zoom_lock_txt,
        zoom_lock_bg,

        // Camera clamp indicator
        camera_clamp_txt,
        camera_clamp_bg,

        // Tutorial
        quest_bg,
        q_headline_bg,
        q_headline,
        q_text_bg,
        q_text,
        accept_button,
        intro_button,
        tut_arrow,
        tut_arrow_txt_bg,
        tut_arrow_txt,
    }
}

/// Specifier to use with `Position::position`
/// Read its documentation for more
// TODO: extend as you need it
#[derive(Clone, Copy)]
pub enum PositionSpecifier {
    // Place the widget near other widget with the given margins
    TopLeftWithMarginsOn(widget::Id, f64, f64),
    TopRightWithMarginsOn(widget::Id, f64, f64),
    MidBottomWithMarginOn(widget::Id, f64),
    BottomLeftWithMarginsOn(widget::Id, f64, f64),
    BottomRightWithMarginsOn(widget::Id, f64, f64),
    // Place the widget near other widget with given margin
    MidTopWithMarginOn(widget::Id, f64),
    // Place the widget near other widget at given distance
    MiddleOf(widget::Id),
    UpFrom(widget::Id, f64),
    DownFrom(widget::Id, f64),
    LeftFrom(widget::Id, f64),
    RightFrom(widget::Id, f64),
}

/// Trait which enables you to declare widget position
/// to use later on widget creation.
/// It is implemented for all widgets which are implement Positionable,
/// so you can easily change your code to use this method.
///
/// Consider this example:
/// ```text
///     let slot1 = slot_maker
///         .fabricate(hotbar::Slot::One, [40.0; 2])
///         .filled_slot(self.imgs.skillbar_slot)
///         .bottom_left_with_margins_on(state.ids.frame, 0.0, 0.0);
///     if condition {
///         call_slot1(slot1);
///     } else {
///         call_slot2(slot1);
///     }
///     let slot2 = slot_maker
///         .fabricate(hotbar::Slot::Two, [40.0; 2])
///         .filled_slot(self.imgs.skillbar_slot)
///         .right_from(state.ids.slot1, slot_offset);
///     if condition {
///         call_slot1(slot2);
///     } else {
///         call_slot2(slot2);
///     }
/// ```
/// Despite being identical, you can't easily deduplicate code
/// which uses slot1 and slot2 as they are calling methods to position itself.
/// This can be solved if you declare position and use it later like so
/// ```text
/// let slots = [
///     (hotbar::Slot::One, BottomLeftWithMarginsOn(state.ids.frame, 0.0, 0.0)),
///     (hotbar::Slot::Two, RightFrom(state.ids.slot1, slot_offset)),
/// ];
/// for (slot, pos) in slots {
///     let slot = slot_maker
///         .fabricate(slot, [40.0; 2])
///         .filled_slot(self.imgs.skillbar_slot)
///         .position(pos);
///     if condition {
///         call_slot1(slot);
///     } else {
///         call_slot2(slot);
///     }
/// }
/// ```
pub trait Position {
    #[must_use]
    fn position(self, request: PositionSpecifier) -> Self;
}

impl<W: Positionable> Position for W {
    fn position(self, request: PositionSpecifier) -> Self {
        match request {
            // Place the widget near other widget with the given margins
            PositionSpecifier::TopLeftWithMarginsOn(other, top, left) => {
                self.top_left_with_margins_on(other, top, left)
            },
            PositionSpecifier::TopRightWithMarginsOn(other, top, right) => {
                self.top_right_with_margins_on(other, top, right)
            },
            PositionSpecifier::MidBottomWithMarginOn(other, margin) => {
                self.mid_bottom_with_margin_on(other, margin)
            },
            PositionSpecifier::BottomRightWithMarginsOn(other, bottom, right) => {
                self.bottom_right_with_margins_on(other, bottom, right)
            },
            PositionSpecifier::BottomLeftWithMarginsOn(other, bottom, left) => {
                self.bottom_left_with_margins_on(other, bottom, left)
            },
            // Place the widget near other widget with given margin
            PositionSpecifier::MidTopWithMarginOn(other, margin) => {
                self.mid_top_with_margin_on(other, margin)
            },
            // Place the widget near other widget at given distance
            PositionSpecifier::MiddleOf(other) => self.middle_of(other),
            PositionSpecifier::UpFrom(other, offset) => self.up_from(other, offset),
            PositionSpecifier::DownFrom(other, offset) => self.down_from(other, offset),
            PositionSpecifier::LeftFrom(other, offset) => self.left_from(other, offset),
            PositionSpecifier::RightFrom(other, offset) => self.right_from(other, offset),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BuffIconKind {
    Buff {
        kind: BuffKind,
        data: BuffData,
        multiplicity: usize,
    },
    Stance(Stance),
}

impl BuffIconKind {
    pub fn image(&self, imgs: &Imgs) -> conrod_core::image::Id {
        match self {
            Self::Buff { kind, .. } => get_buff_image(*kind, imgs),
            Self::Stance(stance) => util::ability_image(imgs, stance.pseudo_ability_id()),
        }
    }

    pub fn max_duration(&self) -> Option<Secs> {
        match self {
            Self::Buff { data, .. } => data.duration,
            Self::Stance(_) => None,
        }
    }

    pub fn title_description<'b>(
        &self,
        localized_strings: &'b Localization,
    ) -> (Cow<'b, str>, Cow<'b, str>) {
        match self {
            Self::Buff {
                kind,
                data,
                multiplicity: _,
            } => (
                get_buff_title(*kind, localized_strings),
                get_buff_desc(*kind, *data, localized_strings),
            ),
            Self::Stance(stance) => {
                util::ability_description(stance.pseudo_ability_id(), localized_strings)
            },
        }
    }
}

impl PartialOrd for BuffIconKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl Ord for BuffIconKind {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (
                BuffIconKind::Buff { kind, .. },
                BuffIconKind::Buff {
                    kind: other_kind, ..
                },
            ) => kind.cmp(other_kind),
            (BuffIconKind::Buff { .. }, BuffIconKind::Stance(_)) => Ordering::Greater,
            (BuffIconKind::Stance(_), BuffIconKind::Buff { .. }) => Ordering::Less,
            (BuffIconKind::Stance(stance), BuffIconKind::Stance(stance_other)) => {
                stance.cmp(stance_other)
            },
        }
    }
}

impl PartialEq for BuffIconKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                BuffIconKind::Buff { kind, .. },
                BuffIconKind::Buff {
                    kind: other_kind, ..
                },
            ) => kind == other_kind,
            (BuffIconKind::Stance(stance), BuffIconKind::Stance(stance_other)) => {
                stance == stance_other
            },
            _ => false,
        }
    }
}

impl Eq for BuffIconKind {}

#[derive(Clone, Copy, Debug)]
pub struct BuffIcon {
    kind: BuffIconKind,
    is_buff: bool,
    end_time: Option<f64>,
}

impl BuffIcon {
    pub fn multiplicity(&self) -> usize {
        match self.kind {
            BuffIconKind::Buff { multiplicity, .. } => multiplicity,
            BuffIconKind::Stance(_) => 1,
        }
    }

    pub fn get_buff_time(&self, time: Time) -> String {
        if let Some(end) = self.end_time {
            format!("{:.0}s", end - time.0)
        } else {
            "".to_string()
        }
    }

    pub fn icons_vec(buffs: &comp::Buffs, stance: Option<&comp::Stance>) -> Vec<Self> {
        buffs
            .iter_active()
            .filter_map(BuffIcon::from_buffs)
            .chain(stance.and_then(BuffIcon::from_stance))
            .collect::<Vec<_>>()
    }

    fn from_stance(stance: &comp::Stance) -> Option<Self> {
        let stance = if let Stance::None = stance {
            return None;
        } else {
            stance
        };
        Some(BuffIcon {
            kind: BuffIconKind::Stance(*stance),
            is_buff: true,
            end_time: None,
        })
    }

    fn from_buffs<'b, I: Iterator<Item = &'b comp::Buff>>(buffs: I) -> Option<Self> {
        let (buff, count) = buffs.fold((None, 0), |(strongest, count), buff| {
            (strongest.or(Some(buff)), count + 1)
        });
        let buff = buff?;
        Some(Self {
            kind: BuffIconKind::Buff {
                kind: buff.kind,
                data: buff.data,
                multiplicity: count,
            },
            is_buff: buff.kind.is_buff(),
            end_time: buff.end_time.map(|end| end.0),
        })
    }
}

pub struct ExpFloater {
    pub owner: Uid,
    pub exp_change: u32,
    pub timer: f32,
    pub jump_timer: f32,
    pub rand_offset: (f32, f32),
    pub xp_pools: HashSet<SkillGroupKind>,
}

pub struct SkillPointGain {
    pub skill_tree: SkillGroupKind,
    pub total_points: u16,
    pub timer: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ComboFloater {
    pub combo: u32,
    pub timer: f64,
}

pub struct BlockFloater {
    pub timer: f32,
}

pub struct DebugInfo {
    pub tps: f64,
    pub frame_time: Duration,
    pub ping_ms: f64,
    pub coordinates: Option<comp::Pos>,
    pub velocity: Option<comp::Vel>,
    pub ori: Option<comp::Ori>,
    pub character_state: Option<comp::CharacterState>,
    pub look_dir: Dir,
    pub in_fluid: Option<comp::Fluid>,
    pub num_chunks: u32,
    pub num_lights: u32,
    pub num_visible_chunks: u32,
    pub num_shadow_chunks: u32,
    pub num_figures: u32,
    pub num_figures_visible: u32,
    pub num_particles: u32,
    pub num_particles_visible: u32,
    pub current_track: String,
    pub current_artist: String,
}

pub struct HudInfo {
    pub is_aiming: bool,
    pub active_mine_tool: Option<ToolKind>,
    pub is_first_person: bool,
    pub viewpoint_entity: specs::Entity,
    pub mutable_viewpoint: bool,
    pub target_entity: Option<specs::Entity>,
    pub selected_entity: Option<(specs::Entity, Instant)>,
    pub persistence_load_error: Option<SkillsPersistenceError>,
}

#[derive(Clone)]
pub enum Event {
    SendMessage(String),
    SendCommand(String, Vec<String>),

    CharacterSelection,
    UseSlot {
        slot: comp::slot::Slot,
        bypass_dialog: bool,
    },
    SwapEquippedWeapons,
    SwapSlots {
        slot_a: comp::slot::Slot,
        slot_b: comp::slot::Slot,
        bypass_dialog: bool,
    },
    SplitSwapSlots {
        slot_a: comp::slot::Slot,
        slot_b: comp::slot::Slot,
        bypass_dialog: bool,
    },
    DropSlot(comp::slot::Slot),
    SplitDropSlot(comp::slot::Slot),
    SortInventory,
    ChangeHotbarState(Box<HotbarState>),
    TradeAction(TradeAction),
    Ability(usize, bool),
    Logout,
    Quit,

    CraftRecipe {
        recipe_name: String,
        craft_sprite: Option<(VolumePos, SpriteKind)>,
        amount: u32,
    },
    SalvageItem {
        slot: InvSlotId,
        salvage_pos: VolumePos,
    },
    CraftModularWeapon {
        primary_slot: InvSlotId,
        secondary_slot: InvSlotId,
        craft_sprite: Option<VolumePos>,
    },
    CraftModularWeaponComponent {
        toolkind: ToolKind,
        material: InvSlotId,
        modifier: Option<InvSlotId>,
        craft_sprite: Option<VolumePos>,
    },
    RepairItem {
        item: Slot,
        sprite_pos: VolumePos,
    },
    InviteMember(Uid),
    AcceptInvite,
    DeclineInvite,
    KickMember(Uid),
    LeaveGroup,
    AssignLeader(Uid),
    RemoveBuff(BuffKind),
    LeaveStance,
    UnlockSkill(Skill),
    SelectExpBar(Option<SkillGroupKind>),

    RequestSiteInfo(SiteId),
    ChangeAbility(usize, AuxiliaryAbility),

    SettingsChange(SettingsChange),
    AcknowledgePersistenceLoadError,
    MapMarkerEvent(MapMarkerChange),
}

// TODO: Are these the possible layouts we want?
// TODO: Maybe replace this with bitflags.
// `map` is not here because it currently is displayed over the top of other
// open windows.
#[derive(PartialEq, Eq)]
pub enum Windows {
    Settings, // Display settings window.
    None,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CrosshairType {
    RoundEdges,
    Edges,
    #[serde(other)]
    Round,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Intro {
    Never,
    #[serde(other)]
    Show,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum XpBar {
    OnGain,
    #[serde(other)]
    Always,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum BarNumbers {
    Percent,
    Off,
    #[serde(other)]
    Values,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ShortcutNumbers {
    Off,
    #[serde(other)]
    On,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum BuffPosition {
    Map,
    #[serde(other)]
    Bar,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PressBehavior {
    Hold = 1,
    #[serde(other)]
    Toggle = 0,
}
/// Similar to [PressBehavior], with different semantics for settings that
/// change state automatically. There is no [PressBehavior::update][update]
/// implementation because it doesn't apply to the use case; this is just a
/// sentinel.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AutoPressBehavior {
    Auto = 1,
    #[serde(other)]
    Toggle = 0,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatTab {
    pub label: String,
    pub filter: ChatFilter,
}
impl Default for ChatTab {
    fn default() -> Self {
        Self {
            label: String::from("Chat"),
            filter: ChatFilter::default(),
        }
    }
}

impl PressBehavior {
    pub fn update(&self, keystate: bool, setting: &mut bool, f: impl FnOnce(bool)) {
        match (self, keystate) {
            // flip the state on key press in toggle mode
            (PressBehavior::Toggle, true) => {
                *setting ^= true;
                f(*setting);
            },
            // do nothing on key release in toggle mode
            (PressBehavior::Toggle, false) => {},
            // set the setting to the key state in hold mode
            (PressBehavior::Hold, state) => {
                *setting = state;
                f(*setting);
            },
        }
    }
}

#[derive(Default, Clone)]
pub struct MapMarkers {
    owned: Option<Vec2<i32>>,
    group: HashMap<Uid, Vec2<i32>>,
}

/// (target slot, input value, inventory quantity, is our inventory, error,
/// trade.offers index of trade slot)
pub struct TradeAmountInput {
    slot: InvSlotId,
    input: String,
    inv: u32,
    ours: bool,
    err: Option<String>,
    who: usize,
    input_painted: bool,
    submit_action: Option<TradeAction>,
}

impl TradeAmountInput {
    pub fn new(slot: InvSlotId, input: String, inv: u32, ours: bool, who: usize) -> Self {
        Self {
            slot,
            input,
            inv,
            ours,
            who,
            err: None,
            input_painted: false,
            submit_action: None,
        }
    }
}

pub struct Show {
    ui: bool,
    intro: bool,
    help: bool,
    crafting: bool,
    bag: bool,
    bag_inv: bool,
    bag_details: bool,
    trade: bool,
    trade_details: bool,
    social: bool,
    diary: bool,
    group: bool,
    quest: bool,
    group_menu: bool,
    esc_menu: bool,
    open_windows: Windows,
    map: bool,
    ingame: bool,
    chat_tab_settings_index: Option<usize>,
    settings_tab: SettingsTab,
    diary_fields: diary::DiaryShow,
    crafting_fields: crafting::CraftingShow,
    social_search_key: Option<String>,
    want_grab: bool,
    stats: bool,
    free_look: bool,
    auto_walk: bool,
    zoom_lock: ChangeNotification,
    camera_clamp: bool,
    prompt_dialog: Option<PromptDialogSettings>,
    location_markers: MapMarkers,
    trade_amount_input_key: Option<TradeAmountInput>,
}
impl Show {
    fn bag(&mut self, open: bool) {
        if !self.esc_menu {
            self.bag = open;
            self.map = false;
            self.crafting_fields.salvage = false;

            if !open {
                self.crafting = false;
            }

            self.want_grab = !self.any_window_requires_cursor();
        }
    }

    fn trade(&mut self, open: bool) {
        if !self.esc_menu {
            self.bag = open;
            self.trade = open;
            self.map = false;
            self.want_grab = !self.any_window_requires_cursor();
        }
    }

    fn map(&mut self, open: bool) {
        if !self.esc_menu {
            self.map = open;
            self.bag = false;
            self.crafting = false;
            self.crafting_fields.salvage = false;
            self.social = false;
            self.quest = false;
            self.diary = false;
            self.want_grab = !self.any_window_requires_cursor();
        }
    }

    fn social(&mut self, open: bool) {
        if !self.esc_menu {
            if !self.social && open {
                // rising edge detector
                self.search_social_players(None);
            }
            self.social = open;
            self.diary = false;
            self.want_grab = !self.any_window_requires_cursor();
        }
    }

    fn quest(&mut self, open: bool) {
        if !self.esc_menu {
            self.quest = open;
            self.diary = false;
            self.map = false;
            self.want_grab = !self.any_window_requires_cursor();
        }
    }

    fn crafting(&mut self, open: bool) {
        if !self.esc_menu {
            if !self.crafting && open {
                // rising edge detector
                self.search_crafting_recipe(None);
            }
            self.crafting = open;
            self.crafting_fields.salvage = false;
            self.crafting_fields.recipe_inputs = HashMap::new();
            self.bag = open;
            self.map = false;
            self.want_grab = !self.any_window_requires_cursor();
        }
    }

    pub fn open_crafting_tab(
        &mut self,
        tab: CraftingTab,
        craft_sprite: Option<(VolumePos, SpriteKind)>,
    ) {
        self.selected_crafting_tab(tab);
        self.crafting(true);
        self.crafting_fields.craft_sprite = self.crafting_fields.craft_sprite.or(craft_sprite);
        self.crafting_fields.salvage = matches!(
            self.crafting_fields.craft_sprite,
            Some((_, SpriteKind::DismantlingBench))
        ) && matches!(tab, CraftingTab::Dismantle);
        self.crafting_fields.initialize_repair = matches!(
            self.crafting_fields.craft_sprite,
            Some((_, SpriteKind::RepairBench))
        );
    }

    fn diary(&mut self, open: bool) {
        if !self.esc_menu {
            self.social = false;
            self.quest = false;
            self.crafting = false;
            self.crafting_fields.salvage = false;
            self.bag = false;
            self.map = false;
            self.diary_fields = diary::DiaryShow::default();
            self.diary = open;
            self.want_grab = !self.any_window_requires_cursor();
        }
    }

    fn settings(&mut self, open: bool) {
        if !self.esc_menu {
            self.open_windows = if open {
                Windows::Settings
            } else {
                Windows::None
            };
            self.bag = false;
            self.social = false;
            self.quest = false;
            self.crafting = false;
            self.crafting_fields.salvage = false;
            self.diary = false;
            self.want_grab = !self.any_window_requires_cursor();
        }
    }

    fn toggle_trade(&mut self) { self.trade(!self.trade); }

    fn toggle_map(&mut self) { self.map(!self.map) }

    fn toggle_social(&mut self) { self.social(!self.social); }

    fn toggle_crafting(&mut self) { self.crafting(!self.crafting) }

    fn toggle_spell(&mut self) { self.diary(!self.diary) }

    fn toggle_ui(&mut self) { self.ui = !self.ui; }

    fn toggle_settings(&mut self, global_state: &GlobalState) {
        match self.open_windows {
            Windows::Settings => {
                #[cfg(feature = "singleplayer")]
                global_state.unpause();

                self.settings(false);
            },
            _ => {
                #[cfg(feature = "singleplayer")]
                global_state.pause();

                self.settings(true)
            },
        };
        #[cfg(not(feature = "singleplayer"))]
        let _global_state = global_state;
    }

    // TODO: Add self updating key-bindings element
    //fn toggle_help(&mut self) { self.help = !self.help }

    fn any_window_requires_cursor(&self) -> bool {
        self.bag
            || self.trade
            || self.esc_menu
            || self.map
            || self.social
            || self.crafting
            || self.diary
            || self.help
            || self.intro
            || self.quest
            || !matches!(self.open_windows, Windows::None)
    }

    fn toggle_windows(&mut self, global_state: &mut GlobalState) {
        if self.any_window_requires_cursor() {
            self.bag = false;
            self.trade = false;
            self.esc_menu = false;
            self.help = false;
            self.intro = false;
            self.map = false;
            self.social = false;
            self.quest = false;
            self.diary = false;
            self.crafting = false;
            self.open_windows = Windows::None;
            self.want_grab = true;

            // Unpause the game if we are on singleplayer
            #[cfg(feature = "singleplayer")]
            global_state.unpause();
        } else {
            self.esc_menu = true;
            self.want_grab = false;

            // Pause the game if we are on singleplayer
            #[cfg(feature = "singleplayer")]
            global_state.pause();
        }
        #[cfg(not(feature = "singleplayer"))]
        let _global_state = global_state;
    }

    fn open_setting_tab(&mut self, tab: SettingsTab) {
        self.open_windows = Windows::Settings;
        self.esc_menu = false;
        self.settings_tab = tab;
        self.bag = false;
        self.want_grab = false;
    }

    fn open_skill_tree(&mut self, tree_sel: SelectedSkillTree) {
        self.diary_fields.skilltreetab = tree_sel;
        self.social = false;
    }

    fn selected_crafting_tab(&mut self, sel_cat: CraftingTab) {
        self.crafting_fields.crafting_tab = sel_cat;
    }

    fn search_crafting_recipe(&mut self, search_key: Option<String>) {
        self.crafting_fields.crafting_search_key = search_key;
    }

    fn search_social_players(&mut self, search_key: Option<String>) {
        self.social_search_key = search_key;
    }

    pub fn update_map_markers(&mut self, event: comp::MapMarkerUpdate) {
        match event {
            comp::MapMarkerUpdate::Owned(event) => match event {
                MapMarkerChange::Update(waypoint) => self.location_markers.owned = Some(waypoint),
                MapMarkerChange::Remove => self.location_markers.owned = None,
            },
            comp::MapMarkerUpdate::GroupMember(user, event) => match event {
                MapMarkerChange::Update(waypoint) => {
                    self.location_markers.group.insert(user, waypoint);
                },
                MapMarkerChange::Remove => {
                    self.location_markers.group.remove(&user);
                },
            },
            comp::MapMarkerUpdate::ClearGroup => {
                self.location_markers.group.clear();
            },
        }
    }
}

pub struct PromptDialogSettings {
    message: String,
    affirmative_event: Event,
    negative_option: bool,
    negative_event: Option<Event>,
    outcome_via_keypress: Option<bool>,
}

impl PromptDialogSettings {
    pub fn new(message: String, affirmative_event: Event, negative_event: Option<Event>) -> Self {
        Self {
            message,
            affirmative_event,
            negative_option: true,
            negative_event,
            outcome_via_keypress: None,
        }
    }

    pub fn set_outcome_via_keypress(&mut self, outcome: bool) {
        self.outcome_via_keypress = Some(outcome);
    }

    #[must_use]
    pub fn with_no_negative_option(mut self) -> Self {
        self.negative_option = false;
        self
    }
}

pub struct Floaters {
    pub exp_floaters: Vec<ExpFloater>,
    pub skill_point_displays: Vec<SkillPointGain>,
    pub combo_floater: Option<ComboFloater>,
    pub block_floaters: Vec<BlockFloater>,
}

#[derive(Clone)]
pub enum HudLootOwner {
    Name(String),
    Group,
    Unknown,
}

#[derive(Clone)]
pub enum HudCollectFailedReason {
    InventoryFull,
    LootOwned {
        owner: HudLootOwner,
        expiry_secs: u64,
    },
}

impl HudCollectFailedReason {
    pub fn from_server_reason(reason: &CollectFailedReason, ecs: &specs::World) -> Self {
        match reason {
            CollectFailedReason::InventoryFull => HudCollectFailedReason::InventoryFull,
            CollectFailedReason::LootOwned { owner, expiry_secs } => {
                let owner = match owner {
                    LootOwnerKind::Player(owner_uid) => {
                        let maybe_owner_name = ecs.entity_from_uid(*owner_uid).and_then(|entity| {
                            ecs.read_storage::<comp::Stats>()
                                .get(entity)
                                .map(|stats| stats.name.clone())
                        });

                        if let Some(name) = maybe_owner_name {
                            HudLootOwner::Name(name)
                        } else {
                            HudLootOwner::Unknown
                        }
                    },
                    LootOwnerKind::Group(_) => HudLootOwner::Group,
                };

                HudCollectFailedReason::LootOwned {
                    owner,
                    expiry_secs: *expiry_secs,
                }
            },
        }
    }
}
#[derive(Clone)]
pub struct CollectFailedData {
    pulse: f32,
    reason: HudCollectFailedReason,
}

impl CollectFailedData {
    pub fn new(pulse: f32, reason: HudCollectFailedReason) -> Self { Self { pulse, reason } }
}

pub struct Hud {
    ui: Ui,
    ids: Ids,
    world_map: (/* Id */ Vec<Rotations>, Vec2<u32>),
    imgs: Imgs,
    item_imgs: ItemImgs,
    item_i18n: ItemI18n,
    fonts: Fonts,
    rot_imgs: ImgsRot,
    failed_block_pickups: HashMap<VolumePos, CollectFailedData>,
    failed_entity_pickups: HashMap<EcsEntity, CollectFailedData>,
    new_loot_messages: VecDeque<LootMessage>,
    new_messages: VecDeque<comp::ChatMsg>,
    new_notifications: VecDeque<Notification>,
    speech_bubbles: HashMap<Uid, comp::SpeechBubble>,
    content_bubbles: Vec<(Vec3<f32>, comp::SpeechBubble)>,
    pub show: Show,
    //never_show: bool,
    //intro: bool,
    //intro_2: bool,
    to_focus: Option<Option<widget::Id>>,
    force_ungrab: bool,
    force_chat_input: Option<String>,
    force_chat_cursor: Option<Index>,
    tab_complete: Option<String>,
    pulse: f32,
    hp_pulse: f32,
    slot_manager: slots::SlotManager,
    hotbar: hotbar::State,
    events: Vec<Event>,
    crosshair_opacity: f32,
    floaters: Floaters,
    voxel_minimap: VoxelMinimap,
    map_drag: Vec2<f64>,
}

impl Hud {
    pub fn new(global_state: &mut GlobalState, client: &Client) -> Self {
        let window = &mut global_state.window;
        let settings = &global_state.settings;

        let mut ui = Ui::new(window).unwrap();
        ui.set_scaling_mode(settings.interface.ui_scale);
        // Generate ids.
        let ids = Ids::new(ui.id_generator());
        // Load world map
        let mut layers = Vec::new();
        for layer in client.world_data().map_layers() {
            // NOTE: Use a border the same color as the LOD ocean color (but with a
            // translucent alpha since UI have transparency and LOD doesn't).
            layers.push(ui.add_graphic_with_rotations(Graphic::Image(
                Arc::clone(layer),
                Some(default_water_color()),
            )));
        }
        let world_map = (layers, client.world_data().chunk_size().map(|e| e as u32));
        // Load images.
        let imgs = Imgs::load(&mut ui).expect("Failed to load images!");
        // Load rotation images.
        let rot_imgs = ImgsRot::load(&mut ui).expect("Failed to load rot images!");
        // Load item images.
        let item_imgs = ItemImgs::new(&mut ui, imgs.not_found);
        // Load item text ("reference" to name and description)
        let item_i18n = ItemI18n::new_expect();
        // Load fonts.
        let fonts = Fonts::load(global_state.i18n.read().fonts(), &mut ui)
            .expect("Impossible to load fonts!");
        // Get the server name.
        let server = &client.server_info().name;
        // Get the id, unwrap is safe because this CANNOT be None at this
        // point.

        let character_id = match client.presence().unwrap() {
            PresenceKind::Character(id) => Some(id),
            PresenceKind::LoadingCharacter(id) => Some(id),
            PresenceKind::Spectator => None,
            PresenceKind::Possessor => None,
        };

        // Create a new HotbarState from the persisted slots.
        let hotbar_state =
            HotbarState::new(global_state.profile.get_hotbar_slots(server, character_id));

        let slot_manager = slots::SlotManager::new(
            ui.id_generator(),
            Vec2::broadcast(40.0),
            global_state.settings.interface.slots_use_prefixes,
            global_state.settings.interface.slots_prefix_switch_point,
            // TODO(heyzoos) Will be useful for whoever works on rendering the number of items
            // "in hand".
            // fonts.cyri.conrod_id,
            // Vec2::new(1.0, 1.0),
            // fonts.cyri.scale(12),
            // TEXT_COLOR,
        );

        Self {
            voxel_minimap: VoxelMinimap::new(&mut ui),
            ui,
            imgs,
            world_map,
            rot_imgs,
            item_imgs,
            item_i18n,
            fonts,
            ids,
            failed_block_pickups: HashMap::default(),
            failed_entity_pickups: HashMap::default(),
            new_loot_messages: VecDeque::new(),
            new_messages: VecDeque::new(),
            new_notifications: VecDeque::new(),
            speech_bubbles: HashMap::new(),
            content_bubbles: Vec::new(),
            //intro: false,
            //intro_2: false,
            show: Show {
                help: false,
                intro: false,
                bag: false,
                bag_inv: false,
                bag_details: false,
                trade: false,
                trade_details: false,
                esc_menu: false,
                open_windows: Windows::None,
                map: false,
                crafting: false,
                ui: true,
                social: false,
                diary: false,
                group: false,
                // Change this before implementation!
                quest: false,
                group_menu: false,
                chat_tab_settings_index: None,
                settings_tab: SettingsTab::Interface,
                diary_fields: diary::DiaryShow::default(),
                crafting_fields: crafting::CraftingShow::default(),
                social_search_key: None,
                want_grab: true,
                ingame: true,
                stats: false,
                free_look: false,
                auto_walk: false,
                zoom_lock: ChangeNotification::default(),
                camera_clamp: false,
                prompt_dialog: None,
                location_markers: MapMarkers::default(),
                trade_amount_input_key: None,
            },
            to_focus: None,
            //never_show: false,
            force_ungrab: false,
            force_chat_input: None,
            force_chat_cursor: None,
            tab_complete: None,
            pulse: 0.0,
            hp_pulse: 0.0,
            slot_manager,
            hotbar: hotbar_state,
            events: Vec::new(),
            crosshair_opacity: 0.0,
            floaters: Floaters {
                exp_floaters: Vec::new(),
                skill_point_displays: Vec::new(),
                combo_floater: None,
                block_floaters: Vec::new(),
            },
            map_drag: Vec2::zero(),
        }
    }

    pub fn set_prompt_dialog(&mut self, prompt_dialog: PromptDialogSettings) {
        self.show.prompt_dialog = Some(prompt_dialog);
    }

    pub fn update_fonts(&mut self, i18n: &Localization) {
        self.fonts = Fonts::load(i18n.fonts(), &mut self.ui).expect("Impossible to load fonts!");
    }

    pub fn set_slots_use_prefixes(&mut self, use_prefixes: bool) {
        self.slot_manager.set_use_prefixes(use_prefixes);
    }

    pub fn set_slots_prefix_switch_point(&mut self, prefix_switch_point: u32) {
        self.slot_manager
            .set_prefix_switch_point(prefix_switch_point);
    }

    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_layout(
        &mut self,
        client: &Client,
        global_state: &mut GlobalState,
        debug_info: &Option<DebugInfo>,
        dt: Duration,
        info: HudInfo,
        camera: &Camera,
        interactable: Option<&Interactable>,
    ) -> Vec<Event> {
        span!(_guard, "update_layout", "Hud::update_layout");
        let mut events = core::mem::take(&mut self.events);
        if global_state.settings.interface.map_show_voxel_map {
            self.voxel_minimap.maintain(client, &mut self.ui);
        }
        let (ref mut ui_widgets, ref mut item_tooltip_manager, ref mut tooltip_manager) =
            &mut self.ui.set_widgets();
        // self.ui.set_item_widgets(); pulse time for pulsating elements
        self.pulse += dt.as_secs_f32();
        // FPS
        let fps = global_state.clock.stats().average_tps;
        let version = common::util::DISPLAY_VERSION_LONG.clone();
        let i18n = &global_state.i18n.read();
        let key_layout = &global_state.window.key_layout;

        if self.show.ingame {
            prof_span!("ingame elements");

            let ecs = client.state().ecs();
            let pos = ecs.read_storage::<comp::Pos>();
            let stats = ecs.read_storage::<comp::Stats>();
            let skill_sets = ecs.read_storage::<comp::SkillSet>();
            let healths = ecs.read_storage::<Health>();
            let buffs = ecs.read_storage::<comp::Buffs>();
            let energy = ecs.read_storage::<comp::Energy>();
            let mut hp_floater_lists = ecs.write_storage::<HpFloaterList>();
            let uids = ecs.read_storage::<Uid>();
            let interpolated = ecs.read_storage::<vcomp::Interpolated>();
            let scales = ecs.read_storage::<comp::Scale>();
            let bodies = ecs.read_storage::<comp::Body>();
            let items = ecs.read_storage::<Item>();
            let inventories = ecs.read_storage::<comp::Inventory>();
            let msm = ecs.read_resource::<MaterialStatManifest>();
            let entities = ecs.entities();
            let me = info.viewpoint_entity;
            let poises = ecs.read_storage::<comp::Poise>();
            let alignments = ecs.read_storage::<comp::Alignment>();
            let is_mounts = ecs.read_storage::<Is<Mount>>();
            let is_riders = ecs.read_storage::<Is<Rider>>();
            let stances = ecs.read_storage::<comp::Stance>();
            let char_activities = ecs.read_storage::<comp::CharacterActivity>();
            let time = ecs.read_resource::<Time>();

            // Check if there was a persistence load error of the skillset, and if so
            // display a dialog prompt
            if self.show.prompt_dialog.is_none() {
                if let Some(persistence_error) = info.persistence_load_error {
                    let persistence_error = match persistence_error {
                        SkillsPersistenceError::HashMismatch => {
                            "There was a difference detected in one of your skill groups since you \
                             last played."
                        },
                        SkillsPersistenceError::DeserializationFailure => {
                            "There was a error in loading some of your skills from the database."
                        },
                        SkillsPersistenceError::SpentExpMismatch => {
                            "The amount of free experience you had in one of your skill groups \
                             differed from when you last played."
                        },
                        SkillsPersistenceError::SkillsUnlockFailed => {
                            "Your skills were not able to be obtained in the same order you \
                             acquired them. Prerequisites or costs may have changed."
                        },
                    };

                    let common_message = "Some of your skill points have been reset. You will \
                                          need to reassign them.";

                    warn!("{}\n{}", persistence_error, common_message);
                    let prompt_dialog = PromptDialogSettings::new(
                        format!("{}\n", common_message),
                        Event::AcknowledgePersistenceLoadError,
                        None,
                    )
                    .with_no_negative_option();
                    // self.set_prompt_dialog(prompt_dialog);
                    self.show.prompt_dialog = Some(prompt_dialog);
                }
            }

            if (client.pending_trade().is_some() && !self.show.trade)
                || (client.pending_trade().is_none() && self.show.trade)
            {
                self.show.toggle_trade();
            }

            //self.input = client.read_storage::<comp::ControllerInputs>();
            if let Some(health) = healths.get(me) {
                // Hurt Frame
                let hp_percentage = health.current() / health.maximum() * 100.0;
                self.hp_pulse += dt.as_secs_f32() * 10.0 / hp_percentage.clamp(3.0, 7.0);
                if hp_percentage < 10.0 && !health.is_dead {
                    let hurt_fade = (self.hp_pulse).sin() * 0.5 + 0.6; //Animation timer
                    Image::new(self.imgs.hurt_bg)
                        .wh_of(ui_widgets.window)
                        .middle_of(ui_widgets.window)
                        .graphics_for(ui_widgets.window)
                        .color(Some(Color::Rgba(1.0, 1.0, 1.0, hurt_fade)))
                        .set(self.ids.hurt_bg, ui_widgets);
                }
                // Alpha Disclaimer
                Text::new(&format!("Veloren {}", &version))
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(10))
                    .color(TEXT_COLOR)
                    .mid_top_with_margin_on(ui_widgets.window, 2.0)
                    .set(self.ids.alpha_text, ui_widgets);

                // Death Frame
                if health.is_dead {
                    Image::new(self.imgs.death_bg)
                        .wh_of(ui_widgets.window)
                        .middle_of(ui_widgets.window)
                        .graphics_for(ui_widgets.window)
                        .color(Some(Color::Rgba(0.0, 0.0, 0.0, 1.0)))
                        .set(self.ids.death_bg, ui_widgets);
                } // Crosshair
                let show_crosshair = (info.is_aiming || info.is_first_person) && !health.is_dead;
                self.crosshair_opacity = Lerp::lerp(
                    self.crosshair_opacity,
                    if show_crosshair { 1.0 } else { 0.0 },
                    5.0 * dt.as_secs_f32(),
                );

                if !self.show.help {
                    Image::new(
                        // TODO: Do we want to match on this every frame?
                        match global_state.settings.interface.crosshair_type {
                            CrosshairType::Round => self.imgs.crosshair_outer_round,
                            CrosshairType::RoundEdges => self.imgs.crosshair_outer_round_edges,
                            CrosshairType::Edges => self.imgs.crosshair_outer_edges,
                        },
                    )
                    .w_h(21.0 * 1.5, 21.0 * 1.5)
                    .middle_of(ui_widgets.window)
                    .color(Some(Color::Rgba(
                        1.0,
                        1.0,
                        1.0,
                        self.crosshair_opacity * global_state.settings.interface.crosshair_opacity,
                    )))
                    .set(self.ids.crosshair_outer, ui_widgets);
                    Image::new(self.imgs.crosshair_inner)
                        .w_h(21.0 * 2.0, 21.0 * 2.0)
                        .middle_of(self.ids.crosshair_outer)
                        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.6)))
                        .set(self.ids.crosshair_inner, ui_widgets);
                }
            }

            // Max amount the sct font size increases when "flashing"
            const FLASH_MAX: u32 = 2;

            // Get player position.
            let player_pos = client
                .state()
                .ecs()
                .read_storage::<comp::Pos>()
                .get(client.entity())
                .map_or(Vec3::zero(), |pos| pos.0);
            // SCT Output values are called hp_damage and floater.info.amount
            // Numbers are currently divided by 10 and rounded
            if global_state.settings.interface.sct {
                // Render Player SCT numbers
                let mut player_sct_bg_id_walker = self.ids.player_sct_bgs.walk();
                let mut player_sct_id_walker = self.ids.player_scts.walk();
                if let (Some(HpFloaterList { floaters, .. }), Some(health)) = (
                    hp_floater_lists
                        .get_mut(me)
                        .filter(|fl| !fl.floaters.is_empty()),
                    healths.get(me),
                ) {
                    let player_font_col = |precise: bool| {
                        if precise {
                            Rgb::new(1.0, 0.9, 0.0)
                        } else {
                            Rgb::new(1.0, 0.1, 0.0)
                        }
                    };

                    fn calc_fade(floater: &HpFloater) -> f32 {
                        ((crate::ecs::sys::floater::MY_HP_SHOWTIME - floater.timer) * 0.25) + 0.2
                    }

                    floaters.retain(|fl| calc_fade(fl) > 0.0);

                    for floater in floaters {
                        let number_speed = 50.0; // Player number speed
                        let player_sct_bg_id = player_sct_bg_id_walker.next(
                            &mut self.ids.player_sct_bgs,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        let player_sct_id = player_sct_id_walker.next(
                            &mut self.ids.player_scts,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        // Clamp the amount so you don't have absurdly large damage numbers
                        let max_hp_frac = floater
                            .info
                            .amount
                            .abs()
                            .clamp(Health::HEALTH_EPSILON, health.maximum() * 1.25)
                            / health.maximum();
                        let hp_dmg_text = if floater.info.amount.abs() < 0.1 {
                            String::new()
                        } else if global_state.settings.interface.sct_damage_rounding
                            && floater.info.amount.abs() >= 1.0
                        {
                            format!("{:.0}", floater.info.amount.abs())
                        } else {
                            format!("{:.1}", floater.info.amount.abs())
                        };
                        let precise = floater.info.precise;

                        // Timer sets text transparency
                        let hp_fade = calc_fade(floater);

                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size =
                            30 + (if precise {
                                (max_hp_frac * 10.0) as u32 * 3 + 10
                            } else {
                                (max_hp_frac * 10.0) as u32 * 3
                            }) + if floater.jump_timer < 0.1 {
                                FLASH_MAX
                                    * (((1.0 - floater.jump_timer * 10.0)
                                        * 10.0
                                        * if precise { 1.25 } else { 1.0 })
                                        as u32)
                            } else {
                                0
                            };
                        let font_col = player_font_col(precise);
                        // Timer sets the widget offset
                        let y = if floater.info.amount < 0.0 {
                            floater.timer as f64
                                * number_speed
                                * floater.info.amount.signum() as f64
                                //* -1.0
                                + 300.0
                                - ui_widgets.win_h * 0.5
                        } else {
                            floater.timer as f64
                                * number_speed
                                * floater.info.amount.signum() as f64
                                * -1.0
                                + 300.0
                                - ui_widgets.win_h * 0.5
                        };
                        // Healing is offset randomly
                        let x = if floater.info.amount < 0.0 {
                            0.0
                        } else {
                            (floater.rand as f64 - 0.5) * 0.08 * ui_widgets.win_w
                                + (0.03 * ui_widgets.win_w * (floater.rand as f64 - 0.5).signum())
                        };
                        Text::new(&hp_dmg_text)
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.0, 0.0, 0.0, hp_fade))
                            .x_y(x, y - 3.0)
                            .set(player_sct_bg_id, ui_widgets);
                        Text::new(&hp_dmg_text)
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if floater.info.amount < 0.0 {
                                Color::Rgba(font_col.r, font_col.g, font_col.b, hp_fade)
                            } else {
                                Color::Rgba(0.1, 1.0, 0.1, hp_fade)
                            })
                            .x_y(x, y)
                            .set(player_sct_id, ui_widgets);
                    }
                }
                // EXP Numbers
                self.floaters.exp_floaters.iter_mut().for_each(|f| {
                    f.timer -= dt.as_secs_f32();
                    f.jump_timer += dt.as_secs_f32();
                });
                self.floaters.exp_floaters.retain(|f| f.timer > 0.0);
                for floater in self.floaters.exp_floaters.iter_mut() {
                    let number_speed = 50.0; // Number Speed for Single EXP
                    let player_sct_bg_id = player_sct_bg_id_walker.next(
                        &mut self.ids.player_sct_bgs,
                        &mut ui_widgets.widget_id_generator(),
                    );
                    let player_sct_id = player_sct_id_walker.next(
                        &mut self.ids.player_scts,
                        &mut ui_widgets.widget_id_generator(),
                    );
                    /*let player_sct_icon_id = player_sct_id_walker.next(
                        &mut self.ids.player_scts,
                        &mut ui_widgets.widget_id_generator(),
                    );*/
                    // Increase font size based on fraction of maximum Experience
                    // "flashes" by having a larger size in the first 100ms
                    let font_size_xp = 30
                        + ((floater.exp_change as f32 / 300.0).min(1.0) * 50.0) as u32
                        + if floater.jump_timer < 0.1 {
                            FLASH_MAX * (((1.0 - floater.jump_timer * 10.0) * 10.0) as u32)
                        } else {
                            0
                        };
                    let y = floater.timer as f64 * number_speed; // Timer sets the widget offset
                    //let fade = ((4.0 - floater.timer as f32) * 0.25) + 0.2; // Timer sets
                    // text transparency
                    let fade = floater.timer.min(1.0);

                    if floater.exp_change > 0 {
                        let xp_pool = &floater.xp_pools;
                        let exp_string =
                            &i18n.get_msg_ctx("hud-sct-experience", &i18n::fluent_args! {
                                // Don't show 0 Exp
                                "amount" => &floater.exp_change.max(1),
                            });
                        Text::new(exp_string)
                            .font_size(font_size_xp)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                            .x_y(
                                ui_widgets.win_w * (0.5 * floater.rand_offset.0 as f64 - 0.25),
                                ui_widgets.win_h * (0.15 * floater.rand_offset.1 as f64) + y - 3.0,
                            )
                            .set(player_sct_bg_id, ui_widgets);
                        Text::new(exp_string)
                            .font_size(font_size_xp)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(
                                if xp_pool.contains(&SkillGroupKind::Weapon(ToolKind::Pick)) {
                                    Color::Rgba(0.18, 0.32, 0.9, fade)
                                } else {
                                    Color::Rgba(0.59, 0.41, 0.67, fade)
                                },
                            )
                            .x_y(
                                ui_widgets.win_w * (0.5 * floater.rand_offset.0 as f64 - 0.25),
                                ui_widgets.win_h * (0.15 * floater.rand_offset.1 as f64) + y,
                            )
                            .set(player_sct_id, ui_widgets);
                        // Exp Source Image (TODO: fix widget id crash)
                        /*if xp_pool.contains(&SkillGroupKind::Weapon(ToolKind::Pick)) {
                            Image::new(self.imgs.pickaxe_ico)
                                .w_h(font_size_xp as f64, font_size_xp as f64)
                                .left_from(player_sct_id, 5.0)
                                .set(player_sct_icon_id, ui_widgets);
                        }*/
                    }
                }

                // Skill points
                self.floaters
                    .skill_point_displays
                    .iter_mut()
                    .for_each(|f| f.timer -= dt.as_secs_f32());
                self.floaters
                    .skill_point_displays
                    .retain(|d| d.timer > 0_f32);
                if let Some(display) = self.floaters.skill_point_displays.iter_mut().next() {
                    let fade = if display.timer < 3.0 {
                        display.timer * 0.33
                    } else if display.timer < 2.0 {
                        display.timer * 0.33 * 0.1
                    } else {
                        1.0
                    };
                    // Background image
                    let offset = if display.timer < 2.0 {
                        300.0 - (display.timer as f64 - 2.0) * -300.0
                    } else {
                        300.0
                    };
                    Image::new(self.imgs.level_up)
                        .w_h(328.0, 126.0)
                        .mid_top_with_margin_on(ui_widgets.window, offset)
                        .graphics_for(ui_widgets.window)
                        .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade)))
                        .set(self.ids.player_rank_up, ui_widgets);
                    // Rank Number
                    let rank = display.total_points;
                    let fontsize = match rank {
                        1..=99 => (20, 8.0),
                        100..=999 => (18, 9.0),
                        1000..=9999 => (17, 10.0),
                        _ => (14, 12.0),
                    };
                    Text::new(&format!("{}", rank))
                        .font_size(fontsize.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade))
                        .mid_top_with_margin_on(self.ids.player_rank_up, fontsize.1)
                        .set(self.ids.player_rank_up_txt_number, ui_widgets);
                    // Static "New Rank!" text
                    Text::new(&i18n.get_msg("hud-rank_up"))
                        .font_size(40)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                        .mid_bottom_with_margin_on(self.ids.player_rank_up, 20.0)
                        .set(self.ids.player_rank_up_txt_0_bg, ui_widgets);
                    Text::new(&i18n.get_msg("hud-rank_up"))
                        .font_size(40)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade))
                        .bottom_left_with_margins_on(self.ids.player_rank_up_txt_0_bg, 2.0, 2.0)
                        .set(self.ids.player_rank_up_txt_0, ui_widgets);
                    // Variable skilltree text
                    let skill = match display.skill_tree {
                        General => i18n.get_msg("common-weapons-general"),
                        Weapon(ToolKind::Hammer) => i18n.get_msg("common-weapons-hammer"),
                        Weapon(ToolKind::Axe) => i18n.get_msg("common-weapons-axe"),
                        Weapon(ToolKind::Sword) => i18n.get_msg("common-weapons-sword"),
                        Weapon(ToolKind::Sceptre) => i18n.get_msg("common-weapons-sceptre"),
                        Weapon(ToolKind::Bow) => i18n.get_msg("common-weapons-bow"),
                        Weapon(ToolKind::Staff) => i18n.get_msg("common-weapons-staff"),
                        Weapon(ToolKind::Pick) => i18n.get_msg("common-tool-mining"),
                        _ => Cow::Borrowed("Unknown"),
                    };
                    Text::new(&skill)
                        .font_size(20)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                        .mid_top_with_margin_on(self.ids.player_rank_up, 45.0)
                        .set(self.ids.player_rank_up_txt_1_bg, ui_widgets);
                    Text::new(&skill)
                        .font_size(20)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade))
                        .bottom_left_with_margins_on(self.ids.player_rank_up_txt_1_bg, 2.0, 2.0)
                        .set(self.ids.player_rank_up_txt_1, ui_widgets);
                    // Variable skilltree icon
                    use crate::hud::SkillGroupKind::{General, Weapon};
                    Image::new(match display.skill_tree {
                        General => self.imgs.swords_crossed,
                        Weapon(ToolKind::Hammer) => self.imgs.hammer,
                        Weapon(ToolKind::Axe) => self.imgs.axe,
                        Weapon(ToolKind::Sword) => self.imgs.sword,
                        Weapon(ToolKind::Sceptre) => self.imgs.sceptre,
                        Weapon(ToolKind::Bow) => self.imgs.bow,
                        Weapon(ToolKind::Staff) => self.imgs.staff,
                        Weapon(ToolKind::Pick) => self.imgs.mining,
                        _ => self.imgs.swords_crossed,
                    })
                    .w_h(20.0, 20.0)
                    .left_from(self.ids.player_rank_up_txt_1_bg, 5.0)
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade)))
                    .set(self.ids.player_rank_up_icon, ui_widgets);
                }

                // Scrolling Combat Text for Parrying an attack
                self.floaters
                    .block_floaters
                    .iter_mut()
                    .for_each(|f| f.timer -= dt.as_secs_f32());
                self.floaters.block_floaters.retain(|f| f.timer > 0_f32);
                for floater in self.floaters.block_floaters.iter_mut() {
                    let number_speed = 50.0;
                    let player_sct_bg_id = player_sct_bg_id_walker.next(
                        &mut self.ids.player_sct_bgs,
                        &mut ui_widgets.widget_id_generator(),
                    );
                    let player_sct_id = player_sct_id_walker.next(
                        &mut self.ids.player_scts,
                        &mut ui_widgets.widget_id_generator(),
                    );
                    let font_size = 30;
                    let y = floater.timer as f64 * number_speed; // Timer sets the widget offset
                    // text transparency
                    let fade = if floater.timer < 0.25 {
                        floater.timer / 0.25
                    } else {
                        1.0
                    };

                    Text::new(&i18n.get_msg("hud-sct-block"))
                        .font_size(font_size)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                        .x_y(
                            ui_widgets.win_w * (0.0),
                            ui_widgets.win_h * (-0.3) + y - 3.0,
                        )
                        .set(player_sct_bg_id, ui_widgets);
                    Text::new(&i18n.get_msg("hud-sct-block"))
                        .font_size(font_size)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(0.69, 0.82, 0.88, fade))
                        .x_y(ui_widgets.win_w * 0.0, ui_widgets.win_h * -0.3 + y)
                        .set(player_sct_id, ui_widgets);
                }
            }

            // Pop speech bubbles
            let now = Instant::now();
            self.speech_bubbles
                .retain(|_uid, bubble| bubble.timeout > now);
            self.content_bubbles
                .retain(|(_pos, bubble)| bubble.timeout > now);

            // Don't show messages from muted players
            self.new_messages.retain(|msg| match msg.uid() {
                Some(uid) => match client.player_list().get(&uid) {
                    Some(player_info) => {
                        if let Some(uuid) = get_player_uuid(client, &player_info.player_alias) {
                            !global_state.profile.mutelist.contains_key(&uuid)
                        } else {
                            true
                        }
                    },
                    None => true,
                },
                None => true,
            });

            // Push speech bubbles
            for msg in self.new_messages.iter() {
                if let Some((bubble, uid)) = msg.to_bubble() {
                    self.speech_bubbles.insert(uid, bubble);
                }
            }

            let mut overhead_walker = self.ids.overheads.walk();
            let mut overitem_walker = self.ids.overitems.walk();
            let mut sct_walker = self.ids.scts.walk();
            let mut sct_bg_walker = self.ids.sct_bgs.walk();
            let pulse = self.pulse;

            let make_overitem =
                |item: &Item, pos, distance, properties, fonts, interaction_options| {
                    let quality = get_quality_col(item);

                    // Item
                    overitem::Overitem::new(
                        util::describe(item, i18n, &self.item_i18n).into(),
                        quality,
                        distance,
                        fonts,
                        i18n,
                        &global_state.settings.controls,
                        properties,
                        pulse,
                        &global_state.window.key_layout,
                        interaction_options,
                    )
                    .x_y(0.0, 100.0)
                    .position_ingame(pos)
                };

            self.failed_block_pickups
                .retain(|_, t| pulse - t.pulse < overitem::PICKUP_FAILED_FADE_OUT_TIME);
            self.failed_entity_pickups
                .retain(|_, t| pulse - t.pulse < overitem::PICKUP_FAILED_FADE_OUT_TIME);

            // Render overitem: name, etc.
            for (entity, pos, item, distance) in (&entities, &pos, &items)
                .join()
                .map(|(entity, pos, item)| (entity, pos, item, pos.0.distance_squared(player_pos)))
                .filter(|(_, _, _, distance)| distance < &MAX_PICKUP_RANGE.powi(2))
            {
                let overitem_id = overitem_walker.next(
                    &mut self.ids.overitems,
                    &mut ui_widgets.widget_id_generator(),
                );

                make_overitem(
                    item,
                    pos.0 + Vec3::unit_z() * 1.2,
                    distance,
                    overitem::OveritemProperties {
                        active: interactable.and_then(|i| i.entity()) == Some(entity),
                        pickup_failed_pulse: self.failed_entity_pickups.get(&entity).cloned(),
                    },
                    &self.fonts,
                    vec![(
                        Some(GameInput::Interact),
                        i18n.get_msg("hud-pick_up").to_string(),
                    )],
                )
                .set(overitem_id, ui_widgets);
            }

            // Render overtime for an interactable block
            if let Some(Interactable::Block(block, pos, interaction)) = interactable
                && let Some((mat, _, _)) = pos.get_block_and_transform(
                    &ecs.read_resource(),
                    &ecs.read_resource(),
                    |e| {
                        ecs.read_storage::<vcomp::Interpolated>()
                            .get(e)
                            .map(|interpolated| (comp::Pos(interpolated.pos), interpolated.ori))
                    },
                    &ecs.read_storage(),
                )
            {
                let overitem_id = overitem_walker.next(
                    &mut self.ids.overitems,
                    &mut ui_widgets.widget_id_generator(),
                );

                let overitem_properties = overitem::OveritemProperties {
                    active: true,
                    pickup_failed_pulse: self.failed_block_pickups.get(pos).cloned(),
                };

                let pos = mat.mul_point(Vec3::broadcast(0.5));
                let over_pos = pos + Vec3::unit_z() * 0.7;

                let interaction_text = |collect_default| match interaction {
                    BlockInteraction::Collect => {
                        vec![(
                            Some(GameInput::Interact),
                            i18n.get_msg(collect_default).to_string(),
                        )]
                    },
                    BlockInteraction::Craft(_) => {
                        vec![(
                            Some(GameInput::Interact),
                            i18n.get_msg("hud-use").to_string(),
                        )]
                    },
                    BlockInteraction::Unlock(kind) => {
                        let item_name = |item_id: &ItemDefinitionIdOwned| {
                            // TODO: get ItemKey and use it with i18n?
                            item_id
                                .as_ref()
                                .itemdef_id()
                                .map(|id| {
                                    let item = Item::new_from_asset_expect(id);
                                    util::describe(&item, i18n, &self.item_i18n)
                                })
                                .unwrap_or_else(|| "modular item".to_string())
                        };

                        vec![(Some(GameInput::Interact), match kind {
                            UnlockKind::Free => i18n.get_msg("hud-open").to_string(),
                            UnlockKind::Requires(item_id) => i18n
                                .get_msg_ctx("hud-unlock-requires", &i18n::fluent_args! {
                                    "item" => item_name(item_id),
                                })
                                .to_string(),
                            UnlockKind::Consumes(item_id) => i18n
                                .get_msg_ctx("hud-unlock-requires", &i18n::fluent_args! {
                                    "item" => item_name(item_id),
                                })
                                .to_string(),
                        })]
                    },
                    BlockInteraction::Mine(mine_tool) => {
                        match (mine_tool, &info.active_mine_tool) {
                            (ToolKind::Pick, Some(ToolKind::Pick)) => {
                                vec![(
                                    Some(GameInput::Primary),
                                    i18n.get_msg("hud-mine").to_string(),
                                )]
                            },
                            (ToolKind::Pick, _) => {
                                vec![(None, i18n.get_msg("hud-mine-needs_pickaxe").to_string())]
                            },
                            (ToolKind::Shovel, Some(ToolKind::Shovel)) => {
                                vec![(
                                    Some(GameInput::Primary),
                                    i18n.get_msg("hud-dig").to_string(),
                                )]
                            },
                            (ToolKind::Shovel, _) => {
                                vec![(None, i18n.get_msg("hud-mine-needs_shovel").to_string())]
                            },
                            _ => {
                                vec![(
                                    None,
                                    i18n.get_msg("hud-mine-needs_unhandled_case").to_string(),
                                )]
                            },
                        }
                    },
                    BlockInteraction::Mount => {
                        let key = match block.get_sprite() {
                            Some(SpriteKind::Helm) => "hud-steer",
                            Some(
                                SpriteKind::Bed
                                | SpriteKind::Bedroll
                                | SpriteKind::BedrollSnow
                                | SpriteKind::BedrollPirate,
                            ) => "hud-lay",
                            _ => "hud-sit",
                        };
                        vec![(Some(GameInput::Mount), i18n.get_msg(key).to_string())]
                    },
                    BlockInteraction::Read(_) => vec![(
                        Some(GameInput::Interact),
                        i18n.get_msg("hud-read").to_string(),
                    )],
                    // TODO: change to turn on/turn off?
                    BlockInteraction::LightToggle(enable) => vec![(
                        Some(GameInput::Interact),
                        i18n.get_msg(if *enable {
                            "hud-activate"
                        } else {
                            "hud-deactivate"
                        })
                        .to_string(),
                    )],
                };

                // This is only done once per frame, so it's not a performance issue
                if let Some(desc) = block
                    .get_sprite()
                    .filter(|s| s.is_container())
                    .and_then(|s| get_sprite_desc(s, i18n))
                {
                    overitem::Overitem::new(
                        desc,
                        overitem::TEXT_COLOR,
                        pos.distance_squared(player_pos),
                        &self.fonts,
                        i18n,
                        &global_state.settings.controls,
                        overitem_properties,
                        self.pulse,
                        &global_state.window.key_layout,
                        interaction_text("hud-open"),
                    )
                    .x_y(0.0, 100.0)
                    .position_ingame(over_pos)
                    .set(overitem_id, ui_widgets);
                }
                // TODO: Handle this better. The items returned from `try_reclaim_from_block`
                // are based on rng. We probably want some function to get only gauranteed items
                // from `LootSpec`.
                else if let Some((amount, mut item)) = Item::try_reclaim_from_block(*block)
                    .into_iter()
                    .flatten()
                    .next()
                {
                    item.set_amount(amount.clamp(1, item.max_amount()))
                        .expect("amount >= 1 and <= max_amount is always a valid amount");
                    make_overitem(
                        &item,
                        over_pos,
                        pos.distance_squared(player_pos),
                        overitem_properties,
                        &self.fonts,
                        interaction_text("hud-collect"),
                    )
                    .set(overitem_id, ui_widgets);
                } else if let Some(desc) = block.get_sprite().and_then(|s| get_sprite_desc(s, i18n))
                {
                    overitem::Overitem::new(
                        desc,
                        overitem::TEXT_COLOR,
                        pos.distance_squared(player_pos),
                        &self.fonts,
                        i18n,
                        &global_state.settings.controls,
                        overitem_properties,
                        self.pulse,
                        &global_state.window.key_layout,
                        interaction_text("hud-collect"),
                    )
                    .x_y(0.0, 100.0)
                    .position_ingame(over_pos)
                    .set(overitem_id, ui_widgets);
                }
            } else if let Some(Interactable::Entity(entity)) = interactable {
                // show hud for campfires and portals
                if let Some(body) = client
                    .state()
                    .ecs()
                    .read_storage::<comp::Body>()
                    .get(*entity)
                    .filter(|b| b.is_campfire() || b.is_portal())
                {
                    let overitem_id = overitem_walker.next(
                        &mut self.ids.overitems,
                        &mut ui_widgets.widget_id_generator(),
                    );

                    let overitem_properties = overitem::OveritemProperties {
                        active: true,
                        pickup_failed_pulse: None,
                    };
                    let pos = client
                        .state()
                        .ecs()
                        .read_storage::<comp::Pos>()
                        .get(*entity)
                        .map_or(Vec3::zero(), |e| e.0);
                    let over_pos = pos + Vec3::unit_z() * 1.5;

                    overitem::Overitem::new(
                        i18n.get_msg(if body.is_campfire() {
                            "hud-crafting-campfire"
                        } else if body.is_portal() {
                            "hud-portal"
                        } else {
                            "hud-use"
                        }),
                        overitem::TEXT_COLOR,
                        pos.distance_squared(player_pos),
                        &self.fonts,
                        i18n,
                        &global_state.settings.controls,
                        overitem_properties,
                        self.pulse,
                        &global_state.window.key_layout,
                        vec![(
                            Some(GameInput::Interact),
                            i18n.get_msg(if body.is_campfire() {
                                "hud-sit"
                            } else if body.is_portal() {
                                "hud-activate"
                            } else {
                                "hud-use"
                            })
                            .to_string(),
                        )],
                    )
                    .x_y(0.0, 100.0)
                    .position_ingame(over_pos)
                    .set(overitem_id, ui_widgets);
                }
            }

            let speech_bubbles = &self.speech_bubbles;
            // Render overhead name tags and health bars
            for (
                entity,
                pos,
                info,
                bubble,
                _,
                _,
                health,
                _,
                scale,
                body,
                hpfl,
                in_group,
                dist_sqr,
                alignment,
                is_mount,
                character_activity,
            ) in (
                &entities,
                &pos,
                interpolated.maybe(),
                &stats,
                &skill_sets,
                healths.maybe(),
                &buffs,
                energy.maybe(),
                scales.maybe(),
                &bodies,
                &mut hp_floater_lists,
                &uids,
                &inventories,
                char_activities.maybe(),
                poises.maybe(),
                (
                    alignments.maybe(),
                    is_mounts.maybe(),
                    is_riders.maybe(),
                    stances.maybe(),
                ),
            )
                .join()
                .filter(|t| {
                    let health = t.5;
                    !health.map_or(false, |h| h.is_dead)
                })
                .filter_map(
                    |(
                        entity,
                        pos,
                        interpolated,
                        stats,
                        skill_set,
                        health,
                        buffs,
                        energy,
                        scale,
                        body,
                        hpfl,
                        uid,
                        inventory,
                        character_activity,
                        poise,
                        (alignment, is_mount, is_rider, stance),
                    )| {
                        // Use interpolated position if available
                        let pos = interpolated.map_or(pos.0, |i| i.pos);
                        let in_group = client.group_members().contains_key(uid);
                        let is_me = entity == me;
                        let dist_sqr = pos.distance_squared(player_pos);

                        // Determine whether to display nametag and healthbar based on whether the
                        // entity is mounted, has been damaged, is targeted/selected, or is in your
                        // group
                        // Note: even if this passes the healthbar can
                        // be hidden in some cases if it is at maximum
                        let display_overhead_info = !is_me
                            && (is_mount.is_none()
                                || health.map_or(true, overhead::should_show_healthbar))
                            && is_rider
                                .map_or(true, |is_rider| Some(&is_rider.mount) != uids.get(me))
                            && (info.target_entity.map_or(false, |e| e == entity)
                                || info.selected_entity.map_or(false, |s| s.0 == entity)
                                || health.map_or(true, overhead::should_show_healthbar)
                                || in_group)
                            && dist_sqr
                                < (if in_group {
                                    NAMETAG_GROUP_RANGE
                                } else if hpfl
                                    .time_since_last_dmg_by_me
                                    .map_or(false, |t| t < NAMETAG_DMG_TIME)
                                {
                                    NAMETAG_DMG_RANGE
                                } else {
                                    NAMETAG_RANGE
                                })
                                .powi(2);

                        let info = display_overhead_info.then(|| overhead::Info {
                            name: Some(&stats.name),
                            health,
                            buffs: Some(buffs),
                            energy,
                            combat_rating: if let (Some(health), Some(energy), Some(poise)) =
                                (health, energy, poise)
                            {
                                Some(combat::combat_rating(
                                    inventory, health, energy, poise, skill_set, *body, &msm,
                                ))
                            } else {
                                None
                            },
                            stance,
                        });
                        // Only render bubble if nearby or if its me and setting is on
                        let bubble = if (dist_sqr < SPEECH_BUBBLE_RANGE.powi(2) && !is_me)
                            || (is_me && global_state.settings.interface.speech_bubble_self)
                        {
                            speech_bubbles.get(uid)
                        } else {
                            None
                        };
                        (info.is_some() || bubble.is_some()).then_some({
                            (
                                entity,
                                pos,
                                info,
                                bubble,
                                stats,
                                skill_set,
                                health,
                                buffs,
                                scale,
                                body,
                                hpfl,
                                in_group,
                                dist_sqr,
                                alignment,
                                is_mount,
                                character_activity,
                            )
                        })
                    },
                )
            {
                let overhead_id = overhead_walker.next(
                    &mut self.ids.overheads,
                    &mut ui_widgets.widget_id_generator(),
                );

                let height_offset = body.height() * scale.map_or(1.0, |s| s.0) + 0.5;
                let ingame_pos = pos + Vec3::unit_z() * height_offset;

                // Speech bubble, name, level, and hp bars
                overhead::Overhead::new(
                    info,
                    bubble,
                    in_group,
                    &global_state.settings.interface,
                    self.pulse,
                    i18n,
                    &global_state.settings.controls,
                    &self.imgs,
                    &self.fonts,
                    &global_state.window.key_layout,
                    match alignment {
                        // TODO: Don't use `MAX_MOUNT_RANGE` here, add dedicated interaction range
                        Some(comp::Alignment::Npc)
                            if dist_sqr < common::consts::MAX_MOUNT_RANGE.powi(2)
                                && interactable.and_then(|i| i.entity()) == Some(entity) =>
                        {
                            vec![
                                (GameInput::Interact, i18n.get_msg("hud-talk").to_string()),
                                (GameInput::Trade, i18n.get_msg("hud-trade").to_string()),
                            ]
                        },
                        Some(comp::Alignment::Owned(owner))
                            if Some(*owner) == client.uid()
                                && dist_sqr < common::consts::MAX_MOUNT_RANGE.powi(2) =>
                        {
                            let mut options = Vec::new();
                            if is_mount.is_none() {
                                options.push((
                                    GameInput::Trade,
                                    i18n.get_msg("hud-trade").to_string(),
                                ));
                                if !client.is_riding()
                                    && is_mountable(body, bodies.get(client.entity()))
                                {
                                    options.push((
                                        GameInput::Mount,
                                        i18n.get_msg("hud-mount").to_string(),
                                    ));
                                }

                                let is_staying = character_activity
                                    .map_or(false, |activity| activity.is_pet_staying);

                                options.push((
                                    GameInput::StayFollow,
                                    i18n.get_msg(if is_staying {
                                        "hud-follow"
                                    } else {
                                        "hud-stay"
                                    })
                                    .to_string(),
                                ));
                            }
                            options
                        },
                        _ => Vec::new(),
                    },
                    &time,
                )
                .x_y(0.0, 100.0)
                .position_ingame(ingame_pos)
                .set(overhead_id, ui_widgets);

                // Enemy SCT
                if global_state.settings.interface.sct && !hpfl.floaters.is_empty() {
                    fn calc_fade(floater: &HpFloater) -> f32 {
                        if floater.info.precise {
                            ((crate::ecs::sys::floater::PRECISE_SHOWTIME - floater.timer) * 0.75)
                                + 0.5
                        } else {
                            ((crate::ecs::sys::floater::HP_SHOWTIME - floater.timer) * 0.25) + 0.2
                        }
                    }

                    hpfl.floaters.retain(|fl| calc_fade(fl) > 0.0);
                    let floaters = &hpfl.floaters;

                    // Colors
                    const WHITE: Rgb<f32> = Rgb::new(1.0, 0.9, 0.8);
                    const LIGHT_OR: Rgb<f32> = Rgb::new(1.0, 0.925, 0.749);
                    const LIGHT_MED_OR: Rgb<f32> = Rgb::new(1.0, 0.85, 0.498);
                    const MED_OR: Rgb<f32> = Rgb::new(1.0, 0.776, 0.247);
                    const DARK_ORANGE: Rgb<f32> = Rgb::new(1.0, 0.7, 0.0);
                    const RED_ORANGE: Rgb<f32> = Rgb::new(1.0, 0.349, 0.0);
                    const DAMAGE_COLORS: [Rgb<f32>; 6] = [
                        WHITE,
                        LIGHT_OR,
                        LIGHT_MED_OR,
                        MED_OR,
                        DARK_ORANGE,
                        RED_ORANGE,
                    ];
                    // Largest value that select the first color is 40, then it shifts colors
                    // every 5
                    let font_col = |font_size: u32, precise: bool| {
                        if precise {
                            Rgb::new(1.0, 0.9, 0.0)
                        } else {
                            DAMAGE_COLORS[(font_size.saturating_sub(36) / 5).min(5) as usize]
                        }
                    };

                    for floater in floaters {
                        let number_speed = 250.0; // Enemy number speed
                        let sct_id = sct_walker
                            .next(&mut self.ids.scts, &mut ui_widgets.widget_id_generator());
                        let sct_bg_id = sct_bg_walker
                            .next(&mut self.ids.sct_bgs, &mut ui_widgets.widget_id_generator());
                        // Clamp the amount so you don't have absurdly large damage numbers
                        let max_hp_frac = floater
                            .info
                            .amount
                            .abs()
                            .clamp(Health::HEALTH_EPSILON, health.map_or(1.0, |h| h.maximum()))
                            / health.map_or(1.0, |h| h.maximum());
                        let hp_dmg_text = if floater.info.amount.abs() < 0.1 {
                            String::new()
                        } else if global_state.settings.interface.sct_damage_rounding
                            && floater.info.amount.abs() >= 1.0
                        {
                            format!("{:.0}", floater.info.amount.abs())
                        } else {
                            format!("{:.1}", floater.info.amount.abs())
                        };
                        let precise = floater.info.precise;
                        // Timer sets text transparency
                        let fade = calc_fade(floater);
                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size =
                            30 + (if precise {
                                (max_hp_frac * 10.0) as u32 * 3 + 10
                            } else {
                                (max_hp_frac * 10.0) as u32 * 3
                            }) + if floater.jump_timer < 0.1 {
                                FLASH_MAX
                                    * (((1.0 - floater.jump_timer * 10.0)
                                        * 10.0
                                        * if precise { 1.25 } else { 1.0 })
                                        as u32)
                            } else {
                                0
                            };
                        let font_col = font_col(font_size, precise);
                        // Timer sets the widget offset
                        let y = if precise {
                            ui_widgets.win_h * (floater.rand as f64 % 0.075)
                                + ui_widgets.win_h * 0.05
                        } else {
                            (floater.timer as f64 / crate::ecs::sys::floater::HP_SHOWTIME as f64
                                * number_speed)
                                + 100.0
                        };

                        let x = if !precise {
                            0.0
                        } else {
                            (floater.rand as f64 - 0.5) * 0.075 * ui_widgets.win_w
                                + (0.03 * ui_widgets.win_w * (floater.rand as f64 - 0.5).signum())
                        };

                        Text::new(&hp_dmg_text)
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if floater.info.amount < 0.0 {
                                Color::Rgba(0.0, 0.0, 0.0, fade)
                            } else {
                                Color::Rgba(0.0, 0.0, 0.0, 1.0)
                            })
                            .x_y(x, y - 3.0)
                            .position_ingame(ingame_pos)
                            .set(sct_bg_id, ui_widgets);
                        Text::new(&hp_dmg_text)
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .x_y(x, y)
                            .color(if floater.info.amount < 0.0 {
                                Color::Rgba(font_col.r, font_col.g, font_col.b, fade)
                            } else {
                                Color::Rgba(0.1, 1.0, 0.1, 1.0)
                            })
                            .position_ingame(ingame_pos)
                            .set(sct_id, ui_widgets);
                    }
                }
            }

            for (pos, bubble) in &self.content_bubbles {
                let overhead_id = overhead_walker.next(
                    &mut self.ids.overheads,
                    &mut ui_widgets.widget_id_generator(),
                );

                overhead::Overhead::new(
                    None,
                    Some(bubble),
                    false,
                    &global_state.settings.interface,
                    self.pulse,
                    i18n,
                    &global_state.settings.controls,
                    &self.imgs,
                    &self.fonts,
                    &global_state.window.key_layout,
                    Vec::new(),
                    &time,
                )
                .x_y(0.0, 100.0)
                .position_ingame(*pos)
                .set(overhead_id, ui_widgets);
            }
        }

        // Display debug window.
        // TODO:
        // Make it use i18n keys.
        if let Some(debug_info) = debug_info {
            prof_span!("debug info");

            const V_PAD: f64 = 5.0;
            const H_PAD: f64 = 5.0;

            // Alpha Version
            Text::new(&version)
                .top_left_with_margins_on(self.ids.debug_bg, V_PAD, H_PAD)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(self.ids.version, ui_widgets);
            // Ticks per second
            Text::new(&format!(
                "FPS: {:.0} ({}ms)",
                debug_info.tps,
                debug_info.frame_time.as_millis()
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.version, V_PAD)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.fps_counter, ui_widgets);
            // Ping
            Text::new(&format!("Ping: {:.0}ms", debug_info.ping_ms))
                .color(TEXT_COLOR)
                .down_from(self.ids.fps_counter, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.ping, ui_widgets);
            // Player's position
            let coordinates_text = match debug_info.coordinates {
                Some(coordinates) => format!(
                    "Coordinates: ({:.0}, {:.0}, {:.0})",
                    coordinates.0.x, coordinates.0.y, coordinates.0.z,
                ),
                None => "Player has no Pos component".to_owned(),
            };
            Text::new(&coordinates_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.ping, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.coordinates, ui_widgets);
            // Player's velocity
            let (velocity_text, glide_ratio_text) = match debug_info.velocity {
                Some(velocity) => {
                    let velocity = velocity.0;
                    let velocity_text = format!(
                        "Velocity: ({:.1}, {:.1}, {:.1}) [{:.1} u/s]",
                        velocity.x,
                        velocity.y,
                        velocity.z,
                        velocity.magnitude()
                    );
                    let horizontal_velocity = velocity.xy().magnitude();
                    let dz = velocity.z;
                    // don't divide by zero
                    let glide_ratio_text = if dz.abs() > 0.0001 {
                        format!("Glide Ratio: {:.1}", (-1.0) * (horizontal_velocity / dz))
                    } else {
                        "Glide Ratio: Altitude is constant".to_owned()
                    };

                    (velocity_text, glide_ratio_text)
                },
                None => {
                    let err = "Player has no Vel component";
                    (err.to_owned(), err.to_owned())
                },
            };
            Text::new(&velocity_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.coordinates, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.velocity, ui_widgets);
            Text::new(&glide_ratio_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.velocity, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.glide_ratio, ui_widgets);
            let glide_angle_text = angle_of_attack_text(
                debug_info.in_fluid,
                debug_info.velocity,
                debug_info.character_state.as_ref(),
            );
            Text::new(&glide_angle_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.glide_ratio, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.glide_aoe, ui_widgets);
            // Player's orientation vector
            let orientation_text = match debug_info.ori {
                Some(ori) => {
                    let orientation = ori.look_dir();
                    format!(
                        "Orientation: ({:.2}, {:.2}, {:.2})",
                        orientation.x, orientation.y, orientation.z,
                    )
                },
                None => "Player has no Ori component".to_owned(),
            };
            Text::new(&orientation_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.glide_aoe, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.orientation, ui_widgets);
            let look_dir_text = {
                let look_vec = debug_info.look_dir.to_vec();

                format!(
                    "Look Direction: ({:.2}, {:.2}, {:.2})",
                    look_vec.x, look_vec.y, look_vec.z,
                )
            };
            Text::new(&look_dir_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.orientation, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.look_direction, ui_widgets);
            // Loaded distance
            Text::new(&format!(
                "View distance: {:.2} blocks ({:.2} chunks)",
                client.loaded_distance(),
                client.loaded_distance() / TerrainChunk::RECT_SIZE.x as f32,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.look_direction, V_PAD)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.loaded_distance, ui_widgets);
            // Time
            let time_in_seconds = client.state().get_time_of_day();
            let current_time = NaiveTime::from_num_seconds_from_midnight_opt(
                // Wraps around back to 0s if it exceeds 24 hours (24 hours = 86400s)
                (time_in_seconds as u64 % 86400) as u32,
                0,
            )
            .expect("time always valid");
            Text::new(&format!("Time: {}", current_time.format("%H:%M")))
                .color(TEXT_COLOR)
                .down_from(self.ids.loaded_distance, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.time, ui_widgets);
            // Weather
            let weather = client.weather_at_player();
            Text::new(&format!(
                "Weather({kind}): {{cloud: {cloud:.2}, rain: {rain:.2}, wind: <{wind_x:.0}, \
                 {wind_y:.0}>}}",
                kind = weather.get_kind(),
                cloud = weather.cloud,
                rain = weather.rain,
                wind_x = weather.wind.x,
                wind_y = weather.wind.y
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.time, V_PAD)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.weather, ui_widgets);

            // Number of entities
            let entity_count = client.state().ecs().entities().join().count();
            Text::new(&format!("Entity count: {}", entity_count))
                .color(TEXT_COLOR)
                .down_from(self.ids.weather, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.entity_count, ui_widgets);

            // Number of chunks
            Text::new(&format!(
                "Chunks: {} ({} visible) & {} (shadow)",
                debug_info.num_chunks, debug_info.num_visible_chunks, debug_info.num_shadow_chunks,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.entity_count, V_PAD)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.num_chunks, ui_widgets);

            // Type of biome
            Text::new(&format!("Biome: {:?}", client.current_biome()))
                .color(TEXT_COLOR)
                .down_from(self.ids.num_chunks, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.current_biome, ui_widgets);

            // Type of site
            Text::new(&format!("Site: {:?}", client.current_site()))
                .color(TEXT_COLOR)
                .down_from(self.ids.current_biome, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.current_site, ui_widgets);

            // Current song info
            Text::new(&format!(
                "Now playing: {} [{}]",
                debug_info.current_track, debug_info.current_artist,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.current_site, V_PAD)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.song_info, ui_widgets);

            // Number of lights
            Text::new(&format!("Lights: {}", debug_info.num_lights,))
                .color(TEXT_COLOR)
                .down_from(self.ids.song_info, V_PAD)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.num_lights, ui_widgets);

            // Number of figures
            Text::new(&format!(
                "Figures: {} ({} visible)",
                debug_info.num_figures, debug_info.num_figures_visible,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.num_lights, V_PAD)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.num_figures, ui_widgets);

            // Number of particles
            Text::new(&format!(
                "Particles: {} ({} visible)",
                debug_info.num_particles, debug_info.num_particles_visible,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.num_figures, V_PAD)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.num_particles, ui_widgets);

            // Graphics backend
            Text::new(&format!(
                "Graphics backend: {}",
                global_state.window.renderer().graphics_backend(),
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.num_particles, V_PAD)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.graphics_backend, ui_widgets);

            let gpu_timings = global_state.window.renderer().timings();
            let mut timings_height = 0.0;

            // GPU timing for different pipelines
            if !gpu_timings.is_empty() {
                let num_timings = gpu_timings.len();
                // Make sure we have enough ids
                if self.ids.gpu_timings.len() < num_timings {
                    self.ids
                        .gpu_timings
                        .resize(num_timings, &mut ui_widgets.widget_id_generator());
                }

                for (i, timing) in gpu_timings.iter().enumerate() {
                    let label = timing.1;
                    // We skip displaying these since they aren't present every frame.
                    if label.starts_with(crate::render::UI_PREMULTIPLY_PASS) {
                        continue;
                    }
                    let timings_text =
                        &format!("{:16}{:.3} ms", &format!("{label}:"), timing.2 * 1000.0,);
                    let timings_widget = Text::new(timings_text)
                        .color(TEXT_COLOR)
                        .down(V_PAD)
                        .x_place_on(
                            self.ids.debug_bg,
                            conrod_core::position::Place::Start(Some(
                                H_PAD + 10.0 * timing.0 as f64,
                            )),
                        )
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14));

                    // Calculate timings height
                    timings_height += timings_widget.get_h(ui_widgets).unwrap_or(0.0) + V_PAD;

                    timings_widget.set(self.ids.gpu_timings[i], ui_widgets);
                }
            }

            // Set debug box dimensions, only timings height is dynamic
            // TODO: Make the background box size fully dynamic

            let debug_bg_size = [375.0, 405.0 + timings_height];

            Rectangle::fill(debug_bg_size)
                .rgba(0.0, 0.0, 0.0, global_state.settings.chat.chat_opacity)
                .top_left_with_margins_on(ui_widgets.window, 10.0, 10.0)
                .set(self.ids.debug_bg, ui_widgets);
        }

        if global_state.settings.interface.toggle_hotkey_hints {
            // Help Window
            if let Some(help_key) = global_state.settings.controls.get_binding(GameInput::Help) {
                Text::new(&i18n.get_msg_ctx(
                    "hud-press_key_to_show_keybindings_fmt",
                    &i18n::fluent_args! {
                        "key" => help_key.display_string(key_layout),
                    },
                ))
                .color(TEXT_COLOR)
                .bottom_left_with_margins_on(ui_widgets.window, 210.0, 10.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(12))
                .set(self.ids.help_info, ui_widgets);
            }
            // Lantern Key
            if let Some(toggle_lantern_key) = global_state
                .settings
                .controls
                .get_binding(GameInput::ToggleLantern)
            {
                Text::new(&i18n.get_msg_ctx(
                    "hud-press_key_to_toggle_lantern_fmt",
                    &i18n::fluent_args! {
                        "key" => toggle_lantern_key.display_string(key_layout),
                    },
                ))
                .color(TEXT_COLOR)
                .up_from(self.ids.help_info, 2.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(12))
                .set(self.ids.lantern_info, ui_widgets);
            }
        }

        // Bag button and nearby icons
        let ecs = client.state().ecs();
        // let entity = info.viewpoint_entity;
        let stats = ecs.read_storage::<comp::Stats>();
        let skill_sets = ecs.read_storage::<comp::SkillSet>();
        let buffs = ecs.read_storage::<comp::Buffs>();
        let msm = ecs.read_resource::<MaterialStatManifest>();
        let time = ecs.read_resource::<Time>();

        match Buttons::new(
            &self.imgs,
            &self.fonts,
            global_state,
            &self.rot_imgs,
            tooltip_manager,
            i18n,
        )
        .set(self.ids.buttons, ui_widgets)
        {
            Some(buttons::Event::ToggleSettings) => self.show.toggle_settings(global_state),
            Some(buttons::Event::ToggleSocial) => self.show.toggle_social(),
            Some(buttons::Event::ToggleMap) => self.show.toggle_map(),
            Some(buttons::Event::ToggleCrafting) => self.show.toggle_crafting(),
            None => {},
        }

        // Group Window
        for event in Group::new(
            &mut self.show,
            client,
            &global_state.settings,
            &self.imgs,
            &self.rot_imgs,
            &self.fonts,
            i18n,
            self.pulse,
            global_state,
            tooltip_manager,
            &msm,
            &time,
        )
        .set(self.ids.group_window, ui_widgets)
        {
            match event {
                group::Event::Accept => events.push(Event::AcceptInvite),
                group::Event::Decline => events.push(Event::DeclineInvite),
                group::Event::Kick(uid) => events.push(Event::KickMember(uid)),
                group::Event::LeaveGroup => events.push(Event::LeaveGroup),
                group::Event::AssignLeader(uid) => events.push(Event::AssignLeader(uid)),
            }
        }
        // Popup (waypoint saved and similar notifications)
        Popup::new(
            i18n,
            client,
            &self.new_notifications,
            &self.fonts,
            &self.show,
        )
        .set(self.ids.popup, ui_widgets);

        // MiniMap
        for event in MiniMap::new(
            client,
            &self.imgs,
            &self.rot_imgs,
            &self.world_map,
            &self.fonts,
            camera.get_orientation(),
            global_state,
            &self.show.location_markers,
            &self.voxel_minimap,
        )
        .set(self.ids.minimap, ui_widgets)
        {
            match event {
                minimap::Event::SettingsChange(interface_change) => {
                    events.push(Event::SettingsChange(interface_change.into()));
                },
            }
        }

        if let Some(prompt_dialog_settings) = &self.show.prompt_dialog {
            // Prompt Dialog
            match PromptDialog::new(
                &self.imgs,
                &self.fonts,
                &global_state.i18n,
                &global_state.settings,
                prompt_dialog_settings,
                &global_state.window.key_layout,
            )
            .set(self.ids.prompt_dialog, ui_widgets)
            {
                Some(dialog_outcome_event) => {
                    match dialog_outcome_event {
                        DialogOutcomeEvent::Affirmative(event) => events.push(event),
                        DialogOutcomeEvent::Negative(event) => {
                            if let Some(event) = event {
                                events.push(event);
                            };
                        },
                    };

                    // Close the prompt dialog once an option has been chosen
                    self.show.prompt_dialog = None;
                },
                None => {},
            }
        }

        // Skillbar
        // Get player stats
        let ecs = client.state().ecs();
        let entity = info.viewpoint_entity;
        let healths = ecs.read_storage::<Health>();
        let inventories = ecs.read_storage::<comp::Inventory>();
        let energies = ecs.read_storage::<comp::Energy>();
        let skillsets = ecs.read_storage::<comp::SkillSet>();
        let active_abilities = ecs.read_storage::<comp::ActiveAbilities>();
        let bodies = ecs.read_storage::<comp::Body>();
        let poises = ecs.read_storage::<comp::Poise>();
        let combos = ecs.read_storage::<comp::Combo>();
        let combo = combos.get(entity);
        let time = ecs.read_resource::<Time>();
        let stances = ecs.read_storage::<comp::Stance>();
        let char_states = ecs.read_storage::<comp::CharacterState>();
        // Combo floater stuffs
        self.floaters.combo_floater = self.floaters.combo_floater.map(|mut f| {
            f.timer -= dt.as_secs_f64();
            f
        });
        self.floaters.combo_floater = self.floaters.combo_floater.filter(|f| f.timer > 0_f64);

        if let (
            Some(health),
            Some(inventory),
            Some(energy),
            Some(poise),
            Some(skillset),
            Some(body),
        ) = (
            healths.get(entity),
            inventories.get(entity),
            energies.get(entity),
            poises.get(entity),
            skillsets.get(entity),
            bodies.get(entity),
        ) {
            let stance = stances.get(entity);
            let context = AbilityContext::from(stance, Some(inventory), combo);
            match Skillbar::new(
                client,
                &info,
                global_state,
                &self.imgs,
                &self.item_imgs,
                &self.fonts,
                &self.rot_imgs,
                health,
                inventory,
                energy,
                poise,
                skillset,
                active_abilities.get(entity),
                body,
                //&character_state,
                self.pulse,
                //&controller,
                &self.hotbar,
                tooltip_manager,
                item_tooltip_manager,
                &mut self.slot_manager,
                i18n,
                &self.item_i18n,
                &msm,
                self.floaters.combo_floater,
                &context,
                combo,
                char_states.get(entity),
                stance,
            )
            .set(self.ids.skillbar, ui_widgets)
            {
                Some(skillbar::Event::OpenDiary(skillgroup)) => {
                    self.show.diary(true);
                    self.show.open_skill_tree(skillgroup);
                },
                Some(skillbar::Event::OpenBag) => self.show.bag = !self.show.bag,
                None => {},
            }
        }
        // Bag contents
        if self.show.bag {
            if let (
                Some(player_stats),
                Some(skill_set),
                Some(health),
                Some(energy),
                Some(body),
                Some(poise),
            ) = (
                stats.get(info.viewpoint_entity),
                skill_sets.get(info.viewpoint_entity),
                healths.get(entity),
                energies.get(entity),
                bodies.get(entity),
                poises.get(entity),
            ) {
                match Bag::new(
                    client,
                    &info,
                    global_state,
                    &self.imgs,
                    &self.item_imgs,
                    &self.fonts,
                    &self.rot_imgs,
                    tooltip_manager,
                    item_tooltip_manager,
                    &mut self.slot_manager,
                    self.pulse,
                    i18n,
                    &self.item_i18n,
                    player_stats,
                    skill_set,
                    health,
                    energy,
                    &self.show,
                    body,
                    &msm,
                    poise,
                )
                .set(self.ids.bag, ui_widgets)
                {
                    Some(bag::Event::BagExpand) => self.show.bag_inv = !self.show.bag_inv,
                    Some(bag::Event::SetDetailsMode(mode)) => self.show.bag_details = mode,
                    Some(bag::Event::Close) => {
                        self.show.stats = false;
                        Self::show_bag(&mut self.slot_manager, &mut self.show, false);
                        if !self.show.social {
                            self.show.want_grab = true;
                            self.force_ungrab = false;
                        } else {
                            self.force_ungrab = true
                        };
                    },
                    Some(bag::Event::SortInventory) => self.events.push(Event::SortInventory),
                    Some(bag::Event::SwapEquippedWeapons) => {
                        self.events.push(Event::SwapEquippedWeapons)
                    },
                    None => {},
                }
            }
        }
        // Trade window
        if self.show.trade {
            if let Some(action) = Trade::new(
                client,
                &info,
                &self.imgs,
                &self.item_imgs,
                &self.fonts,
                &self.rot_imgs,
                tooltip_manager,
                item_tooltip_manager,
                &mut self.slot_manager,
                i18n,
                &self.item_i18n,
                &msm,
                self.pulse,
                &mut self.show,
            )
            .set(self.ids.trade, ui_widgets)
            {
                match action {
                    trade::TradeEvent::HudUpdate(update) => match update {
                        trade::HudUpdate::Focus(idx) => self.to_focus = Some(Some(idx)),
                        trade::HudUpdate::Submit => {
                            let key = self.show.trade_amount_input_key.take();
                            key.map(|k| {
                                k.submit_action.map(|action| {
                                    self.events.push(Event::TradeAction(action));
                                });
                            });
                        },
                    },
                    trade::TradeEvent::TradeAction(action) => {
                        if let TradeAction::Decline = action {
                            self.show.stats = false;
                            self.show.trade(false);
                            if !self.show.social {
                                self.show.want_grab = true;
                                self.force_ungrab = false;
                            } else {
                                self.force_ungrab = true
                            };
                            self.show.prompt_dialog = None;
                        }
                        events.push(Event::TradeAction(action));
                    },
                    trade::TradeEvent::SetDetailsMode(mode) => {
                        self.show.trade_details = mode;
                    },
                    trade::TradeEvent::ShowPrompt(prompt) => {
                        self.show.prompt_dialog = Some(prompt);
                    },
                }
            }
        }

        // Buffs
        if let (Some(player_buffs), Some(health), Some(energy)) = (
            buffs.get(info.viewpoint_entity),
            healths.get(entity),
            energies.get(entity),
        ) {
            for event in BuffsBar::new(
                &self.imgs,
                &self.fonts,
                &self.rot_imgs,
                tooltip_manager,
                i18n,
                player_buffs,
                stances.get(entity),
                self.pulse,
                global_state,
                health,
                energy,
                &time,
            )
            .set(self.ids.buffs, ui_widgets)
            {
                match event {
                    buffs::Event::RemoveBuff(buff_id) => events.push(Event::RemoveBuff(buff_id)),
                    buffs::Event::LeaveStance => events.push(Event::LeaveStance),
                }
            }
        }
        // Crafting
        if self.show.crafting {
            if let Some(inventory) = inventories.get(entity) {
                for event in Crafting::new(
                    //&self.show,
                    client,
                    &info,
                    &self.imgs,
                    &self.fonts,
                    i18n,
                    &self.item_i18n,
                    self.pulse,
                    &self.rot_imgs,
                    item_tooltip_manager,
                    &mut self.slot_manager,
                    &self.item_imgs,
                    inventory,
                    &msm,
                    tooltip_manager,
                    &mut self.show,
                )
                .set(self.ids.crafting_window, ui_widgets)
                {
                    match event {
                        crafting::Event::CraftRecipe {
                            recipe_name,
                            amount,
                        } => {
                            events.push(Event::CraftRecipe {
                                recipe_name,
                                craft_sprite: self.show.crafting_fields.craft_sprite,
                                amount,
                            });
                        },
                        crafting::Event::CraftModularWeapon {
                            primary_slot,
                            secondary_slot,
                        } => {
                            events.push(Event::CraftModularWeapon {
                                primary_slot,
                                secondary_slot,
                                craft_sprite: self
                                    .show
                                    .crafting_fields
                                    .craft_sprite
                                    .map(|(pos, _sprite)| pos),
                            });
                        },
                        crafting::Event::CraftModularWeaponComponent {
                            toolkind,
                            material,
                            modifier,
                        } => {
                            events.push(Event::CraftModularWeaponComponent {
                                toolkind,
                                material,
                                modifier,
                                craft_sprite: self
                                    .show
                                    .crafting_fields
                                    .craft_sprite
                                    .map(|(pos, _sprite)| pos),
                            });
                        },
                        crafting::Event::Close => {
                            self.show.stats = false;
                            self.show.crafting(false);
                            if !self.show.social {
                                self.show.want_grab = true;
                                self.force_ungrab = false;
                            } else {
                                self.force_ungrab = true
                            };
                        },
                        crafting::Event::ChangeCraftingTab(sel_cat) => {
                            self.show.open_crafting_tab(sel_cat, None);
                        },
                        crafting::Event::Focus(widget_id) => {
                            self.to_focus = Some(Some(widget_id));
                        },
                        crafting::Event::SearchRecipe(search_key) => {
                            self.show.search_crafting_recipe(search_key);
                        },
                        crafting::Event::ClearRecipeInputs => {
                            self.show.crafting_fields.recipe_inputs.clear();
                        },
                        crafting::Event::RepairItem { slot } => {
                            if let Some(sprite_pos) = self
                                .show
                                .crafting_fields
                                .craft_sprite
                                .map(|(pos, _sprite)| pos)
                            {
                                events.push(Event::RepairItem {
                                    item: slot,
                                    sprite_pos,
                                });
                            }
                        },
                    }
                }
            }
        }

        // Don't put NPC messages in chat box.
        self.new_messages
            .retain(|m| !matches!(m.chat_type, comp::ChatType::Npc(_)));

        // Chat box
        if global_state.settings.interface.toggle_chat {
            for event in Chat::new(
                &mut self.new_messages,
                client,
                global_state,
                self.pulse,
                &self.imgs,
                &self.fonts,
                i18n,
            )
            .and_then(self.force_chat_input.take(), |c, input| c.input(input))
            .and_then(self.tab_complete.take(), |c, input| {
                c.prepare_tab_completion(input)
            })
            .and_then(self.force_chat_cursor.take(), |c, pos| c.cursor_pos(pos))
            .set(self.ids.chat, ui_widgets)
            {
                match event {
                    chat::Event::TabCompletionStart(input) => {
                        self.tab_complete = Some(input);
                    },
                    chat::Event::SendMessage(message) => {
                        events.push(Event::SendMessage(message));
                    },
                    chat::Event::SendCommand(name, args) => {
                        events.push(Event::SendCommand(name, args));
                    },
                    chat::Event::Focus(focus_id) => {
                        self.to_focus = Some(Some(focus_id));
                    },
                    chat::Event::ChangeChatTab(tab) => {
                        events.push(Event::SettingsChange(ChatChange::ChangeChatTab(tab).into()));
                    },
                    chat::Event::ShowChatTabSettings(tab) => {
                        self.show.chat_tab_settings_index = Some(tab);
                        self.show.settings_tab = SettingsTab::Chat;
                        self.show.settings(true);
                    },
                }
            }
        }

        if global_state.settings.audio.subtitles {
            Subtitles::new(
                client,
                &global_state.settings,
                &global_state.audio.get_listener().clone(),
                &mut global_state.audio.subtitles,
                &self.fonts,
                i18n,
            )
            .set(self.ids.subtitles, ui_widgets);
        }

        self.new_messages = VecDeque::new();
        self.new_notifications = VecDeque::new();

        //Loot
        LootScroller::new(
            &mut self.new_loot_messages,
            client,
            &info,
            &self.show,
            &self.imgs,
            &self.item_imgs,
            &self.rot_imgs,
            &self.fonts,
            i18n,
            &self.item_i18n,
            &msm,
            item_tooltip_manager,
            self.pulse,
        )
        .set(self.ids.loot_scroller, ui_widgets);

        self.new_loot_messages = VecDeque::new();

        // Windows

        // Char Window will always appear at the left side. Other Windows default to the
        // left side, but when the Char Window is opened they will appear to the right
        // of it.

        // Settings
        if let Windows::Settings = self.show.open_windows {
            for event in SettingsWindow::new(
                global_state,
                &self.show,
                &self.imgs,
                &self.fonts,
                i18n,
                client.server_view_distance_limit(),
                fps as f32,
            )
            .set(self.ids.settings_window, ui_widgets)
            {
                match event {
                    settings_window::Event::ChangeTab(tab) => self.show.open_setting_tab(tab),
                    settings_window::Event::Close => {
                        // Unpause the game if we are on singleplayer so that we can logout
                        #[cfg(feature = "singleplayer")]
                        global_state.unpause();
                        self.show.want_grab = true;
                        self.force_ungrab = false;

                        self.show.settings(false)
                    },
                    settings_window::Event::ChangeChatSettingsTab(tab) => {
                        self.show.chat_tab_settings_index = tab;
                    },
                    settings_window::Event::SettingsChange(settings_change) => {
                        match &settings_change {
                            SettingsChange::Interface(interface_change) => match interface_change {
                                InterfaceChange::ToggleHelp(toggle_help) => {
                                    self.show.help = *toggle_help;
                                },
                                InterfaceChange::ResetInterfaceSettings => {
                                    self.show.help = false;
                                },
                                _ => {},
                            },
                            _ => {},
                        }
                        events.push(Event::SettingsChange(settings_change));
                    },
                }
            }
        }
        // Quest Window
        let stats = client.state().ecs().read_storage::<comp::Stats>();
        if self.show.quest {
            if let Some(stats) = stats.get(entity) {
                match Quest::new(
                    &self.show,
                    client,
                    &self.imgs,
                    &self.fonts,
                    i18n,
                    &self.rot_imgs,
                    tooltip_manager,
                    stats,
                    &self.item_imgs,
                    self.pulse,
                )
                .set(self.ids.quest_window, ui_widgets)
                {
                    Some(quest::Event::Close) => {
                        self.show.quest(false);
                        if !self.show.bag {
                            self.show.want_grab = true;
                            self.force_ungrab = false;
                        } else {
                            self.force_ungrab = true
                        };
                    },
                    None => {},
                }
            }
        }

        // Social Window
        if self.show.social {
            let ecs = client.state().ecs();
            let _stats = ecs.read_storage::<comp::Stats>();
            for event in Social::new(
                &self.show,
                client,
                &self.imgs,
                &self.fonts,
                i18n,
                info.selected_entity,
                &self.rot_imgs,
                tooltip_manager,
            )
            .set(self.ids.social_window, ui_widgets)
            {
                match event {
                    social::Event::Close => {
                        self.show.social(false);
                        if !self.show.bag {
                            self.show.want_grab = true;
                            self.force_ungrab = false;
                        } else {
                            self.force_ungrab = true
                        };
                    },
                    social::Event::Focus(widget_id) => {
                        self.to_focus = Some(Some(widget_id));
                    },
                    social::Event::Invite(uid) => events.push(Event::InviteMember(uid)),
                    social::Event::SearchPlayers(search_key) => {
                        self.show.search_social_players(search_key)
                    },
                }
            }
        }

        // Diary
        if self.show.diary {
            let entity = info.viewpoint_entity;
            let skill_sets = ecs.read_storage::<comp::SkillSet>();
            if let (
                Some(skill_set),
                Some(inventory),
                Some(health),
                Some(energy),
                Some(body),
                Some(poise),
            ) = (
                skill_sets.get(entity),
                inventories.get(entity),
                healths.get(entity),
                energies.get(entity),
                bodies.get(entity),
                poises.get(entity),
            ) {
                let context = AbilityContext::from(stances.get(entity), Some(inventory), combo);
                for event in Diary::new(
                    &self.show,
                    client,
                    global_state,
                    skill_set,
                    active_abilities.get(entity).unwrap_or(&Default::default()),
                    inventory,
                    health,
                    energy,
                    poise,
                    body,
                    &msm,
                    &self.imgs,
                    &self.item_imgs,
                    &self.fonts,
                    i18n,
                    &self.rot_imgs,
                    tooltip_manager,
                    &mut self.slot_manager,
                    self.pulse,
                    &context,
                )
                .set(self.ids.diary, ui_widgets)
                {
                    match event {
                        diary::Event::Close => {
                            self.show.diary(false);
                            self.show.want_grab = true;
                            self.force_ungrab = false;
                        },
                        diary::Event::ChangeSkillTree(tree_sel) => {
                            self.show.open_skill_tree(tree_sel)
                        },
                        diary::Event::UnlockSkill(skill) => events.push(Event::UnlockSkill(skill)),
                        diary::Event::ChangeSection(section) => {
                            self.show.diary_fields.section = section;
                        },
                        diary::Event::SelectExpBar(xp_bar) => {
                            events.push(Event::SelectExpBar(xp_bar))
                        },
                    }
                }
            }
        }
        // Map
        if self.show.map {
            for event in Map::new(
                client,
                &self.imgs,
                &self.rot_imgs,
                &self.world_map,
                &self.fonts,
                self.pulse,
                i18n,
                global_state,
                tooltip_manager,
                &self.show.location_markers,
                self.map_drag,
            )
            .set(self.ids.map, ui_widgets)
            {
                match event {
                    map::Event::Close => {
                        self.show.map(false);
                        self.show.want_grab = true;
                        self.force_ungrab = false;
                    },
                    map::Event::SettingsChange(settings_change) => {
                        events.push(Event::SettingsChange(settings_change.into()));
                    },
                    map::Event::RequestSiteInfo(id) => {
                        events.push(Event::RequestSiteInfo(id));
                    },
                    map::Event::SetLocationMarker(pos) => {
                        events.push(Event::MapMarkerEvent(MapMarkerChange::Update(pos)));
                        self.show.location_markers.owned = Some(pos);
                    },
                    map::Event::MapDrag(new_drag) => {
                        self.map_drag = new_drag;
                    },
                    map::Event::RemoveMarker => {
                        self.show.location_markers.owned = None;
                        events.push(Event::MapMarkerEvent(MapMarkerChange::Remove));
                    },
                }
            }
        } else {
            // Reset the map position when it's not showing
            self.map_drag = Vec2::zero();
        }

        if self.show.esc_menu {
            match EscMenu::new(&self.imgs, &self.fonts, i18n).set(self.ids.esc_menu, ui_widgets) {
                Some(esc_menu::Event::OpenSettings(tab)) => {
                    self.show.open_setting_tab(tab);
                },
                Some(esc_menu::Event::Close) => {
                    self.show.esc_menu = false;
                    self.show.want_grab = true;
                    self.force_ungrab = false;

                    // Unpause the game if we are on singleplayer
                    #[cfg(feature = "singleplayer")]
                    global_state.unpause();
                },
                Some(esc_menu::Event::Logout) => {
                    // Unpause the game if we are on singleplayer so that we can logout
                    #[cfg(feature = "singleplayer")]
                    global_state.unpause();

                    events.push(Event::Logout);
                },
                Some(esc_menu::Event::Quit) => events.push(Event::Quit),
                Some(esc_menu::Event::CharacterSelection) => {
                    // Unpause the game if we are on singleplayer so that we can logout
                    #[cfg(feature = "singleplayer")]
                    global_state.unpause();

                    events.push(Event::CharacterSelection)
                },
                None => {},
            }
        }

        let mut indicator_offset = 40.0;

        // Free look indicator
        if let Some(freelook_key) = global_state
            .settings
            .controls
            .get_binding(GameInput::FreeLook)
        {
            if self.show.free_look {
                let msg = i18n.get_msg_ctx("hud-free_look_indicator", &i18n::fluent_args! {
                    "key" => freelook_key.display_string(key_layout),
                });
                Text::new(&msg)
                    .color(TEXT_BG)
                    .mid_top_with_margin_on(ui_widgets.window, indicator_offset)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(20))
                    .set(self.ids.free_look_bg, ui_widgets);
                indicator_offset += 30.0;
                Text::new(&msg)
                    .color(KILL_COLOR)
                    .top_left_with_margins_on(self.ids.free_look_bg, -1.0, -1.0)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(20))
                    .set(self.ids.free_look_txt, ui_widgets);
            }
        };

        // Auto walk indicator
        if self.show.auto_walk {
            Text::new(&i18n.get_msg("hud-auto_walk_indicator"))
                .color(TEXT_BG)
                .mid_top_with_margin_on(ui_widgets.window, indicator_offset)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.auto_walk_bg, ui_widgets);
            indicator_offset += 30.0;
            Text::new(&i18n.get_msg("hud-auto_walk_indicator"))
                .color(KILL_COLOR)
                .top_left_with_margins_on(self.ids.auto_walk_bg, -1.0, -1.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.auto_walk_txt, ui_widgets);
        }

        // Camera zoom lock
        self.show.zoom_lock.update(dt);

        if let Some(zoom_lock) = self.show.zoom_lock.reason {
            let zoom_lock_message = match zoom_lock {
                NotificationReason::Remind => "hud-zoom_lock_indicator-remind",
                NotificationReason::Enable => "hud-zoom_lock_indicator-enable",
                NotificationReason::Disable => "hud-zoom_lock_indicator-disable",
            };

            Text::new(&i18n.get_msg(zoom_lock_message))
                .color(TEXT_BG.alpha(self.show.zoom_lock.alpha))
                .mid_top_with_margin_on(ui_widgets.window, indicator_offset)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.zoom_lock_bg, ui_widgets);
            indicator_offset += 30.0;
            Text::new(&i18n.get_msg(zoom_lock_message))
                .color(TEXT_COLOR.alpha(self.show.zoom_lock.alpha))
                .top_left_with_margins_on(self.ids.zoom_lock_bg, -1.0, -1.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.zoom_lock_txt, ui_widgets);
        }

        // Camera clamp indicator
        if let Some(cameraclamp_key) = global_state
            .settings
            .controls
            .get_binding(GameInput::CameraClamp)
        {
            if self.show.camera_clamp {
                let msg = i18n.get_msg_ctx("hud-camera_clamp_indicator", &i18n::fluent_args! {
                    "key" => cameraclamp_key.display_string(key_layout),
                });
                Text::new(&msg)
                    .color(TEXT_BG)
                    .mid_top_with_margin_on(ui_widgets.window, indicator_offset)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(20))
                    .set(self.ids.camera_clamp_bg, ui_widgets);
                Text::new(&msg)
                    .color(KILL_COLOR)
                    .top_left_with_margins_on(self.ids.camera_clamp_bg, -1.0, -1.0)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(20))
                    .set(self.ids.camera_clamp_txt, ui_widgets);
            }
        }

        // Maintain slot manager
        'slot_events: for event in self.slot_manager.maintain(ui_widgets) {
            use slots::{AbilitySlot, InventorySlot, SlotKind::*};
            let to_slot = |slot_kind| match slot_kind {
                Inventory(
                    i @ InventorySlot {
                        slot: Slot::Inventory(_) | Slot::Overflow(_),
                        ours: true,
                        ..
                    },
                ) => Some(i.slot),
                Inventory(InventorySlot {
                    slot: Slot::Equip(_),
                    ours: true,
                    ..
                }) => None,
                Inventory(InventorySlot { ours: false, .. }) => None,
                Equip(e) => Some(Slot::Equip(e)),
                Hotbar(_) => None,
                Trade(_) => None,
                Ability(_) => None,
                Crafting(_) => None,
            };
            match event {
                slot::Event::Dragged(a, b) => {
                    // Swap between slots
                    if let (Some(a), Some(b)) = (to_slot(a), to_slot(b)) {
                        events.push(Event::SwapSlots {
                            slot_a: a,
                            slot_b: b,
                            bypass_dialog: false,
                        });
                    } else if let (
                        Inventory(InventorySlot {
                            slot, ours: true, ..
                        }),
                        Hotbar(h),
                    ) = (a, b)
                    {
                        if let Slot::Inventory(slot) = slot {
                            if let Some(item) = inventories
                                .get(info.viewpoint_entity)
                                .and_then(|inv| inv.get(slot))
                            {
                                self.hotbar.add_inventory_link(h, item);
                                events.push(Event::ChangeHotbarState(Box::new(
                                    self.hotbar.to_owned(),
                                )));
                            }
                        }
                    } else if let (Hotbar(a), Hotbar(b)) = (a, b) {
                        self.hotbar.swap(a, b);
                        events.push(Event::ChangeHotbarState(Box::new(self.hotbar.to_owned())));
                    } else if let (Inventory(i), Trade(t)) = (a, b) {
                        if i.ours == t.ours {
                            if let (Some(inventory), Slot::Inventory(slot)) =
                                (inventories.get(t.entity), i.slot)
                            {
                                events.push(Event::TradeAction(TradeAction::AddItem {
                                    item: slot,
                                    quantity: i.amount(inventory).unwrap_or(1),
                                    ours: i.ours,
                                }));
                            }
                        }
                    } else if let (Trade(t), Inventory(i)) = (a, b) {
                        if i.ours == t.ours {
                            if let Some(inventory) = inventories.get(t.entity) {
                                if let Some(invslot) = t.invslot {
                                    events.push(Event::TradeAction(TradeAction::RemoveItem {
                                        item: invslot,
                                        quantity: t.amount(inventory).unwrap_or(1),
                                        ours: t.ours,
                                    }));
                                }
                            }
                        }
                    } else if let (Ability(a), Ability(b)) = (a, b) {
                        match (a, b) {
                            (AbilitySlot::Ability(ability), AbilitySlot::Slot(index)) => {
                                events.push(Event::ChangeAbility(index, ability));
                            },
                            (AbilitySlot::Slot(a), AbilitySlot::Slot(b)) => {
                                let me = info.viewpoint_entity;
                                if let Some(active_abilities) = active_abilities.get(me) {
                                    let ability_a = active_abilities
                                        .auxiliary_set(inventories.get(me), skill_sets.get(me))
                                        .get(a)
                                        .copied()
                                        .unwrap_or(AuxiliaryAbility::Empty);
                                    let ability_b = active_abilities
                                        .auxiliary_set(inventories.get(me), skill_sets.get(me))
                                        .get(b)
                                        .copied()
                                        .unwrap_or(AuxiliaryAbility::Empty);
                                    events.push(Event::ChangeAbility(a, ability_b));
                                    events.push(Event::ChangeAbility(b, ability_a));
                                }
                            },
                            (AbilitySlot::Slot(index), _) => {
                                events.push(Event::ChangeAbility(index, AuxiliaryAbility::Empty));
                            },
                            (AbilitySlot::Ability(_), AbilitySlot::Ability(_)) => {},
                        }
                    } else if let (Inventory(i), Crafting(c)) = (a, b) {
                        if let Slot::Inventory(slot) = i.slot {
                            // Add item to crafting input
                            if inventories
                                .get(info.viewpoint_entity)
                                .and_then(|inv| inv.get(slot))
                                .map_or(false, |item| {
                                    (c.requirement)(item, client.component_recipe_book(), c.info)
                                })
                            {
                                self.show
                                    .crafting_fields
                                    .recipe_inputs
                                    .insert(c.index, i.slot);
                            }
                        }
                    } else if let (Equip(e), Crafting(c)) = (a, b) {
                        // Add item to crafting input
                        if inventories
                            .get(client.entity())
                            .and_then(|inv| inv.equipped(e))
                            .map_or(false, |item| {
                                (c.requirement)(item, client.component_recipe_book(), c.info)
                            })
                        {
                            self.show
                                .crafting_fields
                                .recipe_inputs
                                .insert(c.index, Slot::Equip(e));
                        }
                    } else if let (Crafting(c), Inventory(_)) = (a, b) {
                        // Remove item from crafting input
                        self.show.crafting_fields.recipe_inputs.remove(&c.index);
                    } else if let (Ability(AbilitySlot::Ability(ability)), Hotbar(slot)) = (a, b) {
                        if let Some(Some(HotbarSlotContents::Ability(index))) =
                            self.hotbar.slots.get(slot as usize)
                        {
                            events.push(Event::ChangeAbility(*index, ability));
                        }
                    }
                },
                slot::Event::Dropped(from) => {
                    // Drop item
                    if let Some(from) = to_slot(from) {
                        events.push(Event::DropSlot(from));
                    } else if let Hotbar(h) = from {
                        self.hotbar.clear_slot(h);
                        events.push(Event::ChangeHotbarState(Box::new(self.hotbar.to_owned())));
                    } else if let Trade(t) = from {
                        if let Some(inventory) = inventories.get(t.entity) {
                            if let Some(invslot) = t.invslot {
                                events.push(Event::TradeAction(TradeAction::RemoveItem {
                                    item: invslot,
                                    quantity: t.amount(inventory).unwrap_or(1),
                                    ours: t.ours,
                                }));
                            }
                        }
                    } else if let Ability(AbilitySlot::Slot(index)) = from {
                        events.push(Event::ChangeAbility(index, AuxiliaryAbility::Empty));
                    } else if let Crafting(c) = from {
                        // Remove item from crafting input
                        self.show.crafting_fields.recipe_inputs.remove(&c.index);
                    }
                },
                slot::Event::SplitDropped(from) => {
                    // Drop item
                    if let Some(from) = to_slot(from) {
                        events.push(Event::SplitDropSlot(from));
                    } else if let Hotbar(h) = from {
                        self.hotbar.clear_slot(h);
                        events.push(Event::ChangeHotbarState(Box::new(self.hotbar.to_owned())));
                    } else if let Ability(AbilitySlot::Slot(index)) = from {
                        events.push(Event::ChangeAbility(index, AuxiliaryAbility::Empty));
                    }
                },
                slot::Event::SplitDragged(a, b) => {
                    // Swap between slots
                    if let (Some(a), Some(b)) = (to_slot(a), to_slot(b)) {
                        events.push(Event::SplitSwapSlots {
                            slot_a: a,
                            slot_b: b,
                            bypass_dialog: false,
                        });
                    } else if let (Inventory(i), Hotbar(h)) = (a, b) {
                        if let Slot::Inventory(slot) = i.slot {
                            if let Some(item) = inventories
                                .get(info.viewpoint_entity)
                                .and_then(|inv| inv.get(slot))
                            {
                                self.hotbar.add_inventory_link(h, item);
                                events.push(Event::ChangeHotbarState(Box::new(
                                    self.hotbar.to_owned(),
                                )));
                            }
                        }
                    } else if let (Hotbar(a), Hotbar(b)) = (a, b) {
                        self.hotbar.swap(a, b);
                        events.push(Event::ChangeHotbarState(Box::new(self.hotbar.to_owned())));
                    } else if let (Inventory(i), Trade(t)) = (a, b) {
                        if i.ours == t.ours {
                            if let (Some(inventory), Slot::Inventory(slot)) =
                                (inventories.get(t.entity), i.slot)
                            {
                                events.push(Event::TradeAction(TradeAction::AddItem {
                                    item: slot,
                                    quantity: i.amount(inventory).unwrap_or(1) / 2,
                                    ours: i.ours,
                                }));
                            }
                        }
                    } else if let (Trade(t), Inventory(i)) = (a, b) {
                        if i.ours == t.ours {
                            if let Some(inventory) = inventories.get(t.entity) {
                                if let Some(invslot) = t.invslot {
                                    events.push(Event::TradeAction(TradeAction::RemoveItem {
                                        item: invslot,
                                        quantity: t.amount(inventory).unwrap_or(1) / 2,
                                        ours: t.ours,
                                    }));
                                }
                            }
                        }
                    } else if let (Ability(a), Ability(b)) = (a, b) {
                        match (a, b) {
                            (AbilitySlot::Ability(ability), AbilitySlot::Slot(index)) => {
                                events.push(Event::ChangeAbility(index, ability));
                            },
                            (AbilitySlot::Slot(a), AbilitySlot::Slot(b)) => {
                                let me = info.viewpoint_entity;
                                if let Some(active_abilities) = active_abilities.get(me) {
                                    let ability_a = active_abilities
                                        .auxiliary_set(inventories.get(me), skill_sets.get(me))
                                        .get(a)
                                        .copied()
                                        .unwrap_or(AuxiliaryAbility::Empty);
                                    let ability_b = active_abilities
                                        .auxiliary_set(inventories.get(me), skill_sets.get(me))
                                        .get(b)
                                        .copied()
                                        .unwrap_or(AuxiliaryAbility::Empty);
                                    events.push(Event::ChangeAbility(a, ability_b));
                                    events.push(Event::ChangeAbility(b, ability_a));
                                }
                            },
                            (AbilitySlot::Slot(index), _) => {
                                events.push(Event::ChangeAbility(index, AuxiliaryAbility::Empty));
                            },
                            (AbilitySlot::Ability(_), AbilitySlot::Ability(_)) => {},
                        }
                    }
                },
                slot::Event::Used(from) => {
                    // Item used (selected and then clicked again)
                    if let Some(from) = to_slot(from) {
                        if self.show.crafting_fields.salvage
                            && matches!(
                                self.show.crafting_fields.crafting_tab,
                                CraftingTab::Dismantle
                            )
                        {
                            if let (Slot::Inventory(slot), Some((salvage_pos, _sprite_kind))) =
                                (from, self.show.crafting_fields.craft_sprite)
                            {
                                events.push(Event::SalvageItem { slot, salvage_pos })
                            }
                        } else {
                            events.push(Event::UseSlot {
                                slot: from,
                                bypass_dialog: false,
                            });
                        }
                    } else if let Hotbar(h) = from {
                        // Used from hotbar
                        self.hotbar.get(h).map(|s| match s {
                            hotbar::SlotContents::Inventory(i, _) => {
                                if let Some(inv) = inventories.get(info.viewpoint_entity) {
                                    // If the item in the inactive main hand is the same as the item
                                    // pressed in the hotbar, then swap active and inactive hands
                                    // instead of looking for
                                    // the item in the inventory
                                    if inv
                                        .equipped(comp::slot::EquipSlot::InactiveMainhand)
                                        .map_or(false, |item| item.item_hash() == i)
                                    {
                                        events.push(Event::SwapEquippedWeapons);
                                    } else if let Some(slot) = inv.get_slot_from_hash(i) {
                                        events.push(Event::UseSlot {
                                            slot: Slot::Inventory(slot),
                                            bypass_dialog: false,
                                        });
                                    }
                                }
                            },
                            hotbar::SlotContents::Ability(_) => {},
                        });
                    } else if let Ability(AbilitySlot::Slot(index)) = from {
                        events.push(Event::ChangeAbility(index, AuxiliaryAbility::Empty));
                    } else if let Crafting(c) = from {
                        // Remove item from crafting input
                        self.show.crafting_fields.recipe_inputs.remove(&c.index);
                    }
                },
                slot::Event::Request {
                    slot,
                    auto_quantity,
                } => {
                    if let Some((_, trade, prices)) = client.pending_trade() {
                        let ecs = client.state().ecs();
                        let inventories = ecs.read_component::<comp::Inventory>();
                        let get_inventory = |uid: Uid| {
                            if let Some(entity) = ecs.entity_from_uid(uid) {
                                inventories.get(entity)
                            } else {
                                None
                            }
                        };
                        let mut r_inventories = [None, None];
                        for (i, party) in trade.parties.iter().enumerate() {
                            match get_inventory(*party) {
                                Some(inventory) => {
                                    r_inventories[i] = Some(ReducedInventory::from(inventory))
                                },
                                None => continue 'slot_events,
                            };
                        }
                        let who = match ecs
                            .uid_from_entity(info.viewpoint_entity)
                            .and_then(|uid| trade.which_party(uid))
                        {
                            Some(who) => who,
                            None => continue 'slot_events,
                        };
                        let do_auto_quantity =
                            |inventory: &comp::Inventory,
                             slot,
                             ours,
                             remove,
                             quantity: &mut u32| {
                                if let Some(prices) = prices {
                                    if let Some((balance0, balance1)) = prices
                                        .balance(&trade.offers, &r_inventories, who, true)
                                        .zip(prices.balance(
                                            &trade.offers,
                                            &r_inventories,
                                            1 - who,
                                            false,
                                        ))
                                    {
                                        if let Some(item) = inventory.get(slot) {
                                            if let Some(materials) = TradePricing::get_materials(
                                                &item.item_definition_id(),
                                            ) {
                                                let unit_price: f32 = materials
                                                    .iter()
                                                    .map(|e| {
                                                        prices
                                                            .values
                                                            .get(&e.1)
                                                            .cloned()
                                                            .unwrap_or_default()
                                                            * e.0
                                                            * (if ours {
                                                                e.1.trade_margin()
                                                            } else {
                                                                1.0
                                                            })
                                                    })
                                                    .sum();

                                                let mut float_delta = if ours ^ remove {
                                                    (balance1 - balance0) / unit_price
                                                } else {
                                                    (balance0 - balance1) / unit_price
                                                };
                                                if ours ^ remove {
                                                    float_delta = float_delta.ceil();
                                                } else {
                                                    float_delta = float_delta.floor();
                                                }
                                                *quantity = float_delta.max(0.0) as u32;
                                            }
                                        }
                                    }
                                }
                            };
                        match slot {
                            Inventory(i) => {
                                if let Some(inventory) = inventories.get(i.entity) {
                                    if let Slot::Inventory(slot) = i.slot {
                                        let mut quantity = 1;
                                        if auto_quantity {
                                            do_auto_quantity(
                                                inventory,
                                                slot,
                                                i.ours,
                                                false,
                                                &mut quantity,
                                            );
                                            let inv_quantity = i.amount(inventory).unwrap_or(1);
                                            quantity = quantity.min(inv_quantity);
                                        }

                                        events.push(Event::TradeAction(TradeAction::AddItem {
                                            item: slot,
                                            quantity,
                                            ours: i.ours,
                                        }));
                                    }
                                }
                            },
                            Trade(t) => {
                                if let Some(inventory) = inventories.get(t.entity) {
                                    if let Some(invslot) = t.invslot {
                                        let mut quantity = 1;
                                        if auto_quantity {
                                            do_auto_quantity(
                                                inventory,
                                                invslot,
                                                t.ours,
                                                true,
                                                &mut quantity,
                                            );
                                            let inv_quantity = t.amount(inventory).unwrap_or(1);
                                            quantity = quantity.min(inv_quantity);
                                        }
                                        events.push(Event::TradeAction(TradeAction::RemoveItem {
                                            item: invslot,
                                            quantity,
                                            ours: t.ours,
                                        }));
                                    }
                                }
                            },
                            _ => {},
                        }
                    }
                },
            }
        }
        self.hotbar.maintain_abilities(client, &info);

        // Temporary Example Quest
        let arrow_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8; //Animation timer
        let show_intro = self.show.intro; // borrow check doesn't understand closures
        if let Some(toggle_cursor_key) = global_state
            .settings
            .controls
            .get_binding(GameInput::ToggleCursor)
            .filter(|_| !show_intro)
        {
            prof_span!("temporary example quest");
            match global_state.settings.interface.intro_show {
                Intro::Show => {
                    if Button::image(self.imgs.button)
                        .w_h(200.0, 60.0)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .bottom_left_with_margins_on(ui_widgets.window, 350.0, 150.0)
                        .label(&i18n.get_msg("hud-tutorial_btn"))
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .label_font_size(self.fonts.cyri.scale(18))
                        .label_color(TEXT_COLOR)
                        .label_y(conrod_core::position::Relative::Scalar(2.0))
                        .image_color(ENEMY_HP_COLOR)
                        .set(self.ids.intro_button, ui_widgets)
                        .was_clicked()
                    {
                        self.show.intro = true;
                        self.show.want_grab = true;
                    }
                    let tutorial_click_msg =
                        i18n.get_msg_ctx("hud-tutorial_click_here", &i18n::fluent_args! {
                            "key" => toggle_cursor_key.display_string(key_layout),
                        });
                    Image::new(self.imgs.sp_indicator_arrow)
                        .w_h(20.0, 11.0)
                        .mid_top_with_margin_on(self.ids.intro_button, -20.0 + arrow_ani as f64)
                        .color(Some(QUALITY_LEGENDARY))
                        .set(self.ids.tut_arrow, ui_widgets);
                    Text::new(&tutorial_click_msg)
                        .mid_top_with_margin_on(self.ids.tut_arrow, -40.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .center_justify()
                        .color(BLACK)
                        .set(self.ids.tut_arrow_txt_bg, ui_widgets);
                    Text::new(&tutorial_click_msg)
                        .bottom_right_with_margins_on(self.ids.tut_arrow_txt_bg, 1.0, 1.0)
                        .center_justify()
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(QUALITY_LEGENDARY)
                        .set(self.ids.tut_arrow_txt, ui_widgets);
                },
                Intro::Never => {
                    self.show.intro = false;
                },
            }
        }
        // TODO: Add event/stat based tutorial system
        if self.show.intro && !self.show.esc_menu {
            prof_span!("intro show");
            match global_state.settings.interface.intro_show {
                Intro::Show => {
                    if self.show.intro {
                        self.show.want_grab = false;
                        let quest_headline = i18n.get_msg("hud-temp_quest_headline");
                        let quest_text = i18n.get_msg("hud-temp_quest_text");
                        Image::new(self.imgs.quest_bg0)
                            .w_h(404.0, 858.0)
                            .middle_of(ui_widgets.window)
                            .set(self.ids.quest_bg, ui_widgets);

                        Text::new(&quest_headline)
                            .mid_top_with_margin_on(self.ids.quest_bg, 310.0)
                            .font_size(self.fonts.cyri.scale(30))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_BG)
                            .set(self.ids.q_headline_bg, ui_widgets);
                        Text::new(&quest_headline)
                            .bottom_left_with_margins_on(self.ids.q_headline_bg, 1.0, 1.0)
                            .font_size(self.fonts.cyri.scale(30))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_COLOR)
                            .set(self.ids.q_headline, ui_widgets);

                        Text::new(&quest_text)
                            .mid_top_with_margin_on(self.ids.quest_bg, 360.0)
                            .w(350.0)
                            .font_size(self.fonts.cyri.scale(17))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_BG)
                            .set(self.ids.q_text_bg, ui_widgets);
                        Text::new(&quest_text)
                            .bottom_left_with_margins_on(self.ids.q_text_bg, 1.0, 1.0)
                            .w(350.0)
                            .font_size(self.fonts.cyri.scale(17))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_COLOR)
                            .set(self.ids.q_text, ui_widgets);

                        if Button::image(self.imgs.button)
                            .w_h(212.0, 52.0)
                            .hover_image(self.imgs.button_hover)
                            .press_image(self.imgs.button_press)
                            .mid_bottom_with_margin_on(self.ids.q_text_bg, -80.0)
                            .label(&i18n.get_msg("common-close"))
                            .label_font_id(self.fonts.cyri.conrod_id)
                            .label_font_size(self.fonts.cyri.scale(22))
                            .label_color(TEXT_COLOR)
                            .label_y(conrod_core::position::Relative::Scalar(2.0))
                            .set(self.ids.accept_button, ui_widgets)
                            .was_clicked()
                        {
                            self.show.intro = false;
                            events.push(Event::SettingsChange(
                                InterfaceChange::Intro(Intro::Never).into(),
                            ));
                            self.show.want_grab = true;
                        }
                        if !self.show.crafting && !self.show.bag {
                            Image::new(self.imgs.sp_indicator_arrow)
                                .w_h(20.0, 11.0)
                                .bottom_right_with_margins_on(
                                    ui_widgets.window,
                                    40.0 + arrow_ani as f64,
                                    205.0,
                                )
                                .color(Some(QUALITY_LEGENDARY))
                                .set(self.ids.tut_arrow, ui_widgets);
                            Text::new(&i18n.get_msg("hud-tutorial_elements"))
                                .mid_top_with_margin_on(self.ids.tut_arrow, -50.0)
                                .font_id(self.fonts.cyri.conrod_id)
                                .font_size(self.fonts.cyri.scale(40))
                                .color(BLACK)
                                .floating(true)
                                .set(self.ids.tut_arrow_txt_bg, ui_widgets);
                            Text::new(&i18n.get_msg("hud-tutorial_elements"))
                                .bottom_right_with_margins_on(self.ids.tut_arrow_txt_bg, 1.0, 1.0)
                                .font_id(self.fonts.cyri.conrod_id)
                                .font_size(self.fonts.cyri.scale(40))
                                .color(QUALITY_LEGENDARY)
                                .floating(true)
                                .set(self.ids.tut_arrow_txt, ui_widgets);
                        }
                    }
                },
                Intro::Never => {
                    self.show.intro = false;
                },
            }
        }

        events
    }

    fn show_bag(slot_manager: &mut slots::SlotManager, show: &mut Show, state: bool) {
        show.bag(state);
        if !state {
            slot_manager.idle();
        }
    }

    pub fn add_failed_block_pickup(&mut self, pos: VolumePos, reason: HudCollectFailedReason) {
        self.failed_block_pickups
            .insert(pos, CollectFailedData::new(self.pulse, reason));
    }

    pub fn add_failed_entity_pickup(&mut self, entity: EcsEntity, reason: HudCollectFailedReason) {
        self.failed_entity_pickups
            .insert(entity, CollectFailedData::new(self.pulse, reason));
    }

    pub fn new_loot_message(&mut self, item: LootMessage) {
        self.new_loot_messages.push_back(item);
    }

    pub fn new_message(&mut self, msg: comp::ChatMsg) { self.new_messages.push_back(msg); }

    pub fn new_notification(&mut self, msg: Notification) { self.new_notifications.push_back(msg); }

    pub fn set_scaling_mode(&mut self, scale_mode: ScaleMode) {
        self.ui.set_scaling_mode(scale_mode);
    }

    pub fn scale_change(&mut self, scale_change: ScaleChange) -> ScaleMode {
        let scale_mode = match scale_change {
            ScaleChange::Adjust(scale) => ScaleMode::Absolute(scale),
            ScaleChange::ToAbsolute => self.ui.scale().scaling_mode_as_absolute(),
            ScaleChange::ToRelative => self.ui.scale().scaling_mode_as_relative(),
        };
        self.ui.set_scaling_mode(scale_mode);
        scale_mode
    }

    /// Checks if a TextEdit widget has the keyboard captured.
    fn typing(&self) -> bool { Hud::is_captured::<widget::TextEdit>(&self.ui.ui) }

    /// Checks if a widget of type `W` has captured the keyboard
    fn is_captured<W: Widget>(ui: &conrod_core::Ui) -> bool {
        if let Some(id) = ui.global_input().current.widget_capturing_keyboard {
            ui.widget_graph()
                .widget(id)
                .filter(|c| c.type_id == std::any::TypeId::of::<<W as Widget>::State>())
                .is_some()
        } else {
            false
        }
    }

    pub fn handle_event(
        &mut self,
        event: WinEvent,
        global_state: &mut GlobalState,
        client_inventory: Option<&comp::Inventory>,
    ) -> bool {
        // Helper
        fn handle_slot(
            slot: hotbar::Slot,
            state: bool,
            events: &mut Vec<Event>,
            slot_manager: &mut slots::SlotManager,
            hotbar: &mut hotbar::State,
            client_inventory: Option<&comp::Inventory>,
        ) {
            use slots::InventorySlot;
            if let Some(slots::SlotKind::Inventory(InventorySlot {
                slot: Slot::Inventory(i),
                ours: true,
                ..
            })) = slot_manager.selected()
            {
                if let Some(item) = client_inventory.and_then(|inv| inv.get(i)) {
                    hotbar.add_inventory_link(slot, item);
                    events.push(Event::ChangeHotbarState(Box::new(hotbar.to_owned())));
                    slot_manager.idle();
                }
            } else {
                let just_pressed = hotbar.process_input(slot, state);
                hotbar.get(slot).map(|s| match s {
                    hotbar::SlotContents::Inventory(i, _) => {
                        if just_pressed {
                            if let Some(inv) = client_inventory {
                                // If the item in the inactive main hand is the same as the item
                                // pressed in the hotbar, then swap active and inactive hands
                                // instead of looking for the item
                                // in the inventory
                                if inv
                                    .equipped(comp::slot::EquipSlot::InactiveMainhand)
                                    .map_or(false, |item| item.item_hash() == i)
                                {
                                    events.push(Event::SwapEquippedWeapons);
                                } else if let Some(slot) = inv.get_slot_from_hash(i) {
                                    events.push(Event::UseSlot {
                                        slot: comp::slot::Slot::Inventory(slot),
                                        bypass_dialog: false,
                                    });
                                }
                            }
                        }
                    },
                    hotbar::SlotContents::Ability(i) => events.push(Event::Ability(i, state)),
                });
            }
        }

        fn handle_map_zoom(
            factor: f64,
            world_size: Vec2<u32>,
            show: &Show,
            global_state: &mut GlobalState,
        ) -> bool {
            let max_zoom = world_size.reduce_partial_max() as f64;

            if show.map {
                let new_zoom_lvl = (global_state.settings.interface.map_zoom * factor)
                    .clamped(1.25, max_zoom / 64.0);

                global_state.settings.interface.map_zoom = new_zoom_lvl;
                global_state
                    .settings
                    .save_to_file_warn(&global_state.config_dir);
            } else if global_state.settings.interface.minimap_show {
                let new_zoom_lvl = global_state.settings.interface.minimap_zoom * factor;

                global_state.settings.interface.minimap_zoom = new_zoom_lvl;
                global_state
                    .settings
                    .save_to_file_warn(&global_state.config_dir);
            }

            show.map && global_state.settings.interface.minimap_show
        }

        let cursor_grabbed = global_state.window.is_cursor_grabbed();
        let handled = match event {
            WinEvent::Ui(event) => {
                if (self.typing() && event.is_keyboard() && self.show.ui)
                    || !(cursor_grabbed && event.is_keyboard_or_mouse())
                {
                    self.ui.handle_event(event);
                }
                true
            },
            WinEvent::ScaleFactorChanged(scale_factor) => {
                self.ui.scale_factor_changed(scale_factor);
                false
            },
            WinEvent::InputUpdate(GameInput::ToggleInterface, true) if !self.typing() => {
                self.show.toggle_ui();
                true
            },
            WinEvent::InputUpdate(GameInput::ToggleCursor, true) if !self.typing() => {
                self.force_ungrab = !self.force_ungrab;
                true
            },
            WinEvent::InputUpdate(GameInput::AcceptGroupInvite, true) if !self.typing() => {
                if let Some(prompt_dialog) = &mut self.show.prompt_dialog {
                    prompt_dialog.set_outcome_via_keypress(true);
                    true
                } else {
                    false
                }
            },
            WinEvent::InputUpdate(GameInput::DeclineGroupInvite, true) if !self.typing() => {
                if let Some(prompt_dialog) = &mut self.show.prompt_dialog {
                    prompt_dialog.set_outcome_via_keypress(false);
                    true
                } else {
                    false
                }
            },

            // If not showing the ui don't allow keys that change the ui state but do listen for
            // hotbar keys
            WinEvent::InputUpdate(key, state) if !self.show.ui => {
                if let Some(slot) = try_hotbar_slot_from_input(key) {
                    handle_slot(
                        slot,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                        client_inventory,
                    );
                    true
                } else {
                    false
                }
            },

            WinEvent::Zoom(_) => !cursor_grabbed && !self.ui.no_widget_capturing_mouse(),

            WinEvent::InputUpdate(GameInput::Chat, true) => {
                self.ui.focus_widget(if self.typing() {
                    None
                } else {
                    Some(self.ids.chat)
                });
                true
            },
            WinEvent::InputUpdate(GameInput::Escape, true) => {
                if self.typing() {
                    self.ui.focus_widget(None);
                } else if self.show.trade {
                    self.events.push(Event::TradeAction(TradeAction::Decline));
                } else {
                    // Close windows on esc
                    if self.show.bag {
                        self.slot_manager.idle();
                    }
                    self.show.toggle_windows(global_state);
                }
                true
            },

            // Press key while not typing
            WinEvent::InputUpdate(key, state) if !self.typing() => {
                let gs_audio = &global_state.settings.audio;
                let mut toggle_mute = |audio: Audio| {
                    self.events
                        .push(Event::SettingsChange(SettingsChange::Audio(audio)));
                    true
                };

                match key {
                    GameInput::Command if state => {
                        self.force_chat_input = Some("/".to_owned());
                        self.force_chat_cursor = Some(Index { line: 0, char: 1 });
                        self.ui.focus_widget(Some(self.ids.chat));
                        true
                    },
                    GameInput::Map if state => {
                        self.show.toggle_map();
                        true
                    },
                    GameInput::Bag if state => {
                        let state = !self.show.bag;
                        Self::show_bag(&mut self.slot_manager, &mut self.show, state);
                        true
                    },
                    GameInput::Social if state => {
                        self.show.toggle_social();
                        true
                    },
                    GameInput::Crafting if state => {
                        self.show.toggle_crafting();
                        true
                    },
                    GameInput::Spellbook if state => {
                        self.show.toggle_spell();
                        true
                    },
                    GameInput::Settings if state => {
                        self.show.toggle_settings(global_state);
                        true
                    },
                    GameInput::Help if state => {
                        self.show.toggle_settings(global_state);
                        self.show.settings_tab = SettingsTab::Controls;
                        true
                    },
                    GameInput::ToggleDebug if state => {
                        global_state.settings.interface.toggle_debug =
                            !global_state.settings.interface.toggle_debug;
                        true
                    },
                    #[cfg(feature = "egui-ui")]
                    GameInput::ToggleEguiDebug if state => {
                        global_state.settings.interface.toggle_egui_debug =
                            !global_state.settings.interface.toggle_egui_debug;
                        true
                    },
                    GameInput::ToggleChat if state => {
                        global_state.settings.interface.toggle_chat =
                            !global_state.settings.interface.toggle_chat;
                        true
                    },
                    GameInput::ToggleIngameUi if state => {
                        self.show.ingame = !self.show.ingame;
                        true
                    },
                    GameInput::MapZoomIn if state => {
                        handle_map_zoom(2.0, self.world_map.1, &self.show, global_state)
                    },
                    GameInput::MapZoomOut if state => {
                        handle_map_zoom(0.5, self.world_map.1, &self.show, global_state)
                    },
                    GameInput::MuteMaster if state => {
                        toggle_mute(Audio::MuteMasterVolume(!gs_audio.master_volume.muted))
                    },
                    GameInput::MuteInactiveMaster if state => {
                        toggle_mute(Audio::MuteInactiveMasterVolume(
                            !gs_audio.inactive_master_volume_perc.muted,
                        ))
                    },
                    GameInput::MuteMusic if state => {
                        toggle_mute(Audio::MuteMusicVolume(!gs_audio.music_volume.muted))
                    },
                    GameInput::MuteSfx if state => {
                        toggle_mute(Audio::MuteSfxVolume(!gs_audio.sfx_volume.muted))
                    },
                    GameInput::MuteAmbience if state => {
                        toggle_mute(Audio::MuteAmbienceVolume(!gs_audio.ambience_volume.muted))
                    },
                    // Skillbar
                    input => {
                        if let Some(slot) = try_hotbar_slot_from_input(input) {
                            handle_slot(
                                slot,
                                state,
                                &mut self.events,
                                &mut self.slot_manager,
                                &mut self.hotbar,
                                client_inventory,
                            );
                            true
                        } else {
                            false
                        }
                    },
                }
            },
            // Else the player is typing in chat
            WinEvent::InputUpdate(_key, _) => self.typing(),
            WinEvent::Char(_) => self.typing(),
            WinEvent::Focused(state) => {
                self.force_ungrab = !state;
                true
            },
            WinEvent::Moved(_) => {
                // Prevent the cursor from being grabbed while the window is being moved as this
                // causes the window to move erratically
                // TODO: this creates an issue where if you move the window then you need to
                // close a menu to re-grab the mouse (and if one isn't already
                // open you need to open and close a menu)
                self.show.want_grab = false;
                true
            },
            _ => false,
        };
        // Handle cursor grab.
        global_state
            .window
            .grab_cursor(!self.force_ungrab && self.show.want_grab);

        handled
    }

    pub fn maintain(
        &mut self,
        client: &Client,
        global_state: &mut GlobalState,
        debug_info: &Option<DebugInfo>,
        camera: &Camera,
        dt: Duration,
        info: HudInfo,
        interactable: Option<&Interactable>,
    ) -> Vec<Event> {
        span!(_guard, "maintain", "Hud::maintain");
        // conrod eats tabs. Un-eat a tabstop so tab completion can work
        if self.ui.ui.global_input().events().any(|event| {
            use conrod_core::{event, input};
            matches!(
                event,
                /* event::Event::Raw(event::Input::Press(input::Button::Keyboard(input::Key::
                 * Tab))) | */
                event::Event::Ui(event::Ui::Press(_, event::Press {
                    button: event::Button::Keyboard(input::Key::Tab),
                    ..
                },))
            )
        }) {
            self.ui
                .ui
                .handle_event(conrod_core::event::Input::Text("\t".to_string()));
        }

        // Stop selecting a sprite to perform crafting with when out of range or sprite
        // has been removed
        self.show.crafting_fields.craft_sprite =
            self.show
                .crafting_fields
                .craft_sprite
                .filter(|(pos, sprite)| {
                    self.show.crafting
                        && if let Some(player_pos) = client.position() {
                            pos.get_block_and_transform(
                                &client.state().terrain(),
                                &client.state().ecs().read_resource(),
                                |e| {
                                    client
                                        .state()
                                        .read_storage::<vcomp::Interpolated>()
                                        .get(e)
                                        .map(|interpolated| {
                                            (comp::Pos(interpolated.pos), interpolated.ori)
                                        })
                                },
                                &client.state().read_storage(),
                            )
                            .map_or(false, |(mat, _, block)| {
                                block.get_sprite() == Some(*sprite)
                                    && mat.mul_point(Vec3::broadcast(0.5)).distance(player_pos)
                                        < MAX_PICKUP_RANGE
                            })
                        } else {
                            false
                        }
                });

        // Optimization: skip maintaining UI when it's off.
        if !self.show.ui {
            return std::mem::take(&mut self.events);
        }

        if let Some(maybe_id) = self.to_focus.take() {
            self.ui.focus_widget(maybe_id);
        }
        let events = self.update_layout(
            client,
            global_state,
            debug_info,
            dt,
            info,
            camera,
            interactable,
        );
        let camera::Dependents {
            view_mat, proj_mat, ..
        } = camera.dependents();
        let focus_off = camera.get_focus_pos().map(f32::trunc);

        // Check if item images need to be reloaded
        self.item_imgs.reload_if_changed(&mut self.ui);
        // TODO: using a thread pool in the obvious way for speeding up map zoom results
        // in flickering artifacts, figure out a better way to make use of the
        // thread pool
        let _pool = client.state().ecs().read_resource::<SlowJobPool>();
        self.ui.maintain(
            global_state.window.renderer_mut(),
            None,
            //Some(&pool),
            Some(proj_mat * view_mat * Mat4::translation_3d(-focus_off)),
        );

        events
    }

    #[inline]
    pub fn clear_cursor(&mut self) { self.slot_manager.idle(); }

    pub fn render<'a>(&'a self, drawer: &mut UiDrawer<'_, 'a>) {
        span!(_guard, "render", "Hud::render");
        // Don't show anything if the UI is toggled off.
        if self.show.ui {
            self.ui.render(drawer);
        }
    }

    pub fn free_look(&mut self, free_look: bool) { self.show.free_look = free_look; }

    pub fn auto_walk(&mut self, auto_walk: bool) { self.show.auto_walk = auto_walk; }

    pub fn camera_clamp(&mut self, camera_clamp: bool) { self.show.camera_clamp = camera_clamp; }

    /// Remind the player camera zoom is currently locked, for example if they
    /// are trying to zoom.
    pub fn zoom_lock_reminder(&mut self) {
        if self.show.zoom_lock.reason.is_none() {
            self.show.zoom_lock = ChangeNotification::from_reason(NotificationReason::Remind);
        }
    }

    /// Start showing a temporary notification ([ChangeNotification]) that zoom
    /// lock was toggled on/off.
    pub fn zoom_lock_toggle(&mut self, state: bool) {
        self.show.zoom_lock = ChangeNotification::from_state(state);
    }

    pub fn show_content_bubble(&mut self, pos: Vec3<f32>, content: comp::Content) {
        self.content_bubbles.push((
            pos,
            comp::SpeechBubble::new(content, comp::SpeechBubbleType::None),
        ));
    }

    pub fn handle_outcome(
        &mut self,
        outcome: &Outcome,
        client: &Client,
        global_state: &GlobalState,
    ) {
        let interface = &global_state.settings.interface;
        match outcome {
            Outcome::ExpChange { uid, exp, xp_pools } => {
                let ecs = client.state().ecs();
                let uids = ecs.read_storage::<Uid>();
                let me = client.entity();

                if uids.get(me).map_or(false, |me| *me == *uid) {
                    match self.floaters.exp_floaters.last_mut() {
                        Some(floater)
                            if floater.timer
                                > (EXP_FLOATER_LIFETIME - EXP_ACCUMULATION_DURATION)
                                && global_state.settings.interface.accum_experience
                                && floater.owner == *uid =>
                        {
                            floater.jump_timer = 0.0;
                            floater.exp_change += *exp;
                        },
                        _ => self.floaters.exp_floaters.push(ExpFloater {
                            // Store the owner as to not accumulate old experience floaters
                            owner: *uid,
                            exp_change: *exp,
                            timer: EXP_FLOATER_LIFETIME,
                            jump_timer: 0.0,
                            rand_offset: rand::thread_rng().gen::<(f32, f32)>(),
                            xp_pools: xp_pools.clone(),
                        }),
                    }
                }
            },
            Outcome::SkillPointGain {
                uid,
                skill_tree,
                total_points,
                ..
            } => {
                let ecs = client.state().ecs();
                let uids = ecs.read_storage::<Uid>();
                let me = client.entity();

                if uids.get(me).map_or(false, |me| *me == *uid) {
                    self.floaters.skill_point_displays.push(SkillPointGain {
                        skill_tree: *skill_tree,
                        total_points: *total_points,
                        timer: 5.0,
                    });
                }
            },
            Outcome::ComboChange { uid, combo } => {
                let ecs = client.state().ecs();
                let uids = ecs.read_storage::<Uid>();
                let me = client.entity();

                if uids.get(me).map_or(false, |me| *me == *uid) {
                    self.floaters.combo_floater = Some(ComboFloater {
                        combo: *combo,
                        timer: comp::combo::COMBO_DECAY_START,
                    });
                }
            },
            Outcome::Block { uid, parry, .. } if *parry => {
                let ecs = client.state().ecs();
                let uids = ecs.read_storage::<Uid>();
                let me = client.entity();

                if uids.get(me).map_or(false, |me| *me == *uid) {
                    self.floaters
                        .block_floaters
                        .push(BlockFloater { timer: 1.0 });
                }
            },
            Outcome::HealthChange { info, .. } => {
                let ecs = client.state().ecs();
                let mut hp_floater_lists = ecs.write_storage::<HpFloaterList>();
                let uids = ecs.read_storage::<Uid>();
                let me = client.entity();
                let my_uid = uids.get(me);

                if let Some(entity) = ecs.entity_from_uid(info.target) {
                    if let Some(floater_list) = hp_floater_lists.get_mut(entity) {
                        let hit_me = my_uid.map_or(false, |&uid| {
                            (info.target == uid) && global_state.settings.interface.sct_inc_dmg
                        });
                        if match info.by {
                            Some(by) => {
                                let by_me = my_uid.map_or(false, |&uid| by.uid() == uid);
                                // If the attack was by me also reset this timer
                                if by_me {
                                    floater_list.time_since_last_dmg_by_me = Some(0.0);
                                }
                                hit_me || by_me
                            },
                            None => hit_me,
                        } {
                            // Group up damage from the same tick and instance number
                            for floater in floater_list.floaters.iter_mut().rev() {
                                if floater.timer > 0.0 {
                                    break;
                                }
                                if floater.info.instance == info.instance
                                    // Group up precision hits and regular attacks for incoming damage
                                    && (hit_me
                                        || floater.info.precise
                                            == info.precise)
                                {
                                    floater.info.amount += info.amount;
                                    if info.precise {
                                        floater.info.precise = info.precise
                                    }
                                    return;
                                }
                            }

                            // To separate healing and damage floaters alongside the precise and
                            // non-precise ones
                            let last_floater = if !info.precise || hit_me {
                                floater_list.floaters.iter_mut().rev().find(|f| {
                                    (if info.amount < 0.0 {
                                        f.info.amount < 0.0
                                    } else {
                                        f.info.amount > 0.0
                                    }) && f.timer
                                        < if hit_me {
                                            interface.sct_inc_dmg_accum_duration
                                        } else {
                                            interface.sct_dmg_accum_duration
                                        }
                                    // Ignore precise floaters, unless the damage is incoming
                                    && (hit_me || !f.info.precise)
                                })
                            } else {
                                None
                            };

                            match last_floater {
                                Some(f) => {
                                    f.jump_timer = 0.0;
                                    f.info.amount += info.amount;
                                    f.info.precise = info.precise;
                                },
                                _ => {
                                    floater_list.floaters.push(HpFloater {
                                        timer: 0.0,
                                        jump_timer: 0.0,
                                        info: *info,
                                        rand: rand::random(),
                                    });
                                },
                            }
                        }
                    }
                }
            },

            _ => {},
        }
    }
}
// Get item qualities of equipped items and assign a tooltip title/frame color
pub fn get_quality_col<I: ItemDesc + ?Sized>(item: &I) -> Color {
    match item.quality() {
        Quality::Low => QUALITY_LOW,
        Quality::Common => QUALITY_COMMON,
        Quality::Moderate => QUALITY_MODERATE,
        Quality::High => QUALITY_HIGH,
        Quality::Epic => QUALITY_EPIC,
        Quality::Legendary => QUALITY_LEGENDARY,
        Quality::Artifact => QUALITY_ARTIFACT,
        Quality::Debug => QUALITY_DEBUG,
    }
}

fn try_hotbar_slot_from_input(input: GameInput) -> Option<hotbar::Slot> {
    Some(match input {
        GameInput::Slot1 => hotbar::Slot::One,
        GameInput::Slot2 => hotbar::Slot::Two,
        GameInput::Slot3 => hotbar::Slot::Three,
        GameInput::Slot4 => hotbar::Slot::Four,
        GameInput::Slot5 => hotbar::Slot::Five,
        GameInput::Slot6 => hotbar::Slot::Six,
        GameInput::Slot7 => hotbar::Slot::Seven,
        GameInput::Slot8 => hotbar::Slot::Eight,
        GameInput::Slot9 => hotbar::Slot::Nine,
        GameInput::Slot10 => hotbar::Slot::Ten,
        _ => return None,
    })
}

pub fn cr_color(combat_rating: f32) -> Color {
    let common = 2.0;
    let moderate = 3.5;
    let high = 6.5;
    let epic = 8.5;
    let legendary = 10.4;
    let artifact = 122.0;
    let debug = 200.0;

    match combat_rating {
        x if (0.0..common).contains(&x) => QUALITY_LOW,
        x if (common..moderate).contains(&x) => QUALITY_COMMON,
        x if (moderate..high).contains(&x) => QUALITY_MODERATE,
        x if (high..epic).contains(&x) => QUALITY_HIGH,
        x if (epic..legendary).contains(&x) => QUALITY_EPIC,
        x if (legendary..artifact).contains(&x) => QUALITY_LEGENDARY,
        x if (artifact..debug).contains(&x) => QUALITY_ARTIFACT,
        x if x >= debug => QUALITY_DEBUG,
        _ => XP_COLOR,
    }
}

pub fn get_buff_image(buff: BuffKind, imgs: &Imgs) -> conrod_core::image::Id {
    match buff {
        // Buffs
        BuffKind::Regeneration => imgs.buff_plus_0,
        BuffKind::Saturation => imgs.buff_saturation_0,
        BuffKind::Potion => imgs.buff_potion_0,
        // TODO: Need unique image for Agility (uses same as Hastened atm)
        BuffKind::Agility => imgs.buff_haste_0,
        BuffKind::CampfireHeal => imgs.buff_campfire_heal_0,
        BuffKind::EnergyRegen => imgs.buff_energyplus_0,
        BuffKind::IncreaseMaxEnergy => imgs.buff_energyplus_0,
        BuffKind::IncreaseMaxHealth => imgs.buff_healthplus_0,
        BuffKind::Invulnerability => imgs.buff_invincibility_0,
        BuffKind::ProtectingWard => imgs.buff_dmg_red_0,
        BuffKind::Frenzied => imgs.buff_frenzy_0,
        BuffKind::Hastened => imgs.buff_haste_0,
        BuffKind::Fortitude => imgs.buff_fortitude_0,
        BuffKind::Reckless => imgs.buff_reckless,
        BuffKind::Flame => imgs.buff_flame,
        BuffKind::Frigid => imgs.buff_frigid,
        BuffKind::Lifesteal => imgs.buff_lifesteal,
        // TODO: Get image
        // BuffKind::SalamanderAspect => imgs.debuff_burning_0,
        BuffKind::ImminentCritical => imgs.buff_imminentcritical,
        BuffKind::Fury => imgs.buff_fury,
        BuffKind::Sunderer => imgs.buff_sunderer,
        BuffKind::Defiance => imgs.buff_defiance,
        BuffKind::Bloodfeast => imgs.buff_plus_0,
        BuffKind::Berserk => imgs.buff_reckless,
        //  Debuffs
        BuffKind::Bleeding => imgs.debuff_bleed_0,
        BuffKind::Cursed => imgs.debuff_skull_0,
        BuffKind::Burning => imgs.debuff_burning_0,
        BuffKind::Crippled => imgs.debuff_crippled_0,
        BuffKind::Frozen => imgs.debuff_frozen_0,
        BuffKind::Wet => imgs.debuff_wet_0,
        BuffKind::Ensnared => imgs.debuff_ensnared_0,
        BuffKind::Poisoned => imgs.debuff_poisoned_0,
        BuffKind::Parried => imgs.debuff_parried_0,
        BuffKind::PotionSickness => imgs.debuff_potionsickness_0,
        BuffKind::Polymorphed => imgs.debuff_polymorphed,
        BuffKind::Heatstroke => imgs.debuff_heatstroke_0,
    }
}

pub fn get_buff_title(buff: BuffKind, localized_strings: &Localization) -> Cow<str> {
    match buff {
        // Buffs
        BuffKind::Regeneration => localized_strings.get_msg("buff-title-heal"),
        BuffKind::Saturation => localized_strings.get_msg("buff-title-saturation"),
        BuffKind::Potion => localized_strings.get_msg("buff-title-potion"),
        BuffKind::Agility => localized_strings.get_msg("buff-title-agility"),
        BuffKind::CampfireHeal => localized_strings.get_msg("buff-title-campfire_heal"),
        BuffKind::EnergyRegen => localized_strings.get_msg("buff-title-energy_regen"),
        BuffKind::IncreaseMaxHealth => localized_strings.get_msg("buff-title-increase_max_health"),
        BuffKind::IncreaseMaxEnergy => localized_strings.get_msg("buff-title-increase_max_energy"),
        BuffKind::Invulnerability => localized_strings.get_msg("buff-title-invulnerability"),
        BuffKind::ProtectingWard => localized_strings.get_msg("buff-title-protectingward"),
        BuffKind::Frenzied => localized_strings.get_msg("buff-title-frenzied"),
        BuffKind::Hastened => localized_strings.get_msg("buff-title-hastened"),
        BuffKind::Fortitude => localized_strings.get_msg("buff-title-fortitude"),
        BuffKind::Reckless => localized_strings.get_msg("buff-title-reckless"),
        // BuffKind::SalamanderAspect => localized_strings.get_msg("buff-title-salamanderaspect"),
        BuffKind::Flame => localized_strings.get_msg("buff-title-burn"),
        BuffKind::Frigid => localized_strings.get_msg("buff-title-frigid"),
        BuffKind::Lifesteal => localized_strings.get_msg("buff-title-lifesteal"),
        BuffKind::ImminentCritical => localized_strings.get_msg("buff-title-imminentcritical"),
        BuffKind::Fury => localized_strings.get_msg("buff-title-fury"),
        BuffKind::Sunderer => localized_strings.get_msg("buff-title-sunderer"),
        BuffKind::Defiance => localized_strings.get_msg("buff-title-defiance"),
        BuffKind::Bloodfeast => localized_strings.get_msg("buff-title-bloodfeast"),
        BuffKind::Berserk => localized_strings.get_msg("buff-title-berserk"),
        // Debuffs
        BuffKind::Bleeding => localized_strings.get_msg("buff-title-bleed"),
        BuffKind::Cursed => localized_strings.get_msg("buff-title-cursed"),
        BuffKind::Burning => localized_strings.get_msg("buff-title-burn"),
        BuffKind::Crippled => localized_strings.get_msg("buff-title-crippled"),
        BuffKind::Frozen => localized_strings.get_msg("buff-title-frozen"),
        BuffKind::Wet => localized_strings.get_msg("buff-title-wet"),
        BuffKind::Ensnared => localized_strings.get_msg("buff-title-ensnared"),
        BuffKind::Poisoned => localized_strings.get_msg("buff-title-poisoned"),
        BuffKind::Parried => localized_strings.get_msg("buff-title-parried"),
        BuffKind::PotionSickness => localized_strings.get_msg("buff-title-potionsickness"),
        BuffKind::Polymorphed => localized_strings.get_msg("buff-title-polymorphed"),
        BuffKind::Heatstroke => localized_strings.get_msg("buff-title-heatstroke"),
    }
}

pub fn get_buff_desc(buff: BuffKind, data: BuffData, localized_strings: &Localization) -> Cow<str> {
    match buff {
        // Buffs
        BuffKind::Regeneration => localized_strings.get_msg("buff-desc-heal"),
        BuffKind::Saturation => localized_strings.get_msg("buff-desc-saturation"),
        BuffKind::Potion => localized_strings.get_msg("buff-desc-potion"),
        BuffKind::Agility => localized_strings.get_msg("buff-desc-agility"),
        BuffKind::CampfireHeal => {
            localized_strings.get_msg_ctx("buff-desc-campfire_heal", &i18n::fluent_args! {
                "rate" => data.strength * 100.0
            })
        },
        BuffKind::EnergyRegen => localized_strings.get_msg("buff-desc-energy_regen"),
        BuffKind::IncreaseMaxHealth => localized_strings.get_msg("buff-desc-increase_max_health"),
        BuffKind::IncreaseMaxEnergy => localized_strings.get_msg("buff-desc-increase_max_energy"),
        BuffKind::Invulnerability => localized_strings.get_msg("buff-desc-invulnerability"),
        BuffKind::ProtectingWard => localized_strings.get_msg("buff-desc-protectingward"),
        BuffKind::Frenzied => localized_strings.get_msg("buff-desc-frenzied"),
        BuffKind::Hastened => localized_strings.get_msg("buff-desc-hastened"),
        BuffKind::Fortitude => localized_strings.get_msg("buff-desc-fortitude"),
        BuffKind::Reckless => localized_strings.get_msg("buff-desc-reckless"),
        // BuffKind::SalamanderAspect => localized_strings.get_msg("buff-desc-salamanderaspect"),
        BuffKind::Flame => localized_strings.get_msg("buff-desc-flame"),
        BuffKind::Frigid => localized_strings.get_msg("buff-desc-frigid"),
        BuffKind::Lifesteal => localized_strings.get_msg("buff-desc-lifesteal"),
        BuffKind::ImminentCritical => localized_strings.get_msg("buff-desc-imminentcritical"),
        BuffKind::Fury => localized_strings.get_msg("buff-desc-fury"),
        BuffKind::Sunderer => localized_strings.get_msg("buff-desc-sunderer"),
        BuffKind::Defiance => localized_strings.get_msg("buff-desc-defiance"),
        BuffKind::Bloodfeast => localized_strings.get_msg("buff-desc-bloodfeast"),
        BuffKind::Berserk => localized_strings.get_msg("buff-desc-berserk"),
        // Debuffs
        BuffKind::Bleeding => localized_strings.get_msg("buff-desc-bleed"),
        BuffKind::Cursed => localized_strings.get_msg("buff-desc-cursed"),
        BuffKind::Burning => localized_strings.get_msg("buff-desc-burn"),
        BuffKind::Crippled => localized_strings.get_msg("buff-desc-crippled"),
        BuffKind::Frozen => localized_strings.get_msg("buff-desc-frozen"),
        BuffKind::Wet => localized_strings.get_msg("buff-desc-wet"),
        BuffKind::Ensnared => localized_strings.get_msg("buff-desc-ensnared"),
        BuffKind::Poisoned => localized_strings.get_msg("buff-desc-poisoned"),
        BuffKind::Parried => localized_strings.get_msg("buff-desc-parried"),
        BuffKind::PotionSickness => localized_strings.get_msg("buff-desc-potionsickness"),
        BuffKind::Polymorphed => localized_strings.get_msg("buff-desc-polymorphed"),
        BuffKind::Heatstroke => localized_strings.get_msg("buff-desc-heatstroke"),
    }
}

pub fn get_sprite_desc(sprite: SpriteKind, localized_strings: &Localization) -> Option<Cow<str>> {
    let i18n_key = match sprite {
        SpriteKind::Empty => return None,
        SpriteKind::GlassBarrier => return None,
        SpriteKind::Anvil => "hud-crafting-anvil",
        SpriteKind::Cauldron => "hud-crafting-cauldron",
        SpriteKind::CookingPot => "hud-crafting-cooking_pot",
        SpriteKind::RepairBench => "hud-crafting-repair_bench",
        SpriteKind::CraftingBench => "hud-crafting-crafting_bench",
        SpriteKind::Forge => "hud-crafting-forge",
        SpriteKind::Loom => "hud-crafting-loom",
        SpriteKind::SpinningWheel => "hud-crafting-spinning_wheel",
        SpriteKind::TanningRack => "hud-crafting-tanning_rack",
        SpriteKind::DismantlingBench => "hud-crafting-salvaging_station",
        SpriteKind::ChestBuried
        | SpriteKind::Chest
        | SpriteKind::CoralChest
        | SpriteKind::DungeonChest0
        | SpriteKind::DungeonChest1
        | SpriteKind::DungeonChest2
        | SpriteKind::DungeonChest3
        | SpriteKind::DungeonChest4
        | SpriteKind::DungeonChest5 => "common-sprite-chest",
        SpriteKind::Mud => "common-sprite-mud",
        SpriteKind::Grave => "common-sprite-grave",
        SpriteKind::ChairSingle | SpriteKind::ChairDouble => "common-sprite-chair",
        SpriteKind::Crate => "common-sprite-crate",
        sprite => return Some(Cow::Owned(format!("{:?}", sprite))),
    };
    Some(localized_strings.get_msg(i18n_key))
}

pub fn angle_of_attack_text(
    fluid: Option<comp::Fluid>,
    velocity: Option<comp::Vel>,
    character_state: Option<&comp::CharacterState>,
) -> String {
    use comp::CharacterState;

    let glider_ori = if let Some(CharacterState::Glide(data)) = character_state {
        data.ori
    } else {
        return "Angle of Attack: Not gliding".to_owned();
    };

    let fluid = if let Some(fluid) = fluid {
        fluid
    } else {
        return "Angle of Attack: Not in fluid".to_owned();
    };

    let velocity = if let Some(velocity) = velocity {
        velocity
    } else {
        return "Angle of Attack: Player has no vel component".to_owned();
    };
    let rel_flow = fluid.relative_flow(&velocity).0;
    let v_sq = rel_flow.magnitude_squared();

    if v_sq.abs() > 0.0001 {
        let rel_flow_dir = Dir::new(rel_flow / v_sq.sqrt());
        let aoe = fluid_dynamics::angle_of_attack(&glider_ori, &rel_flow_dir);
        format!("Angle of Attack: {:.1}", aoe.to_degrees())
    } else {
        "Angle of Attack: Not moving".to_owned()
    }
}

/// Converts multiplier to percentage.
/// NOTE: floats are not the most precise type.
///
/// # Examples
/// ```
/// use veloren_voxygen::hud::multiplier_to_percentage;
///
/// let positive = multiplier_to_percentage(1.05);
/// assert!((positive - 5.0).abs() < 0.0001);
/// let negative = multiplier_to_percentage(0.85);
/// assert!((negative - (-15.0)).abs() < 0.0001);
/// ```
pub fn multiplier_to_percentage(value: f32) -> f32 { value * 100.0 - 100.0 }
