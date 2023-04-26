use std::collections::VecDeque;

use crate::ui::fonts::Fonts;
use client::Client;
use common::comp;
use conrod_core::{
    widget::{self, Text},
    widget_ids, Colorable, Positionable, Widget, WidgetCommon,
};
use i18n::Localization;

use vek::{Vec2, Vec3};

widget_ids! {
    struct Ids {
        subtitle_box,
        subtitle_box_bg,
        subtitle_message[],
    }
}

#[derive(WidgetCommon)]
pub struct Subtitles<'a> {
    client: &'a Client,

    fonts: &'a Fonts,

    new_subtitles: &'a mut VecDeque<Subtitle>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,

    localized_strings: &'a Localization,
}

impl<'a> Subtitles<'a> {
    pub fn new(
        client: &'a Client,
        new_subtitles: &'a mut VecDeque<Subtitle>,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            client,
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
        });

        let mut subtitles = state
            .ids
            .subtitle_message
            .iter()
            .zip(state.subtitles.iter());

        let fade_amount = |t: &Subtitle| ((t.show_until - time) * 1.5).clamp(0.0, 1.0) as f32;

        let player_dir = player_dir.xy().try_normalized().unwrap_or(Vec2::unit_y());
        let player_right = Vec2::new(player_dir.y, -player_dir.x);

        let message = |subtitle: &Subtitle| {
            let message = self.localized_strings.get_msg(&subtitle.localization);
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
                Some(true) => format!("   {message} >"),
                Some(false) => format!("< {message}   "),
                None => format!("   {message}   "),
            }
        };

        if let Some((id, subtitle)) = subtitles.next() {
            Text::new(&message(subtitle))
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .center_justify()
                .bottom_right_with_margins(40.0, 50.0)
                .color(conrod_core::Color::Rgba(
                    0.9,
                    1.0,
                    1.0,
                    fade_amount(subtitle),
                ))
                .set(*id, ui);
            let mut last_id = *id;
            for (id, subtitle) in subtitles {
                Text::new(&message(subtitle))
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .center_justify()
                    .up_from(last_id, 10.0)
                    .color(conrod_core::Color::Rgba(
                        0.9,
                        1.0,
                        1.0,
                        fade_amount(subtitle),
                    ))
                    .set(*id, ui);
                last_id = *id;
            }
        }
    }
}
