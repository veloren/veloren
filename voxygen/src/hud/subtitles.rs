use std::{cmp::Ordering, collections::VecDeque};

use crate::{audio::Listener, settings::Settings, ui::fonts::Fonts};
use client::Client;
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
    listener: &'a Listener,

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
        listener: &'a Listener,
        new_subtitles: &'a mut VecDeque<Subtitle>,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            client,
            settings,
            listener,
            fonts,
            new_subtitles,
            common: widget::CommonBuilder::default(),
            localized_strings,
        }
    }
}

const MIN_SUBTITLE_DURATION: f64 = 1.5;
const MAX_SUBTITLE_DIST: f32 = 80.0;

#[derive(Debug)]
pub struct Subtitle {
    pub localization: String,
    /// Position the sound is played at, if any.
    pub position: Option<Vec3<f32>>,
    /// Amount of seconds to show the subtitle for.
    pub show_for: f64,
}

#[derive(Clone, PartialEq)]
struct SubtitleData {
    position: Option<Vec3<f32>>,
    /// `Time` to show until.
    show_until: f64,
}

impl SubtitleData {
    /// Prioritize showing nearby sounds, and secondarily prioritize longer
    /// living sounds.
    fn compare_priority(&self, other: &Self, listener_pos: Vec3<f32>) -> Ordering {
        let life_cmp = self
            .show_until
            .partial_cmp(&other.show_until)
            .unwrap_or(Ordering::Equal);
        match (self.position, other.position) {
            (Some(a), Some(b)) => match a
                .distance_squared(listener_pos)
                .partial_cmp(&b.distance_squared(listener_pos))
                .unwrap_or(Ordering::Equal)
            {
                Ordering::Equal => life_cmp,
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
            },
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => life_cmp,
        }
    }
}

#[derive(Clone)]
struct SubtitleList {
    subtitles: Vec<(String, Vec<SubtitleData>)>,
}

impl SubtitleList {
    fn new() -> Self {
        Self {
            subtitles: Vec::new(),
        }
    }

    /// Updates the subtitle state, returns the amount of subtitles that should
    /// be displayed.
    fn update(
        &mut self,
        new_subtitles: impl Iterator<Item = Subtitle>,
        time: f64,
        listener_pos: Vec3<f32>,
    ) -> usize {
        for subtitle in new_subtitles {
            let show_until = time + subtitle.show_for.max(MIN_SUBTITLE_DURATION);
            let data = SubtitleData {
                position: subtitle.position,
                show_until,
            };
            if let Some((_, datas)) = self
                .subtitles
                .iter_mut()
                .find(|(key, _)| key == &subtitle.localization)
            {
                datas.push(data);
            } else {
                self.subtitles.push((subtitle.localization, vec![data]))
            }
        }
        let mut to_display = 0;
        self.subtitles.retain_mut(|(_, data)| {
            data.retain(|subtitle| subtitle.show_until > time);
            // Place the most prioritized subtitle in the back.
            if let Some((i, s)) = data
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.compare_priority(b, listener_pos))
            {
                // We only display subtitles that are in range.
                if s.position.map_or(true, |pos| {
                    pos.distance_squared(listener_pos) < MAX_SUBTITLE_DIST * MAX_SUBTITLE_DIST
                }) {
                    to_display += 1;
                }
                let last = data.len() - 1;
                data.swap(i, last);
                true
            } else {
                // If data is empty we have no sounds with this key.
                false
            }
        });
        to_display
    }
}

pub struct State {
    subtitles: SubtitleList,
    ids: Ids,
}

