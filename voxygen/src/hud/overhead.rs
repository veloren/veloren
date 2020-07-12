use super::{
    img_ids::Imgs, FACTION_COLOR, GROUP_COLOR, HP_COLOR, LOW_HP_COLOR, MANA_COLOR, REGION_COLOR,
    SAY_COLOR, TELL_COLOR, TEXT_BG, TEXT_COLOR,
};
use crate::{
    i18n::VoxygenLocalization,
    settings::GameplaySettings,
    ui::{fonts::ConrodVoxygenFonts, Ingameable},
};
use common::comp::{Energy, SpeechBubble, SpeechBubbleType, Stats};
use conrod_core::{
    position::Align,
    widget::{self, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

const MAX_BUBBLE_WIDTH: f64 = 250.0;

widget_ids! {
    struct Ids {
        // Speech bubble
        speech_bubble_text,
        speech_bubble_shadow,
        speech_bubble_top_left,
        speech_bubble_top,
        speech_bubble_top_right,
        speech_bubble_left,
        speech_bubble_mid,
        speech_bubble_right,
        speech_bubble_bottom_left,
        speech_bubble_bottom,
        speech_bubble_bottom_right,
        speech_bubble_tail,
        speech_bubble_icon,

        // Name
        name_bg,
        name,

        // HP
        level,
        level_skull,
        health_bar,
        health_bar_bg,
        mana_bar,
        health_bar_fg,
    }
}

/// ui widget containing everything that goes over a character's head
/// (Speech bubble, Name, Level, HP/energy bars, etc.)
#[derive(WidgetCommon)]
pub struct Overhead<'a> {
    name: &'a str,
    bubble: Option<&'a SpeechBubble>,
    stats: &'a Stats,
    energy: Option<&'a Energy>,
    own_level: u32,
    in_group: bool,
    settings: &'a GameplaySettings,
    pulse: f32,
    voxygen_i18n: &'a std::sync::Arc<VoxygenLocalization>,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Overhead<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        name: &'a str,
        bubble: Option<&'a SpeechBubble>,
        stats: &'a Stats,
        energy: Option<&'a Energy>,
        own_level: u32,
        in_group: bool,
        settings: &'a GameplaySettings,
        pulse: f32,
        voxygen_i18n: &'a std::sync::Arc<VoxygenLocalization>,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
    ) -> Self {
        Self {
            name,
            bubble,
            stats,
            energy,
            own_level,
            in_group,
            settings,
            pulse,
            voxygen_i18n,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Ingameable for Overhead<'a> {
    fn prim_count(&self) -> usize {
        // Number of conrod primitives contained in the overhead display. TODO maybe
        // this could be done automatically?
        // - 2 Text::new for name
        // - 1 for level: either Text or Image
        // - 4 for HP + mana + fg + bg
        // If there's a speech bubble
        // - 2 Text::new for speech bubble
        // - 1 Image::new for icon
        // - 10 Image::new for speech bubble (9-slice + tail)
        7 + if self.bubble.is_some() { 13 } else { 0 }
    }
}

impl<'a> Widget for Overhead<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;

        const BARSIZE: f64 = 2.0;
        const MANA_BAR_HEIGHT: f64 = BARSIZE * 1.5;
        const MANA_BAR_Y: f64 = MANA_BAR_HEIGHT / 2.0;

        // Name
        Text::new(&self.name)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(30)
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .x_y(-1.0, MANA_BAR_Y + 48.0)
            .set(state.ids.name_bg, ui);
        Text::new(&self.name)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(30)
            .color(if self.in_group {
                Color::Rgba(1.0, 0.5, 0.6, 1.0)
            } else {
                Color::Rgba(0.61, 0.61, 0.89, 1.0)
            })
            .x_y(0.0, MANA_BAR_Y + 50.0)
            .set(state.ids.name, ui);

        // Speech bubble
        if let Some(bubble) = self.bubble {
            let dark_mode = self.settings.speech_bubble_dark_mode;
            let localizer =
                |s: &str, i| -> String { self.voxygen_i18n.get_variation(&s, i).to_string() };
            let bubble_contents: String = bubble.message(localizer);
            let (text_color, shadow_color) = bubble_color(&bubble, dark_mode);
            let mut text = Text::new(&bubble_contents)
                .color(text_color)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(18)
                .up_from(state.ids.name, 20.0)
                .x_align_to(state.ids.name, Align::Middle)
                .parent(id);

            if let Some(w) = text.get_w(ui) {
                if w > MAX_BUBBLE_WIDTH {
                    text = text.w(MAX_BUBBLE_WIDTH);
                }
            }
            Image::new(if dark_mode {
                self.imgs.dark_bubble_top_left
            } else {
                self.imgs.speech_bubble_top_left
            })
            .w_h(16.0, 16.0)
            .top_left_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_top_left, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_top
            } else {
                self.imgs.speech_bubble_top
            })
            .h(16.0)
            .padded_w_of(state.ids.speech_bubble_text, -4.0)
            .mid_top_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_top, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_top_right
            } else {
                self.imgs.speech_bubble_top_right
            })
            .w_h(16.0, 16.0)
            .top_right_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_top_right, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_left
            } else {
                self.imgs.speech_bubble_left
            })
            .w(16.0)
            .padded_h_of(state.ids.speech_bubble_text, -4.0)
            .mid_left_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_left, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_mid
            } else {
                self.imgs.speech_bubble_mid
            })
            .padded_wh_of(state.ids.speech_bubble_text, -4.0)
            .top_left_with_margin_on(state.ids.speech_bubble_text, -4.0)
            .parent(id)
            .set(state.ids.speech_bubble_mid, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_right
            } else {
                self.imgs.speech_bubble_right
            })
            .w(16.0)
            .padded_h_of(state.ids.speech_bubble_text, -4.0)
            .mid_right_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_right, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_bottom_left
            } else {
                self.imgs.speech_bubble_bottom_left
            })
            .w_h(16.0, 16.0)
            .bottom_left_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_bottom_left, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_bottom
            } else {
                self.imgs.speech_bubble_bottom
            })
            .h(16.0)
            .padded_w_of(state.ids.speech_bubble_text, -4.0)
            .mid_bottom_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_bottom, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_bottom_right
            } else {
                self.imgs.speech_bubble_bottom_right
            })
            .w_h(16.0, 16.0)
            .bottom_right_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_bottom_right, ui);
            let tail = Image::new(if dark_mode {
                self.imgs.dark_bubble_tail
            } else {
                self.imgs.speech_bubble_tail
            })
            .parent(id)
            .mid_bottom_with_margin_on(state.ids.speech_bubble_text, -32.0);

            if dark_mode {
                tail.w_h(22.0, 13.0).set(state.ids.speech_bubble_tail, ui)
            } else {
                tail.w_h(22.0, 28.0).set(state.ids.speech_bubble_tail, ui)
            };

            let mut text_shadow = Text::new(&bubble_contents)
                .color(shadow_color)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(18)
                .x_relative_to(state.ids.speech_bubble_text, 1.0)
                .y_relative_to(state.ids.speech_bubble_text, -1.0)
                .parent(id);
            // Move text to front (conrod depth is lowest first; not a z-index)
            text.depth(text_shadow.get_depth() - 1.0)
                .set(state.ids.speech_bubble_text, ui);
            if let Some(w) = text_shadow.get_w(ui) {
                if w > MAX_BUBBLE_WIDTH {
                    text_shadow = text_shadow.w(MAX_BUBBLE_WIDTH);
                }
            }
            text_shadow.set(state.ids.speech_bubble_shadow, ui);
            let icon = if self.settings.speech_bubble_icon {
                bubble_icon(&bubble, &self.imgs)
            } else {
                self.imgs.nothing
            };
            Image::new(icon)
                .w_h(16.0, 16.0)
                .top_left_with_margin_on(state.ids.speech_bubble_text, -16.0)
                .set(state.ids.speech_bubble_icon, ui);
        }

        let hp_percentage =
            self.stats.health.current() as f64 / self.stats.health.maximum() as f64 * 100.0;
        let hp_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 1.0; //Animation timer
        let crit_hp_color: Color = Color::Rgba(0.79, 0.19, 0.17, hp_ani);

        // Background
        Image::new(self.imgs.enemy_health_bg)
            .w_h(84.0 * BARSIZE, 10.0 * BARSIZE)
            .x_y(0.0, MANA_BAR_Y + 6.5) //-25.5)
            .color(Some(Color::Rgba(0.1, 0.1, 0.1, 0.8)))
            .parent(id)
            .set(state.ids.health_bar_bg, ui);

        // % HP Filling
        Image::new(self.imgs.enemy_bar)
            .w_h(73.0 * (hp_percentage / 100.0) * BARSIZE, 6.0 * BARSIZE)
            .x_y(
                (4.5 + (hp_percentage / 100.0 * 36.45 - 36.45)) * BARSIZE,
                MANA_BAR_Y + 7.5,
            )
            .color(Some(if hp_percentage <= 25.0 {
                crit_hp_color
            } else if hp_percentage <= 50.0 {
                LOW_HP_COLOR
            } else {
                HP_COLOR
            }))
            .parent(id)
            .set(state.ids.health_bar, ui);

        // % Mana Filling
        if let Some(energy) = self.energy {
            let energy_factor = energy.current() as f64 / energy.maximum() as f64;

            Rectangle::fill_with(
                [72.0 * energy_factor * BARSIZE, MANA_BAR_HEIGHT],
                MANA_COLOR,
            )
            .x_y(
                ((3.5 + (energy_factor * 36.5)) - 36.45) * BARSIZE,
                MANA_BAR_Y, //-32.0,
            )
            .parent(id)
            .set(state.ids.mana_bar, ui);
        }

        // Foreground
        Image::new(self.imgs.enemy_health)
            .w_h(84.0 * BARSIZE, 10.0 * BARSIZE)
            .x_y(0.0, MANA_BAR_Y + 6.5) //-25.5)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.99)))
            .parent(id)
            .set(state.ids.health_bar_fg, ui);

        // Level
        const LOW: Color = Color::Rgba(0.54, 0.81, 0.94, 0.4);
        const HIGH: Color = Color::Rgba(1.0, 0.0, 0.0, 1.0);
        const EQUAL: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
        // Change visuals of the level display depending on the player level/opponent
        // level
        let level_comp = self.stats.level.level() as i64 - self.own_level as i64;
        // + 10 level above player -> skull
        // + 5-10 levels above player -> high
        // -5 - +5 levels around player level -> equal
        // - 5 levels below player -> low
        if level_comp > 9 {
            let skull_ani = ((self.pulse * 0.7/* speed factor */).cos() * 0.5 + 0.5) * 10.0; //Animation timer
            Image::new(if skull_ani as i32 == 1 && rand::random::<f32>() < 0.9 {
                self.imgs.skull_2
            } else {
                self.imgs.skull
            })
            .w_h(18.0 * BARSIZE, 18.0 * BARSIZE)
            .x_y(-39.0 * BARSIZE, MANA_BAR_Y + 7.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
            .parent(id)
            .set(state.ids.level_skull, ui);
        } else {
            Text::new(&format!("{}", self.stats.level.level()))
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(if self.stats.level.level() > 9 && level_comp < 10 {
                    14
                } else {
                    15
                })
                .color(if level_comp > 4 {
                    HIGH
                } else if level_comp < -5 {
                    LOW
                } else {
                    EQUAL
                })
                .x_y(-37.0 * BARSIZE, MANA_BAR_Y + 9.0)
                .parent(id)
                .set(state.ids.level, ui);
        }
    }
}

