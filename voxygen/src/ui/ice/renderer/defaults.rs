// TODO: expose to user
pub struct Defaults {
    pub text_color: iced::Color,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            text_color: iced::Color::WHITE,
        }
    }
}
