use conrod_core::{event::Input, input::Button};
use vek::*;

#[derive(Clone)]
pub struct Event(pub Input);
impl Event {
    pub fn try_from(
        event: &winit::event::Event<()>,
        window: &glutin::ContextWrapper<glutin::PossiblyCurrent, winit::window::Window>,
    ) -> Option<Self> {
        use conrod_winit::*;
        // A wrapper around the winit window that allows us to implement the trait
        // necessary for enabling the winit <-> conrod conversion functions.
        struct WindowRef<'a>(&'a winit::window::Window);

        // Implement the `WinitWindow` trait for `WindowRef` to allow for generating
        // compatible conversion functions.
        impl<'a> conrod_winit::WinitWindow for WindowRef<'a> {
            fn get_inner_size(&self) -> Option<(u32, u32)> {
                Some(
                    winit::window::Window::inner_size(&self.0)
                        .to_logical::<u32>(self.hidpi_factor())
                        .into(),
                )
            }

            fn hidpi_factor(&self) -> f64 { winit::window::Window::scale_factor(&self.0) }
        }
        convert_event!(event, &WindowRef(window.window())).map(Self)
    }

    pub fn is_keyboard_or_mouse(&self) -> bool {
        match self.0 {
            Input::Press(_)
            | Input::Release(_)
            | Input::Motion(_)
            | Input::Touch(_)
            | Input::Text(_) => true,
            _ => false,
        }
    }

    pub fn is_keyboard(&self) -> bool {
        match self.0 {
            Input::Press(Button::Keyboard(_))
            | Input::Release(Button::Keyboard(_))
            | Input::Text(_) => true,
            _ => false,
        }
    }

    pub fn new_resize(dims: Vec2<f64>) -> Self { Self(Input::Resize(dims.x, dims.y)) }
}
