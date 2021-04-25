mod bag;
mod buffs;
mod buttons;
mod chat;
mod crafting;
mod diary;
mod esc_menu;
mod group;
mod hotbar;
pub mod img_ids;
pub mod item_imgs;
mod map;
mod minimap;
mod overhead;
mod overitem;
mod popup;
mod prompt_dialog;
mod settings_window;
mod skillbar;
mod slots;
mod social;
mod trade;
pub mod util;

pub use crafting::CraftingTab;
pub use hotbar::{SlotContents as HotbarSlotContents, State as HotbarState};
pub use item_imgs::animate_by_pulse;
pub use settings_window::ScaleChange;

use bag::Bag;
use buffs::BuffsBar;
use buttons::Buttons;
use chat::Chat;
use chrono::NaiveTime;
use crafting::Crafting;
use diary::{Diary, SelectedSkillTree};
use esc_menu::EscMenu;
use group::Group;
use img_ids::Imgs;
use item_imgs::ItemImgs;
use map::Map;
use minimap::MiniMap;
use popup::Popup;
use prompt_dialog::PromptDialog;
use serde::{Deserialize, Serialize};
use settings_window::{SettingsTab, SettingsWindow};
use skillbar::Skillbar;
use social::Social;
use trade::Trade;

use crate::{
    ecs::{comp as vcomp, comp::HpFloaterList},
    hud::{img_ids::ImgsRot, prompt_dialog::DialogOutcomeEvent},
    i18n::Localization,
    render::{Consts, Globals, Renderer},
    scene::camera::{self, Camera},
    session::{
        settings_change::{Interface as InterfaceChange, SettingsChange},
        Interactable,
    },
    ui::{
        fonts::Fonts, img_ids::Rotations, slot, slot::SlotKey, Graphic, Ingameable, ScaleMode, Ui,
    },
    window::{Event as WinEvent, GameInput},
    GlobalState,
};
use client::Client;
use common::{
    combat,
    comp::{
        self,
        inventory::trade_pricing::TradePricing,
        item::{tool::ToolKind, ItemDesc, MaterialStatManifest, Quality},
        skills::{Skill, SkillGroupKind},
        BuffData, BuffKind, Item,
    },
    consts::MAX_PICKUP_RANGE,
    outcome::Outcome,
    terrain::{SpriteKind, TerrainChunk},
    trade::{ReducedInventory, TradeAction},
    uid::Uid,
    util::srgba_to_linear,
    vol::RectRasterableVol,
};
use common_base::span;
use common_net::{
    msg::{world_msg::SiteId, Notification, PresenceKind},
    sync::WorldSyncExt,
};
use conrod_core::{
    text::cursor::Index,
    widget::{self, Button, Image, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use hashbrown::HashMap;
use rand::Rng;
use specs::{Join, WorldExt};
use std::{
    borrow::Cow,
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use vek::*;

const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
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
//const TRANSPARENT: Color = Color::Rgba(0.0, 0.0, 0.0, 0.0);
//const FOCUS_COLOR: Color = Color::Rgba(1.0, 0.56, 0.04, 1.0);
//const RAGE_COLOR: Color = Color::Rgba(0.5, 0.04, 0.13, 1.0);
const BUFF_COLOR: Color = Color::Rgba(0.06, 0.69, 0.12, 1.0);
const DEBUFF_COLOR: Color = Color::Rgba(0.79, 0.19, 0.17, 1.0);

// Item Quality Colors
const QUALITY_LOW: Color = Color::Rgba(0.41, 0.41, 0.41, 1.0); // Grey - Trash, can be sold to vendors
const QUALITY_COMMON: Color = Color::Rgba(0.79, 1.09, 1.09, 1.0); // No Color - Crafting mats, food, starting equipment, quest items (like keys), rewards for easy quests
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
/// Color for collected loot messages
const LOOT_COLOR: Color = Color::Rgba(0.69, 0.57, 1.0, 1.0);

//Nametags
const GROUP_MEMBER: Color = Color::Rgba(0.47, 0.84, 1.0, 1.0);
const DEFAULT_NPC: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);

// UI Color-Theme
const UI_MAIN: Color = Color::Rgba(0.61, 0.70, 0.70, 1.0); // Greenish Blue
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
        orientation,
        loaded_distance,
        time,
        entity_count,
        num_chunks,
        num_lights,
        num_figures,
        num_particles,

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
        crafting_window,
        settings_window,
        group_window,
        item_info,

        // Free look indicator
        free_look_txt,
        free_look_bg,

        // Auto walk indicator
        auto_walk_txt,
        auto_walk_bg,

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

#[derive(Clone, Copy)]
pub struct BuffInfo {
    kind: comp::BuffKind,
    data: comp::BuffData,
    is_buff: bool,
    dur: Option<Duration>,
}

pub struct ExpFloater {
    pub owner: Uid,
    pub exp_change: i32,
    pub timer: f32,
    pub rand_offset: (f32, f32),
}

pub struct SkillPointGain {
    pub owner: Uid,
    pub skill_tree: SkillGroupKind,
    pub total_points: u16,
    pub timer: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ComboFloater {
    pub owner: Uid,
    pub combo: u32,
    pub timer: f64,
}

pub struct BlockFloater {
    pub owner: Uid,
    pub timer: f32,
}

pub struct DebugInfo {
    pub tps: f64,
    pub frame_time: Duration,
    pub ping_ms: f64,
    pub coordinates: Option<comp::Pos>,
    pub velocity: Option<comp::Vel>,
    pub ori: Option<comp::Ori>,
    pub num_chunks: u32,
    pub num_lights: u32,
    pub num_visible_chunks: u32,
    pub num_shadow_chunks: u32,
    pub num_figures: u32,
    pub num_figures_visible: u32,
    pub num_particles: u32,
    pub num_particles_visible: u32,
}

pub struct HudInfo {
    pub is_aiming: bool,
    pub is_first_person: bool,
    pub target_entity: Option<specs::Entity>,
    pub selected_entity: Option<(specs::Entity, std::time::Instant)>,
}

#[derive(Clone)]
pub enum Event {
    SendMessage(String),

    CharacterSelection,
    UseSlot {
        slot: comp::slot::Slot,
        bypass_dialog: bool,
    },
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
    Ability3(bool),
    Ability4(bool),
    Logout,
    Quit,

    CraftRecipe {
        recipe: String,
        craft_sprite: Option<(Vec3<i32>, SpriteKind)>,
    },
    InviteMember(Uid),
    AcceptInvite,
    DeclineInvite,
    KickMember(Uid),
    LeaveGroup,
    AssignLeader(Uid),
    RemoveBuff(BuffKind),
    UnlockSkill(Skill),
    RequestSiteInfo(SiteId),

    SettingsChange(SettingsChange),
}

// TODO: Are these the possible layouts we want?
// TODO: Maybe replace this with bitflags.
// `map` is not here because it currently is displayed over the top of other
// open windows.
#[derive(PartialEq)]
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

pub struct Show {
    ui: bool,
    intro: bool,
    help: bool,
    crafting: bool,
    debug: bool,
    bag: bool,
    bag_inv: bool,
    trade: bool,
    social: bool,
    diary: bool,
    group: bool,
    group_menu: bool,
    esc_menu: bool,
    open_windows: Windows,
    map: bool,
    ingame: bool,
    settings_tab: SettingsTab,
    skilltreetab: SelectedSkillTree,
    crafting_tab: CraftingTab,
    crafting_search_key: Option<String>,
    craft_sprite: Option<(Vec3<i32>, SpriteKind)>,
    social_search_key: Option<String>,
    want_grab: bool,
    stats: bool,
    free_look: bool,
    auto_walk: bool,
    camera_clamp: bool,
    prompt_dialog: Option<PromptDialogSettings>,
}
impl Show {
    fn bag(&mut self, open: bool) {
        if !self.esc_menu {
            self.bag = open;
            self.map = false;
            self.want_grab = !open;

            if !open {
                self.crafting = false;
            }
        }
    }

    fn trade(&mut self, open: bool) {
        if !self.esc_menu {
            self.bag = open;
            self.trade = open;
            self.map = false;
            self.want_grab = !open;
        }
    }

    fn map(&mut self, open: bool) {
        if !self.esc_menu {
            self.map = open;
            self.bag = false;
            self.crafting = false;
            self.social = false;
            self.diary = false;
            self.want_grab = !open;
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
            self.want_grab = !open;
        }
    }

    fn crafting(&mut self, open: bool) {
        if !self.esc_menu {
            if !self.crafting && open {
                // rising edge detector
                self.search_crafting_recipe(None);
            }
            self.crafting = open;
            self.bag = open;
            self.map = false;
            self.want_grab = !open;
        }
    }

    pub fn open_crafting_tab(
        &mut self,
        tab: CraftingTab,
        craft_sprite: Option<(Vec3<i32>, SpriteKind)>,
    ) {
        self.selected_crafting_tab(tab);
        self.crafting(true);
        self.craft_sprite = self.craft_sprite.or(craft_sprite);
    }

    fn diary(&mut self, open: bool) {
        if !self.esc_menu {
            self.social = false;
            self.crafting = false;
            self.bag = false;
            self.map = false;
            self.diary = open;
            self.want_grab = !open;
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
            self.crafting = false;
            self.diary = false;
            self.want_grab = !open;
        }
    }

    fn toggle_bag(&mut self) { self.bag(!self.bag); }

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

    fn toggle_windows(&mut self, global_state: &mut GlobalState) {
        if self.bag
            || self.trade
            || self.esc_menu
            || self.map
            || self.social
            || self.crafting
            || self.diary
            || self.help
            || self.intro
            || !matches!(self.open_windows, Windows::None)
        {
            self.bag = false;
            self.trade = false;
            self.esc_menu = false;
            self.help = false;
            self.intro = false;
            self.map = false;
            self.social = false;
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
        self.skilltreetab = tree_sel;
        self.social = false;
    }

    fn selected_crafting_tab(&mut self, sel_cat: CraftingTab) { self.crafting_tab = sel_cat; }

    fn search_crafting_recipe(&mut self, search_key: Option<String>) {
        self.crafting_search_key = search_key;
    }

    fn search_social_players(&mut self, search_key: Option<String>) {
        self.social_search_key = search_key;
    }

    /// If all of the menus are closed, adjusts coordinates of cursor to center
    /// of screen
    fn toggle_cursor_on_menu_close(&self, global_state: &mut GlobalState) {
        if !self.bag
            && !self.trade
            && !self.esc_menu
            && !self.map
            && !self.social
            && !self.crafting
            && !self.diary
            && !self.help
            && !self.intro
            && global_state.window.is_cursor_grabbed()
        {
            global_state.window.center_cursor();
        }
    }
}

pub struct PromptDialogSettings {
    message: String,
    affirmative_event: Event,
    negative_event: Option<Event>,
    outcome_via_keypress: Option<bool>,
}

impl PromptDialogSettings {
    pub fn new(message: String, affirmative_event: Event, negative_event: Option<Event>) -> Self {
        Self {
            message,
            affirmative_event,
            negative_event,
            outcome_via_keypress: None,
        }
    }

    pub fn set_outcome_via_keypress(&mut self, outcome: bool) {
        self.outcome_via_keypress = Some(outcome);
    }
}

pub struct Floaters {
    pub exp_floaters: Vec<ExpFloater>,
    pub skill_point_displays: Vec<SkillPointGain>,
    pub combo_floaters: VecDeque<ComboFloater>,
    pub block_floaters: Vec<BlockFloater>,
}

pub struct Hud {
    ui: Ui,
    ids: Ids,
    world_map: (/* Id */ Vec<Rotations>, Vec2<u32>),
    imgs: Imgs,
    item_imgs: ItemImgs,
    fonts: Fonts,
    rot_imgs: ImgsRot,
    new_messages: VecDeque<comp::ChatMsg>,
    new_notifications: VecDeque<Notification>,
    speech_bubbles: HashMap<Uid, comp::SpeechBubble>,
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
    _velocity: f32,
    slot_manager: slots::SlotManager,
    hotbar: hotbar::State,
    events: Vec<Event>,
    crosshair_opacity: f32,
    floaters: Floaters,
}

impl Hud {
    pub fn new(global_state: &mut GlobalState, client: &Client) -> Self {
        let window = &mut global_state.window;
        let settings = &global_state.settings;

        let mut ui = Ui::new(window).unwrap();
        ui.set_scaling_mode(settings.interface.ui_scale);
        // Generate ids.
        let ids = Ids::new(ui.id_generator());
        // NOTE: Use a border the same color as the LOD ocean color (but with a
        // translucent alpha since UI have transparency and LOD doesn't).
        let water_color = srgba_to_linear(Rgba::new(0.0, 0.18, 0.37, 1.0));
        // Load world map
        let mut layers = Vec::new();
        for layer in client.world_data().map_layers() {
            layers.push(
                ui.add_graphic_with_rotations(Graphic::Image(Arc::clone(layer), Some(water_color))),
            );
        }
        let world_map = (layers, client.world_data().chunk_size().map(|e| e as u32));
        // Load images.
        let imgs = Imgs::load(&mut ui).expect("Failed to load images!");
        // Load rotation images.
        let rot_imgs = ImgsRot::load(&mut ui).expect("Failed to load rot images!");
        // Load item images.
        let item_imgs = ItemImgs::new(&mut ui, imgs.not_found);
        // Load fonts.
        let fonts = Fonts::load(global_state.i18n.read().fonts(), &mut ui)
            .expect("Impossible to load fonts!");
        // Get the server name.
        let server = &client.server_info().name;
        // Get the id, unwrap is safe because this CANNOT be None at this
        // point.

        let character_id = match client.presence().unwrap() {
            PresenceKind::Character(id) => id,
            PresenceKind::Spectator => unreachable!("HUD creation in Spectator mode!"),
        };

        // Create a new HotbarState from the persisted slots.
        let hotbar_state =
            HotbarState::new(global_state.profile.get_hotbar_slots(server, character_id));

        let slot_manager = slots::SlotManager::new(
            ui.id_generator(),
            Vec2::broadcast(40.0)
            // TODO(heyzoos) Will be useful for whoever works on rendering the number of items "in hand".
            // fonts.cyri.conrod_id,
            // Vec2::new(1.0, 1.0),
            // fonts.cyri.scale(12),
            // TEXT_COLOR,
        );

        Self {
            ui,
            imgs,
            world_map,
            rot_imgs,
            item_imgs,
            fonts,
            ids,
            new_messages: VecDeque::new(),
            new_notifications: VecDeque::new(),
            speech_bubbles: HashMap::new(),
            //intro: false,
            //intro_2: false,
            show: Show {
                help: false,
                intro: false,
                debug: false,
                bag: false,
                bag_inv: false,
                trade: false,
                esc_menu: false,
                open_windows: Windows::None,
                map: false,
                crafting: false,
                ui: true,
                social: false,
                diary: false,
                group: false,
                group_menu: false,
                settings_tab: SettingsTab::Interface,
                skilltreetab: SelectedSkillTree::General,
                crafting_tab: CraftingTab::All,
                crafting_search_key: None,
                craft_sprite: None,
                social_search_key: None,
                want_grab: true,
                ingame: true,
                stats: false,
                free_look: false,
                auto_walk: false,
                camera_clamp: false,
                prompt_dialog: None,
            },
            to_focus: None,
            //never_show: false,
            force_ungrab: false,
            force_chat_input: None,
            force_chat_cursor: None,
            tab_complete: None,
            pulse: 0.0,
            _velocity: 0.0,
            slot_manager,
            hotbar: hotbar_state,
            events: Vec::new(),
            crosshair_opacity: 0.0,
            floaters: Floaters {
                exp_floaters: Vec::new(),
                skill_point_displays: Vec::new(),
                combo_floaters: VecDeque::new(),
                block_floaters: Vec::new(),
            },
        }
    }

    pub fn set_prompt_dialog(&mut self, prompt_dialog: PromptDialogSettings) {
        self.show.prompt_dialog = Some(prompt_dialog);
    }

    pub fn update_fonts(&mut self, i18n: &Localization) {
        self.fonts = Fonts::load(i18n.fonts(), &mut self.ui).expect("Impossible to load fonts!");
    }

    #[allow(clippy::assign_op_pattern)] // TODO: Pending review in #587
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_layout(
        &mut self,
        client: &Client,
        global_state: &GlobalState,
        debug_info: &Option<DebugInfo>,
        dt: Duration,
        info: HudInfo,
        camera: &Camera,
        interactable: Option<Interactable>,
    ) -> Vec<Event> {
        span!(_guard, "update_layout", "Hud::update_layout");
        let mut events = std::mem::replace(&mut self.events, Vec::new());
        let (ref mut ui_widgets, ref mut item_tooltip_manager, ref mut tooltip_manager) =
            &mut self.ui.set_widgets();
        // self.ui.set_item_widgets(); pulse time for pulsating elements
        self.pulse = self.pulse + dt.as_secs_f32();
        // FPS
        let fps = global_state.clock.stats().average_tps;
        let version = common::util::DISPLAY_VERSION_LONG.clone();
        let i18n = &global_state.i18n.read();
        let key_layout = &global_state.window.key_layout;

        if self.show.ingame {
            let ecs = client.state().ecs();
            let pos = ecs.read_storage::<comp::Pos>();
            let stats = ecs.read_storage::<comp::Stats>();
            let skill_sets = ecs.read_storage::<comp::SkillSet>();
            let healths = ecs.read_storage::<comp::Health>();
            let buffs = ecs.read_storage::<comp::Buffs>();
            let energy = ecs.read_storage::<comp::Energy>();
            let hp_floater_lists = ecs.read_storage::<vcomp::HpFloaterList>();
            let uids = ecs.read_storage::<Uid>();
            let interpolated = ecs.read_storage::<vcomp::Interpolated>();
            let scales = ecs.read_storage::<comp::Scale>();
            let bodies = ecs.read_storage::<comp::Body>();
            let items = ecs.read_storage::<comp::Item>();
            let inventories = ecs.read_storage::<comp::Inventory>();
            let msm = ecs.read_resource::<MaterialStatManifest>();
            let entities = ecs.entities();
            let me = client.entity();

            if (client.pending_trade().is_some() && !self.show.trade)
                || (client.pending_trade().is_none() && self.show.trade)
            {
                self.show.toggle_trade();
            }

            //self.input = client.read_storage::<comp::ControllerInputs>();
            if let Some(health) = healths.get(me) {
                // Hurt Frame
                let hp_percentage = health.current() as f32 / health.maximum() as f32 * 100.0;
                if hp_percentage < 10.0 && !health.is_dead {
                    let hurt_fade =
                        (self.pulse * (10.0 - hp_percentage as f32) * 0.1/* speed factor */).sin()
                            * 0.5
                            + 0.6; //Animation timer
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
                        self.crosshair_opacity * global_state.settings.interface.crosshair_transp,
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
            // SCT Output values are called hp_damage and floater.hp_change
            // Numbers are currently divided by 10 and rounded
            if global_state.settings.interface.sct {
                // Render Player SCT numbers
                let mut player_sct_bg_id_walker = self.ids.player_sct_bgs.walk();
                let mut player_sct_id_walker = self.ids.player_scts.walk();
                if let (Some(HpFloaterList { floaters, .. }), Some(health)) = (
                    hp_floater_lists
                        .get(me)
                        .filter(|fl| !fl.floaters.is_empty()),
                    healths.get(me),
                ) {
                    if global_state.settings.interface.sct_player_batch {
                        let number_speed = 100.0; // Player Batched Numbers Speed
                        let player_sct_bg_id = player_sct_bg_id_walker.next(
                            &mut self.ids.player_sct_bgs,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        let player_sct_id = player_sct_id_walker.next(
                            &mut self.ids.player_scts,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        // Calculate total change
                        // Ignores healing
                        let hp_damage = floaters.iter().fold(0, |acc, f| f.hp_change.min(0) + acc);
                        // Divide by 10 to stay in the same dimension as the HP display
                        let hp_dmg_rounded_abs = ((hp_damage + 5) / 10).abs();
                        let max_hp_frac = hp_damage.abs() as f32 / health.maximum() as f32;
                        let timer = floaters
                            .last()
                            .expect("There must be at least one floater")
                            .timer;
                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size = 30
                            + ((max_hp_frac * 10.0) as u32) * 3
                            + if timer < 0.1 {
                                FLASH_MAX * (((1.0 - timer / 0.1) * 10.0) as u32)
                            } else {
                                0
                            };
                        // Timer sets the widget offset
                        let y = timer as f64 * number_speed * -1.0;
                        // Timer sets text transparency
                        let hp_fade =
                            ((crate::ecs::sys::floater::MY_HP_SHOWTIME - timer) * 0.25) + 0.2;
                        Text::new(&format!("{}", hp_dmg_rounded_abs))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if hp_damage < 0 {
                                Color::Rgba(0.0, 0.0, 0.0, hp_fade)
                            } else {
                                Color::Rgba(0.0, 0.0, 0.0, 0.0)
                            })
                            .mid_bottom_with_margin_on(ui_widgets.window, 297.0 + y)
                            .set(player_sct_bg_id, ui_widgets);
                        Text::new(&format!("{}", hp_dmg_rounded_abs))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if hp_damage < 0 {
                                Color::Rgba(1.0, 0.1, 0.0, hp_fade)
                            } else {
                                Color::Rgba(0.0, 0.0, 0.0, 0.0)
                            })
                            .mid_bottom_with_margin_on(ui_widgets.window, 300.0 + y)
                            .set(player_sct_id, ui_widgets);
                    };
                    for floater in floaters {
                        // Healing always single numbers so just skip damage when in batch mode

                        if global_state.settings.interface.sct_player_batch && floater.hp_change < 0
                        {
                            continue;
                        }
                        let number_speed = 50.0; // Player Heal Speed
                        let player_sct_bg_id = player_sct_bg_id_walker.next(
                            &mut self.ids.player_sct_bgs,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        let player_sct_id = player_sct_id_walker.next(
                            &mut self.ids.player_scts,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        let max_hp_frac = floater.hp_change.abs() as f32 / health.maximum() as f32;
                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size = 30
                            + ((max_hp_frac * 10.0) as u32) * 3
                            + if floater.timer < 0.1 {
                                FLASH_MAX * (((1.0 - floater.timer / 0.1) * 10.0) as u32)
                            } else {
                                0
                            };
                        // Timer sets the widget offset
                        let y = if floater.hp_change < 0 {
                            floater.timer as f64
                            * number_speed
                            * floater.hp_change.signum() as f64
                            //* -1.0
                            + 300.0
                                - ui_widgets.win_h * 0.5
                        } else {
                            floater.timer as f64
                                * number_speed
                                * floater.hp_change.signum() as f64
                                * -1.0
                                + 300.0
                                - ui_widgets.win_h * 0.5
                        };
                        // Healing is offset randomly
                        let x = if floater.hp_change < 0 {
                            0.0
                        } else {
                            (floater.rand as f64 - 0.5) * 0.2 * ui_widgets.win_w
                        };
                        // Timer sets text transparency
                        let hp_fade = ((crate::ecs::sys::floater::MY_HP_SHOWTIME - floater.timer)
                            * 0.25)
                            + 0.2;
                        if floater.hp_change.abs() > 10 {
                            Text::new(&format!("{}", (floater.hp_change / 10).abs()))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(Color::Rgba(0.0, 0.0, 0.0, hp_fade))
                                .x_y(x, y - 3.0)
                                .set(player_sct_bg_id, ui_widgets);
                            Text::new(&format!("{}", (floater.hp_change / 10).abs()))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(if floater.hp_change < 0 {
                                    Color::Rgba(1.0, 0.1, 0.0, hp_fade)
                                } else {
                                    Color::Rgba(0.1, 1.0, 0.1, hp_fade)
                                })
                                .x_y(x, y)
                                .set(player_sct_id, ui_widgets);
                        } else {
                            Text::new(&format!("{}", (floater.hp_change as f32 / 10.0).abs()))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(Color::Rgba(0.0, 0.0, 0.0, hp_fade))
                                .x_y(x, y - 3.0)
                                .set(player_sct_bg_id, ui_widgets);
                            Text::new(&format!("{}", (floater.hp_change as f32 / 10.0).abs()))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(if floater.hp_change < 0 {
                                    Color::Rgba(1.0, 0.1, 0.0, hp_fade)
                                } else {
                                    Color::Rgba(0.1, 1.0, 0.1, hp_fade)
                                })
                                .x_y(x, y)
                                .set(player_sct_id, ui_widgets);
                        }
                    }
                }
                // EXP Numbers
                self.floaters
                    .exp_floaters
                    .iter_mut()
                    .for_each(|f| f.timer -= dt.as_secs_f32());
                self.floaters.exp_floaters.retain(|f| f.timer > 0_f32);
                if let Some(uid) = uids.get(me) {
                    for floater in self
                        .floaters
                        .exp_floaters
                        .iter_mut()
                        .filter(|f| f.owner == *uid)
                    {
                        let number_speed = 50.0; // Number Speed for Single EXP
                        let player_sct_bg_id = player_sct_bg_id_walker.next(
                            &mut self.ids.player_sct_bgs,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        let player_sct_id = player_sct_id_walker.next(
                            &mut self.ids.player_scts,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size_xp =
                            30 + ((floater.exp_change as f32 / 300.0).min(1.0) * 50.0) as u32;
                        let y = floater.timer as f64 * number_speed; // Timer sets the widget offset
                        //let fade = ((4.0 - floater.timer as f32) * 0.25) + 0.2; // Timer sets
                        // text transparency
                        let fade = if floater.timer < 1.0 {
                            floater.timer as f32
                        } else {
                            1.0
                        };

                        if floater.exp_change > 0 {
                            // Don't show 0 Exp
                            Text::new(&format!("{} Exp", floater.exp_change.max(1)))
                                .font_size(font_size_xp)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                                .x_y(
                                    ui_widgets.win_w * (0.5 * floater.rand_offset.0 as f64 - 0.25),
                                    ui_widgets.win_h * (0.15 * floater.rand_offset.1 as f64) + y
                                        - 3.0,
                                )
                                .set(player_sct_bg_id, ui_widgets);
                            Text::new(&format!("{} Exp", floater.exp_change.max(1)))
                                .font_size(font_size_xp)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(Color::Rgba(0.59, 0.41, 0.67, fade))
                                .x_y(
                                    ui_widgets.win_w * (0.5 * floater.rand_offset.0 as f64 - 0.25),
                                    ui_widgets.win_h * (0.15 * floater.rand_offset.1 as f64) + y,
                                )
                                .set(player_sct_id, ui_widgets);
                        }
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
                if let Some(uid) = uids.get(me) {
                    if let Some(display) = self
                        .floaters
                        .skill_point_displays
                        .iter_mut()
                        .find(|d| d.owner == *uid)
                    {
                        let fade = if display.timer < 3.0 {
                            display.timer as f32 * 0.33
                        } else if display.timer < 2.0 {
                            display.timer as f32 * 0.33 * 0.1
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
                        Text::new(&format!("{}", rank))
                            .font_size(20)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(1.0, 1.0, 1.0, fade))
                            .mid_top_with_margin_on(self.ids.player_rank_up, 8.0)
                            .set(self.ids.player_rank_up_txt_number, ui_widgets);
                        // Static "New Rank!" text
                        Text::new(&i18n.get("hud.rank_up"))
                            .font_size(40)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                            .mid_bottom_with_margin_on(self.ids.player_rank_up, 20.0)
                            .set(self.ids.player_rank_up_txt_0_bg, ui_widgets);
                        Text::new(&i18n.get("hud.rank_up"))
                            .font_size(40)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(1.0, 1.0, 1.0, fade))
                            .bottom_left_with_margins_on(self.ids.player_rank_up_txt_0_bg, 2.0, 2.0)
                            .set(self.ids.player_rank_up_txt_0, ui_widgets);
                        // Variable skilltree text
                        let skill = match display.skill_tree {
                            General => &i18n.get("common.weapons.general"),
                            Weapon(ToolKind::Hammer) => &i18n.get("common.weapons.hammer"),
                            Weapon(ToolKind::Axe) => &i18n.get("common.weapons.axe"),
                            Weapon(ToolKind::Sword) => &i18n.get("common.weapons.sword"),
                            Weapon(ToolKind::Sceptre) => &i18n.get("common.weapons.sceptre"),
                            Weapon(ToolKind::Bow) => &i18n.get("common.weapons.bow"),
                            Weapon(ToolKind::Staff) => &i18n.get("common.weapons.staff"),
                            _ => "Unknown",
                        };
                        Text::new(skill)
                            .font_size(20)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                            .mid_top_with_margin_on(self.ids.player_rank_up, 45.0)
                            .set(self.ids.player_rank_up_txt_1_bg, ui_widgets);
                        Text::new(skill)
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
                            _ => self.imgs.swords_crossed,
                        })
                        .w_h(20.0, 20.0)
                        .left_from(self.ids.player_rank_up_txt_1_bg, 5.0)
                        .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade)))
                        .set(self.ids.player_rank_up_icon, ui_widgets);
                    }
                }
                // Scrolling Combat Text for Parrying an attack
                self.floaters
                    .block_floaters
                    .iter_mut()
                    .for_each(|f| f.timer -= dt.as_secs_f32());
                self.floaters.block_floaters.retain(|f| f.timer > 0_f32);
                if let Some(uid) = uids.get(me) {
                    for floater in self
                        .floaters
                        .block_floaters
                        .iter_mut()
                        .filter(|f| f.owner == *uid)
                    {
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
                            floater.timer as f32 / 0.25
                        } else {
                            1.0
                        };

                        Text::new(&i18n.get("hud.sct.block"))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                            .x_y(
                                ui_widgets.win_w * (0.0),
                                ui_widgets.win_h * (-0.3) + y - 3.0,
                            )
                            .set(player_sct_bg_id, ui_widgets);
                        Text::new(&i18n.get("hud.sct.block"))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.69, 0.82, 0.88, fade))
                            .x_y(ui_widgets.win_w * 0.0, ui_widgets.win_h * -0.3 + y)
                            .set(player_sct_id, ui_widgets);
                    }
                }
            }

            // Pop speech bubbles
            let now = Instant::now();
            self.speech_bubbles
                .retain(|_uid, bubble| bubble.timeout > now);

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

            let make_overitem = |item: &Item, pos, distance, active, fonts| {
                let text = if item.amount() > 1 {
                    format!("{} x {}", item.amount(), item.name())
                } else {
                    item.name().to_string()
                };

                let quality = get_quality_col(item);

                // Item
                overitem::Overitem::new(
                    text.into(),
                    quality,
                    distance,
                    fonts,
                    &global_state.settings.controls,
                    // If we're currently set to interact with the item...
                    active,
                    &global_state.window.key_layout,
                )
                .x_y(0.0, 100.0)
                .position_ingame(pos)
            };

            // Render overitem: name, etc.
            for (entity, pos, item, distance) in (&entities, &pos, &items)
                .join()
                .map(|(entity, pos, item)| (entity, pos, item, pos.0.distance_squared(player_pos)))
                .filter(|(_, _, _, distance)| distance < &common::consts::MAX_PICKUP_RANGE.powi(2))
            {
                let overitem_id = overitem_walker.next(
                    &mut self.ids.overitems,
                    &mut ui_widgets.widget_id_generator(),
                );

                make_overitem(
                    item,
                    pos.0 + Vec3::unit_z() * 1.2,
                    distance,
                    interactable.as_ref().and_then(|i| i.entity()) == Some(entity),
                    &self.fonts,
                )
                .set(overitem_id, ui_widgets);
            }

            // Render overtime for an interactable block
            if let Some(Interactable::Block(block, pos, _)) = interactable {
                let overitem_id = overitem_walker.next(
                    &mut self.ids.overitems,
                    &mut ui_widgets.widget_id_generator(),
                );

                let pos = pos.map(|e| e as f32 + 0.5);
                let over_pos = pos + Vec3::unit_z() * 0.7;

                // This is only done once per frame, so it's not a performance issue
                if block.get_sprite().map_or(false, |s| s.is_container()) {
                    overitem::Overitem::new(
                        "???".into(),
                        overitem::TEXT_COLOR,
                        pos.distance_squared(player_pos),
                        &self.fonts,
                        &global_state.settings.controls,
                        true,
                        &global_state.window.key_layout,
                    )
                    .x_y(0.0, 100.0)
                    .position_ingame(over_pos)
                    .set(overitem_id, ui_widgets);
                } else if let Some(item) = Item::try_reclaim_from_block(block) {
                    make_overitem(
                        &item,
                        over_pos,
                        pos.distance_squared(player_pos),
                        true,
                        &self.fonts,
                    )
                    .set(overitem_id, ui_widgets);
                } else if let Some(sprite) = block.get_sprite() {
                    overitem::Overitem::new(
                        format!("{:?}", sprite).into(), /* TODO: A better way to generate text
                                                         * for this */
                        overitem::TEXT_COLOR,
                        pos.distance_squared(player_pos),
                        &self.fonts,
                        &global_state.settings.controls,
                        true,
                        &global_state.window.key_layout,
                    )
                    .x_y(0.0, 100.0)
                    .position_ingame(over_pos)
                    .set(overitem_id, ui_widgets);
                }
            }

            let speech_bubbles = &self.speech_bubbles;

            // Render overhead name tags and health bars
            for (pos, info, bubble, _, _, health, _, height_offset, hpfl, in_group) in (
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
                &hp_floater_lists,
                &uids,
                &inventories,
            )
                .join()
                .filter(|t| {
                    let health = t.5;
                    let entity = t.0;
                    entity != me && !health.map_or(false, |h| h.is_dead)
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
                    )| {
                        // Use interpolated position if available
                        let pos = interpolated.map_or(pos.0, |i| i.pos);
                        let in_group = client.group_members().contains_key(uid);
                        let dist_sqr = pos.distance_squared(player_pos);
                        // Determine whether to display nametag and healthbar based on whether the
                        // entity has been damaged, is targeted/selected, or is in your group
                        // Note: even if this passes the healthbar can be hidden in some cases if it
                        // is at maximum
                        let display_overhead_info =
                            (info.target_entity.map_or(false, |e| e == entity)
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
                            name: &stats.name,
                            health,
                            buffs,
                            energy,
                            combat_rating: health.map_or(0.0, |health| {
                                combat::combat_rating(inventory, health, skill_set, *body, &msm)
                            }),
                        });
                        let bubble = if dist_sqr < SPEECH_BUBBLE_RANGE.powi(2) {
                            speech_bubbles.get(uid)
                        } else {
                            None
                        };

                        (info.is_some() || bubble.is_some()).then(|| {
                            (
                                pos,
                                info,
                                bubble,
                                stats,
                                skill_set,
                                health,
                                buffs,
                                body.height() * scale.map_or(1.0, |s| s.0) + 0.5,
                                hpfl,
                                in_group,
                            )
                        })
                    },
                )
            {
                let overhead_id = overhead_walker.next(
                    &mut self.ids.overheads,
                    &mut ui_widgets.widget_id_generator(),
                );
                let ingame_pos = pos + Vec3::unit_z() * height_offset;

                //
                // * height_offset

                // Speech bubble, name, level, and hp bars
                overhead::Overhead::new(
                    info,
                    bubble,
                    in_group,
                    &global_state.settings.interface,
                    self.pulse,
                    i18n,
                    &self.imgs,
                    &self.fonts,
                )
                .x_y(0.0, 100.0)
                .position_ingame(ingame_pos)
                .set(overhead_id, ui_widgets);

                // Enemy SCT
                if global_state.settings.interface.sct && !hpfl.floaters.is_empty() {
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
                    let font_col = |font_size: u32| {
                        DAMAGE_COLORS[(font_size.saturating_sub(36) / 5).min(5) as usize]
                    };

                    if global_state.settings.interface.sct_damage_batch {
                        let number_speed = 50.0; // Damage number speed
                        let sct_id = sct_walker
                            .next(&mut self.ids.scts, &mut ui_widgets.widget_id_generator());
                        let sct_bg_id = sct_bg_walker
                            .next(&mut self.ids.sct_bgs, &mut ui_widgets.widget_id_generator());
                        // Calculate total change
                        // Ignores healing
                        let hp_damage = floaters.iter().fold(0, |acc, f| {
                            if f.hp_change < 0 {
                                acc + f.hp_change
                            } else {
                                acc
                            }
                        });
                        // Divide by 10 to stay in the same dimension as the HP display
                        let hp_dmg_rounded_abs = ((hp_damage + 5) / 10).abs();
                        let max_hp_frac =
                            hp_damage.abs() as f32 / health.map_or(1.0, |h| h.maximum() as f32);
                        let timer = floaters
                            .last()
                            .expect("There must be at least one floater")
                            .timer;
                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size = 30
                            + ((max_hp_frac * 10.0) as u32) * 3
                            + if timer < 0.1 {
                                FLASH_MAX * (((1.0 - timer / 0.1) * 10.0) as u32)
                            } else {
                                0
                            };
                        let font_col = font_col(font_size);
                        // Timer sets the widget offset
                        let y = (timer as f64 / crate::ecs::sys::floater::HP_SHOWTIME as f64
                            * number_speed)
                            + 100.0;
                        // Timer sets text transparency
                        let fade = ((crate::ecs::sys::floater::HP_SHOWTIME - timer) * 0.25) + 0.2;
                        if hp_damage.abs() < 10 {
                            // Damage and heal below 10/10 are shown as decimals
                            Text::new(&format!("{}", hp_damage.abs() as f32 / 10.0))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                                .x_y(0.0, y - 3.0)
                                .position_ingame(ingame_pos)
                                .set(sct_bg_id, ui_widgets);
                            Text::new(&format!("{}", hp_damage.abs() as f32 / 10.0))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .x_y(0.0, y)
                                .color(if hp_damage < 0 {
                                    Color::Rgba(font_col.r, font_col.g, font_col.b, fade)
                                } else {
                                    Color::Rgba(0.1, 1.0, 0.1, fade)
                                })
                                .position_ingame(ingame_pos)
                                .set(sct_id, ui_widgets);
                        } else {
                            // Damage and heal above 10/10 are shown rounded
                            Text::new(&format!("{}", hp_dmg_rounded_abs))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                                .x_y(0.0, y - 3.0)
                                .position_ingame(ingame_pos)
                                .set(sct_bg_id, ui_widgets);

                            Text::new(&format!("{}", hp_dmg_rounded_abs))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .x_y(0.0, y)
                                .color(if hp_damage < 0 {
                                    Color::Rgba(font_col.r, font_col.g, font_col.b, fade)
                                } else {
                                    Color::Rgba(0.1, 1.0, 0.1, fade)
                                })
                                .position_ingame(ingame_pos)
                                .set(sct_id, ui_widgets);
                        };
                    } else {
                        for floater in floaters {
                            let number_speed = 250.0; // Single Numbers Speed
                            let sct_id = sct_walker
                                .next(&mut self.ids.scts, &mut ui_widgets.widget_id_generator());
                            let sct_bg_id = sct_bg_walker
                                .next(&mut self.ids.sct_bgs, &mut ui_widgets.widget_id_generator());
                            // Calculate total change
                            let max_hp_frac = floater.hp_change.abs() as f32
                                / health.map_or(1.0, |h| h.maximum() as f32);
                            // Increase font size based on fraction of maximum health
                            // "flashes" by having a larger size in the first 100ms
                            let font_size = 30
                                + ((max_hp_frac * 10.0) as u32) * 3
                                + if floater.timer < 0.1 {
                                    FLASH_MAX * (((1.0 - floater.timer / 0.1) * 10.0) as u32)
                                } else {
                                    0
                                };
                            let font_col = font_col(font_size);
                            // Timer sets the widget offset
                            let y = (floater.timer as f64
                                / crate::ecs::sys::floater::HP_SHOWTIME as f64
                                * number_speed)
                                + 100.0;
                            // Timer sets text transparency
                            let fade = ((crate::ecs::sys::floater::HP_SHOWTIME - floater.timer)
                                * 0.25)
                                + 0.2;
                            if floater.hp_change.abs() < 10 {
                                // Damage and heal below 10/10 are shown as decimals
                                Text::new(&format!("{}", (floater.hp_change.abs() as f32 / 10.0)))
                                    .font_size(font_size)
                                    .font_id(self.fonts.cyri.conrod_id)
                                    .color(if floater.hp_change < 0 {
                                        Color::Rgba(0.0, 0.0, 0.0, fade)
                                    } else {
                                        Color::Rgba(0.0, 0.0, 0.0, 1.0)
                                    })
                                    .x_y(0.0, y - 3.0)
                                    .position_ingame(ingame_pos)
                                    .set(sct_bg_id, ui_widgets);
                                Text::new(&format!("{}", (floater.hp_change.abs() as f32 / 10.0)))
                                    .font_size(font_size)
                                    .font_id(self.fonts.cyri.conrod_id)
                                    .x_y(0.0, y)
                                    .color(if floater.hp_change < 0 {
                                        Color::Rgba(font_col.r, font_col.g, font_col.b, fade)
                                    } else {
                                        Color::Rgba(0.1, 1.0, 0.1, 1.0)
                                    })
                                    .position_ingame(ingame_pos)
                                    .set(sct_id, ui_widgets);
                            } else {
                                // Damage and heal above 10/10 are shown rounded
                                Text::new(&format!("{}", (floater.hp_change / 10).abs()))
                                    .font_size(font_size)
                                    .font_id(self.fonts.cyri.conrod_id)
                                    .color(if floater.hp_change < 0 {
                                        Color::Rgba(0.0, 0.0, 0.0, fade)
                                    } else {
                                        Color::Rgba(0.0, 0.0, 0.0, 1.0)
                                    })
                                    .x_y(0.0, y - 3.0)
                                    .position_ingame(ingame_pos)
                                    .set(sct_bg_id, ui_widgets);
                                Text::new(&format!("{}", (floater.hp_change / 10).abs()))
                                    .font_size(font_size)
                                    .font_id(self.fonts.cyri.conrod_id)
                                    .x_y(0.0, y)
                                    .color(if floater.hp_change < 0 {
                                        Color::Rgba(font_col.r, font_col.g, font_col.b, fade)
                                    } else {
                                        Color::Rgba(0.1, 1.0, 0.1, 1.0)
                                    })
                                    .position_ingame(ingame_pos)
                                    .set(sct_id, ui_widgets);
                            }
                        }
                    }
                }
            }
        }

        // Temporary Example Quest
        let arrow_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8; //Animation timer
        if let Some(toggle_cursor_key) = global_state
            .settings
            .controls
            .get_binding(GameInput::ToggleCursor)
        {
            if !self.show.intro {
                match global_state.settings.interface.intro_show {
                    Intro::Show => {
                        if Button::image(self.imgs.button)
                            .w_h(150.0, 40.0)
                            .hover_image(self.imgs.button_hover)
                            .press_image(self.imgs.button_press)
                            .bottom_left_with_margins_on(ui_widgets.window, 200.0, 120.0)
                            .label(&i18n.get("hud.tutorial_btn"))
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
                        Image::new(self.imgs.sp_indicator_arrow)
                            .w_h(20.0, 11.0)
                            .mid_top_with_margin_on(self.ids.intro_button, -20.0 + arrow_ani as f64)
                            .color(Some(QUALITY_LEGENDARY))
                            .set(self.ids.tut_arrow, ui_widgets);
                        Text::new(&i18n.get("hud.tutorial_click_here").replace(
                            "{key}",
                            toggle_cursor_key.display_string(key_layout).as_str(),
                        ))
                        .mid_top_with_margin_on(self.ids.tut_arrow, -18.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(BLACK)
                        .set(self.ids.tut_arrow_txt_bg, ui_widgets);
                        Text::new(&i18n.get("hud.tutorial_click_here").replace(
                            "{key}",
                            toggle_cursor_key.display_string(key_layout).as_str(),
                        ))
                        .bottom_right_with_margins_on(self.ids.tut_arrow_txt_bg, 1.0, 1.0)
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
        }
        // TODO: Add event/stat based tutorial system
        if self.show.intro && !self.show.esc_menu {
            match global_state.settings.interface.intro_show {
                Intro::Show => {
                    if self.show.intro {
                        self.show.want_grab = false;
                        let quest_headline = &i18n.get("hud.temp_quest_headline");
                        let quest_text = &i18n.get("hud.temp_quest_text");
                        Image::new(self.imgs.quest_bg)
                            .w_h(404.0, 858.0)
                            .middle_of(ui_widgets.window)
                            .set(self.ids.quest_bg, ui_widgets);

                        Text::new(quest_headline)
                            .mid_top_with_margin_on(self.ids.quest_bg, 310.0)
                            .font_size(self.fonts.cyri.scale(30))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_BG)
                            .set(self.ids.q_headline_bg, ui_widgets);
                        Text::new(quest_headline)
                            .bottom_left_with_margins_on(self.ids.q_headline_bg, 1.0, 1.0)
                            .font_size(self.fonts.cyri.scale(30))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_COLOR)
                            .set(self.ids.q_headline, ui_widgets);

                        Text::new(quest_text)
                            .mid_top_with_margin_on(self.ids.quest_bg, 360.0)
                            .w(350.0)
                            .font_size(self.fonts.cyri.scale(17))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_BG)
                            .set(self.ids.q_text_bg, ui_widgets);
                        Text::new(quest_text)
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
                            .label(&i18n.get("common.close"))
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
                            Text::new(&i18n.get("hud.tutorial_elements"))
                                .mid_top_with_margin_on(self.ids.tut_arrow, -50.0)
                                .font_id(self.fonts.cyri.conrod_id)
                                .font_size(self.fonts.cyri.scale(40))
                                .color(BLACK)
                                .floating(true)
                                .set(self.ids.tut_arrow_txt_bg, ui_widgets);
                            Text::new(&i18n.get("hud.tutorial_elements"))
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

        // Display debug window.
        if let Some(debug_info) = debug_info {
            self._velocity = match debug_info.velocity {
                Some(velocity) => velocity.0.magnitude(),
                None => 0.0,
            };
            // Alpha Version
            Text::new(&version)
                .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
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
            .down_from(self.ids.version, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.fps_counter, ui_widgets);
            // Ping
            Text::new(&format!("Ping: {:.0}ms", debug_info.ping_ms))
                .color(TEXT_COLOR)
                .down_from(self.ids.fps_counter, 5.0)
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
                .down_from(self.ids.ping, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.coordinates, ui_widgets);
            // Player's velocity
            let velocity_text = match debug_info.velocity {
                Some(velocity) => format!(
                    "Velocity: ({:.1}, {:.1}, {:.1}) [{:.1} u/s]",
                    velocity.0.x,
                    velocity.0.y,
                    velocity.0.z,
                    velocity.0.magnitude()
                ),
                None => "Player has no Vel component".to_owned(),
            };
            Text::new(&velocity_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.coordinates, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.velocity, ui_widgets);
            // Player's orientation vector
            let orientation_text = match debug_info.ori {
                Some(ori) => {
                    let look_dir = ori.look_dir();
                    format!(
                        "Orientation: ({:.1}, {:.1}, {:.1})",
                        look_dir.x, look_dir.y, look_dir.z,
                    )
                },
                None => "Player has no Ori component".to_owned(),
            };
            Text::new(&orientation_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.velocity, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.orientation, ui_widgets);
            // Loaded distance
            Text::new(&format!(
                "View distance: {:.2} blocks ({:.2} chunks)",
                client.loaded_distance(),
                client.loaded_distance() / TerrainChunk::RECT_SIZE.x as f32,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.orientation, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.loaded_distance, ui_widgets);
            // Time
            let time_in_seconds = client.state().get_time_of_day();
            let current_time = NaiveTime::from_num_seconds_from_midnight(
                // Wraps around back to 0s if it exceeds 24 hours (24 hours = 86400s)
                (time_in_seconds as u64 % 86400) as u32,
                0,
            );
            Text::new(&format!(
                "Time: {}",
                current_time.format("%H:%M").to_string()
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.loaded_distance, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.time, ui_widgets);

            // Number of entities
            let entity_count = client.state().ecs().entities().join().count();
            Text::new(&format!("Entity count: {}", entity_count))
                .color(TEXT_COLOR)
                .down_from(self.ids.time, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.entity_count, ui_widgets);

            // Number of chunks
            Text::new(&format!(
                "Chunks: {} ({} visible) & {} (shadow)",
                debug_info.num_chunks, debug_info.num_visible_chunks, debug_info.num_shadow_chunks,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.entity_count, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.num_chunks, ui_widgets);

            // Number of lights
            Text::new(&format!("Lights: {}", debug_info.num_lights,))
                .color(TEXT_COLOR)
                .down_from(self.ids.num_chunks, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.num_lights, ui_widgets);

            // Number of figures
            Text::new(&format!(
                "Figures: {} ({} visible)",
                debug_info.num_figures, debug_info.num_figures_visible,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.num_lights, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.num_figures, ui_widgets);

            // Number of particles
            Text::new(&format!(
                "Particles: {} ({} visible)",
                debug_info.num_particles, debug_info.num_particles_visible,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.num_figures, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.num_particles, ui_widgets);

            // Help Window
            if let Some(help_key) = global_state.settings.controls.get_binding(GameInput::Help) {
                Text::new(
                    &i18n
                        .get("hud.press_key_to_toggle_keybindings_fmt")
                        .replace("{key}", help_key.display_string(key_layout).as_str()),
                )
                .color(TEXT_COLOR)
                .down_from(self.ids.num_particles, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.help_info, ui_widgets);
            }
            // Info about Debug Shortcut
            if let Some(toggle_debug_key) = global_state
                .settings
                .controls
                .get_binding(GameInput::ToggleDebug)
            {
                Text::new(&i18n.get("hud.press_key_to_toggle_debug_info_fmt").replace(
                    "{key}",
                    toggle_debug_key.display_string(key_layout).as_str(),
                ))
                .color(TEXT_COLOR)
                .down_from(self.ids.help_info, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.debug_info, ui_widgets);
            }
        } else {
            // Help Window
            if let Some(help_key) = global_state.settings.controls.get_binding(GameInput::Help) {
                Text::new(
                    &i18n
                        .get("hud.press_key_to_show_keybindings_fmt")
                        .replace("{key}", help_key.display_string(key_layout).as_str()),
                )
                .color(TEXT_COLOR)
                .bottom_left_with_margins_on(ui_widgets.window, 210.0, 10.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(12))
                .set(self.ids.help_info, ui_widgets);
            }
            // Info about Debug Shortcut
            if let Some(toggle_debug_key) = global_state
                .settings
                .controls
                .get_binding(GameInput::ToggleDebug)
            {
                Text::new(&i18n.get("hud.press_key_to_show_debug_info_fmt").replace(
                    "{key}",
                    toggle_debug_key.display_string(key_layout).as_str(),
                ))
                .color(TEXT_COLOR)
                .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(12))
                .set(self.ids.debug_info, ui_widgets);
            }
            // Lantern Key
            if let Some(toggle_lantern_key) = global_state
                .settings
                .controls
                .get_binding(GameInput::ToggleLantern)
            {
                Text::new(&i18n.get("hud.press_key_to_toggle_lantern_fmt").replace(
                    "{key}",
                    toggle_lantern_key.display_string(key_layout).as_str(),
                ))
                .color(TEXT_COLOR)
                .up_from(self.ids.help_info, 2.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(12))
                .set(self.ids.lantern_info, ui_widgets);
            }
        }

        // Help Text
        // TODO Add dynamic controls display
        /*if self.show.help && !self.show.map && !self.show.esc_menu {
            Image::new(self.imgs.help)
                .middle_of(ui_widgets.window)
                .w_h(1260.0, 519.0)
                .set(self.ids.help, ui_widgets);
            // X-button
            if Button::image(self.imgs.close_button)
                .w_h(40.0, 40.0)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.help, 0.0, 0.0)
                .color(Color::Rgba(1.0, 1.0, 1.0, 0.8))
                .set(self.ids.button_help2, ui_widgets)
                .was_clicked()
            {
                self.show.help = false;
            };
        }*/

        // Bag button and nearby icons
        let ecs = client.state().ecs();
        let entity = client.entity();
        let stats = ecs.read_storage::<comp::Stats>();
        let skill_sets = ecs.read_storage::<comp::SkillSet>();
        let buffs = ecs.read_storage::<comp::Buffs>();
        let msm = ecs.read_resource::<MaterialStatManifest>();
        if let (Some(player_stats), Some(skill_set)) = (stats.get(entity), skill_sets.get(entity)) {
            match Buttons::new(
                client,
                self.show.bag,
                &self.imgs,
                &self.fonts,
                global_state,
                &self.rot_imgs,
                tooltip_manager,
                i18n,
                &player_stats,
                &skill_set,
                self.pulse,
            )
            .set(self.ids.buttons, ui_widgets)
            {
                Some(buttons::Event::ToggleBag) => self.show.toggle_bag(),
                Some(buttons::Event::ToggleSettings) => self.show.toggle_settings(global_state),
                Some(buttons::Event::ToggleSocial) => self.show.toggle_social(),
                Some(buttons::Event::ToggleSpell) => self.show.toggle_spell(),
                Some(buttons::Event::ToggleMap) => self.show.toggle_map(),
                Some(buttons::Event::ToggleCrafting) => self.show.toggle_crafting(),
                None => {},
            }
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
            &global_state,
            tooltip_manager,
            &msm,
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
            &global_state,
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
                &prompt_dialog_settings,
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
        let entity = client.entity();
        let healths = ecs.read_storage::<comp::Health>();
        let inventories = ecs.read_storage::<comp::Inventory>();
        let energies = ecs.read_storage::<comp::Energy>();
        let skillsets = ecs.read_storage::<comp::SkillSet>();
        let character_states = ecs.read_storage::<comp::CharacterState>();
        let controllers = ecs.read_storage::<comp::Controller>();
        let ability_map = ecs.fetch::<comp::item::tool::AbilityMap>();
        let bodies = ecs.read_storage::<comp::Body>();
        // Combo floater stuffs
        self.floaters
            .combo_floaters
            .iter_mut()
            .for_each(|f| f.timer -= dt.as_secs_f64());
        self.floaters.combo_floaters.retain(|f| f.timer > 0_f64);
        let combo = if let Some(uid) = ecs.read_storage::<Uid>().get(entity) {
            self.floaters
                .combo_floaters
                .iter()
                .find(|c| c.owner == *uid)
                .copied()
        } else {
            None
        };

        if let (
            Some(health),
            Some(inventory),
            Some(energy),
            Some(skillset),
            Some(_character_state),
            Some(_controller),
        ) = (
            healths.get(entity),
            inventories.get(entity),
            energies.get(entity),
            skillsets.get(entity),
            character_states.get(entity),
            controllers.get(entity).map(|c| &c.inputs),
        ) {
            Skillbar::new(
                client,
                global_state,
                &self.imgs,
                &self.item_imgs,
                &self.fonts,
                &self.rot_imgs,
                &health,
                &inventory,
                &energy,
                &skillset,
                //&character_state,
                self.pulse,
                //&controller,
                &self.hotbar,
                tooltip_manager,
                item_tooltip_manager,
                &mut self.slot_manager,
                i18n,
                &ability_map,
                &msm,
                combo,
            )
            .set(self.ids.skillbar, ui_widgets);
        }
        // Bag contents
        if self.show.bag {
            if let (Some(player_stats), Some(skill_set), Some(health), Some(energy), Some(body)) = (
                stats.get(client.entity()),
                skill_sets.get(client.entity()),
                healths.get(entity),
                energies.get(entity),
                bodies.get(entity),
            ) {
                match Bag::new(
                    client,
                    &self.imgs,
                    &self.item_imgs,
                    &self.fonts,
                    &self.rot_imgs,
                    tooltip_manager,
                    item_tooltip_manager,
                    &mut self.slot_manager,
                    self.pulse,
                    i18n,
                    &player_stats,
                    &skill_set,
                    &health,
                    &energy,
                    &self.show,
                    &body,
                    &msm,
                )
                .set(self.ids.bag, ui_widgets)
                {
                    Some(bag::Event::BagExpand) => self.show.bag_inv = !self.show.bag_inv,
                    Some(bag::Event::Close) => {
                        self.show.stats = false;
                        self.show.bag(false);
                        if !self.show.social {
                            self.show.want_grab = true;
                            self.force_ungrab = false;
                        } else {
                            self.force_ungrab = true
                        };
                    },
                    Some(bag::Event::SortInventory) => self.events.push(Event::SortInventory),
                    None => {},
                }
            }
        }
        // Trade window
        if self.show.trade {
            match Trade::new(
                client,
                &self.imgs,
                &self.item_imgs,
                &self.fonts,
                &self.rot_imgs,
                item_tooltip_manager,
                &mut self.slot_manager,
                i18n,
                &msm,
                self.pulse,
            )
            .set(self.ids.trade, ui_widgets)
            {
                Some(action) => {
                    if let TradeAction::Decline = action {
                        self.show.stats = false;
                        self.show.trade(false);
                        if !self.show.social {
                            self.show.want_grab = true;
                            self.force_ungrab = false;
                        } else {
                            self.force_ungrab = true
                        };
                    }
                    events.push(Event::TradeAction(action));
                },
                None => {},
            }
        }

        // Buffs
        let ecs = client.state().ecs();
        let entity = client.entity();
        let health = ecs.read_storage::<comp::Health>();
        let energy = ecs.read_storage::<comp::Energy>();
        if let (Some(player_buffs), Some(health), Some(energy)) = (
            buffs.get(client.entity()),
            health.get(entity),
            energy.get(entity),
        ) {
            for event in BuffsBar::new(
                &self.imgs,
                &self.fonts,
                &self.rot_imgs,
                tooltip_manager,
                i18n,
                &player_buffs,
                self.pulse,
                &global_state,
                &health,
                &energy,
            )
            .set(self.ids.buffs, ui_widgets)
            {
                match event {
                    buffs::Event::RemoveBuff(buff_id) => events.push(Event::RemoveBuff(buff_id)),
                }
            }
        }
        // Crafting
        if self.show.crafting {
            if let Some(inventory) = inventories.get(entity) {
                for event in Crafting::new(
                    //&self.show,
                    client,
                    &self.imgs,
                    &self.fonts,
                    &*i18n,
                    self.pulse,
                    &self.rot_imgs,
                    item_tooltip_manager,
                    &self.item_imgs,
                    &inventory,
                    &msm,
                    tooltip_manager,
                    &mut self.show,
                )
                .set(self.ids.crafting_window, ui_widgets)
                {
                    match event {
                        crafting::Event::CraftRecipe(recipe) => {
                            events.push(Event::CraftRecipe {
                                recipe,
                                craft_sprite: self.show.craft_sprite,
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
                    }
                }
            }
        }

        // Don't put NPC messages in chat box.
        self.new_messages
            .retain(|m| !matches!(m.chat_type, comp::ChatType::Npc(_, _)));

        // Chat box
        match Chat::new(
            &mut self.new_messages,
            &client,
            global_state,
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
            Some(chat::Event::TabCompletionStart(input)) => {
                self.tab_complete = Some(input);
            },
            Some(chat::Event::SendMessage(message)) => {
                events.push(Event::SendMessage(message));
            },
            Some(chat::Event::Focus(focus_id)) => {
                self.to_focus = Some(Some(focus_id));
            },
            None => {},
        }

        self.new_messages = VecDeque::new();
        self.new_notifications = VecDeque::new();

        // Windows

        // Char Window will always appear at the left side. Other Windows default to the
        // left side, but when the Char Window is opened they will appear to the right
        // of it.

        // Settings
        if let Windows::Settings = self.show.open_windows {
            for event in SettingsWindow::new(
                &global_state,
                &self.show,
                &self.imgs,
                &self.fonts,
                i18n,
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
                    settings_window::Event::SettingsChange(settings_change) => {
                        match &settings_change {
                            SettingsChange::Interface(interface_change) => match interface_change {
                                InterfaceChange::ToggleHelp(toggle_help) => {
                                    self.show.help = *toggle_help;
                                },
                                InterfaceChange::ToggleDebug(toggle_debug) => {
                                    self.show.debug = *toggle_debug;
                                },
                                InterfaceChange::ResetInterfaceSettings => {
                                    self.show.help = false;
                                    self.show.debug = false;
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

        // Social Window
        if self.show.social {
            let ecs = client.state().ecs();
            let _stats = ecs.read_storage::<comp::Stats>();
            let me = client.entity();
            if let Some(_stats) = stats.get(me) {
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
        }

        // Diary
        if self.show.diary {
            let entity = client.entity();
            let skill_sets = ecs.read_storage::<comp::SkillSet>();
            if let Some(skill_set) = skill_sets.get(entity) {
                for event in Diary::new(
                    &self.show,
                    client,
                    &skill_set,
                    &self.imgs,
                    &self.item_imgs,
                    &self.fonts,
                    i18n,
                    &self.rot_imgs,
                    tooltip_manager,
                    self.pulse,
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
                &global_state,
                tooltip_manager,
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
                }
            }
        } else {
            // Reset the map position when it's not showing
            let drag = &global_state.settings.interface.map_drag;
            if drag.x != 0.0 || drag.y != 0.0 {
                events.push(Event::SettingsChange(
                    InterfaceChange::MapDrag(Vec2::zero()).into(),
                ))
            }
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
                let msg = i18n
                    .get("hud.free_look_indicator")
                    .replace("{key}", freelook_key.display_string(key_layout).as_str());
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
            Text::new(i18n.get("hud.auto_walk_indicator"))
                .color(TEXT_BG)
                .mid_top_with_margin_on(ui_widgets.window, indicator_offset)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.auto_walk_bg, ui_widgets);
            indicator_offset += 30.0;
            Text::new(i18n.get("hud.auto_walk_indicator"))
                .color(KILL_COLOR)
                .top_left_with_margins_on(self.ids.auto_walk_bg, -1.0, -1.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.auto_walk_txt, ui_widgets);
        }

        // Camera clamp indicator
        if let Some(cameraclamp_key) = global_state
            .settings
            .controls
            .get_binding(GameInput::CameraClamp)
        {
            if self.show.camera_clamp {
                let msg = i18n
                    .get("hud.camera_clamp_indicator")
                    .replace("{key}", cameraclamp_key.display_string(key_layout).as_str());
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
            use comp::slot::Slot;
            use slots::{InventorySlot, SlotKind::*};
            let to_slot = |slot_kind| match slot_kind {
                Inventory(InventorySlot {
                    slot, ours: true, ..
                }) => Some(Slot::Inventory(slot)),
                Inventory(InventorySlot { ours: false, .. }) => None,
                Equip(e) => Some(Slot::Equip(e)),
                Hotbar(_) => None,
                Trade(_) => None,
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
                        self.hotbar.add_inventory_link(h, slot);
                        events.push(Event::ChangeHotbarState(Box::new(self.hotbar.to_owned())));
                    } else if let (Hotbar(a), Hotbar(b)) = (a, b) {
                        self.hotbar.swap(a, b);
                        events.push(Event::ChangeHotbarState(Box::new(self.hotbar.to_owned())));
                    } else if let (Inventory(i), Trade(t)) = (a, b) {
                        if i.ours == t.ours {
                            if let Some(inventory) = inventories.get(t.entity) {
                                events.push(Event::TradeAction(TradeAction::AddItem {
                                    item: i.slot,
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
                    }
                },
                slot::Event::SplitDropped(from) => {
                    // Drop item
                    if let Some(from) = to_slot(from) {
                        events.push(Event::SplitDropSlot(from));
                    } else if let Hotbar(h) = from {
                        self.hotbar.clear_slot(h);
                        events.push(Event::ChangeHotbarState(Box::new(self.hotbar.to_owned())));
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
                        self.hotbar.add_inventory_link(h, i.slot);
                        events.push(Event::ChangeHotbarState(Box::new(self.hotbar.to_owned())));
                    } else if let (Hotbar(a), Hotbar(b)) = (a, b) {
                        self.hotbar.swap(a, b);
                        events.push(Event::ChangeHotbarState(Box::new(self.hotbar.to_owned())));
                    } else if let (Inventory(i), Trade(t)) = (a, b) {
                        if i.ours == t.ours {
                            if let Some(inventory) = inventories.get(t.entity) {
                                events.push(Event::TradeAction(TradeAction::AddItem {
                                    item: i.slot,
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
                    }
                },
                slot::Event::Used(from) => {
                    // Item used (selected and then clicked again)
                    if let Some(from) = to_slot(from) {
                        events.push(Event::UseSlot {
                            slot: from,
                            bypass_dialog: false,
                        });
                    } else if let Hotbar(h) = from {
                        // Used from hotbar
                        self.hotbar.get(h).map(|s| {
                            match s {
                                hotbar::SlotContents::Inventory(i) => {
                                    events.push(Event::UseSlot {
                                        slot: comp::slot::Slot::Inventory(i),
                                        bypass_dialog: false,
                                    });
                                },
                                hotbar::SlotContents::Ability3 | hotbar::SlotContents::Ability4 => {
                                }, /* Event::Ability3(true),
                                    * sticks */
                            }
                        });
                    }
                },
                slot::Event::Request {
                    slot,
                    auto_quantity,
                } => {
                    if let Some((_, trade, prices)) = client.pending_trade() {
                        let ecs = client.state().ecs();
                        let inventories = ecs.read_component::<common::comp::Inventory>();
                        let get_inventory = |uid: Uid| {
                            if let Some(entity) = ecs.entity_from_uid(uid.0) {
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
                            .uid_from_entity(client.entity())
                            .and_then(|uid| trade.which_party(uid))
                        {
                            Some(who) => who,
                            None => continue 'slot_events,
                        };
                        let do_auto_quantity =
                            |inventory: &common::comp::Inventory,
                             slot,
                             ours,
                             remove,
                             quantity: &mut u32| {
                                if let Some(prices) = prices {
                                    let balance0 =
                                        prices.balance(&trade.offers, &r_inventories, who, true);
                                    let balance1 = prices.balance(
                                        &trade.offers,
                                        &r_inventories,
                                        1 - who,
                                        false,
                                    );
                                    if let Some(item) = inventory.get(slot) {
                                        let (material, factor) =
                                            TradePricing::get_material(item.item_definition_id());
                                        let mut unit_price = prices
                                            .values
                                            .get(&material)
                                            .cloned()
                                            .unwrap_or_default()
                                            * factor;
                                        if ours {
                                            unit_price *= material.trade_margin();
                                        }
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
                            };
                        match slot {
                            Inventory(i) => {
                                if let Some(inventory) = inventories.get(i.entity) {
                                    let mut quantity = 1;
                                    if auto_quantity {
                                        do_auto_quantity(
                                            inventory,
                                            i.slot,
                                            i.ours,
                                            false,
                                            &mut quantity,
                                        );
                                        let inv_quantity = i.amount(inventory).unwrap_or(1);
                                        quantity = quantity.min(inv_quantity);
                                    }

                                    events.push(Event::TradeAction(TradeAction::AddItem {
                                        item: i.slot,
                                        quantity,
                                        ours: i.ours,
                                    }));
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
        self.hotbar.maintain_ability3(client);
        self.hotbar.maintain_ability4(client);

        events
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

    // Checks if a TextEdit widget has the keyboard captured.
    fn typing(&self) -> bool {
        if let Some(id) = self.ui.widget_capturing_keyboard() {
            self.ui
                .widget_graph()
                .widget(id)
                .filter(|c| {
                    c.type_id == std::any::TypeId::of::<<widget::TextEdit as Widget>::State>()
                })
                .is_some()
        } else {
            false
        }
    }

    pub fn handle_event(&mut self, event: WinEvent, global_state: &mut GlobalState) -> bool {
        // Helper
        fn handle_slot(
            slot: hotbar::Slot,
            state: bool,
            events: &mut Vec<Event>,
            slot_manager: &mut slots::SlotManager,
            hotbar: &mut hotbar::State,
        ) {
            use slots::InventorySlot;
            if let Some(slots::SlotKind::Inventory(InventorySlot {
                slot: i,
                ours: true,
                ..
            })) = slot_manager.selected()
            {
                hotbar.add_inventory_link(slot, i);
                events.push(Event::ChangeHotbarState(Box::new(hotbar.to_owned())));
                slot_manager.idle();
            } else {
                let just_pressed = hotbar.process_input(slot, state);
                hotbar.get(slot).map(|s| match s {
                    hotbar::SlotContents::Inventory(i) => {
                        if just_pressed {
                            events.push(Event::UseSlot {
                                slot: comp::slot::Slot::Inventory(i),
                                bypass_dialog: false,
                            });
                        }
                    },
                    hotbar::SlotContents::Ability3 => events.push(Event::Ability3(state)),
                    hotbar::SlotContents::Ability4 => events.push(Event::Ability4(state)),
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
            } else if global_state.settings.interface.minimap_show {
                let new_zoom_lvl = global_state.settings.interface.minimap_zoom * factor;

                global_state.settings.interface.minimap_zoom = new_zoom_lvl;
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
                    self.show.toggle_windows(global_state);
                }
                true
            },

            // Press key while not typing
            WinEvent::InputUpdate(key, state) if !self.typing() => {
                let matching_key = match key {
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
                        self.show.toggle_bag();
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
                        self.show.debug = global_state.settings.interface.toggle_debug;
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
                    // Skillbar
                    input => {
                        if let Some(slot) = try_hotbar_slot_from_input(input) {
                            handle_slot(
                                slot,
                                state,
                                &mut self.events,
                                &mut self.slot_manager,
                                &mut self.hotbar,
                            );
                            true
                        } else {
                            false
                        }
                    },
                };

                // When a player closes all menus, resets the cursor
                // to the center of the screen
                self.show.toggle_cursor_on_menu_close(global_state);
                matching_key
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

    #[allow(clippy::blocks_in_if_conditions)] // TODO: Pending review in #587
    pub fn maintain(
        &mut self,
        client: &Client,
        global_state: &mut GlobalState,
        debug_info: &Option<DebugInfo>,
        camera: &Camera,
        dt: Duration,
        info: HudInfo,
        interactable: Option<Interactable>,
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

        // Stop selecting a sprite to perform crafting with when out of range
        self.show.craft_sprite = self.show.craft_sprite.filter(|(pos, _)| {
            self.show.crafting
                && if let Some(player_pos) = client.position() {
                    pos.map(|e| e as f32 + 0.5).distance(player_pos) < MAX_PICKUP_RANGE
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

        self.ui.maintain(
            &mut global_state.window.renderer_mut(),
            Some(proj_mat * view_mat * Mat4::translation_3d(-focus_off)),
        );

        events
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        span!(_guard, "render", "Hud::render");
        // Don't show anything if the UI is toggled off.
        if self.show.ui {
            self.ui.render(renderer, Some(globals));
        }
    }

    pub fn free_look(&mut self, free_look: bool) { self.show.free_look = free_look; }

    pub fn auto_walk(&mut self, auto_walk: bool) { self.show.auto_walk = auto_walk; }

    pub fn camera_clamp(&mut self, camera_clamp: bool) { self.show.camera_clamp = camera_clamp; }

    pub fn handle_outcome(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::ExpChange { uid, exp } => self.floaters.exp_floaters.push(ExpFloater {
                owner: *uid,
                exp_change: *exp,
                timer: 4.0,
                rand_offset: rand::thread_rng().gen::<(f32, f32)>(),
            }),
            Outcome::SkillPointGain {
                uid,
                skill_tree,
                total_points,
                ..
            } => self.floaters.skill_point_displays.push(SkillPointGain {
                owner: *uid,
                skill_tree: *skill_tree,
                total_points: *total_points,
                timer: 5.0,
            }),
            Outcome::ComboChange { uid, combo } => {
                self.floaters.combo_floaters.push_front(ComboFloater {
                    owner: *uid,
                    combo: *combo,
                    timer: comp::combo::COMBO_DECAY_START,
                })
            },
            Outcome::Block { uid, parry, .. } if *parry => {
                self.floaters.block_floaters.push(BlockFloater {
                    owner: *uid,
                    timer: 1.0,
                })
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
// Get info about applied buffs
fn get_buff_info(buff: &comp::Buff) -> BuffInfo {
    BuffInfo {
        kind: buff.kind,
        data: buff.data,
        is_buff: buff.kind.is_buff(),
        dur: buff.time,
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
        BuffKind::Regeneration { .. } => imgs.buff_plus_0,
        BuffKind::Saturation { .. } => imgs.buff_saturation_0,
        BuffKind::Potion { .. } => imgs.buff_potion_0,
        BuffKind::CampfireHeal { .. } => imgs.buff_campfire_heal_0,
        BuffKind::IncreaseMaxEnergy { .. } => imgs.buff_energyplus_0,
        BuffKind::IncreaseMaxHealth { .. } => imgs.buff_healthplus_0,
        BuffKind::Invulnerability => imgs.buff_invincibility_0,
        BuffKind::ProtectingWard => imgs.buff_dmg_red_0,
        //  Debuffs
        BuffKind::Bleeding { .. } => imgs.debuff_bleed_0,
        BuffKind::Cursed { .. } => imgs.debuff_skull_0,
        BuffKind::Burning { .. } => imgs.debuff_burning_0,
    }
}

pub fn get_buff_title(buff: BuffKind, localized_strings: &Localization) -> &str {
    match buff {
        // Buffs
        BuffKind::Regeneration { .. } => localized_strings.get("buff.title.heal"),
        BuffKind::Saturation { .. } => localized_strings.get("buff.title.saturation"),
        BuffKind::Potion { .. } => localized_strings.get("buff.title.potion"),
        BuffKind::CampfireHeal { .. } => localized_strings.get("buff.title.campfire_heal"),
        BuffKind::IncreaseMaxHealth { .. } => localized_strings.get("buff.title.IncreaseMaxHealth"),
        BuffKind::IncreaseMaxEnergy { .. } => localized_strings.get("buff.title.staminaup"),
        BuffKind::Invulnerability => localized_strings.get("buff.title.invulnerability"),
        BuffKind::ProtectingWard => localized_strings.get("buff.title.protectingward"),
        // Debuffs
        BuffKind::Bleeding { .. } => localized_strings.get("buff.title.bleed"),
        BuffKind::Cursed { .. } => localized_strings.get("buff.title.cursed"),
        BuffKind::Burning { .. } => localized_strings.get("buff.title.burn"),
    }
}

pub fn get_buff_desc(buff: BuffKind, data: BuffData, localized_strings: &Localization) -> Cow<str> {
    match buff {
        // Buffs
        BuffKind::Regeneration { .. } => Cow::Borrowed(localized_strings.get("buff.desc.heal")),
        BuffKind::Saturation { .. } => Cow::Borrowed(localized_strings.get("buff.desc.saturation")),
        BuffKind::Potion { .. } => Cow::Borrowed(localized_strings.get("buff.desc.potion")),
        BuffKind::CampfireHeal { .. } => Cow::Owned(
            localized_strings
                .get("buff.desc.campfire_heal")
                .replace("{rate}", &format!("{:.0}", data.strength * 100.0)),
        ),
        BuffKind::IncreaseMaxHealth { .. } => {
            Cow::Borrowed(localized_strings.get("buff.desc.IncreaseMaxHealth"))
        },
        BuffKind::IncreaseMaxEnergy { .. } => {
            Cow::Borrowed(localized_strings.get("buff.desc.IncreaseMaxEnergy"))
        },
        BuffKind::Invulnerability => {
            Cow::Borrowed(localized_strings.get("buff.desc.invulnerability"))
        },
        BuffKind::ProtectingWard => {
            Cow::Borrowed(localized_strings.get("buff.desc.protectingward"))
        },
        // Debuffs
        BuffKind::Bleeding { .. } => Cow::Borrowed(localized_strings.get("buff.desc.bleed")),
        BuffKind::Cursed { .. } => Cow::Borrowed(localized_strings.get("buff.desc.cursed")),
        BuffKind::Burning { .. } => Cow::Borrowed(localized_strings.get("buff.desc.burn")),
    }
}

pub fn get_buff_time(buff: BuffInfo) -> String {
    if let Some(dur) = buff.dur {
        format!("{:.0}s", dur.as_secs_f32())
    } else {
        "".to_string()
    }
}
