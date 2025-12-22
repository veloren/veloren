use crate::{
    GlobalState, Settings,
    session::interactable::{EntityInteraction, Interactable},
    ui::{TooltipManager, fonts::Fonts},
};
use client::Client;
use common::{
    DamageSource,
    comp::{self, Vel},
    resources::TimeOfDay,
    terrain::SiteKindMeta,
};
use conrod_core::{
    Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon, color,
    widget::{self, Image, Rectangle, RoundedRectangle, Scrollbar, Text},
    widget_ids,
};
use i18n::Localization;
use inline_tweak::*;
use serde::{Deserialize, Serialize};
use specs::WorldExt;
use std::{borrow::Cow, time::Duration};
use vek::*;

use super::{
    GameInput, Outcome, Show, TEXT_COLOR, UserNotification,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hint {
    Move,
    Jump,
    OpenInventory,
    FallDamage,
    OpenGlider,
    Glider,
    StallGlider,
    Roll,
    Attacked,
    Unwield,
    Campfire,
    Waypoint,
    OpenDiary,
    FullInventory,
    RespawnDurability,
    RecipeAvailable,
    EnergyLow,
    Chat,
    Sneak,
    Lantern,
    Zoom,
    FirstPerson,
    Swim,
    OpenMap,
    UseItem,
    Crafting,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Achievement {
    Moved,
    Jumped,
    OpenInventory,
    OpenGlider,
    StallGlider,
    Rolled,
    Wield,
    Unwield,
    FindCampfire,
    SetWaypoint,
    OpenDiary,
    FullInventory,
    Respawned,
    RecipeAvailable,
    OpenCrafting,
    EnergyLow,
    ReceivedChatMsg,
    NearEnemies,
    InDark,
    UsedLantern,
    Swim,
    Zoom,
    OpenMap,
}

impl Hint {
    const FADE_TIME: f32 = 5.0;

    fn get_msg<'a>(&self, settings: &Settings, i18n: &'a Localization) -> Cow<'a, str> {
        let get_key = |key| {
            settings
                .controls
                .get_binding(key)
                .map(|key| key.display_string())
                .unwrap_or_else(|| "<unbound>".to_string())
        };
        let key = &format!("tutorial-{self:?}");
        match self {
            Self::Move => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "w" => get_key(GameInput::MoveForward),
                "a" => get_key(GameInput::MoveLeft),
                "s" => get_key(GameInput::MoveBack),
                "d" => get_key(GameInput::MoveRight),
            }),
            Self::Jump => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Jump),
            }),
            Self::OpenInventory => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Inventory),
            }),
            Self::FallDamage | Self::OpenGlider => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Glide),
            }),
            Self::Roll => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Roll),
            }),
            Self::Attacked => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Primary),
            }),
            Self::Unwield => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::ToggleWield),
            }),
            Self::Campfire => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Sit),
            }),
            Self::OpenDiary => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Diary),
            }),
            Self::RecipeAvailable => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Crafting),
            }),
            Self::Chat => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Chat),
            }),
            Self::Sneak => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Sneak),
            }),
            Self::Lantern => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::ToggleLantern),
            }),
            Self::Zoom => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "in" => get_key(GameInput::ZoomIn),
                "out" => get_key(GameInput::ZoomOut),
            }),
            Self::Swim => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "down" => get_key(GameInput::SwimDown),
                "up" => get_key(GameInput::SwimUp),
            }),
            Self::OpenMap => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Map),
            }),
            _ => i18n.get_msg(key),
        }
    }
}