impl<'a> Widget for Subtitles<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            subtitles: SubtitleList::new(),
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Chat::update");

        let widget::UpdateArgs { state, ui, .. } = args;
        let time = self.client.state().get_time();
        let listener_pos = self.listener.pos;
        let listener_forward = self.listener.ori;

        // Update subtitles and look for changes
        let mut subtitles = state.subtitles.clone();

        let has_new = !self.new_subtitles.is_empty();

        let show_count = subtitles.update(self.new_subtitles.drain(..), time, listener_pos);

        let subtitles = if has_new || show_count != state.ids.subtitle_message.len() {
            state.update(|s| {
                s.subtitles = subtitles;
                s.ids
                    .subtitle_message
                    .resize(show_count, &mut ui.widget_id_generator());
                s.ids
                    .subtitle_dir
                    .resize(show_count, &mut ui.widget_id_generator());
            });
            &state.subtitles
        } else {
            &subtitles
        };
        let color = |t: &SubtitleData| -> conrod_core::Color {
            conrod_core::Color::Rgba(
                0.9,
                1.0,
                1.0,
                ((t.show_until - time) * 2.0).clamp(0.0, 1.0) as f32,
            )
        };

        let listener_forward = listener_forward
            .xy()
            .try_normalized()
            .unwrap_or(Vec2::unit_y());
        let listener_right = Vec2::new(listener_forward.y, -listener_forward.x);

        let dir = |subtitle: &SubtitleData, id: &Id, dir_id: &Id, ui: &mut UiCell| {
            enum Side {
                /// Also used for sounds without direction.
                Forward,
                Right,
                Left,
            }
            let is_right = subtitle
                .position
                .map(|pos| {
                    let dist = pos.distance(listener_pos);
                    let dir = (pos - listener_pos) / dist;

                    let dot = dir.xy().dot(listener_forward);
                    if dist < 2.0 || dot > 0.85 {
                        Side::Forward
                    } else if dir.xy().dot(listener_right) >= 0.0 {
                        Side::Right
                    } else {
                        Side::Left
                    }
                })
                .unwrap_or(Side::Forward);

            match is_right {
                Side::Right => Text::new(">  ")
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .parent(state.ids.subtitle_box_bg)
                    .align_right_of(state.ids.subtitle_box_bg)
                    .align_middle_y_of(*id)
                    .color(color(subtitle))
                    .set(*dir_id, ui),
                Side::Left => Text::new("  <")
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .parent(state.ids.subtitle_box_bg)
                    .align_left_of(state.ids.subtitle_box_bg)
                    .align_middle_y_of(*id)
                    .color(color(subtitle))
                    .set(*dir_id, ui),
                Side::Forward => Text::new("")
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .parent(state.ids.subtitle_box_bg)
                    .color(color(subtitle))
                    .set(*dir_id, ui),
            }
        };

        Rectangle::fill([200.0, 22.0 * show_count as f64])
            .rgba(0.0, 0.0, 0.0, self.settings.chat.chat_opacity)
            .bottom_right_with_margins_on(ui.window, 40.0, 30.0)
            .set(state.ids.subtitle_box_bg, ui);

        let mut subtitles = state
            .ids
            .subtitle_message
            .iter()
            .zip(state.ids.subtitle_dir.iter())
            .zip(
                subtitles
                    .subtitles
                    .iter()
                    .filter_map(|(localization, data)| {
                        Some((localization, data.last()?))
                    })
                    .filter(|(_, data)| {
                        data.position.map_or(true, |pos| {
                            pos.distance_squared(listener_pos)
                                < MAX_SUBTITLE_DIST * MAX_SUBTITLE_DIST
                        })
                    })
                    .map(|(localization, data)| {
                        (self.localized_strings.get_msg(localization), data)
                    }),
            );

        if let Some(((id, dir_id), (message, data))) = subtitles.next() {
            Text::new(&message)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .parent(state.ids.subtitle_box_bg)
                .center_justify()
                .mid_bottom_with_margin_on(state.ids.subtitle_box_bg, 6.0)
                .color(color(data))
                .set(*id, ui);

            dir(data, id, dir_id, ui);

            let mut last_id = *id;
            for ((id, dir_id), (message, data)) in subtitles {
                Text::new(&message)
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .parent(state.ids.subtitle_box_bg)
                    .up_from(last_id, 8.0)
                    .align_middle_x_of(last_id)
                    .color(color(data))
                    .set(*id, ui);

                dir(data, id, dir_id, ui);

                last_id = *id;
            }
        }
    }
}
