// Copied and adapted from `iced_winit` (MIT licensed)
// Original version at https://github.com/Imberflur/iced/tree/veloren-winit-0.28/winit

use iced::{Event, Point, keyboard, mouse, touch, window};
use winit::{event::WindowEvent, keyboard::NamedKey};

/// A buffer for short-term storage and transfer within and between
/// applications.
pub struct Clipboard {
    connection: Option<window_clipboard::Clipboard>,
}

impl Clipboard {
    /// Creates a new [`Clipboard`] for the given window.
    pub fn connect(window: &winit::window::Window) -> Clipboard {
        #[expect(unsafe_code)]
        let connection = unsafe { window_clipboard::Clipboard::connect(window) }.ok();

        Clipboard { connection }
    }

    /// Reads the current content of the [`Clipboard`] as text.
    pub fn read(&self) -> Option<String> { self.connection.as_ref()?.read().ok() }

    /// Writes the given text contents to the [`Clipboard`].
    pub fn write(&mut self, contents: String) {
        if let Some(clipboard) = &mut self.connection
            && let Err(error) = clipboard.write(contents)
        {
            tracing::warn!("error writing to clipboard: {}", error)
        }
    }
}

impl iced::Clipboard for Clipboard {
    fn read(&self) -> Option<String> { self.read() }

    fn write(&mut self, contents: String) { self.write(contents) }
}

/// Converts a winit window event into an iced event.
pub fn window_event(
    event: &WindowEvent,
    scale_factor: f64,
    modifiers: winit::keyboard::ModifiersState,
) -> Option<Event> {
    match event {
        WindowEvent::Resized(new_size) => {
            let logical_size = new_size.to_logical(scale_factor);

            Some(Event::Window(window::Event::Resized {
                width: logical_size.width,
                height: logical_size.height,
            }))
        },
        WindowEvent::CloseRequested => Some(Event::Window(window::Event::CloseRequested)),
        WindowEvent::CursorMoved { position, .. } => {
            let position = position.to_logical::<f64>(scale_factor);

            Some(Event::Mouse(mouse::Event::CursorMoved {
                position: Point::new(position.x as f32, position.y as f32),
            }))
        },
        WindowEvent::CursorEntered { .. } => Some(Event::Mouse(mouse::Event::CursorEntered)),
        WindowEvent::CursorLeft { .. } => Some(Event::Mouse(mouse::Event::CursorLeft)),
        WindowEvent::MouseInput { button, state, .. } => {
            let button = mouse_button(*button)?;

            Some(Event::Mouse(match state {
                winit::event::ElementState::Pressed => mouse::Event::ButtonPressed(button),
                winit::event::ElementState::Released => mouse::Event::ButtonReleased(button),
            }))
        },
        WindowEvent::MouseWheel { delta, .. } => match delta {
            winit::event::MouseScrollDelta::LineDelta(delta_x, delta_y) => {
                Some(Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines {
                        x: *delta_x,
                        y: *delta_y,
                    },
                }))
            },
            winit::event::MouseScrollDelta::PixelDelta(position) => {
                Some(Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels {
                        x: position.x as f32,
                        y: position.y as f32,
                    },
                }))
            },
        },
        WindowEvent::KeyboardInput { event, .. } => Some(Event::Keyboard({
            let modifiers = self::modifiers(modifiers);

            // `iced` expects different events for text input and pressed keys.
            // We work around that by sending the key as text but only if no modifier is
            // pressed, so shortcuts still work.
            if let Some(text) = &event.text
                && let Some(c) = text.chars().next()
                && !c.is_control()
                && !modifiers.alt
                && !modifiers.control
                && !modifiers.logo
            {
                return event
                    .state
                    .is_pressed()
                    .then_some(Event::Keyboard(keyboard::Event::CharacterReceived(c)));
            }

            let key_code = key_code(&event.logical_key)?;
            match event.state {
                winit::event::ElementState::Pressed => keyboard::Event::KeyPressed {
                    key_code,
                    modifiers,
                },
                winit::event::ElementState::Released => keyboard::Event::KeyReleased {
                    key_code,
                    modifiers,
                },
            }
        })),
        WindowEvent::ModifiersChanged(new_modifiers) => Some(Event::Keyboard(
            keyboard::Event::ModifiersChanged(self::modifiers(new_modifiers.state())),
        )),
        WindowEvent::Focused(focused) => Some(Event::Window(if *focused {
            window::Event::Focused
        } else {
            window::Event::Unfocused
        })),
        WindowEvent::HoveredFile(path) => {
            Some(Event::Window(window::Event::FileHovered(path.clone())))
        },
        WindowEvent::DroppedFile(path) => {
            Some(Event::Window(window::Event::FileDropped(path.clone())))
        },
        WindowEvent::HoveredFileCancelled => Some(Event::Window(window::Event::FilesHoveredLeft)),
        WindowEvent::Touch(touch) => Some(Event::Touch(touch_event(*touch, scale_factor))),
        _ => None,
    }
}

