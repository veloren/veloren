// Library
use glutin;
use gfx_window_glutin;
use vek::*;
use std::collections::HashMap;

// Crate
use crate::{
    Error,
    render::{
        Renderer,
        TgtColorFmt,
        TgtDepthFmt,
    },
};

pub struct Window {
    events_loop: glutin::EventsLoop,
    renderer: Renderer,
    window: glutin::GlWindow,
    cursor_grabbed: bool,
    key_map: HashMap<glutin::VirtualKeyCode, Key>,
}


impl Window {
    pub fn new() -> Result<Window, Error> {
        let events_loop = glutin::EventsLoop::new();

        let win_builder = glutin::WindowBuilder::new()
            .with_title("Veloren (Voxygen)")
            .with_dimensions(glutin::dpi::LogicalSize::new(800.0, 500.0))
            .with_maximized(false);

        let ctx_builder = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
            .with_vsync(true);

        let (
            window,
            device,
            factory,
            tgt_color_view,
            tgt_depth_view,
        ) = gfx_window_glutin::init::<TgtColorFmt, TgtDepthFmt>(
            win_builder,
            ctx_builder,
            &events_loop,
        ).map_err(|err| Error::BackendError(Box::new(err)))?;

        let mut key_map = HashMap::new();
        key_map.insert(glutin::VirtualKeyCode::Escape, Key::ToggleCursor);
        key_map.insert(glutin::VirtualKeyCode::W, Key::MoveForward);
        key_map.insert(glutin::VirtualKeyCode::A, Key::MoveLeft);
        key_map.insert(glutin::VirtualKeyCode::S, Key::MoveBack);
        key_map.insert(glutin::VirtualKeyCode::D, Key::MoveRight);

        let tmp = Ok(Self {
            events_loop,
            renderer: Renderer::new(
                device,
                factory,
                tgt_color_view,
                tgt_depth_view,
            )?,
            window,
            cursor_grabbed: false,
            key_map,
        });
        tmp
    }

    pub fn renderer(&self) -> &Renderer { &self.renderer }
    pub fn renderer_mut(&mut self) -> &mut Renderer { &mut self.renderer }

    pub fn fetch_events(&mut self) -> Vec<Event> {
        // Copy data that is needed by the events closure to avoid lifetime errors
        // TODO: Remove this if/when the compiler permits it
        let cursor_grabbed = self.cursor_grabbed;
        let renderer = &mut self.renderer;
        let window = &mut self.window;
        let key_map = &self.key_map;

        let mut events = vec![];
        self.events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => events.push(Event::Close),
                glutin::WindowEvent::Resized(glutin::dpi::LogicalSize { width, height }) => {
                    let (mut color_view, mut depth_view) = renderer.target_views_mut();
                    gfx_window_glutin::update_views(
                        &window,
                        &mut color_view,
                        &mut depth_view,
                    );
                    events.push(Event::Resize(Vec2::new(width as u32, height as u32)));
                },
                glutin::WindowEvent::ReceivedCharacter(c) => events.push(Event::Char(c)),
                glutin::WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                    Some(keycode) => match key_map.get(&keycode) {
                        Some(&key) => events.push(match input.state {
                            glutin::ElementState::Pressed => Event::KeyDown(key),
                            _ => Event::KeyUp(key),
                        }),
                        _ => {},
                    },
                    _ => {},
                },
                _ => {},
            },
            glutin::Event::DeviceEvent { event, .. } => match event {
                glutin::DeviceEvent::MouseMotion { delta: (dx, dy), .. } if cursor_grabbed =>
                    events.push(Event::CursorPan(Vec2::new(dx as f32, dy as f32))),
                glutin::DeviceEvent::MouseWheel {
                    delta: glutin::MouseScrollDelta::LineDelta(_x, y),
                    ..
                } if cursor_grabbed => events.push(Event::Zoom(y as f32)),
                _ => {},
            },
            _ => {},
        });
        events
    }

    pub fn swap_buffers(&self) -> Result<(), Error> {
        self.window.swap_buffers()
            .map_err(|err| Error::BackendError(Box::new(err)))
    }

    pub fn is_cursor_grabbed(&self) -> bool {
        self.cursor_grabbed
    }

    pub fn grab_cursor(&mut self, grab: bool) {
        self.cursor_grabbed = grab;
        self.window.hide_cursor(grab);
        self.window.grab_cursor(grab)
            .expect("Failed to grab/ungrab cursor");
    }
}

/// Represents a key that the game recognises after keyboard mapping
#[derive(Clone, Copy)]
pub enum Key {
    ToggleCursor,
    MoveForward,
    MoveBack,
    MoveLeft,
    MoveRight,
}

/// Represents an incoming event from the window
pub enum Event {
    /// The window has been requested to close.
    Close,
    /// The window has been resized
    Resize(Vec2<u32>),
    /// A key has been typed that corresponds to a specific character.
    Char(char),
    /// The cursor has been panned across the screen while grabbed.
    CursorPan(Vec2<f32>),
    /// The camera has been requested to zoom.
    Zoom(f32),
    /// A key that the game recognises has been pressed down
    KeyDown(Key),
    /// A key that the game recognises has been released down
    KeyUp(Key),
}
