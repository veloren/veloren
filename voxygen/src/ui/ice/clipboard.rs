// Taken from https://github.com/hecrj/iced/blob/e1438774af809c2951c4c7446638500446c81111/winit/src/clipboard.rs
pub struct Clipboard(window_clipboard::Clipboard);

impl Clipboard {
    pub fn new(window: &winit::Window) -> Option<Clipboard> {
        window_clipboard::Clipboard::new(window).map(Clipboard).ok()
    }
}

impl iced::Clipboard for Clipboard {
    fn content(&self) -> Option<String> { self.0.read().ok() }
}