/// Converts a `MouseButton` from [`winit`] to an [`iced`] mouse button.
pub fn mouse_button(mouse_button: winit::event::MouseButton) -> Option<mouse::Button> {
    Some(match mouse_button {
        winit::event::MouseButton::Left => mouse::Button::Left,
        winit::event::MouseButton::Right => mouse::Button::Right,
        winit::event::MouseButton::Middle => mouse::Button::Middle,
        winit::event::MouseButton::Other(other) => mouse::Button::Other(other as u8),
        winit::event::MouseButton::Back | winit::event::MouseButton::Forward => return None,
    })
}

/// Converts some `ModifiersState` from [`winit`] to an [`iced`]
/// modifiers state.
pub fn modifiers(modifiers: winit::keyboard::ModifiersState) -> keyboard::Modifiers {
    keyboard::Modifiers {
        shift: modifiers.shift_key(),
        control: modifiers.control_key(),
        alt: modifiers.alt_key(),
        logo: modifiers.super_key(),
    }
}

/// Converts a `Touch` from [`winit`] to an [`iced`] touch event.
pub fn touch_event(touch: winit::event::Touch, scale_factor: f64) -> touch::Event {
    let id = touch::Finger(touch.id);
    let position = {
        let location = touch.location.to_logical::<f64>(scale_factor);

        Point::new(location.x as f32, location.y as f32)
    };

    match touch.phase {
        winit::event::TouchPhase::Started => touch::Event::FingerPressed { id, position },
        winit::event::TouchPhase::Moved => touch::Event::FingerMoved { id, position },
        winit::event::TouchPhase::Ended => touch::Event::FingerLifted { id, position },
        winit::event::TouchPhase::Cancelled => touch::Event::FingerLost { id, position },
    }
}

