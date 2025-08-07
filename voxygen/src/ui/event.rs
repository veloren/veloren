use conrod_core::{
    event::Input,
    input::{self, Button},
};
use vek::*;
use winit::event::{self, WindowEvent};

// All `conrod_convert_*` functions were copied from `conrod_winit` (MIT
// licensed).
// Original version found at https://gitlab.com/veloren/conrod/-/blob/copypasta_0.7/backends/conrod_winit/src/macros.rs

fn conrod_convert_key(key: &winit::keyboard::Key) -> input::Key {
    match key {
        winit::keyboard::Key::Named(key) => match key {
            winit::keyboard::NamedKey::Escape => input::keyboard::Key::Escape,
            winit::keyboard::NamedKey::F1 => input::keyboard::Key::F1,
            winit::keyboard::NamedKey::F2 => input::keyboard::Key::F2,
            winit::keyboard::NamedKey::F3 => input::keyboard::Key::F3,
            winit::keyboard::NamedKey::F4 => input::keyboard::Key::F4,
            winit::keyboard::NamedKey::F5 => input::keyboard::Key::F5,
            winit::keyboard::NamedKey::F6 => input::keyboard::Key::F6,
            winit::keyboard::NamedKey::F7 => input::keyboard::Key::F7,
            winit::keyboard::NamedKey::F8 => input::keyboard::Key::F8,
            winit::keyboard::NamedKey::F9 => input::keyboard::Key::F9,
            winit::keyboard::NamedKey::F10 => input::keyboard::Key::F10,
            winit::keyboard::NamedKey::F11 => input::keyboard::Key::F11,
            winit::keyboard::NamedKey::F12 => input::keyboard::Key::F12,
            winit::keyboard::NamedKey::F13 => input::keyboard::Key::F13,
            winit::keyboard::NamedKey::F14 => input::keyboard::Key::F14,
            winit::keyboard::NamedKey::F15 => input::keyboard::Key::F15,
            winit::keyboard::NamedKey::F16 => input::keyboard::Key::F16,
            winit::keyboard::NamedKey::F17 => input::keyboard::Key::F17,
            winit::keyboard::NamedKey::F18 => input::keyboard::Key::F18,
            winit::keyboard::NamedKey::F19 => input::keyboard::Key::F19,
            winit::keyboard::NamedKey::F20 => input::keyboard::Key::F20,
            winit::keyboard::NamedKey::F21 => input::keyboard::Key::F21,
            winit::keyboard::NamedKey::F22 => input::keyboard::Key::F22,
            winit::keyboard::NamedKey::F23 => input::keyboard::Key::F23,
            winit::keyboard::NamedKey::F24 => input::keyboard::Key::F24,
            winit::keyboard::NamedKey::ScrollLock => input::keyboard::Key::ScrollLock,
            winit::keyboard::NamedKey::Pause => input::keyboard::Key::Pause,
            winit::keyboard::NamedKey::Insert => input::keyboard::Key::Insert,
            winit::keyboard::NamedKey::Home => input::keyboard::Key::Home,
            winit::keyboard::NamedKey::Delete => input::keyboard::Key::Delete,
            winit::keyboard::NamedKey::End => input::keyboard::Key::End,
            winit::keyboard::NamedKey::PageDown => input::keyboard::Key::PageDown,
            winit::keyboard::NamedKey::PageUp => input::keyboard::Key::PageUp,
            winit::keyboard::NamedKey::ArrowLeft => input::keyboard::Key::Left,
            winit::keyboard::NamedKey::ArrowUp => input::keyboard::Key::Up,
            winit::keyboard::NamedKey::ArrowRight => input::keyboard::Key::Right,
            winit::keyboard::NamedKey::ArrowDown => input::keyboard::Key::Down,
            winit::keyboard::NamedKey::Backspace => input::keyboard::Key::Backspace,
            winit::keyboard::NamedKey::Enter => input::keyboard::Key::Return,
            winit::keyboard::NamedKey::Space => input::keyboard::Key::Space,
            winit::keyboard::NamedKey::AudioVolumeMute => input::keyboard::Key::AudioMute,
            winit::keyboard::NamedKey::MediaTrackNext => input::keyboard::Key::AudioNext,
            winit::keyboard::NamedKey::Power => input::keyboard::Key::Power,
            winit::keyboard::NamedKey::MediaTrackPrevious => input::keyboard::Key::AudioPrev,
            winit::keyboard::NamedKey::Tab => input::keyboard::Key::Tab,
            winit::keyboard::NamedKey::AudioVolumeDown => input::keyboard::Key::VolumeDown,
            winit::keyboard::NamedKey::AudioVolumeUp => input::keyboard::Key::VolumeUp,
            winit::keyboard::NamedKey::Copy => input::keyboard::Key::Copy,
            winit::keyboard::NamedKey::Paste => input::keyboard::Key::Paste,
            winit::keyboard::NamedKey::Cut => input::keyboard::Key::Cut,
            _ => input::keyboard::Key::Unknown,
        },
        winit::keyboard::Key::Character(c) => match c.as_str() {
            "a" | "A" => input::keyboard::Key::A,
            "b" | "B" => input::keyboard::Key::B,
            "c" | "C" => input::keyboard::Key::C,
            "d" | "D" => input::keyboard::Key::D,
            "e" | "E" => input::keyboard::Key::E,
            "f" | "F" => input::keyboard::Key::F,
            "g" | "G" => input::keyboard::Key::G,
            "h" | "H" => input::keyboard::Key::H,
            "i" | "I" => input::keyboard::Key::I,
            "j" | "J" => input::keyboard::Key::J,
            "k" | "K" => input::keyboard::Key::K,
            "l" | "L" => input::keyboard::Key::L,
            "m" | "M" => input::keyboard::Key::M,
            "n" | "N" => input::keyboard::Key::N,
            "o" | "O" => input::keyboard::Key::O,
            "p" | "P" => input::keyboard::Key::P,
            "q" | "Q" => input::keyboard::Key::Q,
            "r" | "R" => input::keyboard::Key::R,
            "s" | "S" => input::keyboard::Key::S,
            "t" | "T" => input::keyboard::Key::T,
            "u" | "U" => input::keyboard::Key::U,
            "v" | "V" => input::keyboard::Key::V,
            "w" | "W" => input::keyboard::Key::W,
            "x" | "X" => input::keyboard::Key::X,
            "y" | "Y" => input::keyboard::Key::Y,
            "z" | "Z" => input::keyboard::Key::Z,
            "*" => input::keyboard::Key::Asterisk,
            "\\" => input::keyboard::Key::Backslash,
            "^" => input::keyboard::Key::Caret,
            ":" => input::keyboard::Key::Colon,
            "," => input::keyboard::Key::Comma,
            "=" => input::keyboard::Key::Equals,
            "-" => input::keyboard::Key::Minus,
            "+" => input::keyboard::Key::Plus,
            ";" => input::keyboard::Key::Semicolon,
            "/" => input::keyboard::Key::Slash,
            " " => input::keyboard::Key::Space,
            "_" => input::keyboard::Key::Underscore,
            _ => input::keyboard::Key::Unknown,
        },
        winit::keyboard::Key::Unidentified(_) | winit::keyboard::Key::Dead(_) => {
            input::keyboard::Key::Unknown
        },
    }
}

