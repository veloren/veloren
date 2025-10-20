use crate::{
    error::Error,
    game_input::GameInput,
    render::Renderer,
    settings::{ControlSettings, ControllerSettings, Settings, controller::*},
    ui,
};
use common_base::span;
use crossbeam_channel as channel;
use gilrs::{Button as GilButton, EventType, Gilrs};
use hashbrown::HashMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strum::{AsRefStr, EnumIter};
use tracing::{error, warn};
use vek::*;
use winit::monitor::VideoModeHandle;

/// Represents a key that the game menus recognise after input mapping
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize,
    AsRefStr,
    EnumIter,
)]
pub enum MenuInput {
    Up,
    Down,
    Left,
    Right,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    Home,
    End,
    Apply,
    Back,
    Exit,
}

impl MenuInput {
    pub fn get_localization_key(&self) -> &str { self.as_ref() }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum AnalogMenuInput {
    MoveX(f32),
    MoveY(f32),
    ScrollX(f32),
    ScrollY(f32),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum AnalogGameInput {
    MovementX(f32),
    MovementY(f32),
    CameraX(f32),
    CameraY(f32),
}

/// Represents an incoming event from the window.
#[derive(Clone, Debug)]
pub enum Event {
    /// The window has been requested to close.
    Close,
    /// The window has been resized.
    Resize(Vec2<u32>),
    /// The window scale factor has been changed
    ScaleFactorChanged(f64),
    /// The window has been moved.
    Moved(Vec2<u32>),
    /// The cursor has been panned across the screen while grabbed.
    CursorPan(Vec2<f32>),
    /// The cursor has been moved across the screen while ungrabbed.
    CursorMove(Vec2<f32>),
    /// A mouse button has been pressed or released
    MouseButton(MouseButton, PressState),
    /// The camera has been requested to zoom.
    Zoom(f32),
    /// A key that the game recognises has been pressed or released.
    InputUpdate(GameInput, bool),
    /// Event that the ui uses.
    Ui(ui::Event),
    /// Event that the iced ui uses.
    IcedUi(ui::ice::Event),
    /// The view distance has changed.
    ViewDistanceChanged(u32),
    /// Game settings have changed.
    SettingsChanged,
    /// The window is (un)focused
    Focused(bool),
    /// A key that the game recognises for menu navigation has been pressed or
    /// released
    MenuInput(MenuInput, bool),
    /// Update of the analog inputs recognized by the menus
    AnalogMenuInput(AnalogMenuInput),
    /// Update of the analog inputs recognized by the game
    AnalogGameInput(AnalogGameInput),
    /// We tried to save a screenshot
    ScreenshotMessage(String),
}

pub type MouseButton = winit::event::MouseButton;
pub type PressState = winit::event::ElementState;
pub type EventLoop = winit::event_loop::EventLoop<()>;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum KeyMouse {
    Key(winit::keyboard::Key),
    Mouse(winit::event::MouseButton),
}

impl KeyMouse {
    pub fn into_upper(mut self) -> Self {
        if let KeyMouse::Key(winit::keyboard::Key::Character(c)) = &mut self {
            *c = c.to_ascii_uppercase().into();
        }
        self
    }

    /// Returns key description (e.g Left Shift)
    pub fn display_string(&self) -> String {
        use self::KeyMouse::*;
        use winit::{event::MouseButton, keyboard::Key::*};

        match self {
            Key(key) => match key {
                Named(key) => format!("{key:?}"),
                Character(c) => c.to_string(),
                Unidentified(key) => format!("Unknown ({key:?})"),
                Dead(dead) => format!("Dead ({dead:?})"),
            },
            Mouse(MouseButton::Left) => String::from("Left Click"),
            Mouse(MouseButton::Right) => String::from("Right Click"),
            Mouse(MouseButton::Middle) => String::from("Middle Click"),
            Mouse(MouseButton::Forward) => String::from("Mouse Forward"),
            Mouse(MouseButton::Back) => String::from("Mouse Back"),
            Mouse(MouseButton::Other(button)) => {
                // Additional mouse buttons after middle click start at 1
                format!("Mouse {}", button + 3)
            },
        }
    }

    /// If it exists, returns the shortened version of a key name
    /// (e.g. Left Click -> M1)
    pub fn try_shortened(&self) -> Option<String> {
        use self::KeyMouse::*;
        use winit::event::MouseButton;
        let key_string = match self {
            Mouse(MouseButton::Left) => "M1",
            Mouse(MouseButton::Right) => "M2",
            Mouse(MouseButton::Middle) => "M3",
            Mouse(MouseButton::Other(button)) => {
                // Additional mouse buttons after middle click start at 1
                return Some(format!("M{}", button + 3));
            },
            _ => return None,
        };

        Some(key_string.to_owned())
    }

    /// Returns shortest name of key (e.g. Left Click - M1)
    /// If key doesn't have shorter version, use regular one.
    ///
    /// Use it in case if space does really matter.
    pub fn display_shortest(&self) -> String {
        self.try_shortened()
            .unwrap_or_else(|| self.display_string())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LastInput {
    KeyboardMouse,
    Controller,
}

pub struct Window {
    renderer: Renderer,
    window: Arc<winit::window::Window>,
    cursor_grabbed: bool,
    pub pan_sensitivity: u32,
    pub zoom_sensitivity: u32,
    pub zoom_inversion: bool,
    pub mouse_y_inversion: bool,
    fullscreen: FullScreenSettings,
    modifiers: winit::keyboard::ModifiersState,
    // Track if at least one Resized event has occured since the last `fetch_events` call
    // Used for deduplication of resizes.
    resized: bool,
    scale_factor: f64,
    needs_refresh_resize: bool,
    keypress_map: HashMap<GameInput, winit::event::ElementState>,
    pub remapping_keybindings: Option<GameInput>,
    events: Vec<Event>,
    pub focused: bool,
    gilrs: Option<Gilrs>,
    pub controller_modifiers: Vec<Button>,
    pub game_layer_lb: bool,
    pub game_layer_rb: bool,
    cursor_position: winit::dpi::PhysicalPosition<f64>,
    mouse_emulation_vec: Vec2<f32>,
    last_input: LastInput,
    // Currently used to send and receive screenshot result messages
    message_sender: channel::Sender<String>,
    message_receiver: channel::Receiver<String>,
    // Used for screenshots & fullscreen toggle to deduplicate/postpone to after event handler
    take_screenshot: bool,
    toggle_fullscreen: bool,
}

impl Window {
    pub fn new(
        settings: &Settings,
        runtime: &tokio::runtime::Runtime,
    ) -> Result<(Window, EventLoop), Error> {
        let event_loop = EventLoop::new().unwrap();

        let window = settings.graphics.window;

        let attributes = winit::window::Window::default_attributes()
            .with_title("Veloren")
            .with_inner_size(winit::dpi::LogicalSize::new(
                window.size[0] as f64,
                window.size[1] as f64,
            ))
            .with_maximized(window.maximised);

        // Avoid cpal / winit OleInitialize conflict
        // See: https://github.com/rust-windowing/winit/pull/1524
        #[cfg(target_family = "windows")]
        let attributes = winit::platform::windows::WindowAttributesExtWindows::with_drag_and_drop(
            attributes, false,
        );

        #[expect(deprecated)]
        let window = Arc::new(event_loop.create_window(attributes).unwrap());

        let renderer = Renderer::new(
            Arc::clone(&window),
            settings.graphics.render_mode.clone(),
            runtime,
        )?;

        let keypress_map = HashMap::new();

        let gilrs = match Gilrs::new() {
            Ok(gilrs) => Some(gilrs),
            Err(gilrs::Error::NotImplemented(_dummy)) => {
                warn!("Controller input is unsupported on this platform.");
                None
            },
            Err(gilrs::Error::InvalidAxisToBtn) => {
                error!(
                    "Invalid AxisToBtn controller mapping. Falling back to no controller support."
                );
                None
            },
            Err(gilrs::Error::Other(e)) => {
                error!(
                    ?e,
                    "Platform-specific error when creating a Gilrs instance. Falling back to no \
                     controller support."
                );
                None
            },
            Err(e) => {
                error!(
                    ?e,
                    "Unspecified error when creating a Gilrs instance. Falling back to no \
                     controller support."
                );
                None
            },
        };

        let (message_sender, message_receiver): (
            channel::Sender<String>,
            channel::Receiver<String>,
        ) = channel::unbounded::<String>();

        let scale_factor = window.scale_factor();

        let mut this = Self {
            renderer,
            window,
            cursor_grabbed: false,
            pan_sensitivity: settings.gameplay.pan_sensitivity,
            zoom_sensitivity: settings.gameplay.zoom_sensitivity,
            zoom_inversion: settings.gameplay.zoom_inversion,
            mouse_y_inversion: settings.gameplay.mouse_y_inversion,
            fullscreen: FullScreenSettings::default(),
            modifiers: Default::default(),
            scale_factor,
            resized: false,
            needs_refresh_resize: false,
            keypress_map,
            remapping_keybindings: None,
            game_layer_lb: false,
            game_layer_rb: true,
            events: Vec::new(),
            focused: true,
            gilrs,
            controller_modifiers: Vec::new(),
            cursor_position: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            mouse_emulation_vec: Vec2::zero(),
            last_input: LastInput::KeyboardMouse,
            // Currently used to send and receive screenshot result messages
            message_sender,
            message_receiver,
            take_screenshot: false,
            toggle_fullscreen: false,
        };

        this.set_fullscreen_mode(settings.graphics.fullscreen);

        Ok((this, event_loop))
    }

    pub fn renderer(&self) -> &Renderer { &self.renderer }

    pub fn renderer_mut(&mut self) -> &mut Renderer { &mut self.renderer }

    pub fn resolve_deduplicated_events(
        &mut self,
        settings: &mut Settings,
        config_dir: &std::path::Path,
    ) {
        // Handle screenshots and toggling fullscreen
        if self.take_screenshot {
            self.take_screenshot = false;
            self.take_screenshot(settings);
        }
        if self.toggle_fullscreen {
            self.toggle_fullscreen = false;
            self.toggle_fullscreen(settings, config_dir);
        }
    }

    #[expect(clippy::get_first)]
    pub fn fetch_events(&mut self, settings: &mut Settings) -> Vec<Event> {
        span!(_guard, "fetch_events", "Window::fetch_events");

        let controller = &mut settings.controller;
        // Refresh ui size (used when changing playstates)
        if self.needs_refresh_resize {
            let scale_factor = self.window.scale_factor();
            let physical = self.window.inner_size();

            let logical_size =
                Vec2::from(<(f64, f64)>::from(physical.to_logical::<f64>(scale_factor)));
            self.events
                .push(Event::Ui(ui::Event::new_resize(logical_size)));
            self.events.push(Event::IcedUi(iced::Event::Window(
                iced::window::Event::Resized {
                    width: logical_size.x as u32,
                    height: logical_size.y as u32,
                },
            )));
            self.events.push(Event::ScaleFactorChanged(scale_factor));
            self.needs_refresh_resize = false;
        }

        // Handle deduplicated resizing that occured
        if self.resized {
            self.resized = false;
            // We don't use the size provided by the event because more resize events could
            // have happened since, making the value outdated, so we must query directly
            // from the window to prevent errors
            let physical = self.window.inner_size();
            let scale_factor = self.window.scale_factor();
            let is_maximized = self.window.is_maximized();

            self.renderer
                .on_resize(Vec2::new(physical.width, physical.height));
            self.events
                .push(Event::Resize(Vec2::new(physical.width, physical.height)));

            let logical_size =
                Vec2::from(<(f64, f64)>::from(physical.to_logical::<f64>(scale_factor)));

            // Emit event for the UI
            self.events
                .push(Event::Ui(ui::Event::new_resize(logical_size)));
            self.events.push(Event::IcedUi(iced::Event::Window(
                iced::window::Event::Resized {
                    width: logical_size.x as u32,
                    height: logical_size.y as u32,
                },
            )));

            // Save new window state in settings
            //
            // We don't save the size if it's less than 1 because wgpu fails to create a
            // surface with zero size.
            if logical_size.x >= 1.0 && logical_size.y >= 1.0 {
                settings.graphics.window.size = [logical_size.x as u32, logical_size.y as u32];
            }
            settings.graphics.window.maximised = is_maximized;
        }

        // Receive any messages sent through the message channel
        for message in self.message_receiver.try_iter() {
            self.events.push(Event::ScreenshotMessage(message))
        }

        if let Some(gilrs) = &mut self.gilrs {
            while let Some(event) = gilrs.next_event() {
                fn handle_buttons(
                    settings: &mut ControllerSettings,
                    remapping: &mut Option<GameInput>,
                    modifiers: &mut Vec<Button>,
                    events: &mut Vec<Event>,
                    button: &Button,
                    is_pressed: bool,
                    last_input: &mut LastInput,
                    mod1: bool,
                    mod2: bool,
                ) {
                    // update last input to be controller if button was pressed
                    *last_input = LastInput::Controller;

                    if settings.modifier_buttons.contains(button) {
                        if is_pressed {
                            modifiers.push(*button);
                        // There is a possibility of voxygen not having
                        // registered the initial press event (either because it
                        // hadn't started yet, or whatever else) hence the
                        // modifier has no position in the list, unwrapping
                        // here would cause a crash in those cases
                        } else if let Some(index) =
                            modifiers.iter().position(|modifier| modifier == button)
                        {
                            modifiers.remove(index);
                        }
                    }

                    // have to make two LayerEntries so LB+RB can be treated equivalent to RB+LB
                    let l_entry1 = LayerEntry {
                        button: *button,
                        mod1: modifiers.get(0).copied().unwrap_or_default(),
                        mod2: modifiers.get(1).copied().unwrap_or_default(),
                    };
                    let l_entry2 = LayerEntry {
                        button: *button,
                        mod1: modifiers.get(1).copied().unwrap_or_default(),
                        mod2: modifiers.get(0).copied().unwrap_or_default(),
                    };

                    // have to check l_entry1 and then l_entry2 so LB+RB can be treated equivalent
                    // to RB+LB
                    // make sure layer is currently not being remapped before being used
                    if let Some(game_inputs) = Window::map_controller_layer_input(
                        l_entry1, settings, remapping, last_input, mod1, mod2,
                    ) {
                        for game_input in game_inputs {
                            events.push(Event::InputUpdate(*game_input, is_pressed));
                        }
                    } else if let Some(game_inputs) = Window::map_controller_layer_input(
                        l_entry2, settings, remapping, last_input, mod1, mod2,
                    ) {
                        for game_input in game_inputs {
                            events.push(Event::InputUpdate(*game_input, is_pressed));
                        }
                    }

                    // make sure button is currently not being remapped before being used
                    if let Some(game_inputs) = Window::map_controller_button_input(
                        *button, settings, remapping, last_input,
                    ) {
                        for game_input in game_inputs {
                            events.push(Event::InputUpdate(*game_input, is_pressed));
                        }
                    }
                    if let Some(evs) = settings.inverse_menu_button_map.get(button) {
                        for ev in evs {
                            events.push(Event::MenuInput(*ev, is_pressed));
                        }
                    }
                }

                match event.event {
                    EventType::ButtonPressed(button, code)
                    | EventType::ButtonRepeated(button, code) => {
                        handle_buttons(
                            controller,
                            &mut self.remapping_keybindings,
                            &mut self.controller_modifiers,
                            &mut self.events,
                            &Button::from((button, code)),
                            true,
                            &mut self.last_input,
                            settings.interface.gamelayer_mod1,
                            settings.interface.gamelayer_mod2,
                        );
                    },
                    EventType::ButtonReleased(button, code) => {
                        handle_buttons(
                            controller,
                            &mut self.remapping_keybindings,
                            &mut self.controller_modifiers,
                            &mut self.events,
                            &Button::from((button, code)),
                            false,
                            &mut self.last_input,
                            settings.interface.gamelayer_mod1,
                            settings.interface.gamelayer_mod2,
                        );
                    },
                    EventType::ButtonChanged(button, _value, code) => {
                        if let Some(actions) = controller
                            .inverse_game_analog_button_map
                            .get(&AnalogButton::from((button, code)))
                        {
                            #[expect(clippy::never_loop)]
                            for action in actions {
                                match *action {}
                            }
                        }
                        if let Some(actions) = controller
                            .inverse_menu_analog_button_map
                            .get(&AnalogButton::from((button, code)))
                        {
                            #[expect(clippy::never_loop)]
                            for action in actions {
                                match *action {}
                            }
                        }
                    },

                    EventType::AxisChanged(axis, value, code) => {
                        let value = if controller.inverted_axes.contains(&Axis::from((axis, code)))
                        {
                            -value
                        } else {
                            value
                        };

                        let value =
                            controller.apply_axis_deadzone(&Axis::from((axis, code)), value);

                        // update last input to be controller if axis was moved
                        if value.abs() > 0.0001 {
                            self.last_input = LastInput::Controller;
                        }

                        if self.cursor_grabbed {
                            if let Some(actions) = controller
                                .inverse_game_axis_map
                                .get(&Axis::from((axis, code)))
                            {
                                for action in actions {
                                    match *action {
                                        AxisGameAction::MovementX => {
                                            self.events.push(Event::AnalogGameInput(
                                                AnalogGameInput::MovementX(value),
                                            ));
                                        },
                                        AxisGameAction::MovementY => {
                                            self.events.push(Event::AnalogGameInput(
                                                AnalogGameInput::MovementY(value),
                                            ));
                                        },
                                        AxisGameAction::CameraX => {
                                            self.events.push(Event::AnalogGameInput(
                                                AnalogGameInput::CameraX(
                                                    value * controller.pan_sensitivity as f32
                                                        / 100.0,
                                                ),
                                            ));
                                        },
                                        AxisGameAction::CameraY => {
                                            let pan_invert_y = match controller.pan_invert_y {
                                                true => -1.0,
                                                false => 1.0,
                                            };

                                            self.events.push(Event::AnalogGameInput(
                                                AnalogGameInput::CameraY(
                                                    -value
                                                        * controller.pan_sensitivity as f32
                                                        * pan_invert_y
                                                        / 100.0,
                                                ),
                                            ));
                                        },
                                    }
                                }
                            }
                        } else if let Some(actions) = controller
                            .inverse_menu_axis_map
                            .get(&Axis::from((axis, code)))
                        {
                            // TODO: possibly add sensitivity settings when this is used
                            for action in actions {
                                match *action {
                                    AxisMenuAction::MoveX => {
                                        self.events.push(Event::AnalogMenuInput(
                                            AnalogMenuInput::MoveX(value),
                                        ));
                                    },
                                    AxisMenuAction::MoveY => {
                                        self.events.push(Event::AnalogMenuInput(
                                            AnalogMenuInput::MoveY(value),
                                        ));
                                    },
                                    AxisMenuAction::ScrollX => {
                                        self.events.push(Event::AnalogMenuInput(
                                            AnalogMenuInput::ScrollX(value),
                                        ));
                                    },
                                    AxisMenuAction::ScrollY => {
                                        self.events.push(Event::AnalogMenuInput(
                                            AnalogMenuInput::ScrollY(value),
                                        ));
                                    },
                                }
                            }
                        }
                    },
                    _ => {},
                }
            }
        }

        let mut events = std::mem::take(&mut self.events);
        // Mouse emulation for the menus, to be removed when a proper menu navigation
        // system is available
        if !self.cursor_grabbed {
            events = events
                .into_iter()
                .filter_map(|event| match event {
                    Event::AnalogMenuInput(input) => match input {
                        AnalogMenuInput::MoveX(d) => {
                            self.mouse_emulation_vec.x = d;
                            None
                        },
                        AnalogMenuInput::MoveY(d) => {
                            // This just has to be inverted for some reason
                            self.mouse_emulation_vec.y = -d;
                            None
                        },
                        input => Some(Event::AnalogMenuInput(input)),
                    },
                    Event::MenuInput(MenuInput::Apply, state) => Some(match state {
                        true => Event::Ui(ui::Event(conrod_core::event::Input::Press(
                            conrod_core::input::Button::Mouse(
                                conrod_core::input::state::mouse::Button::Left,
                            ),
                        ))),
                        false => Event::Ui(ui::Event(conrod_core::event::Input::Release(
                            conrod_core::input::Button::Mouse(
                                conrod_core::input::state::mouse::Button::Left,
                            ),
                        ))),
                    }),
                    _ => Some(event),
                })
                .collect();

            let sensitivity = controller.mouse_emulation_sensitivity;
            // TODO: make this independent of framerate
            // TODO: consider multiplying by scale factor
            self.offset_cursor(self.mouse_emulation_vec * sensitivity as f32);
        }

        events
    }

    pub fn handle_device_event(&mut self, event: winit::event::DeviceEvent) {
        use winit::event::DeviceEvent;

        let mouse_y_inversion = match self.mouse_y_inversion {
            true => -1.0,
            false => 1.0,
        };

        match event {
            DeviceEvent::MouseMotion {
                delta: (dx, dy), ..
            } if self.focused => {
                // update last input to be Keyboard/Mouse if motion was made
                self.last_input = LastInput::KeyboardMouse;

                let delta = Vec2::new(
                    dx as f32 * (self.pan_sensitivity as f32 / 100.0),
                    dy as f32 * (self.pan_sensitivity as f32 * mouse_y_inversion / 100.0),
                );

                if self.cursor_grabbed {
                    self.events.push(Event::CursorPan(delta));
                } else {
                    self.events.push(Event::CursorMove(delta));
                }
            },
            _ => {},
        }
    }

    pub fn handle_window_event(
        &mut self,
        event: winit::event::WindowEvent,
        settings: &mut Settings,
    ) {
        use winit::event::WindowEvent;

        let controls = &mut settings.controls;

        match event {
            WindowEvent::CloseRequested => self.events.push(Event::Close),
            WindowEvent::Resized(_) => {
                self.resized = true;
            },
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // TODO: is window resized event emitted? or do we need to handle that here?
                self.scale_factor = scale_factor;
                self.events.push(Event::ScaleFactorChanged(scale_factor));
            },
            WindowEvent::Moved(winit::dpi::PhysicalPosition { x, y }) => {
                self.events
                    .push(Event::Moved(Vec2::new(x as u32, y as u32)));
            },
            WindowEvent::MouseInput { button, state, .. } => {
                if let (true, Some(game_inputs)) =
                    // Mouse input not mapped to input if it is not grabbed
                    (
                        self.cursor_grabbed,
                        Window::map_input(
                            KeyMouse::Mouse(button),
                            controls,
                            &mut self.remapping_keybindings,
                            &mut self.last_input,
                        ),
                    )
                {
                    for game_input in game_inputs {
                        self.events.push(Event::InputUpdate(
                            *game_input,
                            state == winit::event::ElementState::Pressed,
                        ));
                    }
                }
                self.events.push(Event::MouseButton(button, state));
            },
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                // Ignore synthetic keydown events so that we don't e.g. get tabs when
                // alt-tabbing back into the window
                if matches!(event.state, winit::event::ElementState::Pressed) && is_synthetic {
                    return;
                }
                // Ignore Alt-F4 so we don't try to do anything heavy like take a screenshot
                // when the window is about to close
                if matches!(event, winit::event::KeyEvent {
                    state: winit::event::ElementState::Pressed,
                    logical_key: winit::keyboard::Key::Named(winit::keyboard::NamedKey::F4),
                    ..
                }) && self.modifiers.alt_key()
                {
                    return;
                }

                if let Some(game_inputs) = Window::map_input(
                    KeyMouse::Key(event.logical_key),
                    controls,
                    &mut self.remapping_keybindings,
                    &mut self.last_input,
                ) {
                    for game_input in game_inputs {
                        match game_input {
                            GameInput::Fullscreen => {
                                if event.state == winit::event::ElementState::Pressed
                                    && !Self::is_pressed(
                                        &mut self.keypress_map,
                                        GameInput::Fullscreen,
                                    )
                                {
                                    self.toggle_fullscreen = !self.toggle_fullscreen;
                                }
                                Self::set_pressed(
                                    &mut self.keypress_map,
                                    GameInput::Fullscreen,
                                    event.state,
                                );
                            },
                            GameInput::Screenshot => {
                                self.take_screenshot = event.state
                                    == winit::event::ElementState::Pressed
                                    && !Self::is_pressed(
                                        &mut self.keypress_map,
                                        GameInput::Screenshot,
                                    );
                                Self::set_pressed(
                                    &mut self.keypress_map,
                                    GameInput::Screenshot,
                                    event.state,
                                );
                            },
                            _ => self.events.push(Event::InputUpdate(
                                *game_input,
                                event.state == winit::event::ElementState::Pressed,
                            )),
                        }
                    }
                }
            },
            WindowEvent::Focused(state) => {
                self.focused = state;
                self.events.push(Event::Focused(state));
            },
            WindowEvent::CursorMoved { position, .. } => {
                if self.cursor_grabbed {
                    self.reset_cursor_position();
                } else {
                    self.cursor_position = position;
                }
            },
            WindowEvent::MouseWheel { delta, .. } if self.cursor_grabbed && self.focused => {
                const DIFFERENCE_FROM_DEVICE_EVENT_ON_X11: f32 = -15.0;
                self.events.push(Event::Zoom({
                    let y = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_x, y) => y,
                        // TODO: Check to see if there is a better way to find the "line
                        // height" than just hardcoding 16.0 pixels.  Alternately we could
                        // get rid of this and have the user set zoom sensitivity, since
                        // it's unlikely people would expect a configuration file to work
                        // across operating systems.
                        winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.y / 16.0) as f32,
                    };
                    y * (self.zoom_sensitivity as f32 / 100.0)
                        * if self.zoom_inversion { -1.0 } else { 1.0 }
                        * DIFFERENCE_FROM_DEVICE_EVENT_ON_X11
                }))
            },
            _ => {},
        }
    }

    /// Moves cursor by an offset
    pub fn offset_cursor(&self, d: Vec2<f32>) {
        if d != Vec2::zero()
            && let Err(err) = self
                .window
                .set_cursor_position(winit::dpi::LogicalPosition::new(
                    d.x as f64 + self.cursor_position.x,
                    d.y as f64 + self.cursor_position.y,
                ))
        {
            // Log this error once rather than every frame
            static SPAM_GUARD: std::sync::Once = std::sync::Once::new();
            SPAM_GUARD.call_once(|| {
                error!("Error setting cursor position: {:?}", err);
            })
        }
    }

    pub fn is_cursor_grabbed(&self) -> bool { self.cursor_grabbed }

    pub fn grab_cursor(&mut self, grab: bool) {
        use winit::window::CursorGrabMode;

        self.cursor_grabbed = grab;
        self.window.set_cursor_visible(!grab);
        let res = if grab {
            self.window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_e| self.window.set_cursor_grab(CursorGrabMode::Confined))
        } else {
            self.window.set_cursor_grab(CursorGrabMode::None)
        };

        if let Err(e) = res {
            error!(?e, ?grab, "Failed to toggle cursor grab");
        }
    }

    /// Reset the cursor position to the last position
    /// This is used when handling the CursorMoved event to maintain the cursor
    /// position when it is grabbed
    fn reset_cursor_position(&self) {
        if let Err(err) = self.window.set_cursor_position(self.cursor_position) {
            // Log this error once rather than every frame
            static SPAM_GUARD: std::sync::Once = std::sync::Once::new();
            SPAM_GUARD.call_once(|| {
                error!("Error resetting cursor position: {:?}", err);
            })
        }
    }

    pub fn toggle_fullscreen(&mut self, settings: &mut Settings, config_dir: &std::path::Path) {
        let fullscreen = FullScreenSettings {
            enabled: !self.is_fullscreen(),
            ..settings.graphics.fullscreen
        };

        self.set_fullscreen_mode(fullscreen);
        settings.graphics.fullscreen = fullscreen;
        settings.save_to_file_warn(config_dir);
    }

    pub fn is_fullscreen(&self) -> bool { self.fullscreen.enabled }

    /// Select a video mode that fits the specified requirements
    /// Returns None if a matching video mode doesn't exist or if
    /// the current monitor can't be retrieved
    fn select_video_mode_rec(
        &self,
        resolution: [u16; 2],
        bit_depth: Option<u16>,
        refresh_rate_millihertz: Option<u32>,
        correct_res: Option<Vec<VideoModeHandle>>,
        correct_depth: Option<Option<VideoModeHandle>>,
        correct_rate: Option<Option<VideoModeHandle>>,
    ) -> Option<VideoModeHandle> {
        // if a previous iteration of this method filtered the available video modes for
        // the correct resolution already, load that value, otherwise filter it
        // in this iteration
        let correct_res = match correct_res {
            Some(correct_res) => correct_res,
            None => self
                .window
                .current_monitor()?
                .video_modes()
                .filter(|mode| mode.size().width == resolution[0] as u32)
                .filter(|mode| mode.size().height == resolution[1] as u32)
                .collect(),
        };

        match bit_depth {
            // A bit depth is given
            Some(depth) => {
                // analogous to correct_res
                let correct_depth = correct_depth.unwrap_or_else(|| {
                    correct_res
                        .iter()
                        .find(|mode| mode.bit_depth() == depth)
                        .cloned()
                });

                match refresh_rate_millihertz {
                    // A bit depth and a refresh rate is given
                    Some(rate) => {
                        // analogous to correct_res
                        let correct_rate = correct_rate.unwrap_or_else(|| {
                            correct_res
                                .iter()
                                .find(|mode| mode.refresh_rate_millihertz() == rate)
                                .cloned()
                        });

                        // if no video mode with the given bit depth and refresh rate exists, fall
                        // back to a video mode that fits the resolution and either bit depth or
                        // refresh rate depending on which parameter was causing the correct video
                        // mode not to be found
                        correct_res
                            .iter()
                            .filter(|mode| mode.bit_depth() == depth)
                            .find(|mode| mode.refresh_rate_millihertz() == rate)
                            .cloned()
                            .or_else(|| {
                                if correct_depth.is_none() && correct_rate.is_none() {
                                    warn!(
                                        "Bit depth and refresh rate specified in settings are \
                                         incompatible with the monitor. Choosing highest bit \
                                         depth and refresh rate possible instead."
                                    );
                                }

                                self.select_video_mode_rec(
                                    resolution,
                                    correct_depth.is_some().then_some(depth),
                                    correct_rate.is_some().then_some(rate),
                                    Some(correct_res),
                                    Some(correct_depth),
                                    Some(correct_rate),
                                )
                            })
                    },
                    // A bit depth and no refresh rate is given
                    // if no video mode with the given bit depth exists, fall
                    // back to a video mode that fits only the resolution
                    None => match correct_depth {
                        Some(mode) => Some(mode),
                        None => {
                            warn!(
                                "Bit depth specified in settings is incompatible with the \
                                 monitor. Choosing highest bit depth possible instead."
                            );

                            self.select_video_mode_rec(
                                resolution,
                                None,
                                None,
                                Some(correct_res),
                                Some(correct_depth),
                                None,
                            )
                        },
                    },
                }
            },
            // No bit depth is given
            None => match refresh_rate_millihertz {
                // No bit depth and a refresh rate is given
                Some(rate) => {
                    // analogous to correct_res
                    let correct_rate = correct_rate.unwrap_or_else(|| {
                        correct_res
                            .iter()
                            .find(|mode| mode.refresh_rate_millihertz() == rate)
                            .cloned()
                    });

                    // if no video mode with the given bit depth exists, fall
                    // back to a video mode that fits only the resolution
                    match correct_rate {
                        Some(mode) => Some(mode),
                        None => {
                            warn!(
                                "Refresh rate specified in settings is incompatible with the \
                                 monitor. Choosing highest refresh rate possible instead."
                            );

                            self.select_video_mode_rec(
                                resolution,
                                None,
                                None,
                                Some(correct_res),
                                None,
                                Some(correct_rate),
                            )
                        },
                    }
                },
                // No bit depth and no refresh rate is given
                // get the video mode with the specified resolution and the max bit depth and
                // refresh rate
                None => correct_res
                    .into_iter()
                    // Prefer bit depth over refresh rate
                    .sorted_by_key(|mode| mode.bit_depth())
                    .max_by_key(|mode| mode.refresh_rate_millihertz()),
            },
        }
    }

    fn select_video_mode(
        &self,
        resolution: [u16; 2],
        bit_depth: Option<u16>,
        refresh_rate_millihertz: Option<u32>,
    ) -> Option<VideoModeHandle> {
        // (resolution, bit depth, refresh rate) represents a video mode
        // spec: as specified
        // max: maximum value available

        // order of fallbacks as follows:
        // (spec, spec, spec)
        // (spec, spec, max), (spec, max, spec)
        // (spec, max, max)
        // (max, max, max)
        match self.select_video_mode_rec(
            resolution,
            bit_depth,
            refresh_rate_millihertz,
            None,
            None,
            None,
        ) {
            Some(mode) => Some(mode),
            // if there is no video mode with the specified resolution,
            // fall back to the video mode with max resolution, bit depth and refresh rate
            None => {
                warn!(
                    "Resolution specified in settings is incompatible with the monitor. Choosing \
                     highest resolution possible instead."
                );
                if let Some(monitor) = self.window.current_monitor() {
                    let mode = monitor
                        .video_modes()
                        // Prefer bit depth over refresh rate
                        .sorted_by_key(|mode| mode.refresh_rate_millihertz())
                        .sorted_by_key(|mode| mode.bit_depth())
                        .max_by_key(|mode| mode.size().width);

                    if mode.is_none() {
                        warn!("Failed to select video mode, no video modes available!!")
                    }

                    mode
                } else {
                    warn!("Failed to select video mode, can't get the current monitor!");
                    None
                }
            },
        }
    }

    pub fn set_fullscreen_mode(&mut self, fullscreen: FullScreenSettings) {
        let window = &self.window;
        self.fullscreen = fullscreen;
        window.set_fullscreen(fullscreen.enabled.then(|| match fullscreen.mode {
            FullscreenMode::Exclusive => {
                if let Some(video_mode) = self.select_video_mode(
                    fullscreen.resolution,
                    fullscreen.bit_depth,
                    fullscreen.refresh_rate_millihertz,
                ) {
                    winit::window::Fullscreen::Exclusive(video_mode)
                } else {
                    warn!(
                        "Failed to select a video mode for exclusive fullscreen. Falling back to \
                         borderless fullscreen."
                    );
                    winit::window::Fullscreen::Borderless(None)
                }
            },
            FullscreenMode::Borderless => {
                // None here will fullscreen on the current monitor
                winit::window::Fullscreen::Borderless(None)
            },
        }));
    }

    pub fn needs_refresh_resize(&mut self) { self.needs_refresh_resize = true; }

    pub fn set_size(&mut self, new_size: Vec2<u32>) {
        self.window
            .set_min_inner_size(Some(winit::dpi::LogicalSize::new(
                new_size.x as f64,
                new_size.y as f64,
            )));
    }

    pub fn send_event(&mut self, event: Event) { self.events.push(event) }

    pub fn take_screenshot(&mut self, settings: &Settings) {
        let sender = self.message_sender.clone();
        let mut path = settings.screenshots_path.clone();
        self.renderer.create_screenshot(move |image| {
            use std::time::SystemTime;

            // Handle any error if there was one when generating the image.
            let image = match image {
                Ok(i) => i,
                Err(e) => {
                    warn!(?e, "Couldn't generate screenshot");
                    let _result = sender.send(format!("Error when generating screenshot: {}", e));
                    return;
                },
            };

            // Check if folder exists and create it if it does not
            if !path.exists()
                && let Err(e) = std::fs::create_dir_all(&path)
            {
                warn!(?e, ?path, "Couldn't create folder for screenshot");
                let _result = sender.send(String::from("Couldn't create folder for screenshot"));
            }
            path.push(format!(
                "screenshot_{}.png",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ));
            // Try to save the image
            if let Err(e) = image.save(&path) {
                warn!(?e, ?path, "Couldn't save screenshot");
                let _result = sender.send(String::from("Couldn't save screenshot"));
            } else {
                let _result =
                    sender.send(format!("Screenshot saved to {}", path.to_string_lossy()));
            }
        });
    }

    fn is_pressed(
        map: &mut HashMap<GameInput, winit::event::ElementState>,
        input: GameInput,
    ) -> bool {
        *(map
            .entry(input)
            .or_insert(winit::event::ElementState::Released))
            == winit::event::ElementState::Pressed
    }

    fn set_pressed(
        map: &mut HashMap<GameInput, winit::event::ElementState>,
        input: GameInput,
        state: winit::event::ElementState,
    ) {
        map.insert(input, state);
    }

    // Function used to handle Mouse and Key events. It first checks if we're in
    // remapping mode for a specific GameInput. If we are, we modify the binding
    // of that GameInput with the KeyMouse passed. Else, we return an iterator of
    // the GameInputs for that KeyMouse.
    fn map_input<'a>(
        key_mouse: KeyMouse,
        controls: &'a mut ControlSettings,
        remapping: &mut Option<GameInput>,
        last_input: &mut LastInput,
    ) -> Option<impl Iterator<Item = &'a GameInput> + use<'a>> {
        let key_mouse = key_mouse.into_upper();

        // update last input to be Keyboard/Mouse if button was pressed
        *last_input = LastInput::KeyboardMouse;

        match *remapping {
            // TODO: save settings
            Some(game_input) => {
                controls.modify_binding(game_input, key_mouse);
                *remapping = None;
                None
            },
            None => controls
                .get_associated_game_inputs(&key_mouse)
                .map(|game_inputs| game_inputs.iter()),
        }
    }

    // same thing as map_input, but for controller buttons
    fn map_controller_button_input<'a>(
        button: Button,
        controller: &'a mut ControllerSettings,
        remapping: &mut Option<GameInput>,
        last_input: &mut LastInput,
    ) -> Option<impl Iterator<Item = &'a GameInput> + use<'a>> {
        // update last input to be controller
        *last_input = LastInput::Controller;

        match *remapping {
            Some(game_input) => {
                controller.modify_button_binding(game_input, button);
                *remapping = None;
                None
            },
            None => controller
                .get_associated_game_button_inputs(&button)
                .map(|game_inputs| game_inputs.iter()),
        }
    }

    // I'll combine all of these eventually
    fn map_controller_layer_input<'a>(
        layers: LayerEntry,
        controller: &'a mut ControllerSettings,
        remapping: &mut Option<GameInput>,
        last_input: &mut LastInput,
        mod1_input: bool,
        mod2_input: bool,
    ) -> Option<impl Iterator<Item = &'a GameInput> + use<'a>> {
        // update last input to be controller
        *last_input = LastInput::Controller;

        match *remapping {
            Some(game_input) => {
                // create a new layerentry
                let new_layer_entry = LayerEntry {
                    button: layers.button, // use the pressed button
                    // mod1: Button::Simple(GilButton::RightTrigger),
                    // mod2: Button::Simple(GilButton::Unknown),
                    mod1: if mod1_input {
                        Button::Simple(GilButton::RightTrigger)
                    } else {
                        Button::Simple(GilButton::Unknown)
                    },
                    mod2: if mod2_input {
                        Button::Simple(GilButton::LeftTrigger)
                    } else {
                        Button::Simple(GilButton::Unknown)
                    },
                };

                controller.modify_layer_binding(game_input, new_layer_entry);
                *remapping = None;
                None
            },
            None => controller
                .get_associated_game_layer_inputs(&layers)
                .map(|layer_inputs| layer_inputs.iter()),
        }
    }

    pub fn set_keybinding_mode(&mut self, game_input: GameInput) {
        self.remapping_keybindings = Some(game_input);
    }

    pub fn window(&self) -> &winit::window::Window { &self.window }

    pub fn modifiers(&self) -> winit::keyboard::ModifiersState { self.modifiers }

    pub fn scale_factor(&self) -> f64 { self.scale_factor }

    pub fn last_input(&self) -> LastInput { self.last_input }
}

#[derive(Default, Copy, Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum FullscreenMode {
    Exclusive,
    #[serde(other)]
    #[default]
    Borderless,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSettings {
    pub size: [u32; 2],
    pub maximised: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            size: [1280, 720],
            maximised: false,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FullScreenSettings {
    pub enabled: bool,
    pub mode: FullscreenMode,
    pub resolution: [u16; 2],
    pub bit_depth: Option<u16>,
    pub refresh_rate_millihertz: Option<u32>,
}

impl Default for FullScreenSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: FullscreenMode::Borderless,
            resolution: [1920, 1080],
            bit_depth: None,
            refresh_rate_millihertz: None,
        }
    }
}
