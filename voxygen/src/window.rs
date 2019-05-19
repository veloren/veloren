use crate::{
    render::{Renderer, WinColorFmt, WinDepthFmt},
    settings::Settings,
    ui, Error,
};
use std::collections::HashMap;
use vek::*;

pub struct Window {
    events_loop: glutin::EventsLoop,
    renderer: Renderer,
    window: glutin::GlWindow,
    cursor_grabbed: bool,
    needs_refresh_resize: bool,
    key_map: HashMap<glutin::VirtualKeyCode, Key>,
    supplement_events: Vec<Event>,
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
        key_map.insert(settings.controls.toggle_cursor, Key::ToggleCursor);
        key_map.insert(settings.controls.escape, Key::Escape);
        key_map.insert(settings.controls.enter, Key::Enter);
        key_map.insert(settings.controls.move_forward, Key::MoveForward);
        key_map.insert(settings.controls.move_left, Key::MoveLeft);
        key_map.insert(settings.controls.move_back, Key::MoveBack);
        key_map.insert(settings.controls.move_right, Key::MoveRight);
        key_map.insert(settings.controls.jump, Key::Jump);
        key_map.insert(settings.controls.glide, Key::Glide);
        key_map.insert(settings.controls.map, Key::Map);
        key_map.insert(settings.controls.bag, Key::Bag);
        key_map.insert(settings.controls.quest_log, Key::QuestLog);
        key_map.insert(settings.controls.character_window, Key::CharacterWindow);
        key_map.insert(settings.controls.social, Key::Social);
        key_map.insert(settings.controls.spellbook, Key::Spellbook);
        key_map.insert(settings.controls.settings, Key::Settings);
        key_map.insert(settings.controls.help, Key::Help);
        key_map.insert(settings.controls.toggle_interface, Key::ToggleInterface);

        let tmp = Ok(Self {
            events_loop,
            renderer: Renderer::new(device, factory, win_color_view, win_depth_view)?,
            window,
            cursor_grabbed: false,
            needs_refresh_resize: false,
            key_map,
            supplement_events: vec![],
        });
        tmp
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
        let key_map = &self.key_map;

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

                    glutin::WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode
                    {
                        Some(keycode) => match key_map.get(&keycode) {
                            Some(&key) => events.push(match input.state {
                                glutin::ElementState::Pressed => Event::KeyDown(key),
                                _ => Event::KeyUp(key),
                            }),
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                },
                glutin::Event::DeviceEvent { event, .. } => match event {
                    glutin::DeviceEvent::MouseMotion {
                        delta: (dx, dy), ..
                    } if cursor_grabbed => {
                        events.push(Event::CursorPan(Vec2::new(dx as f32, dy as f32)))
                    }
                    glutin::DeviceEvent::MouseWheel {
                        delta: glutin::MouseScrollDelta::LineDelta(_x, y),
                        ..
                    } if cursor_grabbed => events.push(Event::Zoom(y as f32)),
                    _ => {}
                },
                _ => {}
            }
        });
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
}

/// Represents a key that the game recognises after keyboard mapping.
#[derive(Clone, Copy)]
pub enum Key {
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
    /// A key that the game recognises has been pressed down.
    KeyDown(Key),
    /// A key that the game recognises has been released down.
    KeyUp(Key),
    /// Event that the ui uses.
    Ui(ui::Event),
    // The view distance has been changed
    ViewDistanceChanged(u32),
    /// Game settings have changed.
    SettingsChanged,
}
