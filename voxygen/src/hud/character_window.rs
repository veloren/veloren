use super::{img_ids::Imgs, Fonts, Show, TEXT_COLOR, XP_COLOR};
use crate::i18n::VoxygenLocalization;
use common::comp::Stats;
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    pub struct Ids {
        charwindow,
        charwindow_gradient,
        charwindow_close,
        charwindow_exp_progress_rectangle,
        charwindow_exp_rectangle,
        charwindow_frame,
        content_align,
        charwindow_rectangle,
        charwindow_tab1,
        charwindow_tab1_exp,
        charwindow_tab1_expbar,
        charwindow_tab1_level,
        charwindow_tab1_statnames,
        charwindow_tab1_stats,
        charwindow_tab_bg,
        charwindow_title,
        window_3,
        tab_bg,
        tab_small_open,
        tab_small_closed,
        xp_charwindow,
        divider,
        head_bg,
        shoulders_bg,
        hands_bg,
        belt_bg,
        legs_bg,
        feet_bg,
        ring_r_bg,
        ring_l_bg,
        tabard_bg,
        chest_bg,
        back_bg,
        gem_bg,
        necklace_bg,
        mainhand_bg,
        offhand_bg,
        charwindow_bg,
        head_grid,
        shoulders_grid,
        hands_grid,
        belt_grid,
        legs_grid,
        feet_grid,
        ring_r_grid,
        ring_l_grid,
        tabard_grid,
        chest_grid,
        back_grid,
        gem_grid,
        necklace_grid,
        mainhand_grid,
        offhand_grid,


    }
}

#[derive(WidgetCommon)]
pub struct CharacterWindow<'a> {
    _show: &'a Show,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    stats: &'a Stats,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> CharacterWindow<'a> {
    pub fn new(
        _show: &'a Show,
        stats: &'a Stats,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    ) -> Self {
        Self {
            _show,
            imgs,
            fonts,
            stats,
            localized_strings,
            common: widget::CommonBuilder::default(),
        }
    }
}

/*pub struct State {
    ids: Ids,
}*/

pub enum Event {
    Close,
}

impl<'a> Widget for CharacterWindow<'a> {
    type State = Ids;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        Ids::new(id_gen)
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;

        let exp_percentage = (self.stats.exp.current() as f64) / (self.stats.exp.maximum() as f64);
        let exp_treshold = format!("{}/{}", self.stats.exp.current(), self.stats.exp.maximum());
        let level = (self.stats.level.level()).to_string();

        // Frame
        Image::new(self.imgs.window_3)
            .middle_of(id)
            .top_left_with_margins_on(ui.window, 200.0, 215.0)
            .w_h(103.0 * 4.0, 122.0 * 4.0)
            .set(state.charwindow_frame, ui);

        // Icon
        //Image::new(self.imgs.charwindow_icon)
        //.w_h(40.0, 40.0)
        //.top_left_with_margins_on(state.charwindow_frame, 4.0, 4.0)
        //.set(state.charwindow_icon, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.charwindow_frame, 0.0, 0.0)
            .set(state.charwindow_close, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        }

        // Title
        // TODO: Use an actual character name.
        Text::new(
            &self
                .localized_strings
                .get("character_window.character_name"),
        )
        .mid_top_with_margin_on(state.charwindow_frame, 6.0)
        .font_id(self.fonts.cyri)
        .font_size(14)
        .color(TEXT_COLOR)
        .set(state.charwindow_title, ui);

        // Content Alignment
        Rectangle::fill_with([95.0 * 4.0, 108.0 * 4.0], color::TRANSPARENT)
            .mid_top_with_margin_on(state.charwindow_frame, 40.0)
            .set(state.content_align, ui);

        // Gradient BG
        Image::new(self.imgs.charwindow_gradient)
            .w_h(95.0 * 4.0, 108.0 * 4.0)
            .middle_of(state.content_align)
            .set(state.charwindow_gradient, ui);

        // Contents

