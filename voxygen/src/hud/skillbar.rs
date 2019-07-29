use super::{img_ids::Imgs, Fonts, HP_COLOR, MANA_COLOR, TEXT_COLOR, XP_COLOR};
use common::comp::Stats;
use conrod_core::{
    widget::{self, Image, Rectangle, Text},
    widget_ids, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        health_bar,
        health_bar_color,
        health_tooltip,
        l_click,
        level_text,
        mana_bar,
        mana_bar_color,
        next_level_text,
        r_click,
        sb_grid_bg_l,
        sb_grid_bg_r,
        sb_grid_l,
        sb_grid_r,
        test,
        xp_bar,
        xp_bar_progress,
        crosshair,
    }
}

#[derive(WidgetCommon)]
pub struct Skillbar<'a> {
    imgs: &'a Imgs,
    _fonts: &'a Fonts,
    stats: &'a Stats,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Skillbar<'a> {
    pub fn new(imgs: &'a Imgs, fonts: &'a Fonts, stats: &'a Stats) -> Self {
        Self {
            imgs,
            _fonts: fonts,
            stats,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {}

impl<'a> Widget for Skillbar<'a> {
    type State = State;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let level = (self.stats.level.get_level()).to_string();
        let next_level = (self.stats.level.get_level() + 1).to_string();

        let exp_percentage = self.stats.exp.get_current() / self.stats.exp.get_maximum();

        let hp_percentage =
            self.stats.health.get_current() as f64 / self.stats.health.get_maximum() as f64;
        let mana_percentage = 1.0;

        // TODO: Only show while aiming with a bow or when casting a spell.
        // Crosshair
        // Image::new(self.imgs.crosshair)
        // .w_h(101.0 * 0.5, 101.0 * 0.5)
        // .mid_top_with_margin_on(ui.window, 500.0)
        // .set(state.ids.crosshair, ui);

        // Experience-Bar
        Image::new(self.imgs.xp_bar)
            .w_h(672.0 / 1.5, 29.0 / 1.5)
            .mid_bottom_of(ui.window)
            .set(state.ids.xp_bar, ui);

        Rectangle::fill_with([406.0 * (exp_percentage), 5.0], XP_COLOR) // "W=406*[Exp. %]"
            .top_left_with_margins_on(state.ids.xp_bar, 5.0, 21.0)
            .set(state.ids.xp_bar_progress, ui);

        // Left Grid
        Image::new(self.imgs.sb_grid)
            .w_h(280.0 / 1.5, 56.0 / 1.5)
            .up_from(state.ids.xp_bar, 0.0)
            .align_left_of(state.ids.xp_bar)
            .set(state.ids.sb_grid_l, ui);

        Image::new(self.imgs.sb_grid_bg)
            .w_h(280.0 / 1.5, 56.0 / 1.5)
            .middle_of(state.ids.sb_grid_l)
            .set(state.ids.sb_grid_bg_l, ui);

        // Right Grid
        Image::new(self.imgs.sb_grid)
            .w_h(280.0 / 1.5, 56.0 / 1.5)
            .up_from(state.ids.xp_bar, 0.0)
            .align_right_of(state.ids.xp_bar)
            .set(state.ids.sb_grid_r, ui);

        Image::new(self.imgs.sb_grid_bg)
            .w_h(280.0 / 1.5, 56.0 / 1.5)
            .middle_of(state.ids.sb_grid_r)
            .set(state.ids.sb_grid_bg_r, ui);

        // Right and Left Click
        Image::new(self.imgs.l_click)
            .w_h(56.0 / 1.5, 80.0 / 1.5)
            .right_from(state.ids.sb_grid_bg_l, 0.0)
            .align_bottom_of(state.ids.sb_grid_bg_l)
            .set(state.ids.l_click, ui);

        Image::new(self.imgs.r_click)
            .w_h(56.0 / 1.5, 80.0 / 1.5)
            .left_from(state.ids.sb_grid_bg_r, 0.0)
            .align_bottom_of(state.ids.sb_grid_bg_r)
            .set(state.ids.r_click, ui);

        // Health Bar
        Image::new(self.imgs.health_bar)
            .w_h(1120.0 / 6.0, 96.0 / 6.0)
            .left_from(state.ids.l_click, 0.0)
            .align_top_of(state.ids.l_click)
            .set(state.ids.health_bar, ui);

        // Filling
        Rectangle::fill_with([182.0 * hp_percentage, 6.0], HP_COLOR) // "W=182.0 * [Health. %]"
            .top_right_with_margins_on(state.ids.health_bar, 5.0, 0.0)
            .set(state.ids.health_bar_color, ui);

        // Mana Bar
        Image::new(self.imgs.mana_bar)
            .w_h(1120.0 / 6.0, 96.0 / 6.0)
            .right_from(state.ids.r_click, 0.0)
            .align_top_of(state.ids.r_click)
            .set(state.ids.mana_bar, ui);

        // Filling
        Rectangle::fill_with([182.0 * mana_percentage, 6.0], MANA_COLOR) // "W=182.0 * [Mana. %]"
            .top_left_with_margins_on(state.ids.mana_bar, 5.0, 0.0)
            .set(state.ids.mana_bar_color, ui);

        // Buffs/Debuffs

        // Buffs

        // Debuffs

        // Level Display

        Text::new(&level)
            .left_from(state.ids.xp_bar, -15.0)
            .font_size(10)
            .color(TEXT_COLOR)
            .set(state.ids.level_text, ui);

        Text::new(&next_level)
            .right_from(state.ids.xp_bar, -15.0)
            .font_size(10)
            .color(TEXT_COLOR)
            .set(state.ids.next_level_text, ui);

        None
    }
}
