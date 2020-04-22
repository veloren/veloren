// Using reference impl: https://github.com/hecrj/iced/blob/e1438774af809c2951c4c7446638500446c81111/winit/src/conversion.rs
use iced::{
    keyboard::{self, KeyCode, ModifiersState},
    mouse, window, Event,
};

/// Converts a winit event into an iced event.
pub fn window_event(event: winit::WindowEvent) -> Option<Event> {
    use winit::WindowEvent;

    match event {
        WindowEvent::Resized(new_size) => {
            let logical_size: winit::dpi::LogicalSize = new_size;

            Some(Event::Window(window::Event::Resized {
                width: logical_size.width as u32,
                height: logical_size.height as u32,
            }))
        },
        WindowEvent::CursorMoved { position, .. } => {
            let position: winit::dpi::LogicalPosition = position;

            Some(Event::Mouse(mouse::Event::CursorMoved {
                x: position.x as f32,
                y: position.y as f32,
            }))
        },
        WindowEvent::MouseInput { button, state, .. } => {
            let button = mouse_button(button);

            Some(Event::Mouse(match state {
                winit::ElementState::Pressed => mouse::Event::ButtonPressed(button),
                winit::ElementState::Released => mouse::Event::ButtonReleased(button),
            }))
        },
        WindowEvent::MouseWheel { delta, .. } => match delta {
            winit::MouseScrollDelta::LineDelta(delta_x, delta_y) => {
                Some(Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines {
                        x: delta_x,
                        y: delta_y,
                    },
                }))
            },
            winit::MouseScrollDelta::PixelDelta(position) => {
                let position: winit::dpi::LogicalPosition = position;
                Some(Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels {
                        x: position.x as f32,
                        y: position.y as f32,
                    },
                }))
            },
        },
        WindowEvent::ReceivedCharacter(c) => {
            Some(Event::Keyboard(keyboard::Event::CharacterReceived(c)))
        },
        WindowEvent::KeyboardInput {
            input:
                winit::KeyboardInput {
                    virtual_keycode: Some(virtual_keycode),
                    state,
                    modifiers,
                    ..
                },
            ..
        } => Some(Event::Keyboard({
            let key_code = key_code(virtual_keycode);
            let modifiers = modifiers_state(modifiers);

            match state {
                winit::ElementState::Pressed => keyboard::Event::KeyPressed {
                    key_code,
                    modifiers,
                },
                winit::ElementState::Released => keyboard::Event::KeyReleased {
                    key_code,
                    modifiers,
                },
            }
        })),
        // iced also can use file hovering events but we don't need them right now
        _ => None,
    }
}

// iced has a function for converting mouse cursors here

/// Converts winit mouse button to iced mouse button
fn mouse_button(mouse_button: winit::MouseButton) -> mouse::Button {
    match mouse_button {
        winit::MouseButton::Left => mouse::Button::Left,
        winit::MouseButton::Right => mouse::Button::Right,
        winit::MouseButton::Middle => mouse::Button::Middle,
        winit::MouseButton::Other(other) => mouse::Button::Other(other),
    }
}

/// Converts winit `ModifiersState` to iced `ModifiersState`
fn modifiers_state(modifiers: winit::ModifiersState) -> ModifiersState {
    ModifiersState {
        shift: modifiers.shift,
        control: modifiers.ctrl,
        alt: modifiers.alt,
        logo: modifiers.logo,
    }
}