        // Head
        Image::new(self.imgs.head_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .mid_top_with_margin_on(state.content_align, 5.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.head_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.head_bg)
            .set(state.head_grid, ui);

        // Ring R
        Image::new(self.imgs.ring_r_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .bottom_right_with_margins_on(state.content_align, 20.0, 20.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.ring_r_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.ring_r_bg)
            .set(state.ring_r_grid, ui);
        // Feet
        Image::new(self.imgs.feet_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.ring_r_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.feet_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.feet_bg)
            .set(state.feet_grid, ui);
        // Legs
        Image::new(self.imgs.legs_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.feet_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.legs_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.legs_bg)
            .set(state.legs_grid, ui);
        // Belt
        Image::new(self.imgs.belt_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.legs_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.belt_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.belt_bg)
            .set(state.belt_grid, ui);
        // Hands
        Image::new(self.imgs.hands_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.belt_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.hands_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.hands_bg)
            .set(state.hands_grid, ui);
        // Shoulders
        Image::new(self.imgs.shoulders_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.hands_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.shoulders_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.shoulders_bg)
            .set(state.shoulders_grid, ui);
        // Ring L
        Image::new(self.imgs.ring_l_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .bottom_left_with_margins_on(state.content_align, 20.0, 20.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.ring_l_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.ring_l_bg)
            .set(state.ring_l_grid, ui);
        // Tabard
        Image::new(self.imgs.tabard_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.ring_l_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.tabard_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.tabard_bg)
            .set(state.tabard_grid, ui);
        // Chest
        Image::new(self.imgs.chest_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.tabard_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.chest_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.chest_bg)
            .set(state.chest_grid, ui);
        // Back
        Image::new(self.imgs.back_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.chest_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.back_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.back_bg)
            .set(state.back_grid, ui);
        // Gem
        Image::new(self.imgs.gem_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.back_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.gem_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.gem_bg)
            .set(state.gem_grid, ui);
        // Necklace
        Image::new(self.imgs.necklace_bg)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .up_from(state.gem_bg, 10.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.necklace_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 1.8, 28.0 * 1.8)
            .middle_of(state.necklace_bg)
            .set(state.necklace_grid, ui);

        // Weapon Main Hand
        Image::new(self.imgs.mainhand_bg)
            .w_h(28.0 * 2.2, 28.0 * 2.2)
            .bottom_right_with_margins_on(state.ring_l_bg, 0.0, -115.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.mainhand_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 2.2, 28.0 * 2.2)
            .middle_of(state.mainhand_bg)
            .set(state.mainhand_grid, ui);
        // Weapon Off-Hand
        Image::new(self.imgs.offhand_bg)
            .w_h(28.0 * 2.2, 28.0 * 2.2)
            .bottom_left_with_margins_on(state.ring_r_bg, 0.0, -115.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.1)))
            .set(state.offhand_bg, ui);
        Button::image(self.imgs.grid)
            .w_h(28.0 * 2.2, 28.0 * 2.2)
            .middle_of(state.offhand_bg)
            .set(state.offhand_grid, ui);

        // Stats Tab

        // Tab BG
        Image::new(self.imgs.tab_bg)
            .w_h(51.0 * 4.0, 115.0 * 4.0)
            .top_left_with_margins_on(state.charwindow_frame, 28.0, -200.0)
            .set(state.charwindow_tab_bg, ui);

        // Tab Rectangle
        Rectangle::fill_with([45.0 * 4.0, 104.0 * 4.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.charwindow_tab_bg, 7.0 * 4.0, 4.0 * 4.0)
            .set(state.charwindow_rectangle, ui);

        // TODO: Add this back in when we have multiple tabs.
        // Tab Button ->
        // Button::image(self.imgs.charwindow_tab)
        //.w_h(65.0, 23.0)
        //.top_left_with_margins_on(state.charwindow_tab_bg, -18.0, 1.8)
        //.label("Stats")
        //.label_color(TEXT_COLOR)
        //.label_font_size(14)
        //.set(state.charwindow_tab1, ui);

        // Level
        Text::new(&level)
            .mid_top_with_margin_on(state.charwindow_rectangle, 10.0)
            .font_id(self.fonts.cyri)
            .font_size(30)
            .color(TEXT_COLOR)
            .set(state.charwindow_tab1_level, ui);

        // Exp-Bar Background
        Rectangle::fill_with([170.0, 10.0], color::BLACK)
            .mid_top_with_margin_on(state.charwindow_rectangle, 50.0)
            .set(state.charwindow_exp_rectangle, ui);

        // Exp-Bar Progress
        Rectangle::fill_with([170.0 * (exp_percentage), 6.0], XP_COLOR) // 0.8 = Experience percentage
            .mid_left_with_margin_on(state.charwindow_tab1_expbar, 1.0)
            .set(state.charwindow_exp_progress_rectangle, ui);

        // Exp-Bar Foreground Frame
        Image::new(self.imgs.progress_frame)
            .w_h(170.0, 10.0)
            .middle_of(state.charwindow_exp_rectangle)
            .set(state.charwindow_tab1_expbar, ui);

        // Exp-Text
        Text::new(&exp_treshold)
            .mid_top_with_margin_on(state.charwindow_tab1_expbar, 10.0)
            .font_id(self.fonts.cyri)
            .font_size(15)
            .color(TEXT_COLOR)
            .set(state.charwindow_tab1_exp, ui);

        // Divider

        Image::new(self.imgs.divider)
            .w_h(38.0 * 4.0, 5.0 * 4.0)
            .mid_top_with_margin_on(state.charwindow_tab1_exp, 30.0)
            .set(state.divider, ui);

        // Stats
        Text::new(
            &self
                .localized_strings
                .get("character_window.character_stats"),
        )
        .top_left_with_margins_on(state.charwindow_rectangle, 140.0, 5.0)
        .font_id(self.fonts.cyri)
        .font_size(16)
        .color(TEXT_COLOR)
        .set(state.charwindow_tab1_statnames, ui);

        // TODO: Shows actual stat points.
        Text::new(&format!(
            "{}\n\
        \n\
        {}\n\
        \n\
        {}",
            self.stats.endurance, self.stats.fitness, self.stats.willpower
        ))
        .top_right_with_margins_on(state.charwindow_rectangle, 140.0, 5.0)
        .font_id(self.fonts.cyri)
        .font_size(16)
        .color(TEXT_COLOR)
        .set(state.charwindow_tab1_stats, ui);

        None
    }
}