impl Achievement {
    fn get_msg<'a>(&self, _settings: &Settings, i18n: &'a Localization) -> Cow<'a, str> {
        i18n.get_msg(&format!("achievement-{self:?}"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TutorialState {
    // (_, time_since_active)
    current: Option<(Hint, Option<Achievement>, Duration)>,
    // (_, cancel if achieved, time until display)
    pending: Vec<(Hint, Option<Achievement>, Duration)>,

    time_ingame: Duration,
    goals: Vec<Achievement>,
    done: Vec<Achievement>,
}

impl Default for TutorialState {
    fn default() -> Self {
        let mut this = Self {
            current: None,
            pending: Vec::new(),
            goals: Vec::new(),
            done: Default::default(),
            time_ingame: Duration::ZERO,
        };
        this.add_hinted_goal(Hint::Move, Achievement::Moved, Duration::from_secs(10));
        this.show_hint(Hint::Zoom, Duration::from_mins(3));
        this
    }
}

impl TutorialState {
    fn update(&mut self, dt: Duration) {
        self.time_ingame += dt;
        self.current.take_if(|(_, a, dur)| {
            if a.map_or(false, |a| self.done.contains(&a)) {
                *dur = (*dur).max(Duration::from_secs_f32(Hint::FADE_TIME * 2.0 - 1.0));
            }
            *dur > Duration::from_secs(12)
        });
        for (_, _, dur) in &mut self.pending {
            *dur = dur.saturating_sub(dt);
        }
        self.pending.retain(|(hint, achievement, dur)| {
            if dur.is_zero() && self.current.is_none() {
                self.current = Some((*hint, *achievement, Duration::ZERO));
                false
            } else if let Some(a) = achievement
                && self.done.contains(a)
            {
                false
            } else {
                true
            }
        });
    }

    fn done(&self, achievement: Achievement) -> bool { self.done.contains(&achievement) }

    fn earn_achievement(&mut self, achievement: Achievement) -> bool {
        if !self.done.contains(&achievement) {
            self.done.push(achievement);
            self.goals.retain(|a| a != &achievement);
            self.pending.retain(|(_, a, _)| a != &Some(achievement));
            true
        } else {
            false
        }
    }

    fn add_goal(&mut self, achievement: Achievement) {
        if !self.done.contains(&achievement) && !self.goals.contains(&achievement) {
            self.goals.push(achievement);
        }
    }

    fn add_hinted_goal(&mut self, hint: Hint, achievement: Achievement, timeout: Duration) {
        if self.pending.iter().all(|(h, _, _)| h != &hint) && !self.done(achievement) {
            self.add_goal(achievement);
            self.pending.push((hint, Some(achievement), timeout));
        }
    }

    fn show_hint(&mut self, hint: Hint, timeout: Duration) {
        self.pending.push((hint, None, timeout));
    }

    pub(crate) fn event_tick(&mut self, client: &Client) {
        if let Some(comp::CharacterState::Glide(glide)) = client.current::<comp::CharacterState>()
            && glide.ori.look_dir().z > 0.4
            && let Some(vel) = client.current::<Vel>()
            && vel.0.z < -vel.0.xy().magnitude()
            && self.earn_achievement(Achievement::StallGlider)
        {
            self.show_hint(Hint::StallGlider, Duration::ZERO);
        }

        if let Some(cs) = client.current::<comp::CharacterState>() {
            if cs.is_wield() && self.earn_achievement(Achievement::Wield) {
                self.add_hinted_goal(Hint::Unwield, Achievement::Unwield, Duration::from_mins(2));
            }

            if !cs.is_wield() && self.done(Achievement::Wield) {
                self.earn_achievement(Achievement::Unwield);
            }
        }

        if let Some(ps) = client.current::<comp::PhysicsState>()
            && ps.in_liquid().is_some()
            && self.earn_achievement(Achievement::Swim)
        {
            self.show_hint(Hint::Swim, Duration::from_secs(3));
        }

        if let Some(inv) = client.current::<comp::Inventory>()
            && inv.free_slots() == 0
            && self.earn_achievement(Achievement::FullInventory)
        {
            self.show_hint(Hint::FullInventory, Duration::from_secs(2));
        }

        if let Some(energy) = client.current::<comp::Energy>()
            && energy.fraction() < 0.25
            && self.earn_achievement(Achievement::EnergyLow)
        {
            self.show_hint(Hint::EnergyLow, Duration::ZERO);
        }

        if !self.done(Achievement::RecipeAvailable) && !client.available_recipes().is_empty() {
            self.earn_achievement(Achievement::RecipeAvailable);
            self.add_hinted_goal(
                Hint::RecipeAvailable,
                Achievement::OpenCrafting,
                Duration::from_secs(1),
            );
        }

        if self.time_ingame > Duration::from_mins(10)
            && let Some(chunk) = client.current_chunk()
            && let Some(pos) = client.current::<comp::Pos>()
            && let in_cave = pos.0.z < chunk.meta().alt() - 20.0
            && let near_enemies = matches!(
                chunk.meta().site(),
                Some(SiteKindMeta::Dungeon(_) | SiteKindMeta::Cave)
            )
            && (in_cave || near_enemies)
            && self.earn_achievement(Achievement::NearEnemies)
        {
            self.show_hint(Hint::Sneak, Duration::ZERO);
        }

        if self.time_ingame > Duration::from_mins(3)
            && client
                .state()
                .ecs()
                .read_resource::<TimeOfDay>()
                .day_period()
                .is_dark()
            && self.earn_achievement(Achievement::InDark)
        {
            self.add_hinted_goal(
                Hint::Lantern,
                Achievement::UsedLantern,
                Duration::from_secs(10),
            );
        }
    }

    pub(crate) fn event_move(&mut self) {
        self.earn_achievement(Achievement::Moved);
        self.add_hinted_goal(Hint::Jump, Achievement::Jumped, Duration::from_secs(15));
    }

    pub(crate) fn event_jump(&mut self) {
        self.earn_achievement(Achievement::Jumped);
        self.add_hinted_goal(Hint::Roll, Achievement::Rolled, Duration::from_secs(30));
    }

    pub(crate) fn event_roll(&mut self) {
        self.earn_achievement(Achievement::Rolled);
        self.add_hinted_goal(
            Hint::OpenGlider,
            Achievement::OpenGlider,
            Duration::from_mins(2),
        );
    }

    pub(crate) fn event_collect(&mut self) {
        self.add_hinted_goal(
            Hint::OpenInventory,
            Achievement::OpenInventory,
            Duration::from_secs(1),
        );
    }

    pub(crate) fn event_respawn(&mut self) {
        if self.earn_achievement(Achievement::Respawned) {
            self.show_hint(Hint::RespawnDurability, Duration::from_secs(5));
        }
    }

    pub(crate) fn event_open_inventory(&mut self) {
        if self.earn_achievement(Achievement::OpenInventory) {
            self.show_hint(Hint::UseItem, Duration::from_secs(1));
        }
    }

    pub(crate) fn event_open_diary(&mut self) { self.earn_achievement(Achievement::OpenDiary); }

    pub(crate) fn event_open_crafting(&mut self) {
        if self.earn_achievement(Achievement::OpenCrafting) {
            self.show_hint(Hint::Crafting, Duration::from_secs(1));
        }
    }

    pub(crate) fn event_open_map(&mut self) { self.earn_achievement(Achievement::OpenMap); }

    pub(crate) fn event_map_marker(&mut self) {
        self.add_hinted_goal(Hint::OpenMap, Achievement::OpenMap, Duration::from_secs(1));
    }

    pub(crate) fn event_lantern(&mut self) { self.earn_achievement(Achievement::UsedLantern); }

    pub(crate) fn event_zoom(&mut self, delta: f32) {
        if delta < 0.0
            && self.time_ingame > Duration::from_mins(15)
            && self.earn_achievement(Achievement::Zoom)
        {
            self.show_hint(Hint::FirstPerson, Duration::from_secs(2));
        }
    }

    pub(crate) fn event_outcome(&mut self, client: &Client, outcome: &Outcome) {
        match outcome {
            Outcome::HealthChange { info, .. }
                if Some(info.target) == client.uid()
                    && info.cause == Some(DamageSource::Falling) =>
            {
                self.add_hinted_goal(
                    Hint::FallDamage,
                    Achievement::OpenGlider,
                    Duration::from_secs(1),
                );
            },
            Outcome::HealthChange { info, .. }
                if Some(info.target) == client.uid()
                    && !matches!(info.cause, Some(DamageSource::Falling))
                    && info.amount < 0.0 =>
            {
                self.add_hinted_goal(Hint::Attacked, Achievement::Wield, Duration::ZERO);
            },
            Outcome::SkillPointGain { uid, .. } if Some(*uid) == client.uid() => {
                self.add_hinted_goal(
                    Hint::OpenDiary,
                    Achievement::OpenDiary,
                    Duration::from_secs(3),
                );
            },
            _ => {},
        }
    }

    pub(crate) fn event_open_glider(&mut self) {
        if self.earn_achievement(Achievement::OpenGlider) {
            self.show_hint(Hint::Glider, Duration::from_secs(1));
        }
    }

    pub(crate) fn event_find_interactable(&mut self, inter: &Interactable) {
        #[allow(clippy::single_match)]
        match inter {
            Interactable::Entity {
                interaction: EntityInteraction::CampfireSit,
                ..
            } => {
                if self.earn_achievement(Achievement::FindCampfire) {
                    self.show_hint(Hint::Campfire, Duration::from_secs(1));
                }
            },
            _ => {},
        }
    }

    pub(crate) fn event_notification(&mut self, notif: &UserNotification) {
        #[allow(clippy::single_match)]
        match notif {
            UserNotification::WaypointUpdated => {
                if self.earn_achievement(Achievement::SetWaypoint) {
                    self.show_hint(Hint::Waypoint, Duration::from_secs(1));
                }
            },
        }
    }

    pub(crate) fn event_chat_msg(&mut self, msg: &comp::ChatMsg) {
        if msg.chat_type.is_player_msg() && self.earn_achievement(Achievement::ReceivedChatMsg) {
            self.show_hint(Hint::Chat, Duration::from_secs(2));
        }
    }
}

pub struct State {
    ids: Ids,
}

widget_ids! {
    pub struct Ids {
        bg,
        text,
        old_frame,
        old_scrollbar,
        old_bg[],
        old_text[],
        old_icon[],
    }
}

#[derive(WidgetCommon)]
pub struct Tutorial<'a> {
    _show: &'a Show,
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    global_state: &'a mut GlobalState,
    _rot_imgs: &'a ImgsRot,
    _tooltip_manager: &'a mut TooltipManager,
    _item_imgs: &'a ItemImgs,
    pulse: f32,
    dt: Duration,
    esc_menu: bool,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

const MARGIN: f64 = 16.0;

impl<'a> Tutorial<'a> {
    pub fn new(
        _show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        global_state: &'a mut GlobalState,
        _rot_imgs: &'a ImgsRot,
        _tooltip_manager: &'a mut TooltipManager,
        _item_imgs: &'a ItemImgs,
        pulse: f32,
        dt: Duration,
        esc_menu: bool,
    ) -> Self {
        Self {
            _show,
            client,
            imgs,
            _rot_imgs,
            fonts,
            localized_strings,
            global_state,
            _tooltip_manager,
            _item_imgs,
            pulse,
            dt,
            esc_menu,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub enum Event {
    #[allow(dead_code)]
    Close,
}

impl Widget for Tutorial<'_> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        Self::State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        self.global_state.profile.tutorial.update(self.dt);
        self.global_state.profile.tutorial.event_tick(self.client);

        let mut old = Vec::new();
        if self.esc_menu {
            old.extend(
                self.global_state
                    .profile
                    .tutorial
                    .goals
                    .iter()
                    .rev()
                    .map(|n| (*n, false)),
            );
            old.extend(
                self.global_state
                    .profile
                    .tutorial
                    .done
                    .iter()
                    .rev()
                    .map(|n| (*n, true)),
            );
        }

        if state.ids.old_bg.len() < old.len() {
            state.update(|s| {
                s.ids
                    .old_bg
                    .resize(old.len(), &mut ui.widget_id_generator());
                s.ids
                    .old_text
                    .resize(old.len(), &mut ui.widget_id_generator());
                s.ids
                    .old_icon
                    .resize(old.len(), &mut ui.widget_id_generator());
            })
        }

        if !old.is_empty() {
            Rectangle::fill_with([tweak!(130.0), tweak!(100.0)], color::TRANSPARENT)
                .mid_right_with_margin_on(ui.window, 0.0)
                .scroll_kids_vertically()
                .w_h(750.0, 350.0)
                .set(state.ids.old_frame, ui);
            Scrollbar::y_axis(state.ids.old_frame)
                .thickness(5.0)
                .auto_hide(true)
                .rgba(1.0, 1.0, 1.0, 0.2)
                .set(state.ids.old_scrollbar, ui);
        }

        const BACKGROUND: Color = Color::Rgba(0.0, 0.0, 0.0, 0.85);

        for (i, (node, is_done)) in old.iter().copied().enumerate() {
            let bg = RoundedRectangle::fill_with([tweak!(230.0), tweak!(100.0)], 20.0, BACKGROUND);
            let bg = if i == 0 {
                bg.top_left_with_margins_on(state.ids.old_frame, tweak!(8.0), tweak!(8.0))
            } else {
                bg.down_from(state.ids.old_bg[i - 1], 8.0)
            };
            bg.w_h(750.0, 52.0)
                .parent(state.ids.old_frame)
                .set(state.ids.old_bg[i], ui);

            Image::new(if is_done {
                self.imgs.check_checked
            } else {
                self.imgs.check
            })
            .mid_left_with_margin_on(state.ids.old_bg[i], MARGIN)
            .w_h(24.0, 24.0)
            .set(state.ids.old_icon[i], ui);

            Text::new(&node.get_msg(&self.global_state.settings, self.localized_strings))
                .mid_left_with_margin_on(state.ids.old_bg[i], 24.0 + MARGIN * 2.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(16))
                .color(TEXT_COLOR)
                .set(state.ids.old_text[i], ui);
        }

        if let Some((current, _, anim)) = &mut self.global_state.profile.tutorial.current {
            *anim += self.dt;

            let anim = ((Hint::FADE_TIME - (anim.as_secs_f32() - Hint::FADE_TIME).abs()) * 3.0)
                .clamped(0.0, 1.0);
            let anim_movement = anim * (1.0 + (self.pulse * 3.0).sin() * 0.35);

            RoundedRectangle::fill_with(
                [tweak!(130.0), tweak!(100.0)],
                20.0,
                BACKGROUND.with_alpha(0.85 * anim),
            )
            .mid_top_with_margin_on(ui.window, 80.0 * anim_movement.sqrt() as f64)
            .w_h(750.0, 52.0)
            .set(state.ids.bg, ui);

            Text::new(&current.get_msg(&self.global_state.settings, self.localized_strings))
                .mid_left_with_margin_on(state.ids.bg, MARGIN)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(16))
                .color(TEXT_COLOR.with_alpha(anim))
                .set(state.ids.text, ui);
        }

        None
    }
}