/// Converts winit `VirtualKeyCode` to iced `KeyCode`
fn key_code(virtual_keycode: winit::VirtualKeyCode) -> KeyCode {
    match virtual_keycode {
        winit::VirtualKeyCode::Key1 => KeyCode::Key1,
        winit::VirtualKeyCode::Key2 => KeyCode::Key2,
        winit::VirtualKeyCode::Key3 => KeyCode::Key3,
        winit::VirtualKeyCode::Key4 => KeyCode::Key4,
        winit::VirtualKeyCode::Key5 => KeyCode::Key5,
        winit::VirtualKeyCode::Key6 => KeyCode::Key6,
        winit::VirtualKeyCode::Key7 => KeyCode::Key7,
        winit::VirtualKeyCode::Key8 => KeyCode::Key8,
        winit::VirtualKeyCode::Key9 => KeyCode::Key9,
        winit::VirtualKeyCode::Key0 => KeyCode::Key0,
        winit::VirtualKeyCode::A => KeyCode::A,
        winit::VirtualKeyCode::B => KeyCode::B,
        winit::VirtualKeyCode::C => KeyCode::C,
        winit::VirtualKeyCode::D => KeyCode::D,
        winit::VirtualKeyCode::E => KeyCode::E,
        winit::VirtualKeyCode::F => KeyCode::F,
        winit::VirtualKeyCode::G => KeyCode::G,
        winit::VirtualKeyCode::H => KeyCode::H,
        winit::VirtualKeyCode::I => KeyCode::I,
        winit::VirtualKeyCode::J => KeyCode::J,
        winit::VirtualKeyCode::K => KeyCode::K,
        winit::VirtualKeyCode::L => KeyCode::L,
        winit::VirtualKeyCode::M => KeyCode::M,
        winit::VirtualKeyCode::N => KeyCode::N,
        winit::VirtualKeyCode::O => KeyCode::O,
        winit::VirtualKeyCode::P => KeyCode::P,
        winit::VirtualKeyCode::Q => KeyCode::Q,
        winit::VirtualKeyCode::R => KeyCode::R,
        winit::VirtualKeyCode::S => KeyCode::S,
        winit::VirtualKeyCode::T => KeyCode::T,
        winit::VirtualKeyCode::U => KeyCode::U,
        winit::VirtualKeyCode::V => KeyCode::V,
        winit::VirtualKeyCode::W => KeyCode::W,
        winit::VirtualKeyCode::X => KeyCode::X,
        winit::VirtualKeyCode::Y => KeyCode::Y,
        winit::VirtualKeyCode::Z => KeyCode::Z,
        winit::VirtualKeyCode::Escape => KeyCode::Escape,
        winit::VirtualKeyCode::F1 => KeyCode::F1,
        winit::VirtualKeyCode::F2 => KeyCode::F2,
        winit::VirtualKeyCode::F3 => KeyCode::F3,
        winit::VirtualKeyCode::F4 => KeyCode::F4,
        winit::VirtualKeyCode::F5 => KeyCode::F5,
        winit::VirtualKeyCode::F6 => KeyCode::F6,
        winit::VirtualKeyCode::F7 => KeyCode::F7,
        winit::VirtualKeyCode::F8 => KeyCode::F8,
        winit::VirtualKeyCode::F9 => KeyCode::F9,
        winit::VirtualKeyCode::F10 => KeyCode::F10,
        winit::VirtualKeyCode::F11 => KeyCode::F11,
        winit::VirtualKeyCode::F12 => KeyCode::F12,
        winit::VirtualKeyCode::F13 => KeyCode::F13,
        winit::VirtualKeyCode::F14 => KeyCode::F14,
        winit::VirtualKeyCode::F15 => KeyCode::F15,
        winit::VirtualKeyCode::F16 => KeyCode::F16,
        winit::VirtualKeyCode::F17 => KeyCode::F17,
        winit::VirtualKeyCode::F18 => KeyCode::F18,
        winit::VirtualKeyCode::F19 => KeyCode::F19,
        winit::VirtualKeyCode::F20 => KeyCode::F20,
        winit::VirtualKeyCode::F21 => KeyCode::F21,
        winit::VirtualKeyCode::F22 => KeyCode::F22,
        winit::VirtualKeyCode::F23 => KeyCode::F23,
        winit::VirtualKeyCode::F24 => KeyCode::F24,
        winit::VirtualKeyCode::Snapshot => KeyCode::Snapshot,
        winit::VirtualKeyCode::Scroll => KeyCode::Scroll,
        winit::VirtualKeyCode::Pause => KeyCode::Pause,
        winit::VirtualKeyCode::Insert => KeyCode::Insert,
        winit::VirtualKeyCode::Home => KeyCode::Home,
        winit::VirtualKeyCode::Delete => KeyCode::Delete,
        winit::VirtualKeyCode::End => KeyCode::End,
        winit::VirtualKeyCode::PageDown => KeyCode::PageDown,
        winit::VirtualKeyCode::PageUp => KeyCode::PageUp,
        winit::VirtualKeyCode::Left => KeyCode::Left,
        winit::VirtualKeyCode::Up => KeyCode::Up,
        winit::VirtualKeyCode::Right => KeyCode::Right,
        winit::VirtualKeyCode::Down => KeyCode::Down,
        winit::VirtualKeyCode::Back => KeyCode::Backspace,
        winit::VirtualKeyCode::Return => KeyCode::Enter,
        winit::VirtualKeyCode::Space => KeyCode::Space,
        winit::VirtualKeyCode::Compose => KeyCode::Compose,
        winit::VirtualKeyCode::Caret => KeyCode::Caret,
        winit::VirtualKeyCode::Numlock => KeyCode::Numlock,
        winit::VirtualKeyCode::Numpad0 => KeyCode::Numpad0,
        winit::VirtualKeyCode::Numpad1 => KeyCode::Numpad1,
        winit::VirtualKeyCode::Numpad2 => KeyCode::Numpad2,
        winit::VirtualKeyCode::Numpad3 => KeyCode::Numpad3,
        winit::VirtualKeyCode::Numpad4 => KeyCode::Numpad4,
        winit::VirtualKeyCode::Numpad5 => KeyCode::Numpad5,
        winit::VirtualKeyCode::Numpad6 => KeyCode::Numpad6,
        winit::VirtualKeyCode::Numpad7 => KeyCode::Numpad7,
        winit::VirtualKeyCode::Numpad8 => KeyCode::Numpad8,
        winit::VirtualKeyCode::Numpad9 => KeyCode::Numpad9,
        winit::VirtualKeyCode::AbntC1 => KeyCode::AbntC1,
        winit::VirtualKeyCode::AbntC2 => KeyCode::AbntC2,
        winit::VirtualKeyCode::Add => KeyCode::Add,
        winit::VirtualKeyCode::Apostrophe => KeyCode::Apostrophe,
        winit::VirtualKeyCode::Apps => KeyCode::Apps,
        winit::VirtualKeyCode::At => KeyCode::At,
        winit::VirtualKeyCode::Ax => KeyCode::Ax,
        winit::VirtualKeyCode::Backslash => KeyCode::Backslash,
        winit::VirtualKeyCode::Calculator => KeyCode::Calculator,
        winit::VirtualKeyCode::Capital => KeyCode::Capital,
        winit::VirtualKeyCode::Colon => KeyCode::Colon,
        winit::VirtualKeyCode::Comma => KeyCode::Comma,
        winit::VirtualKeyCode::Convert => KeyCode::Convert,
        winit::VirtualKeyCode::Decimal => KeyCode::Decimal,
        winit::VirtualKeyCode::Divide => KeyCode::Divide,
        winit::VirtualKeyCode::Equals => KeyCode::Equals,
        winit::VirtualKeyCode::Grave => KeyCode::Grave,
        winit::VirtualKeyCode::Kana => KeyCode::Kana,
        winit::VirtualKeyCode::Kanji => KeyCode::Kanji,
        winit::VirtualKeyCode::LAlt => KeyCode::LAlt,
        winit::VirtualKeyCode::LBracket => KeyCode::LBracket,
        winit::VirtualKeyCode::LControl => KeyCode::LControl,
        winit::VirtualKeyCode::LShift => KeyCode::LShift,
        winit::VirtualKeyCode::LWin => KeyCode::LWin,
        winit::VirtualKeyCode::Mail => KeyCode::Mail,
        winit::VirtualKeyCode::MediaSelect => KeyCode::MediaSelect,
        winit::VirtualKeyCode::MediaStop => KeyCode::MediaStop,
        winit::VirtualKeyCode::Minus => KeyCode::Minus,
        winit::VirtualKeyCode::Multiply => KeyCode::Multiply,
        winit::VirtualKeyCode::Mute => KeyCode::Mute,
        winit::VirtualKeyCode::MyComputer => KeyCode::MyComputer,
        winit::VirtualKeyCode::NavigateForward => KeyCode::NavigateForward,
        winit::VirtualKeyCode::NavigateBackward => KeyCode::NavigateBackward,
        winit::VirtualKeyCode::NextTrack => KeyCode::NextTrack,
        winit::VirtualKeyCode::NoConvert => KeyCode::NoConvert,
        winit::VirtualKeyCode::NumpadComma => KeyCode::NumpadComma,
        winit::VirtualKeyCode::NumpadEnter => KeyCode::NumpadEnter,
        winit::VirtualKeyCode::NumpadEquals => KeyCode::NumpadEquals,
        winit::VirtualKeyCode::OEM102 => KeyCode::OEM102,
        winit::VirtualKeyCode::Period => KeyCode::Period,
        winit::VirtualKeyCode::PlayPause => KeyCode::PlayPause,
        winit::VirtualKeyCode::Power => KeyCode::Power,
        winit::VirtualKeyCode::PrevTrack => KeyCode::PrevTrack,
        winit::VirtualKeyCode::RAlt => KeyCode::RAlt,
        winit::VirtualKeyCode::RBracket => KeyCode::RBracket,
        winit::VirtualKeyCode::RControl => KeyCode::RControl,
        winit::VirtualKeyCode::RShift => KeyCode::RShift,
        winit::VirtualKeyCode::RWin => KeyCode::RWin,
        winit::VirtualKeyCode::Semicolon => KeyCode::Semicolon,
        winit::VirtualKeyCode::Slash => KeyCode::Slash,
        winit::VirtualKeyCode::Sleep => KeyCode::Sleep,
        winit::VirtualKeyCode::Stop => KeyCode::Stop,
        winit::VirtualKeyCode::Subtract => KeyCode::Subtract,
        winit::VirtualKeyCode::Sysrq => KeyCode::Sysrq,
        winit::VirtualKeyCode::Tab => KeyCode::Tab,
        winit::VirtualKeyCode::Underline => KeyCode::Underline,
        winit::VirtualKeyCode::Unlabeled => KeyCode::Unlabeled,
        winit::VirtualKeyCode::VolumeDown => KeyCode::VolumeDown,
        winit::VirtualKeyCode::VolumeUp => KeyCode::VolumeUp,
        winit::VirtualKeyCode::Wake => KeyCode::Wake,
        winit::VirtualKeyCode::WebBack => KeyCode::WebBack,
        winit::VirtualKeyCode::WebFavorites => KeyCode::WebFavorites,
        winit::VirtualKeyCode::WebForward => KeyCode::WebForward,
        winit::VirtualKeyCode::WebHome => KeyCode::WebHome,
        winit::VirtualKeyCode::WebRefresh => KeyCode::WebRefresh,
        winit::VirtualKeyCode::WebSearch => KeyCode::WebSearch,
        winit::VirtualKeyCode::WebStop => KeyCode::WebStop,
        winit::VirtualKeyCode::Yen => KeyCode::Yen,
        winit::VirtualKeyCode::Copy => KeyCode::Copy,
        winit::VirtualKeyCode::Paste => KeyCode::Paste,
        winit::VirtualKeyCode::Cut => KeyCode::Cut,
    }
}
