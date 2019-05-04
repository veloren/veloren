use conrod_core::{
    builder_methods, color,
    text::font,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

use super::{
    img_ids::Imgs,
    font_ids::Fonts,
    TEXT_COLOR,
};

widget_ids! {
    struct Ids {
        frame,
        bg,
        title,
        icon,
        close,
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SmallWindowType {
    Spellbook,
    Social,
    QuestLog,
}

#[derive(WidgetCommon)]
pub struct SmallWindow<'a> {
    content: SmallWindowType,

    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    style: (),
}

impl<'a> SmallWindow<'a> {
    pub fn new(content: SmallWindowType, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            content,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            style: (),
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    Close,
}

impl<'a> Widget for SmallWindow<'a> {
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
        let widget::UpdateArgs {
            id,
            state,
            ui,
            style,
            ..
        } = args;

        let (title, icon) = match self.content {
            SmallWindowType::Social => ("Social", self.imgs.social_icon),
            SmallWindowType::Spellbook => ("Spellbook", self.imgs.spellbook_icon),
            SmallWindowType::QuestLog => ("QuestLog", self.imgs.questlog_icon),
        };

        // Frame
        // TODO: Relative to Char Window?
        if true { //char_window_open {
            Image::new(self.imgs.window_frame)
                // TODO: Position
                // .right_from(state.ids.character_window, 20.0)
                .w_h(107.0*4.0, 125.0*4.0)
                .set(state.ids.frame, ui);
        } else {
            Image::new(self.imgs.window_frame)
                .top_left_with_margins_on(ui.window, 200.0, 10.0)
                .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                .set(state.ids.frame, ui);
        }

        // Icon
        Image::new(icon)
            .w_h(40.0, 40.0)
            .top_left_with_margins_on(state.ids.frame, 4.0, 4.0)
            .set(state.ids.icon, ui);

        // Content alignment
        Rectangle::fill_with([362.0, 418.0], color::TRANSPARENT)
            .bottom_right_with_margins_on(state.ids.frame, 17.0, 17.0)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.bg, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(20.0, 20.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.frame, 17.0, 5.0)
            .set(state.ids.close, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        }
        // Title
        Text::new(title)
            .mid_top_with_margin_on(state.ids.frame, 16.0)
            .font_id(self.fonts.metamorph)
            .font_size(14)
            .color(TEXT_COLOR)
            .set(state.ids.title, ui);

        match self.content {
            SmallWindowType::Social => {}
            SmallWindowType::Spellbook => {}
            SmallWindowType::QuestLog => {}
        }

        None
    }
}

