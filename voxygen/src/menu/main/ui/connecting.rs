use super::{IcedImgs as Imgs, Message};
use crate::{
    i18n::Localization,
    ui::ice::{
        component::neat_button,
        widget::{image, BackgroundContainer, Image},
        Element,
    },
};
use iced::{button, Length, Space};

/// Connecting screen for the main menu
pub struct Screen {
    cancel_button: button::State,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            cancel_button: Default::default(),
        }
    }

    pub(super) fn view(
        &mut self,
        imgs: &Imgs,
        bg_img: image::Handle,
        start: &std::time::Instant,
        i18n: &Localization,
    ) -> Element<Message> {
        let content = Space::new(Length::Fill, Length::Fill);
        // Note: could replace this with styling on iced's container since we aren't
        // using fixed aspect ratio
        BackgroundContainer::new(Image::new(bg_img), content).into()
    }
}
