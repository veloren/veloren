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

/// Whether a key is pressed or unpressed
/// and how long it has been in that state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Input {
    /// Should not be pub because duration should
    /// always be reset when state is updated
    state: bool,
    /// Should only be updated by npc agents
    /// through appropriate fn
    duration: Duration,
    /// How many update ticks the button has been in its current state for
    ticks_held: u32,
}

impl Input {
    fn tick(&mut self, old: Input, dt: Duration) {
        // Increase how long input has been in current state
        self.duration = self.duration.checked_add(dt).unwrap_or_default();

        match (self.is_pressed(), old.is_pressed()) {
            (false, true) | (true, false) => {
                println!("{:?}", self);
                self.duration = Duration::default();
                self.ticks_held = 1;
                println!("{:?}", self);
            },
            (_, _) => {
                self.ticks_held += 1;
                println!("____");
            },
        };
    }

    /// Whether input is being pressed down
    pub fn is_pressed(&self) -> bool { self.state == true }

    /// Whether it's the first frame this input has been pressed
    pub fn is_just_pressed(&self) -> bool { self.is_pressed() && self.ticks_held == 1 }

    /// Whether it's the first frame this input has been unpressed
    pub fn is_just_unpressed(&self) -> bool { !self.is_pressed() && self.ticks_held == 1 }

    /// Whether input has been pressed longer than
    /// `DEFAULT_HOLD_DURATION`
    pub fn is_held_down(&self) -> bool {
        self.is_pressed() && self.duration >= DEFAULT_HOLD_DURATION
    }

    /// Whether input has been unpressed longer than
    /// `DEFAULT_HOLD_DURATION`
    pub fn is_held_up(&self) -> bool {
        !self.is_pressed() && self.duration >= DEFAULT_HOLD_DURATION
    }

    /// Whether input has been pressed for longer than `threshold` duration
    pub fn held_for_dur(&self, threshold: Duration) -> bool {
        self.is_pressed() && self.duration >= threshold
    }

    /// Whether input has been pressed for longer than `count` number of ticks
    pub fn held_for_ticks(&self, count: u32) -> bool {
        self.is_pressed() && self.ticks_held >= count
    }

    /// Handles logic of updating state of Input
    pub fn set_state(&mut self, new_state: bool) { self.state = new_state; }

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
            state: false,
            duration: Duration::default(),
            ticks_held: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControllerInputs {
    pub primary: Input,
    pub secondary: Input,
    pub ability3: Input,
    pub sit: Input,
    pub jump: Input,
    pub roll: Input,
    pub glide: Input,
    pub climb: Input,
    pub climb_down: Input,
    pub wall_leap: Input,
    pub respawn: Input,
    pub toggle_wield: Input,
    pub swap_loadout: Input,
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
    pub fn calculate_change(&mut self, old: ControllerInputs, dt: Duration) {
        self.primary.tick(old.primary, dt);
        self.secondary.tick(old.secondary, dt);
        self.ability3.tick(old.ability3, dt);
        self.sit.tick(old.sit, dt);
        self.jump.tick(old.jump, dt);
        self.roll.tick(old.roll, dt);
        self.glide.tick(old.glide, dt);
        self.climb.tick(old.climb, dt);
        self.climb_down.tick(old.climb_down, dt);
        self.wall_leap.tick(old.wall_leap, dt);
        self.respawn.tick(old.respawn, dt);
        self.toggle_wield.tick(old.toggle_wield, dt);
        self.swap_loadout.tick(old.swap_loadout, dt);
        self.charge.tick(old.charge, dt);
    }

    pub fn holding_ability_key(&self) -> bool {
        self.primary.is_pressed() || self.secondary.is_pressed() || self.ability3.is_pressed()
    }
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
