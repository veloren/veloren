use crate::{
    comp::{
        inventory::slot::{EquipSlot, InvSlotId, Slot},
        invite::{InviteKind, InviteResponse},
        BuffKind,
    },
    trade::{TradeAction, TradeId},
    uid::Uid,
    util::Dir,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;
use vek::*;

/// Default duration before an input is considered 'held'.
pub const DEFAULT_HOLD_DURATION: Duration = Duration::from_millis(200);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InventoryEvent {
    Pickup(Uid),
    Collect(Vec3<i32>),
    Swap(InvSlotId, InvSlotId),
    SplitSwap(InvSlotId, InvSlotId),
    Drop(InvSlotId),
    SplitDrop(InvSlotId),
    CraftRecipe(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InventoryAction {
    Swap(EquipSlot, Slot),
    Drop(EquipSlot),
    Use(Slot),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InventoryManip {
    Pickup(Uid),
    Collect(Vec3<i32>),
    Use(Slot),
    Swap(Slot, Slot),
    SplitSwap(Slot, Slot),
    Drop(Slot),
    SplitDrop(Slot),
    CraftRecipe(String),
}

impl From<InventoryAction> for InventoryManip {
    fn from(inv_action: InventoryAction) -> Self {
        match inv_action {
            InventoryAction::Use(slot) => Self::Use(slot),
            InventoryAction::Swap(equip, slot) => Self::Swap(Slot::Equip(equip), slot),
            InventoryAction::Drop(equip) => Self::Drop(Slot::Equip(equip)),
        }
    }
}

impl From<InventoryEvent> for InventoryManip {
    fn from(inv_event: InventoryEvent) -> Self {
        match inv_event {
            InventoryEvent::Pickup(pickup) => Self::Pickup(pickup),
            InventoryEvent::Collect(collect) => Self::Collect(collect),
            InventoryEvent::Swap(inv1, inv2) => {
                Self::Swap(Slot::Inventory(inv1), Slot::Inventory(inv2))
            },
            InventoryEvent::SplitSwap(inv1, inv2) => {
                Self::SplitSwap(Slot::Inventory(inv1), Slot::Inventory(inv2))
            },
            InventoryEvent::Drop(inv) => Self::Drop(Slot::Inventory(inv)),
            InventoryEvent::SplitDrop(inv) => Self::SplitDrop(Slot::Inventory(inv)),
            InventoryEvent::CraftRecipe(recipe) => Self::CraftRecipe(recipe),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GroupManip {
    Leave,
    Kick(Uid),
    AssignLeader(Uid),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ControlEvent {
    //ToggleLantern,
    EnableLantern,
    DisableLantern,
    Interact(Uid),
    InitiateInvite(Uid, InviteKind),
    InviteResponse(InviteResponse),
    PerformTradeAction(TradeId, TradeAction),
    Mount(Uid),
    Unmount,
    InventoryEvent(InventoryEvent),
    GroupManip(GroupManip),
    RemoveBuff(BuffKind),
    Respawn,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ControlAction {
    SwapEquippedWeapons,
    InventoryAction(InventoryAction),
    Wield,
    GlideWield,
    Unwield,
    Sit,
    Dance,
    Sneak,
    Stand,
    Talk,
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
    /// Used to update inputs with input received from clients
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
    pub ability4: Input,
    pub jump: Input,
    pub roll: Input,
    pub glide: Input,
    pub fly: Input, // Flying entities only
    pub wall_leap: Input,
    pub charge: Input,
    pub climb: Option<Climb>,
    pub move_dir: Vec2<f32>,
    pub move_z: f32, /* z axis (not combined with move_dir because they may have independent
                      * limits) */
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
        self.ability4.tick(dt);
        self.jump.tick(dt);
        self.roll.tick(dt);
        self.glide.tick(dt);
        self.fly.tick(dt);
        self.wall_leap.tick(dt);
        self.charge.tick(dt);
    }

    pub fn tick_freshness(&mut self) {
        self.primary.tick_freshness();
        self.secondary.tick_freshness();
        self.ability3.tick_freshness();
        self.ability4.tick_freshness();
        self.jump.tick_freshness();
        self.roll.tick_freshness();
        self.glide.tick_freshness();
        self.fly.tick_freshness();
        self.wall_leap.tick_freshness();
        self.charge.tick_freshness();
    }

    /// Updates Controller inputs with new version received from the client
    pub fn update_with_new(&mut self, new: Self) {
        self.primary.update_with_new(new.primary);
        self.secondary.update_with_new(new.secondary);
        self.ability3.update_with_new(new.ability3);
        self.ability4.update_with_new(new.ability4);
        self.jump.update_with_new(new.jump);
        self.roll.update_with_new(new.roll);
        self.glide.update_with_new(new.glide);
        self.fly.update_with_new(new.fly);
        self.wall_leap.update_with_new(new.wall_leap);
        self.charge.update_with_new(new.charge);
        self.climb = new.climb;
        self.move_dir = new.move_dir;
        self.move_z = new.move_z;
        self.look_dir = new.look_dir;
    }

    pub fn holding_ability_key(&self) -> bool {
        self.primary.is_pressed()
            || self.secondary.is_pressed()
            || self.ability3.is_pressed()
            || self.ability4.is_pressed()
    }
}

impl Controller {
    /// Sets all inputs to default
    pub fn reset(&mut self) { *self = Self::default(); }

    pub fn clear_events(&mut self) { self.events.clear(); }

    pub fn push_event(&mut self, event: ControlEvent) { self.events.push(event); }
}

impl Component for Controller {
    type Storage = IdvStorage<Self>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MountState {
    Unmounted,
    MountedBy(Uid),
}

impl Component for MountState {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mounting(pub Uid);

impl Component for Mounting {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
