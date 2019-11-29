use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use sphynx::Uid;
use std::ops::Add;
use std::time::Duration;
use vek::*;

/// Default duration for how long before an input is considered 'held'.
pub const DEFAULT_HOLD_DURATION: Duration = Duration::from_millis(250);

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
    // Should not be pub because duration should
    // always be reset when state is updated
    state: InputState,
    // Should only be updated by npc agents
    // through appropriate fn
    duration: Duration,
}

impl Input {
    /// Whether input is in `InputState::Pressed` state
    pub fn is_pressed(&self) -> bool {
        self.state == InputState::Pressed
    }
    /// Whether input has been in current state longer than
    /// `DEFAULT_HOLD_DURATION`
    pub fn is_held_down(&self) -> bool {
        (self.is_pressed() && self.duration >= DEFAULT_HOLD_DURATION)
    }

    /// Sets the `input::state` and resets `input::duration`
    ///
    ///
    /// `new_state` == `true` -> `InputState::Pressed`
    ///
    /// `new_state` == `false` -> `InputState::Unpressed`
    pub fn set_state(&mut self, new_state: bool) {
        // Only update if state switches
        match (new_state, self.is_pressed()) {
            (true, false) => {
                self.state = InputState::Pressed;
                self.duration = Duration::default();
            }
            (false, true) => {
                self.state = InputState::Unpressed;
                self.duration = Duration::default();
            }
            (_, _) => {}
        };
    }

    /// Sets `input::duration`
    pub fn inc_dur(&mut self, dur: Duration) {
        self.duration = self.duration + dur;
    }

    /// Returns `input::duration`
    pub fn get_dur(&self) -> Duration {
        self.duration
    }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            state: InputState::Unpressed,
            duration: Duration::default(),
        }
    }
}

impl Add<Duration> for Input {
    type Output = Self;

    fn add(self, dur: Duration) -> Self {
        Self {
            state: self.state,
            duration: self.duration.checked_add(dur).unwrap_or_default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControllerInputs {
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
        self.primary = self.primary + dt;
        self.secondary = self.secondary + dt;
        self.sit = self.sit + dt;
        self.jump = self.jump + dt;
        self.roll = self.roll + dt;
        self.glide = self.glide + dt;
        self.climb = self.climb + dt;
        self.climb_down = self.climb_down + dt;
        self.wall_leap = self.wall_leap + dt;
        self.respawn = self.respawn + dt;
        self.toggle_wield = self.toggle_wield + dt;
    }
}

impl Controller {
    /// Sets all inputs to default
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn push_event(&mut self, event: ControlEvent) {
        self.events.push(event);
    }
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