fn conrod_convert_mouse_button(button: &event::MouseButton) -> input::Button {
    input::Button::Mouse(match button {
        event::MouseButton::Left => input::MouseButton::Left,
        event::MouseButton::Right => input::MouseButton::Right,
        event::MouseButton::Middle => input::MouseButton::Middle,
        event::MouseButton::Other(0) => input::MouseButton::X1,
        event::MouseButton::Other(1) => input::MouseButton::X2,
        event::MouseButton::Other(2) => input::MouseButton::Button6,
        event::MouseButton::Other(3) => input::MouseButton::Button7,
        event::MouseButton::Other(4) => input::MouseButton::Button8,
        _ => input::MouseButton::Unknown,
    })
}

fn conrod_convert_event(event: &WindowEvent, window: &winit::window::Window) -> Option<Input> {
    let hidpi = window.scale_factor();
    let winit::dpi::LogicalSize { width, height } = window.inner_size().to_logical::<f64>(hidpi);
    let tx = |x: f64| x - width / 2.0;
    let ty = |y: f64| -(y - height / 2.0);

    Some(match event {
        WindowEvent::Resized(physical_size) => {
            let winit::dpi::LogicalSize { width, height } = physical_size.to_logical::<f64>(hidpi);
            Input::Resize(width as _, height as _)
        },
        WindowEvent::Focused(focused) => Input::Focus(*focused),
        WindowEvent::KeyboardInput { event, .. } => {
            let key = input::Button::Keyboard(conrod_convert_key(&event.logical_key));

            match event.state {
                event::ElementState::Pressed => Input::Press(key),
                event::ElementState::Released => Input::Release(key),
            }
        },
        WindowEvent::Touch(event::Touch {
            phase,
            location,
            id,
            ..
        }) => {
            let winit::dpi::LogicalPosition { x, y } = location.to_logical::<f64>(hidpi);
            let phase = match phase {
                event::TouchPhase::Started => input::touch::Phase::Start,
                event::TouchPhase::Moved => input::touch::Phase::Move,
                event::TouchPhase::Cancelled => input::touch::Phase::Cancel,
                event::TouchPhase::Ended => input::touch::Phase::End,
            };
            let xy = [tx(x), ty(y)];
            let id = input::touch::Id::new(*id);
            let touch = input::Touch { phase, id, xy };
            Input::Touch(touch)
        },
        WindowEvent::CursorMoved { position, .. } => {
            let winit::dpi::LogicalPosition { x, y } = position.to_logical::<f64>(hidpi);
            let x = tx(x);
            let y = ty(y);
            let motion = input::Motion::MouseCursor { x, y };
            Input::Motion(motion)
        },
        WindowEvent::MouseWheel { delta, .. } => match delta {
            event::MouseScrollDelta::PixelDelta(physical_position) => {
                let winit::dpi::LogicalPosition { x, y } =
                    physical_position.to_logical::<f64>(hidpi);
                let x = x as conrod_core::Scalar;
                let y = -y as conrod_core::Scalar;
                let motion = input::Motion::Scroll { x, y };
                Input::Motion(motion)
            },
            event::MouseScrollDelta::LineDelta(x, y) => {
                const ARBITRARY_POINTS_PER_LINE_FACTOR: conrod_core::Scalar = 10.0;
                let x = ARBITRARY_POINTS_PER_LINE_FACTOR * *x as conrod_core::Scalar;
                let y = ARBITRARY_POINTS_PER_LINE_FACTOR * -y as conrod_core::Scalar;
                Input::Motion(input::Motion::Scroll { x, y })
            },
        },
        WindowEvent::MouseInput { state, button, .. } => {
            let button = conrod_convert_mouse_button(button);
            match state {
                event::ElementState::Pressed => Input::Press(button),
                event::ElementState::Released => Input::Release(button),
            }
        },
        WindowEvent::RedrawRequested => Input::Redraw,
        _ => return None,
    })
}

#[derive(Clone, Debug)]
pub struct Event(pub Input);

impl Event {
    pub fn try_from(event: &event::Event<()>, window: &winit::window::Window) -> Option<Self> {
        match event {
            event::Event::WindowEvent { event, .. } => {
                conrod_convert_event(event, window).map(Self)
            },
            _ => None,
        }
    }

    pub fn is_keyboard_or_mouse(&self) -> bool {
        matches!(
            self.0,
            Input::Press(_)
                | Input::Release(_)
                | Input::Motion(_)
                | Input::Touch(_)
                | Input::Text(_)
        )
    }

    pub fn is_keyboard(&self) -> bool {
        matches!(
            self.0,
            Input::Press(Button::Keyboard(_))
                | Input::Release(Button::Keyboard(_))
                | Input::Text(_)
        )
    }

    pub fn new_resize(dims: Vec2<f64>) -> Self { Self(Input::Resize(dims.x, dims.y)) }
}
