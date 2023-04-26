use std::collections::VecDeque;

use crate::{settings::Settings, ui::fonts::Fonts};
use client::Client;
use common::comp;
use conrod_core::{
    widget::{self, Id, Rectangle, Text},
    widget_ids, Colorable, Positionable, UiCell, Widget, WidgetCommon,
};
use i18n::Localization;

use vek::{Vec2, Vec3};

widget_ids! {
    struct Ids {
        subtitle_box_bg,
        subtitle_message[],
        subtitle_dir[],
    }
}

#[derive(WidgetCommon)]
pub struct Subtitles<'a> {
    client: &'a Client,
    settings: &'a Settings,

    fonts: &'a Fonts,

    new_subtitles: &'a mut VecDeque<Subtitle>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,

    localized_strings: &'a Localization,
}

impl<'a> Subtitles<'a> {
    pub fn new(
        client: &'a Client,
        settings: &'a Settings,
        new_subtitles: &'a mut VecDeque<Subtitle>,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            client,
            settings,
            fonts,
            new_subtitles,
            common: widget::CommonBuilder::default(),
            localized_strings,
        }
    }
}

const MAX_SUBTITLE_DIST: f32 = 80.0;

#[derive(Debug)]
pub struct Subtitle {
    pub localization: String,
    pub position: Option<Vec3<f32>>,
    pub show_until: f64,
}

pub struct State {
    subtitles: Vec<Subtitle>,
    ids: Ids,
}

impl<'a> Widget for Subtitles<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            subtitles: Vec::new(),
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Chat::update");

        let widget::UpdateArgs { state, ui, .. } = args;
        let time = self.client.state().get_time();
        let player_pos = self.client.position().unwrap_or_default();
        let player_dir = self
            .client
            .state()
            .read_storage::<comp::Ori>()
            .get(self.client.entity())
            .map_or(Vec3::unit_y(), |ori| ori.look_vec());
        // Empty old subtitles and add new.
        state.update(|s| {
            s.subtitles.retain(|subtitle| {
                time <= subtitle.show_until
                    && subtitle
                        .position
                        .map_or(true, |pos| pos.distance(player_pos) <= MAX_SUBTITLE_DIST)
            });
            for mut subtitle in self.new_subtitles.drain(..) {
                if subtitle
                    .position
                    .map_or(false, |pos| pos.distance(player_pos) > MAX_SUBTITLE_DIST)
                {
                    continue;
                }
                let t = time + subtitle.show_until;
                if let Some(s) = s
                    .subtitles
                    .iter_mut()
                    .find(|s| s.localization == subtitle.localization)
                {
                    if t > s.show_until {
                        s.show_until = t;
                        s.position = subtitle.position;
                    }
                } else {
                    subtitle.show_until = t;
                    s.subtitles.push(subtitle);
                }
            }
            s.ids
                .subtitle_message
                .resize(s.subtitles.len(), &mut ui.widget_id_generator());
            s.ids
                .subtitle_dir
                .resize(s.subtitles.len(), &mut ui.widget_id_generator());
        });

        let color = |t: &Subtitle| {
            conrod_core::Color::Rgba(
                0.9,
                1.0,
                1.0,
                ((t.show_until - time) * 1.5).clamp(0.0, 1.0) as f32,
            )
        };

        let player_dir = player_dir.xy().try_normalized().unwrap_or(Vec2::unit_y());
        let player_right = Vec2::new(player_dir.y, -player_dir.x);

        let message = |subtitle: &Subtitle| self.localized_strings.get_msg(&subtitle.localization);

        let dir = |subtitle: &Subtitle, id: &Id, dir_id: &Id, ui: &mut UiCell| {
            let is_right = subtitle.position.and_then(|pos| {
                let dist = pos.distance(player_pos);
                let dir = (pos - player_pos) / dist;

                let dot = dir.xy().dot(player_dir);
                if dist < 2.0 || dot > 0.85 {
                    None
                } else {
                    Some(dir.xy().dot(player_right) >= 0.0)
                }
            });
            match is_right {
                Some(true) => Text::new(">  ")
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .parent(state.ids.subtitle_box_bg)
                    .align_right_of(state.ids.subtitle_box_bg)
                    .align_middle_y_of(*id)
                    .color(color(subtitle))
                    .set(*dir_id, ui),
                Some(false) => Text::new("  <")
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .parent(state.ids.subtitle_box_bg)
                    .align_left_of(state.ids.subtitle_box_bg)
                    .align_middle_y_of(*id)
                    .color(color(subtitle))
                    .set(*dir_id, ui),
                None => Text::new("")
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .parent(state.ids.subtitle_box_bg)
                    .color(color(subtitle))
                    .set(*dir_id, ui),
            }
        };

        Rectangle::fill([200.0, 2.0 + 22.0 * state.subtitles.len() as f64])
            .rgba(0.0, 0.0, 0.0, self.settings.chat.chat_opacity)
            .bottom_right_with_margins_on(ui.window, 40.0, 50.0)
            .set(state.ids.subtitle_box_bg, ui);

        let mut subtitles = state
            .ids
            .subtitle_message
            .iter()
            .zip(state.ids.subtitle_dir.iter())
            .zip(state.subtitles.iter());

        if let Some(((id, dir_id), subtitle)) = subtitles.next() {
            Text::new(&message(subtitle))
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .parent(state.ids.subtitle_box_bg)
                .center_justify()
                .mid_bottom_with_margin_on(state.ids.subtitle_box_bg, 6.0)
                .color(color(subtitle))
                .set(*id, ui);

            dir(subtitle, id, dir_id, ui);

            let mut last_id = *id;
            for ((id, dir_id), subtitle) in subtitles {
                Text::new(&message(subtitle))
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .parent(state.ids.subtitle_box_bg)
                    .up_from(last_id, 8.0)
                    .align_middle_x_of(last_id)
                    .color(color(subtitle))
                    .set(*id, ui);

                dir(subtitle, id, dir_id, ui);

                last_id = *id;
            }
        }
    }
}
