use crate::ui::ice as ui;
use iced::{Container, Element, Text};
use ui::{
    style,
    widget::{Tooltip, TooltipManager},
};

// :( all tooltips have to copy because this is needed outside the function
#[derive(Copy, Clone)]
pub struct Style {
    pub container: style::container::Style,
    pub text_color: iced::Color,
    pub text_size: u16,
    pub padding: u16,
}

/// Tooltip that is just text
pub fn text<'a, M: 'a>(text: &str, style: Style) -> Element<'a, M, ui::IcedRenderer> {
    Container::new(
        Text::new(text)
            .color(style.text_color)
            .size(style.text_size),
    )
    .style(style.container)
    .padding(style.padding)
    .into()
}

pub trait WithTooltip<'a, M, R: ui::widget::tooltip::Renderer> {
    fn with_tooltip<H>(self, manager: &'a TooltipManager, hover_content: H) -> Tooltip<'a, M, R>
    where
        H: 'a + FnMut() -> Element<'a, M, R>;
}

impl<'a, M, R: ui::widget::tooltip::Renderer, E: Into<Element<'a, M, R>>> WithTooltip<'a, M, R>
    for E
{
    fn with_tooltip<H>(self, manager: &'a TooltipManager, hover_content: H) -> Tooltip<'a, M, R>
    where
        H: 'a + FnMut() -> Element<'a, M, R>,
    {
        Tooltip::new(self, hover_content, manager)
    }
}
