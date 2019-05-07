use super::{font_ids::Fonts, img_ids::Imgs, TEXT_COLOR, XP_COLOR};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    pub struct Ids {
        charwindow,
        charwindow_bg,
        charwindow_close,
        charwindow_exp_progress_rectangle,
        charwindow_exp_rectangle,
        charwindow_frame,
        charwindow_icon,
        charwindow_rectangle,
        charwindow_tab1,
        charwindow_tab1_exp,
        charwindow_tab1_expbar,
        charwindow_tab1_level,
        charwindow_tab1_statnames,
        charwindow_tab1_stats,
        charwindow_tab_bg,
        charwindow_title,
    }
}

#[derive(WidgetCommon)]
pub struct CharacterWindow<'a> {
    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> CharacterWindow<'a> {
    pub fn new(imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

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

        // TODO: Read from parameter / character struct
        let xp_percentage = 0.4;

        // Frame
        Image::new(self.imgs.window_frame)
            .middle_of(id)
            .top_left_with_margins_on(ui.window, 200.0, 215.0)
            .w_h(107.0 * 4.0, 125.0 * 4.0)
            .set(state.charwindow_frame, ui);

        // Icon
        //Image::new(self.imgs.charwindow_icon)
        //.w_h(224.0 / 3.0, 224.0 / 3.0)
        //.top_left_with_margins_on(state.charwindow_frame, -10.0, -10.0)
        //.set(state.charwindow_icon, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.charwindow_frame, 12.0, 0.0)
            .set(state.charwindow_close, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        }

        // Title
        Text::new("Character Name") // Add in actual Character Name
            .mid_top_with_margin_on(state.charwindow_frame, 17.0)
            .font_id(self.fonts.metamorph)
            .font_size(14)
            .color(TEXT_COLOR)
            .set(state.charwindow_title, ui);

        // Tab BG
        Image::new(self.imgs.charwindow_tab_bg)
            .w_h(205.0, 412.0)
            .mid_left_with_margin_on(state.charwindow_frame, -205.0)
            .set(state.charwindow_tab_bg, ui);

        // Tab Rectangle
        Rectangle::fill_with([192.0, 371.0], color::rgba(0.0, 0.0, 0.0, 0.8))
            .top_right_with_margins_on(state.charwindow_tab_bg, 20.0, 0.0)
            .set(state.charwindow_rectangle, ui);

        // Tab Button
        Button::image(self.imgs.charwindow_tab)
            .w_h(65.0, 23.0)
            .top_left_with_margins_on(state.charwindow_tab_bg, -18.0, 2.0)
            .label("Stats")
            .label_color(TEXT_COLOR)
            .label_font_size(14)
            .set(state.charwindow_tab1, ui);

        Text::new("1") //Add in actual Character Level
            .mid_top_with_margin_on(state.charwindow_rectangle, 10.0)
            .font_id(self.fonts.opensans)
            .font_size(30)
            .color(TEXT_COLOR)
            .set(state.charwindow_tab1_level, ui);

        // Exp-Bar Background
        Rectangle::fill_with([170.0, 10.0], color::BLACK)
            .mid_top_with_margin_on(state.charwindow_rectangle, 50.0)
            .set(state.charwindow_exp_rectangle, ui);

        // Exp-Bar Progress
        Rectangle::fill_with([170.0 * (xp_percentage), 6.0], XP_COLOR) // 0.8 = Experience percentage
            .mid_left_with_margin_on(state.charwindow_tab1_expbar, 1.0)
            .set(state.charwindow_exp_progress_rectangle, ui);

        // Exp-Bar Foreground Frame
        Image::new(self.imgs.progress_frame)
            .w_h(170.0, 10.0)
            .middle_of(state.charwindow_exp_rectangle)
            .set(state.charwindow_tab1_expbar, ui);

        // Exp-Text
        Text::new("120/170") // Shows the Exp / Exp to reach the next level
            .mid_top_with_margin_on(state.charwindow_tab1_expbar, 10.0)
            .font_id(self.fonts.opensans)
            .font_size(15)
            .color(TEXT_COLOR)
            .set(state.charwindow_tab1_exp, ui);

        // Stats
        Text::new(
            "Stamina\n\
             \n\
             Strength\n\
             \n\
             Dexterity\n\
             \n\
             Intelligence",
        )
        .top_left_with_margins_on(state.charwindow_rectangle, 100.0, 20.0)
        .font_id(self.fonts.opensans)
        .font_size(16)
        .color(TEXT_COLOR)
        .set(state.charwindow_tab1_statnames, ui);

        Text::new(
            "1234\n\
             \n\
             12312\n\
             \n\
             12414\n\
             \n\
             124124",
        )
        .right_from(state.charwindow_tab1_statnames, 10.0)
        .font_id(self.fonts.opensans)
        .font_size(16)
        .color(TEXT_COLOR)
        .set(state.charwindow_tab1_stats, ui);

        None
    }
}