/// Converts a `VirtualKeyCode` from [`winit`] to an [`iced`] key code.
pub fn key_code(key: &winit::keyboard::Key) -> Option<keyboard::KeyCode> {
    use keyboard::KeyCode;

    Some(match key {
        winit::keyboard::Key::Named(key) => match key {
            NamedKey::Escape => KeyCode::Escape,
            NamedKey::F1 => KeyCode::F1,
            NamedKey::F2 => KeyCode::F2,
            NamedKey::F3 => KeyCode::F3,
            NamedKey::F4 => KeyCode::F4,
            NamedKey::F5 => KeyCode::F5,
            NamedKey::F6 => KeyCode::F6,
            NamedKey::F7 => KeyCode::F7,
            NamedKey::F8 => KeyCode::F8,
            NamedKey::F9 => KeyCode::F9,
            NamedKey::F10 => KeyCode::F10,
            NamedKey::F11 => KeyCode::F11,
            NamedKey::F12 => KeyCode::F12,
            NamedKey::F13 => KeyCode::F13,
            NamedKey::F14 => KeyCode::F14,
            NamedKey::F15 => KeyCode::F15,
            NamedKey::F16 => KeyCode::F16,
            NamedKey::F17 => KeyCode::F17,
            NamedKey::F18 => KeyCode::F18,
            NamedKey::F19 => KeyCode::F19,
            NamedKey::F20 => KeyCode::F20,
            NamedKey::F21 => KeyCode::F21,
            NamedKey::F22 => KeyCode::F22,
            NamedKey::F23 => KeyCode::F23,
            NamedKey::F24 => KeyCode::F24,
            NamedKey::ScrollLock => KeyCode::Scroll,
            NamedKey::Pause => KeyCode::Pause,
            NamedKey::Insert => KeyCode::Insert,
            NamedKey::Home => KeyCode::Home,
            NamedKey::Delete => KeyCode::Delete,
            NamedKey::End => KeyCode::End,
            NamedKey::PageDown => KeyCode::PageDown,
            NamedKey::PageUp => KeyCode::PageUp,
            NamedKey::ArrowLeft => KeyCode::Left,
            NamedKey::ArrowUp => KeyCode::Up,
            NamedKey::ArrowRight => KeyCode::Right,
            NamedKey::ArrowDown => KeyCode::Down,
            NamedKey::Backspace => KeyCode::Backspace,
            NamedKey::Enter => KeyCode::Enter,
            NamedKey::Space => KeyCode::Space,
            NamedKey::Compose => KeyCode::Compose,
            NamedKey::NumLock => KeyCode::Numlock,
            NamedKey::Convert => KeyCode::Convert,
            NamedKey::KanaMode => KeyCode::Kana,
            NamedKey::KanjiMode => KeyCode::Kanji,
            NamedKey::MediaStop => KeyCode::MediaStop,
            NamedKey::AudioVolumeMute => KeyCode::Mute,
            NamedKey::MediaTrackNext => KeyCode::NextTrack,
            NamedKey::NonConvert => KeyCode::NoConvert,
            NamedKey::MediaPlayPause => KeyCode::PlayPause,
            NamedKey::Power => KeyCode::Power,
            NamedKey::MediaTrackPrevious => KeyCode::PrevTrack,
            NamedKey::Tab => KeyCode::Tab,
            NamedKey::AudioVolumeDown => KeyCode::VolumeDown,
            NamedKey::AudioVolumeUp => KeyCode::VolumeUp,
            NamedKey::WakeUp => KeyCode::Wake,
            NamedKey::Copy => KeyCode::Copy,
            NamedKey::Paste => KeyCode::Paste,
            NamedKey::Cut => KeyCode::Cut,
            _ => return None,
        },
        winit::keyboard::Key::Character(c) => match c.as_str() {
            "a" | "A" => KeyCode::A,
            "b" | "B" => KeyCode::B,
            "c" | "C" => KeyCode::C,
            "d" | "D" => KeyCode::D,
            "e" | "E" => KeyCode::E,
            "f" | "F" => KeyCode::F,
            "g" | "G" => KeyCode::G,
            "h" | "H" => KeyCode::H,
            "i" | "I" => KeyCode::I,
            "j" | "J" => KeyCode::J,
            "k" | "K" => KeyCode::K,
            "l" | "L" => KeyCode::L,
            "m" | "M" => KeyCode::M,
            "n" | "N" => KeyCode::N,
            "o" | "O" => KeyCode::O,
            "p" | "P" => KeyCode::P,
            "q" | "Q" => KeyCode::Q,
            "r" | "R" => KeyCode::R,
            "s" | "S" => KeyCode::S,
            "t" | "T" => KeyCode::T,
            "u" | "U" => KeyCode::U,
            "v" | "V" => KeyCode::V,
            "w" | "W" => KeyCode::W,
            "x" | "X" => KeyCode::X,
            "y" | "Y" => KeyCode::Y,
            "z" | "Z" => KeyCode::Z,
            "0" => KeyCode::Key0,
            "1" => KeyCode::Key1,
            "2" => KeyCode::Key2,
            "3" => KeyCode::Key3,
            "4" => KeyCode::Key4,
            "5" => KeyCode::Key5,
            "6" => KeyCode::Key6,
            "7" => KeyCode::Key7,
            "8" => KeyCode::Key8,
            "9" => KeyCode::Key9,
            "'" => KeyCode::Apostrophe,
            "*" => KeyCode::Asterisk,
            "\\" => KeyCode::Backslash,
            "^" => KeyCode::Caret,
            ":" => KeyCode::Colon,
            "," => KeyCode::Comma,
            "=" => KeyCode::Equals,
            "-" => KeyCode::Minus,
            "." => KeyCode::Period,
            "+" => KeyCode::Plus,
            ";" => KeyCode::Semicolon,
            "/" => KeyCode::Slash,
            " " => KeyCode::Space,
            "_" => KeyCode::Underline,
            _ => return None,
        },
        winit::keyboard::Key::Unidentified(_) | winit::keyboard::Key::Dead(_) => return None,
    })
}
