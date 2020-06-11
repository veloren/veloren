use super::Show;
use crate::{i18n::VoxygenLocalization, ui::fonts::ConrodVoxygenFonts};
use client::{self, Client, Event as ClientEvent};
use common::msg::Notification;
use conrod_core::{
    widget::{self, Text},
    widget_ids, Color, Colorable, Positionable, Widget, WidgetCommon,
};
use std::{collections::VecDeque, time::Instant};

widget_ids! {
    struct Ids {
        error_bg,
        error_text,
        info_bg,
        info_text,
        message_bg,
        message_text,
    }
}

#[derive(WidgetCommon)]
pub struct Popup<'a> {
    voxygen_i18n: &'a std::sync::Arc<VoxygenLocalization>,
    client: &'a Client,
    new_messages: &'a VecDeque<ClientEvent>,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    show: &'a Show,
}

/// Popup notifications for messages such as <Chunk Name>, Waypoint Saved,
/// Dungeon Cleared (TODO), and Quest Completed (TODO)
impl<'a> Popup<'a> {
    pub fn new(
        voxygen_i18n: &'a std::sync::Arc<VoxygenLocalization>,
        client: &'a Client,
        new_messages: &'a VecDeque<ClientEvent>,
        fonts: &'a ConrodVoxygenFonts,
        show: &'a Show,
    ) -> Self {
        Self {
            voxygen_i18n,
            client,
            new_messages,
            fonts,
            common: widget::CommonBuilder::default(),
            show,
        }
    }
}

pub struct State {
    ids: Ids,
    errors: VecDeque<String>,
    infos: VecDeque<String>,
    messages: VecDeque<String>,
    last_error_update: Instant,
    last_info_update: Instant,
    last_message_update: Instant,
    last_region_name: Option<String>,
}

impl<'a> Widget for Popup<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            errors: VecDeque::new(),
            infos: VecDeque::new(),
            messages: VecDeque::new(),
            last_error_update: Instant::now(),
            last_info_update: Instant::now(),
            last_message_update: Instant::now(),
            last_region_name: None,
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        const FADE_IN: f32 = 0.5;
        const FADE_HOLD: f32 = 1.0;
        const FADE_OUT: f32 = 3.0;

        let bg_color = |fade| Color::Rgba(0.0, 0.0, 0.0, fade);
        let error_color = |fade| Color::Rgba(1.0, 0.0, 0.0, fade);
        let info_color = |fade| Color::Rgba(1.0, 1.0, 0.0, fade);
        let message_color = |fade| Color::Rgba(1.0, 1.0, 1.0, fade);

        // Push chunk name to message queue
        if let Some(chunk) = self.client.current_chunk() {
            let current = chunk.meta().name();
            // Check if no other popup is displayed and a new one is needed
            if state.messages.is_empty()
                && state
                    .last_region_name
                    .as_ref()
                    .map(|l| l != current)
                    .unwrap_or(true)
            {
                // Update last_region
                state.update(|s| {
                    if s.messages.is_empty() {
                        s.last_message_update = Instant::now();
                    }
                    s.last_region_name = Some(current.to_owned());
                    s.messages.push_back(current.to_owned());
                });
            }
        }

        // Push waypoint to message queue
        for notification in self.new_messages {
            match notification {
                ClientEvent::Notification(Notification::WaypointSaved) => {
                    state.update(|s| {
                        if s.infos.is_empty() {
                            s.last_info_update = Instant::now();
                        }
                        let text = self.voxygen_i18n.get("hud.waypoint_saved");
                        s.infos.push_back(text.to_string());
                    });
                },
                _ => {},
            }
        }

        // Get next error from queue
        if !state.errors.is_empty()
            && state.last_error_update.elapsed().as_secs_f32() > FADE_IN + FADE_HOLD + FADE_OUT
        {
            state.update(|s| {
                s.errors.pop_front();
                s.last_error_update = Instant::now();
            });
        }

        // Display error as popup
        if let Some(error) = state.errors.front() {
            let seconds = state.last_error_update.elapsed().as_secs_f32();
            let fade = if seconds < FADE_IN {
                seconds / FADE_IN
            } else if seconds < FADE_IN + FADE_HOLD {
                1.0
            } else {
                (1.0 - (seconds - FADE_IN - FADE_HOLD) / FADE_OUT).max(0.0)
            };
            Text::new(error)
                .mid_top_with_margin_on(ui.window, 50.0)
                .font_size(self.fonts.cyri.scale(20))
                .font_id(self.fonts.cyri.conrod_id)
                .color(bg_color(fade))
                .set(state.ids.error_bg, ui);
            Text::new(error)
                .top_left_with_margins_on(state.ids.error_bg, -1.0, -1.0)
                .font_size(self.fonts.cyri.scale(20))
                .font_id(self.fonts.cyri.conrod_id)
                .color(error_color(fade))
                .set(state.ids.error_text, ui);
        }

        // Get next info from queue
        if !state.infos.is_empty()
            && state.last_info_update.elapsed().as_secs_f32() > FADE_IN + FADE_HOLD + FADE_OUT
        {
            state.update(|s| {
                s.infos.pop_front();
                s.last_info_update = Instant::now();
            });
        }

        // Display info as popup
        if !self.show.intro {
            if let Some(info) = state.infos.front() {
                let seconds = state.last_info_update.elapsed().as_secs_f32();
                let fade = if seconds < FADE_IN {
                    seconds / FADE_IN
                } else if seconds < FADE_IN + FADE_HOLD {
                    1.0
                } else {
                    (1.0 - (seconds - FADE_IN - FADE_HOLD) / FADE_OUT).max(0.0)
                };

                Text::new(info)
                    .mid_top_with_margin_on(ui.window, 100.0)
                    .font_size(self.fonts.cyri.scale(20))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(bg_color(fade))
                    .set(state.ids.info_bg, ui);
                Text::new(info)
                    .top_left_with_margins_on(state.ids.info_bg, -1.0, -1.0)
                    .font_size(self.fonts.cyri.scale(20))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(info_color(fade))
                    .set(state.ids.info_text, ui);
            }
        }

        // Get next message from queue
        if !state.messages.is_empty()
            && state.last_message_update.elapsed().as_secs_f32() > FADE_IN + FADE_HOLD + FADE_OUT
        {
            state.update(|s| {
                s.messages.pop_front();
                s.last_message_update = Instant::now();
            });
        }

        // Display message as popup
        if !self.show.intro {
            if let Some(message) = state.messages.front() {
                let seconds = state.last_message_update.elapsed().as_secs_f32();
                let fade = if seconds < FADE_IN {
                    seconds / FADE_IN
                } else if seconds < FADE_IN + FADE_HOLD {
                    1.0
                } else {
                    (1.0 - (seconds - FADE_IN - FADE_HOLD) / FADE_OUT).max(0.0)
                };
                Text::new(message)
                    .mid_top_with_margin_on(ui.window, 200.0)
                    .font_size(self.fonts.alkhemi.scale(70))
                    .font_id(self.fonts.alkhemi.conrod_id)
                    .color(bg_color(fade))
                    .set(state.ids.message_bg, ui);
                Text::new(message)
                    .top_left_with_margins_on(state.ids.message_bg, -2.5, -2.5)
                    .font_size(self.fonts.alkhemi.scale(70))
                    .font_id(self.fonts.alkhemi.conrod_id)
                    .color(message_color(fade))
                    .set(state.ids.message_text, ui);
            }
        }
    }
}
