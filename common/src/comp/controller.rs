use crate::sync::Uid;
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use std::time::Duration;
use vek::*;

/// Default duration before an input is considered 'held'.
pub const DEFAULT_HOLD_DURATION: Duration = Duration::from_millis(200);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ControlEvent {
    Mount(Uid),
    Unmount,
    InventoryManip(InventoryManip),
    //Respawn,
}

/// The various states an input can be in
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputState {
    Pressed,
    Unpressed,
}

/// Whether a key is pressed or unpressed
/// and how long it has been in that state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Input {
    /// Should not be pub because duration should
    /// always be reset when state is updated
    state: InputState,
    /// Should only be updated by npc agents
    /// through appropriate fn
    duration: Duration,
    /// Turned off first tick after switching states
    just_changed: bool,
    /// Set when `set_state` is called. Needed so
    /// tick after change doesn't immediately unset `just_changed`
    dirty: bool,
}

impl Input {
    fn tick(&mut self, dt: Duration) {
        // Increase how long input has been in current state
        self.duration = self.duration.checked_add(dt).unwrap_or_default();
        if self.dirty {
            // Unset dirty first tick after changing into current state
            self.dirty = false;
        } else {
            // Otherwise, just changed is always false
            self.just_changed = false;
        }
    }

    /// Whether input is in `InputState::Pressed` state
    pub fn is_pressed(&self) -> bool { self.state == InputState::Pressed }

    /// Whether it's the first frame this input has been in
    /// its current state
    pub fn is_just_pressed(&self) -> bool { (self.just_changed && self.is_pressed()) }

    /// Whether input has been in current state longer than
    /// `DEFAULT_HOLD_DURATION`
    pub fn is_held_down(&self) -> bool {
        (self.is_pressed() && self.duration >= DEFAULT_HOLD_DURATION)
    }

    /// Whether input has been pressed for longer than `threshold`
    pub fn is_long_press(&self, threshold: Duration) -> bool {
        (self.is_pressed() && self.duration >= threshold)
    }

    /// Handles logic of updating state of Input
    pub fn set_state(&mut self, new_state: bool) {
        // Only update if state switches
        match (self.is_pressed(), new_state) {
            (false, true) => {
                self.just_changed = true;
                self.dirty = true;
                self.state = InputState::Pressed;
                self.duration = Duration::default();
            },
            (true, false) => {
                self.just_changed = true;
                self.dirty = true;
                self.state = InputState::Unpressed;
                self.duration = Duration::default();
            },
            (_, _) => {},
        };
    }

    /// Increases `input::duration` by `dur`
    pub fn inc_dur(&mut self, dur: Duration) {
        self.duration = self.duration.checked_add(dur).unwrap_or_default();
    }

    /// Returns `input::duration`
    pub fn get_dur(&self) -> Duration { self.duration }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            state: InputState::Unpressed,
            duration: Duration::default(),
            just_changed: false,
            dirty: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControllerInputs {
    // When adding new inputs:
    // 1. Add to tick() update
    pub primary: Input,
    pub secondary: Input,
    pub sit: Input,
    pub jump: Input,
    pub roll: Input,
    pub glide: Input,
    pub climb: Input,
    pub climb_down: Input,
    pub wall_leap: Input,
    pub respawn: Input,
    pub toggle_wield: Input,
    pub charge: Input,
    pub move_dir: Vec2<f32>,
    pub look_dir: Vec3<f32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub inputs: ControllerInputs,
    // TODO: consider SmallVec
    pub events: Vec<ControlEvent>,
}

impl ControllerInputs {
    /// Updates all inputs, accounting for delta time
    pub fn tick(&mut self, dt: Duration) {
        self.primary.tick(dt);
        self.secondary.tick(dt);
        self.sit.tick(dt);
        self.jump.tick(dt);
        self.roll.tick(dt);
        self.glide.tick(dt);
        self.climb.tick(dt);
        self.climb_down.tick(dt);
        self.wall_leap.tick(dt);
        self.respawn.tick(dt);
        self.toggle_wield.tick(dt);
        self.charge.tick(dt);
    }

    /*
    /// Updates `inputs.move_dir`.
    pub fn update_move_dir(&mut self) {
        self.move_dir = if self.move_dir.magnitude_squared() > 1.0 {
            // Cap move_dir to 1
            self.move_dir.normalized()
        } else {
            self.move_dir
        };
    }

    /// Updates `inputs.look_dir`
    pub fn update_look_dir(&mut self) {
        self.look_dir
            .try_normalized()
            .unwrap_or(self.move_dir.into());
    }*/
}

impl Controller {
    /// Sets all inputs to default
    pub fn reset(&mut self) { *self = Self::default(); }

    pub fn clear_events(&mut self) { self.events.clear(); }

    pub fn push_event(&mut self, event: ControlEvent) { self.events.push(event); }
}

impl Component for Controller {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MountState {
    Unmounted,
    MountedBy(Uid),
}

impl Component for MountState {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mounting(pub Uid);

impl Component for Mounting {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InventoryManip {
    Pickup(Uid),
    Collect(Vec3<i32>),
    Use(usize),
    Swap(usize, usize),
    Drop(usize),
}
