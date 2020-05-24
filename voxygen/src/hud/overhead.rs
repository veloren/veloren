use super::{img_ids::Imgs, HP_COLOR, LOW_HP_COLOR, MANA_COLOR};
use crate::ui::{fonts::ConrodVoxygenFonts, Ingameable};
use common::comp::{Energy, Stats};
use conrod_core::{
    position::Align,
    widget::{self, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        // Chat bubble
        chat_bubble_text,
        chat_bubble_text2,
        chat_bubble_top_left,
        chat_bubble_top,
        chat_bubble_top_right,
        chat_bubble_left,
        chat_bubble_mid,
        chat_bubble_right,
        chat_bubble_bottom_left,
        chat_bubble_bottom,
        chat_bubble_bottom_right,
        chat_bubble_tail,

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
    stats: &'a Stats,
    energy: &'a Energy,
    own_level: u32,
    pulse: f32,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Overhead<'a> {
    pub fn new(
        name: &'a str,
        stats: &'a Stats,
        energy: &'a Energy,
        own_level: u32,
        pulse: f32,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
    ) -> Self {
        Self {
            name,
            stats,
            energy,
            own_level,
            pulse,
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
        // - 2 Text::new for speech bubble
        // - 10 Image::new for speech bubble (9-slice + tail)
        // - 1 for level: either Text or Image
        // - 4 for HP + mana + fg + bg
        19
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
            .color(Color::Rgba(0.61, 0.61, 0.89, 1.0))
            .x_y(0.0, MANA_BAR_Y + 50.0)
            .set(state.ids.name, ui);

        // Speech bubble
        Text::new("Hello")
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(15)
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .up_from(state.ids.name, 10.0)
            .x_align_to(state.ids.name, Align::Middle)
            .parent(id)
            .set(state.ids.chat_bubble_text, ui);
        Image::new(self.imgs.chat_bubble_top_left)
            .w_h(10.0, 10.0)
            .top_left_with_margin_on(state.ids.chat_bubble_text, -10.0)
            .parent(id)
            .set(state.ids.chat_bubble_top_left, ui);
        Image::new(self.imgs.chat_bubble_top)
            .h(10.0)
            .w_of(state.ids.chat_bubble_text)
            .mid_top_with_margin_on(state.ids.chat_bubble_text, -10.0)
            .parent(id)
            .set(state.ids.chat_bubble_top, ui);
        Image::new(self.imgs.chat_bubble_top_right)
            .w_h(10.0, 10.0)
            .top_right_with_margin_on(state.ids.chat_bubble_text, -10.0)
            .parent(id)
            .set(state.ids.chat_bubble_top_right, ui);
        Image::new(self.imgs.chat_bubble_left)
            .w(10.0)
            .h_of(state.ids.chat_bubble_text)
            .mid_left_with_margin_on(state.ids.chat_bubble_text, -10.0)
            .parent(id)
            .set(state.ids.chat_bubble_left, ui);
        Image::new(self.imgs.chat_bubble_mid)
            .wh_of(state.ids.chat_bubble_text)
            .top_left_of(state.ids.chat_bubble_text)
            .parent(id)
            .set(state.ids.chat_bubble_mid, ui);
        Image::new(self.imgs.chat_bubble_right)
            .w(10.0)
            .h_of(state.ids.chat_bubble_text)
            .mid_right_with_margin_on(state.ids.chat_bubble_text, -10.0)
            .parent(id)
            .set(state.ids.chat_bubble_right, ui);
        Image::new(self.imgs.chat_bubble_bottom_left)
            .w_h(10.0, 10.0)
            .bottom_left_with_margin_on(state.ids.chat_bubble_text, -10.0)
            .parent(id)
            .set(state.ids.chat_bubble_bottom_left, ui);
        Image::new(self.imgs.chat_bubble_bottom)
            .h(10.0)
            .w_of(state.ids.chat_bubble_text)
            .mid_bottom_with_margin_on(state.ids.chat_bubble_text, -10.0)
            .parent(id)
            .set(state.ids.chat_bubble_bottom, ui);
        Image::new(self.imgs.chat_bubble_bottom_right)
            .w_h(10.0, 10.0)
            .bottom_right_with_margin_on(state.ids.chat_bubble_text, -10.0)
            .parent(id)
            .set(state.ids.chat_bubble_bottom_right, ui);
        Image::new(self.imgs.chat_bubble_tail)
            .w_h(11.0, 16.0)
            .mid_bottom_with_margin_on(state.ids.chat_bubble_text, -16.0)
            .parent(id)
            .set(state.ids.chat_bubble_tail, ui);
        // Why is there a second text widget?: The first is to position the 9-slice
        // around and the second is to display text. Changing .depth manually
        // causes strange problems in unrelated parts of the ui (the debug
        // overlay is offset by a npc's screen position) TODO
        Text::new("Hello")
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(15)
            .top_left_of(state.ids.chat_bubble_text)
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .parent(id)
            .set(state.ids.chat_bubble_text2, ui);

        let hp_percentage =
            self.stats.health.current() as f64 / self.stats.health.maximum() as f64 * 100.0;
        let energy_percentage = self.energy.current() as f64 / self.energy.maximum() as f64 * 100.0;
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
        Rectangle::fill_with(
            [
                72.0 * (self.energy.current() as f64 / self.energy.maximum() as f64) * BARSIZE,
                MANA_BAR_HEIGHT,
            ],
            MANA_COLOR,
        )
        .x_y(
            ((3.5 + (energy_percentage / 100.0 * 36.5)) - 36.45) * BARSIZE,
            MANA_BAR_Y, //-32.0,
        )
        .parent(id)
        .set(state.ids.mana_bar, ui);

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
