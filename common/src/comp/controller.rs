use crate::{comp::inventory::slot::Slot, sync::Uid, util::Dir};
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use std::time::Duration;
use vek::*;

/// Default duration before an input is considered 'held'.
pub const DEFAULT_HOLD_DURATION: Duration = Duration::from_millis(200);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InventoryManip {
    Pickup(Uid),
    Collect(Vec3<i32>),
    Use(Slot),
    Swap(Slot, Slot),
    Drop(Slot),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ControlEvent {
    Mount(Uid),
    Unmount,
    InventoryManip(InventoryManip),
    Respawn,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ControlAction {
    SwapLoadout,
    Wield,
    Unwield,
    Sit,
    Stand,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Freshness {
    New,
    TickedOnce,
    Old,
}

/// Whether a key is pressed or unpressed
/// and how long it has been in that state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Input {
    /// Should not be pub because duration should
    /// always be reset when state is updated
    pressed: bool,
    /// Should only be updated by npc agents
    /// through appropriate fn
    duration: Duration,
    /// How fresh is the last change to the input state
    freshness: Freshness,
}

impl Input {
    fn tick(&mut self, dt: Duration) {
        // Increase how long input has been in current state
        self.duration = self.duration.checked_add(dt).unwrap_or_default();
        self.tick_freshness();
    }

    fn tick_freshness(&mut self) {
        self.freshness = match self.freshness {
            Freshness::New => Freshness::TickedOnce,
            Freshness::TickedOnce => Freshness::Old,
            Freshness::Old => Freshness::Old,
        };
    }

    /// Update input with newer version
    /// Used to update inputs with input recieved from clients
    pub fn update_with_new(&mut self, new: Self) {
        if self.pressed != new.pressed {
            self.freshness = Freshness::New;
        }

        self.pressed = new.pressed;
        self.duration = new.duration;
    }

    /// Whether input is being pressed down
    pub fn is_pressed(&self) -> bool { self.pressed }

    /// Whether it's the first frame this input has been pressed
    pub fn is_just_pressed(&self) -> bool { self.is_pressed() && self.freshness != Freshness::Old }

    /// Whether it's the first frame this input has been unpressed
    pub fn is_just_unpressed(&self) -> bool {
        !self.is_pressed() && self.freshness != Freshness::Old
    }

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

    /// Handles logic of updating state of Input
    pub fn set_state(&mut self, pressed: bool) {
        if self.pressed != pressed {
            self.pressed = pressed;
            self.duration = Duration::default();
            self.freshness = Freshness::New;
        }
    }

    /// Increases `input.duration` by `dur`
    pub fn inc_dur(&mut self, dur: Duration) {
        self.duration = self.duration.checked_add(dur).unwrap_or_default();
    }

    /// Returns `input.duration`
    pub fn get_dur(&self) -> Duration { self.duration }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            pressed: false,
            duration: Duration::default(),
            freshness: Freshness::New,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Climb {
    Up,
    Down,
    Hold,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControllerInputs {
    pub primary: Input,
    pub secondary: Input,
    pub ability3: Input,
    pub jump: Input,
    pub roll: Input,
    pub glide: Input,
    pub wall_leap: Input,
    pub charge: Input,
    pub climb: Option<Climb>,
    pub move_dir: Vec2<f32>,
    pub look_dir: Dir,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub inputs: ControllerInputs,
    // TODO: consider SmallVec
    pub events: Vec<ControlEvent>,
    pub actions: Vec<ControlAction>,
}

impl ControllerInputs {
    /// Updates all inputs, accounting for delta time
    pub fn tick(&mut self, dt: Duration) {
        self.primary.tick(dt);
        self.secondary.tick(dt);
        self.ability3.tick(dt);
        self.jump.tick(dt);
        self.roll.tick(dt);
        self.glide.tick(dt);
        self.wall_leap.tick(dt);
        self.charge.tick(dt);
    }

    pub fn tick_freshness(&mut self) {
        self.primary.tick_freshness();
        self.secondary.tick_freshness();
        self.ability3.tick_freshness();
        self.jump.tick_freshness();
        self.roll.tick_freshness();
        self.glide.tick_freshness();
        self.wall_leap.tick_freshness();
        self.charge.tick_freshness();
    }

    /// Updates Controller inputs with new version received from the client
    pub fn update_with_new(&mut self, new: Self) {
        self.primary.update_with_new(new.primary);
        self.secondary.update_with_new(new.secondary);
        self.ability3.update_with_new(new.ability3);
        self.jump.update_with_new(new.jump);
        self.roll.update_with_new(new.roll);
        self.glide.update_with_new(new.glide);
        self.wall_leap.update_with_new(new.wall_leap);
        self.charge.update_with_new(new.charge);
        self.climb = new.climb;
        self.move_dir = new.move_dir;
        self.look_dir = new.look_dir;
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
