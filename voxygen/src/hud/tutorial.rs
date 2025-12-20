use crate::{
    GlobalState, Settings,
    ui::{TooltipManager, fonts::Fonts},
};
use client::Client;
use common::{
    DamageSource,
    comp::{self, CharacterState, ItemKey, Vel},
    rtsim,
};
use conrod_core::{
    Borderable, Color, Colorable, Positionable, Sizeable, UiCell, Widget, WidgetCommon, color,
    widget::{self, Button, Image, Rectangle, RoundedRectangle, Scrollbar, Text},
    widget_ids,
};
use hashbrown::HashSet;
use i18n::Localization;
use inline_tweak::*;
use serde::{Deserialize, Serialize};
use specs::WorldExt;
use std::{
    borrow::Cow,
    time::{Duration, Instant},
};
use vek::*;

use super::{
    GameInput, Outcome, Show, TEXT_COLOR, animate_by_pulse,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hint {
    Move,
    Jump,
    OpenInventory,
    OpenGlider,
    Glider,
    StallGlider,
    Roll,
    Attacked,
    Unwield,
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
            Self::Jump => i18n.get_msg_ctx(&key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Jump),
            }),
            Self::OpenInventory => i18n.get_msg_ctx(key, &i18n::fluent_args! {
                "key" => get_key(GameInput::Inventory),
            }),
            Self::OpenGlider => i18n.get_msg_ctx(key, &i18n::fluent_args! {
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
            _ => i18n.get_msg(key),
        }
    }
}

impl Achievement {
    fn get_msg<'a>(&self, settings: &Settings, i18n: &'a Localization) -> Cow<'a, str> {
        i18n.get_msg(&format!("achievement-{self:?}"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TutorialState {
    // (_, time_since_active)
    current: Option<(Hint, Option<Achievement>, Duration)>,
    // (_, cancel if achieved, time until display)
    pending: Vec<(Hint, Achievement, Duration)>,

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
        };
        this.add_hinted_goal(Hint::Move, Achievement::Moved, Duration::from_secs(10));
        this
    }
}

impl TutorialState {
    fn update(&mut self, dt: Duration) {
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
                self.current = Some((*hint, Some(*achievement), Duration::ZERO));
                false
            } else if self.done.contains(achievement) {
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
            self.pending.retain(|(_, a, _)| a != &achievement);
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
        if self.pending.iter().all(|(h, _, _)| h != &hint) {
            self.add_goal(achievement);
            self.pending.push((hint, achievement, timeout));
        }
    }

    fn show_hint(&mut self, hint: Hint) { self.current = Some((hint, None, Duration::ZERO)); }

    pub(crate) fn event_tick(&mut self, client: &Client) {
        if let Some(CharacterState::Glide(glide)) = client.current::<CharacterState>()
            && glide.ori.look_dir().z > 0.5
            && let Some(vel) = client.current::<Vel>()
            && vel.0.z < 0.0
            && self.earn_achievement(Achievement::StallGlider)
        {
            self.show_hint(Hint::StallGlider);
        }

        if let Some(cs) = client.current::<CharacterState>() {
            if cs.is_wield() && self.earn_achievement(Achievement::Wield) {
                self.add_hinted_goal(Hint::Unwield, Achievement::Unwield, Duration::from_mins(2));
            }

            if !cs.is_wield() && self.done(Achievement::Wield) {
                self.earn_achievement(Achievement::Unwield);
            }
        }
    }

    pub(crate) fn event_move(&mut self) {
        self.earn_achievement(Achievement::Moved);
        self.add_hinted_goal(Hint::Jump, Achievement::Jumped, Duration::from_secs(10));
    }

    pub(crate) fn event_jump(&mut self) {
        self.earn_achievement(Achievement::Jumped);
        self.add_hinted_goal(Hint::Roll, Achievement::Rolled, Duration::from_secs(15));
    }

    pub(crate) fn event_roll(&mut self) { self.earn_achievement(Achievement::Rolled); }

    pub(crate) fn event_collect(&mut self) {
        self.add_hinted_goal(
            Hint::OpenInventory,
            Achievement::OpenInventory,
            Duration::from_secs(2),
        );
    }

    pub(crate) fn event_open_inventory(&mut self) {
        self.earn_achievement(Achievement::OpenInventory);
    }

    pub(crate) fn event_outcome(&mut self, client: &Client, outcome: &Outcome) {
        match outcome {
            Outcome::HealthChange { info, .. }
                if Some(info.target) == client.uid()
                    && info.cause == Some(DamageSource::Falling) =>
            {
                self.add_hinted_goal(
                    Hint::OpenGlider,
                    Achievement::OpenGlider,
                    Duration::from_secs(1),
                );
            },
            Outcome::HealthChange { info, .. }
                if Some(info.target) == client.uid()
                    && !matches!(info.cause, Some(DamageSource::Falling)) =>
            {
                self.add_hinted_goal(Hint::Attacked, Achievement::Wield, Duration::ZERO);
            },
            _ => {},
        }
    }

    pub(crate) fn event_open_glider(&mut self) {
        if self.earn_achievement(Achievement::OpenGlider) {
            self.show_hint(Hint::Glider);
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
    item_imgs: &'a ItemImgs,
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
        item_imgs: &'a ItemImgs,
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
            item_imgs,
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
        let mut event = None;

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

            let anim_alpha =
                (Hint::FADE_TIME - (anim.as_secs_f32() - Hint::FADE_TIME).abs()).clamped(0.0, 1.0);

            RoundedRectangle::fill_with(
                [tweak!(130.0), tweak!(100.0)],
                20.0,
                BACKGROUND.with_alpha(0.85 * anim_alpha),
            )
            .mid_top_with_margin_on(ui.window, 80.0)
            .w_h(750.0, 52.0)
            .set(state.ids.bg, ui);

            Text::new(&current.get_msg(&self.global_state.settings, self.localized_strings))
                .mid_left_with_margin_on(state.ids.bg, MARGIN)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(16))
                .color(TEXT_COLOR.with_alpha(anim_alpha))
                .set(state.ids.text, ui);
        }

        event
    }
}