fn bubble_color(bubble: &SpeechBubble, dark_mode: bool) -> (Color, Color) {
    let light_color = match bubble.icon {
        SpeechBubbleType::Tell => TELL_COLOR,
        SpeechBubbleType::Say => SAY_COLOR,
        SpeechBubbleType::Region => REGION_COLOR,
        SpeechBubbleType::Group => GROUP_COLOR,
        SpeechBubbleType::Faction => FACTION_COLOR,
        SpeechBubbleType::World
        | SpeechBubbleType::Quest
        | SpeechBubbleType::Trade
        | SpeechBubbleType::None => TEXT_COLOR,
    };
    if dark_mode {
        (light_color, TEXT_BG)
    } else {
        (TEXT_BG, light_color)
    }
}

fn bubble_icon(sb: &SpeechBubble, imgs: &Imgs) -> conrod_core::image::Id {
    match sb.icon {
        // One for each chat mode
        SpeechBubbleType::Tell => imgs.chat_tell_small,
        SpeechBubbleType::Say => imgs.chat_say_small,
        SpeechBubbleType::Region => imgs.chat_region_small,
        SpeechBubbleType::Group => imgs.chat_group_small,
        SpeechBubbleType::Faction => imgs.chat_faction_small,
        SpeechBubbleType::World => imgs.chat_world_small,
        SpeechBubbleType::Quest => imgs.nothing, // TODO not implemented
        SpeechBubbleType::Trade => imgs.nothing, // TODO not implemented
        SpeechBubbleType::None => imgs.nothing,  // No icon (default for npcs)
    }
}
