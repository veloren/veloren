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
use std::collections::BTreeMap;
use vek::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InventoryEvent {
    Pickup(Uid),
    Collect(Vec3<i32>),
    Swap(InvSlotId, InvSlotId),
    SplitSwap(InvSlotId, InvSlotId),
    Drop(InvSlotId),
    SplitDrop(InvSlotId),
    Sort,
    CraftRecipe {
        recipe: String,
        craft_sprite: Option<Vec3<i32>>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InventoryAction {
    Swap(EquipSlot, Slot),
    Drop(EquipSlot),
    Use(Slot),
    Sort,
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
    Sort,
    CraftRecipe {
        recipe: String,
        craft_sprite: Option<Vec3<i32>>,
    },
}

impl From<InventoryAction> for InventoryManip {
    fn from(inv_action: InventoryAction) -> Self {
        match inv_action {
            InventoryAction::Use(slot) => Self::Use(slot),
            InventoryAction::Swap(equip, slot) => Self::Swap(Slot::Equip(equip), slot),
            InventoryAction::Drop(equip) => Self::Drop(Slot::Equip(equip)),
            InventoryAction::Sort => Self::Sort,
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
            InventoryEvent::Sort => Self::Sort,
            InventoryEvent::CraftRecipe {
                recipe,
                craft_sprite,
            } => Self::CraftRecipe {
                recipe,
                craft_sprite,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GroupManip {
    Leave,
    Kick(Uid),
    AssignLeader(Uid),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UtteranceKind {
    Calm,
    Angry,
    Surprised,
    Hurt,
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
    Utterance(UtteranceKind),
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
    StartInput {
        input: InputKind,
        target_entity: Option<Uid>,
        // Some inputs need a selected position, such as mining
        select_pos: Option<Vec3<f32>>,
    },
    CancelInput(InputKind),
}

impl ControlAction {
    pub fn basic_input(input: InputKind) -> Self {
        ControlAction::StartInput {
            input,
            target_entity: None,
            select_pos: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Ord, PartialOrd)]
#[repr(u32)]
pub enum InputKind {
    Primary = 0,
    Secondary = 1,
    Block = 2,
    Ability(usize) = 3,
    Roll = 4,
    Jump = 5,
    Fly = 6,
}

impl InputKind {
    pub fn is_ability(self) -> bool {
        matches!(
            self,
            Self::Primary | Self::Secondary | Self::Ability(_) | Self::Block
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputAttr {
    pub select_pos: Option<Vec3<f32>>,
    pub target_entity: Option<Uid>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Climb {
    Up,
    Down,
    Hold,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControllerInputs {
    pub climb: Option<Climb>,
    pub move_dir: Vec2<f32>,
    pub move_z: f32, /* z axis (not combined with move_dir because they may have independent
                      * limits) */
    pub look_dir: Dir,
    pub select_pos: Option<Vec3<f32>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub inputs: ControllerInputs,
    pub queued_inputs: BTreeMap<InputKind, InputAttr>,
    // TODO: consider SmallVec
    pub events: Vec<ControlEvent>,
    pub actions: Vec<ControlAction>,
}

impl ControllerInputs {
    /// Updates Controller inputs with new version received from the client
    pub fn update_with_new(&mut self, new: Self) {
        self.climb = new.climb;
        self.move_dir = new.move_dir;
        self.move_z = new.move_z;
        self.look_dir = new.look_dir;
        self.select_pos = new.select_pos;
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
