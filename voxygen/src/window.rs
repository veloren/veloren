use crate::{
    render::{Renderer, WinColorFmt, WinDepthFmt},
    settings::Settings,
    ui, Error,
};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use vek::*;

/// Represents a key that the game recognises after keyboard mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum GameInput {
    ToggleCursor,
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
    Jump,
    Glide,
    Enter,
    Escape,
    Map,
    Bag,
    QuestLog,
    CharacterWindow,
    Social,
    Spellbook,
    Settings,
    ToggleInterface,
    Help,
    ToggleDebug,
    Fullscreen,
    Screenshot,
    ToggleIngameUi,
    Attack,
    Respawn,
}

/// Represents an incoming event from the window.
#[derive(Clone)]
pub enum Event {
    /// The window has been requested to close.
    Close,
    /// The window has been resized.
    Resize(Vec2<u32>),
    /// A key has been typed that corresponds to a specific character.
    Char(char),
    /// The cursor has been panned across the screen while grabbed.
    CursorPan(Vec2<f32>),
    /// The camera has been requested to zoom.
    Zoom(f32),
    /// A key that the game recognises has been pressed or released.
    InputUpdate(GameInput, bool),
    /// Event that the ui uses.
    Ui(ui::Event),
    // The view distance has been changed
    ViewDistanceChanged(u32),
    /// Game settings have changed.
    SettingsChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum KeyMouse {
    Key(glutin::VirtualKeyCode),
    Mouse(glutin::MouseButton),
}

pub struct Window {
    events_loop: glutin::EventsLoop,
    renderer: Renderer,
    window: glutin::GlWindow,
    cursor_grabbed: bool,
    pub pan_sensitivity: f32,
    pub zoom_sensitivity: f32,
    fullscreen: bool,
    needs_refresh_resize: bool,
    key_map: HashMap<KeyMouse, GameInput>,
    supplement_events: Vec<Event>,
    focused: bool,
}

impl Window {
    pub fn new(settings: &Settings) -> Result<Window, Error> {
        let events_loop = glutin::EventsLoop::new();

        let win_builder = glutin::WindowBuilder::new()
            .with_title("Veloren")
            .with_dimensions(glutin::dpi::LogicalSize::new(1366.0, 768.0))
            .with_maximized(true);

        let ctx_builder = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
            .with_vsync(false);

        let (window, device, factory, win_color_view, win_depth_view) =
            gfx_window_glutin::init::<WinColorFmt, WinDepthFmt>(
                win_builder,
                ctx_builder,
                &events_loop,
            )
            .map_err(|err| Error::BackendError(Box::new(err)))?;

        let mut key_map = HashMap::new();
        key_map.insert(settings.controls.toggle_cursor, GameInput::ToggleCursor);
        key_map.insert(settings.controls.escape, GameInput::Escape);
        key_map.insert(settings.controls.enter, GameInput::Enter);
        key_map.insert(settings.controls.move_forward, GameInput::MoveForward);
        key_map.insert(settings.controls.move_left, GameInput::MoveLeft);
        key_map.insert(settings.controls.move_back, GameInput::MoveBack);
        key_map.insert(settings.controls.move_right, GameInput::MoveRight);
        key_map.insert(settings.controls.jump, GameInput::Jump);
        key_map.insert(settings.controls.glide, GameInput::Glide);
        key_map.insert(settings.controls.map, GameInput::Map);
        key_map.insert(settings.controls.bag, GameInput::Bag);
        key_map.insert(settings.controls.quest_log, GameInput::QuestLog);
        key_map.insert(
            settings.controls.character_window,
            GameInput::CharacterWindow,
        );
        key_map.insert(settings.controls.social, GameInput::Social);
        key_map.insert(settings.controls.spellbook, GameInput::Spellbook);
        key_map.insert(settings.controls.settings, GameInput::Settings);
        key_map.insert(settings.controls.help, GameInput::Help);
        key_map.insert(
            settings.controls.toggle_interface,
            GameInput::ToggleInterface,
        );
        key_map.insert(settings.controls.toggle_debug, GameInput::ToggleDebug);
        key_map.insert(settings.controls.fullscreen, GameInput::Fullscreen);
        key_map.insert(settings.controls.screenshot, GameInput::Screenshot);
        key_map.insert(
            settings.controls.toggle_ingame_ui,
            GameInput::ToggleIngameUi,
        );
        key_map.insert(settings.controls.attack, GameInput::Attack);

        Ok(Self {
            events_loop,
            renderer: Renderer::new(device, factory, win_color_view, win_depth_view)?,
            window,
            cursor_grabbed: false,
            pan_sensitivity: settings.controls.pan_sensitivity,
            zoom_sensitivity: settings.controls.zoom_sensitivity,
            fullscreen: false,
            needs_refresh_resize: false,
            key_map,
            supplement_events: vec![],
            focused: true,
        })
    }

    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }
    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    pub fn fetch_events(&mut self) -> Vec<Event> {
        let mut events = vec![];
        events.append(&mut self.supplement_events);
        // Refresh ui size (used when changing playstates)
        if self.needs_refresh_resize {
            events.push(Event::Ui(ui::Event::new_resize(self.logical_size())));
            self.needs_refresh_resize = false;
        }

        // Copy data that is needed by the events closure to avoid lifetime errors.
        // TODO: Remove this if/when the compiler permits it.
        let cursor_grabbed = self.cursor_grabbed;
        let renderer = &mut self.renderer;
        let window = &mut self.window;
        let focused = &mut self.focused;
        let key_map = &self.key_map;
        let pan_sensitivity = self.pan_sensitivity;
        let zoom_sensitivity = self.zoom_sensitivity;
        let mut toggle_fullscreen = false;
        let mut take_screenshot = false;

        self.events_loop.poll_events(|event| {
            // Get events for ui.
            if let Some(event) = ui::Event::try_from(event.clone(), &window) {
                events.push(Event::Ui(event));
            }

            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::CloseRequested => events.push(Event::Close),
                    glutin::WindowEvent::Resized(glutin::dpi::LogicalSize { width, height }) => {
                        let (mut color_view, mut depth_view) = renderer.win_views_mut();
                        gfx_window_glutin::update_views(&window, &mut color_view, &mut depth_view);
                        renderer.on_resize().unwrap();
                        events.push(Event::Resize(Vec2::new(width as u32, height as u32)));
                    }
                    glutin::WindowEvent::ReceivedCharacter(c) => events.push(Event::Char(c)),
                    glutin::WindowEvent::MouseInput { button, state, .. } if cursor_grabbed => {
                        if let Some(&game_input) = key_map.get(&KeyMouse::Mouse(button)) {
                            events.push(Event::InputUpdate(
                                game_input,
                                state == glutin::ElementState::Pressed,
                            ))
                        }
                    }
                    glutin::WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode
                    {
                        Some(key) => match key_map.get(&KeyMouse::Key(key)) {
                            Some(GameInput::Fullscreen) => match input.state {
                                glutin::ElementState::Pressed => {
                                    toggle_fullscreen = !toggle_fullscreen
                                }
                                _ => (),
                            },
                            Some(GameInput::Screenshot) => match input.state {
                                glutin::ElementState::Pressed => take_screenshot = true,
                                _ => {}
                            },
                            Some(&game_input) => events.push(Event::InputUpdate(
                                game_input,
                                input.state == glutin::ElementState::Pressed,
                            )),
                            _ => {}
                        },
                        _ => {}
                    },
                    glutin::WindowEvent::Focused(new_focus) => *focused = new_focus,
                    _ => {}
                },
                glutin::Event::DeviceEvent { event, .. } => match event {
                    glutin::DeviceEvent::MouseMotion {
                        delta: (dx, dy), ..
                    } if cursor_grabbed && *focused => events.push(Event::CursorPan(Vec2::new(
                        dx as f32 * pan_sensitivity,
                        dy as f32 * pan_sensitivity,
                    ))),
                    glutin::DeviceEvent::MouseWheel {
                        delta: glutin::MouseScrollDelta::LineDelta(_x, y),
                        ..
                    } if cursor_grabbed && *focused => {
                        events.push(Event::Zoom(y as f32 * zoom_sensitivity))
                    }
                    _ => {}
                },
                _ => {}
            }
        });

        if take_screenshot {
            self.take_screenshot();
        }

        if toggle_fullscreen {
            self.fullscreen(!self.is_fullscreen());
        }

        events
    }

    pub fn swap_buffers(&self) -> Result<(), Error> {
        self.window
            .swap_buffers()
            .map_err(|err| Error::BackendError(Box::new(err)))
    }

    pub fn is_cursor_grabbed(&self) -> bool {
        self.cursor_grabbed
    }

    pub fn grab_cursor(&mut self, grab: bool) {
        self.cursor_grabbed = grab;
        self.window.hide_cursor(grab);
        let _ = self.window.grab_cursor(grab);
    }

    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn fullscreen(&mut self, fullscreen: bool) {
        self.fullscreen = fullscreen;
        if fullscreen {
            self.window
                .set_fullscreen(Some(self.window.get_current_monitor()));
        } else {
            self.window.set_fullscreen(None);
        }
    }

    pub fn needs_refresh_resize(&mut self) {
        self.needs_refresh_resize = true;
    }

    pub fn logical_size(&self) -> Vec2<f64> {
        let (w, h) = self
            .window
            .get_inner_size()
            .unwrap_or(glutin::dpi::LogicalSize::new(0.0, 0.0))
            .into();
        Vec2::new(w, h)
    }

    pub fn send_supplement_event(&mut self, event: Event) {
        self.supplement_events.push(event)
    }

    pub fn take_screenshot(&mut self) {
        match self.renderer.create_screenshot() {
            Ok(img) => {
                std::thread::spawn(move || {
                    use std::{path::PathBuf, time::SystemTime};
                    // Check if folder exists and create it if it does not
                    let mut path = PathBuf::from("./screenshots");
                    if !path.exists() {
                        if let Err(err) = std::fs::create_dir(&path) {
                            log::error!("Coudn't create folder for screenshot: {:?}", err);
                        }
                    }
                    path.push(format!(
                        "screenshot_{}.png",
                        SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0)
                    ));
                    if let Err(err) = img.save(&path) {
                        log::error!("Coudn't save screenshot: {:?}", err);
                    }
                });
            }
            Err(err) => log::error!("Coudn't create screenshot due to renderer error: {:?}", err),
        }
    }
}
