use conrod_core::{
    builder_methods, color,
    text::font,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

use super::{
    imgs::Imgs,
    WindowStyle, XP_COLOR,
};

widget_ids! {
    struct Ids {
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
    xp_percentage: f64,
    imgs: &'a Imgs,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    style: WindowStyle,
}

impl<'a> CharacterWindow<'a> {
    pub fn new(imgs: &'a Imgs) -> Self {
        Self {
            xp_percentage: 0.4,
            imgs,
            common: widget::CommonBuilder::default(),
            style: WindowStyle::default(),
        }
    }
    pub fn font_id(mut self, font_id: font::Id) -> Self {
        self.style.font_id = Some(Some(font_id));
        self
    }
    builder_methods! {
        pub text_color { style.text_color = Some(Color) }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    Close,
}

impl<'a> Widget for CharacterWindow<'a> {
    type State = State;
    type Style = WindowStyle;
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            ui,
            style,
            ..
        } = args;

        let font_id = style.font_id(&ui.theme).or(ui.fonts.ids().next());
        let text_color = style.text_color(&ui.theme);

        // Frame
        Image::new(self.imgs.window_frame)
            .middle_of(id)
            .set(state.ids.charwindow_frame, ui);

        // BG
        Image::new(self.imgs.window_bg)
            .w_h(348.0, 404.0)
            .mid_top_with_margin_on(state.ids.charwindow_frame, 48.0)
            .set(state.ids.charwindow_bg, ui);

        // Overlay
        Image::new(self.imgs.charwindow)
            .middle_of(state.ids.charwindow_bg)
            .set(state.ids.charwindow, ui);

        // Icon
        //Image::new(self.imgs.charwindow_icon)
        //.w_h(224.0 / 3.0, 224.0 / 3.0)
        //.top_left_with_margins_on(state.ids.charwindow_frame, -10.0, -10.0)
        //.set(state.ids.charwindow_icon, ui);

        // X-Button
        let closed = Button::image(self.imgs.close_button)
            .w_h(244.0 * 0.22 / 4.0, 244.0 * 0.22 / 4.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.charwindow_frame, 4.0, 4.0)
            .set(state.ids.charwindow_close, ui)
            .was_clicked();

        // Title
        Text::new("Character Name") // Add in actual Character Name
            .mid_top_with_margin_on(state.ids.charwindow_frame, 7.0)
            .color(text_color)
            .set(state.ids.charwindow_title, ui);
        // Tab BG
        Image::new(self.imgs.charwindow_tab_bg)
            .w_h(205.0, 412.0)
            .mid_left_with_margin_on(state.ids.charwindow_frame, -205.0)
            .set(state.ids.charwindow_tab_bg, ui);
        // Tab Rectangle
        Rectangle::fill_with([192.0, 371.0], color::rgba(0.0, 0.0, 0.0, 0.8))
            .top_right_with_margins_on(state.ids.charwindow_tab_bg, 20.0, 0.0)
            .set(state.ids.charwindow_rectangle, ui);
        // Tab Button
        Button::image(self.imgs.charwindow_tab)
            .w_h(65.0, 23.0)
            .top_left_with_margins_on(state.ids.charwindow_tab_bg, -18.0, 2.0)
            .label("Stats")
            .label_color(text_color)
            .and_then(font_id, Button::label_font_id)
            .label_font_size(14)
            .set(state.ids.charwindow_tab1, ui);
        Text::new("1") //Add in actual Character Level
            .mid_top_with_margin_on(state.ids.charwindow_rectangle, 10.0)
            .and_then(font_id, Text::font_id)
            .font_size(30)
            .color(text_color)
            .set(state.ids.charwindow_tab1_level, ui);
        // Exp-Bar Background
        Rectangle::fill_with([170.0, 10.0], color::BLACK)
            .mid_top_with_margin_on(state.ids.charwindow_rectangle, 50.0)
            .set(state.ids.charwindow_exp_rectangle, ui);
        // Exp-Bar Progress
        Rectangle::fill_with([170.0 * (self.xp_percentage), 6.0], XP_COLOR) // 0.8 = Experience percantage
            .mid_left_with_margin_on(state.ids.charwindow_tab1_expbar, 1.0)
            .set(state.ids.charwindow_exp_progress_rectangle, ui);
        // Exp-Bar Foreground Frame
        Image::new(self.imgs.progress_frame)
            .w_h(170.0, 10.0)
            .middle_of(state.ids.charwindow_exp_rectangle)
            .set(state.ids.charwindow_tab1_expbar, ui);
        // Exp-Text
        Text::new("120/170") // Shows the Exp / Exp to reach the next level
            .mid_top_with_margin_on(state.ids.charwindow_tab1_expbar, 10.0)
            .and_then(font_id, Text::font_id)
            .font_size(15)
            .color(text_color)
            .set(state.ids.charwindow_tab1_exp, ui);

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
        .top_left_with_margins_on(state.ids.charwindow_rectangle, 100.0, 20.0)
        .and_then(font_id, Text::font_id)
        .font_size(16)
        .color(text_color)
        .set(state.ids.charwindow_tab1_statnames, ui);

        Text::new(
            "1234\n\
             \n\
             12312\n\
             \n\
             12414\n\
             \n\
             124124",
        )
        .right_from(state.ids.charwindow_tab1_statnames, 10.0)
        .and_then(font_id, Text::font_id)
        .font_size(16)
        .color(text_color)
        .set(state.ids.charwindow_tab1_stats, ui);

        if closed {
            Some(Event::Close)
        } else {
            None
        }
    }
}
