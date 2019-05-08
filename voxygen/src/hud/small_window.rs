use super::{img_ids::Imgs, Fonts, Windows, TEXT_COLOR};
use crate::hud::Show;
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
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
    show: &'a Show,

    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> SmallWindow<'a> {
    pub fn new(content: SmallWindowType, show: &'a Show, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            content,
            show,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
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
        let widget::UpdateArgs { state, ui, .. } = args;

        let (title, icon) = match self.content {
            SmallWindowType::Social => ("Social", self.imgs.social_icon),
            SmallWindowType::Spellbook => ("Spellbook", self.imgs.spellbook_icon),
            SmallWindowType::QuestLog => ("QuestLog", self.imgs.questlog_icon),
        };

        // Frame
        // TODO: Relative to Char Window?
        if let Windows::CharacterAnd(_) = self.show.open_windows {
            Image::new(self.imgs.window_frame)
                .top_left_with_margins_on(ui.window, 200.0, 658.0)
                .w_h(107.0 * 4.0, 125.0 * 4.0)
                .set(state.ids.frame, ui);
        } else {
            Image::new(self.imgs.window_frame)
                .top_left_with_margins_on(ui.window, 200.0, 10.0)
                .w_h(107.0 * 4.0, 125.0 * 4.0)
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
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.frame, 12.0, 0.0)
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
