use self::ActionState::*;
use super::states::*;
use crate::{
    comp::{Body, ControllerInputs, Ori, PhysicsState, Pos, Stats, Vel},
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
};
use serde::Deserialize;
use serde::Serialize;
use specs::LazyUpdate;
use specs::{Component, Entity, FlaggedStorage, HashMapStorage, NullStorage};
use sphynx::Uid;
use std::time::Duration;

pub struct EcsStateData<'a> {
    pub entity: &'a Entity,
    pub uid: &'a Uid,
    pub character: &'a CharacterState,
    pub pos: &'a Pos,
    pub vel: &'a Vel,
    pub ori: &'a Ori,
    pub dt: &'a DeltaTime,
    pub inputs: &'a ControllerInputs,
    pub stats: &'a Stats,
    pub body: &'a Body,
    pub physics: &'a PhysicsState,
    pub updater: &'a LazyUpdate,
    pub server_bus: &'a EventBus<ServerEvent>,
    pub local_bus: &'a EventBus<LocalEvent>,
}

pub struct StateUpdate {
    pub character: CharacterState,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum MoveState {
    Stand(Option<stand::State>),
    Run(Option<run::State>),
    Sit(Option<sit::State>),
    Jump(Option<jump::State>),
    Fall(Option<fall::State>),
    Glide(Option<glide::State>),
    Swim(Option<swim::State>),
    Climb(Option<climb::State>),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum ActionState {
    Idle(Option<idle::State>),
    Wield(Option<wield::State>),
    Attack(AttackKind),
    Block(BlockKind),
    Dodge(DodgeKind),
    // Interact?,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum AttackKind {
    BasicAttack(Option<basic_attack::State>),
    Charge(Option<charge_attack::State>),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum BlockKind {
    BasicBlock(Option<basic_block::State>),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum DodgeKind {
    Roll(Option<roll::State>),
}

impl ActionState {
    pub fn is_equip_finished(&self) -> bool {
        match self {
            Wield(Some(wield::State { equip_delay })) => *equip_delay == Duration::default(),
            _ => true,
        }
    }

    /// Returns the current `equip_delay` if in `WieldState`, otherwise `Duration::default()`
    pub fn get_delay(&self) -> Duration {
        match self {
            Wield(Some(wield::State { equip_delay })) => *equip_delay,
            _ => Duration::default(),
        }
    }

    pub fn is_attacking(&self) -> bool {
        match self {
            Attack(_) => true,
            _ => false,
        }
    }

    pub fn is_blocking(&self) -> bool {
        match self {
            Block(_) => true,
            _ => false,
        }
    }

    pub fn is_dodging(&self) -> bool {
        match self {
            Dodge(_) => true,
            _ => false,
        }
    }
    /// Compares `action_state`s for shallow equality (does not check internal struct equality)
    pub fn equals(&self, other: &Self) -> bool {
        // Check if state is the same without looking at the inner data
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

impl MoveState {
    /// Compares `move_state`s for shallow equality (does not check internal struct equality)
    pub fn equals(&self, other: &Self) -> bool {
        // Check if state is the same without looking at the inner data
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

/// __A concurrent state machine that allows for separate `ActionState`s and `MoveState`s.__
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct CharacterState {
    /// __How the character is currently moving, e.g. Running, Standing, Falling.__
    ///
    /// _Primarily `handle()`s updating `Pos`, `Vel`, `Ori`, and lower body animations.
    pub move_state: MoveState,

    /// __How the character is currently acting, e.g. Wielding, Attacking, Dodging.__
    ///
    /// _Primarily `handle()`s how character interacts with world, and upper body animations.
    pub action_state: ActionState,
}

impl CharacterState {
    /// Compares both `move_state`s and `action_state`a for shallow equality
    /// (does not check internal struct equality)
    pub fn equals(&self, other: &Self) -> bool {
        self.move_state.equals(&other.move_state) && self.action_state.equals(&other.action_state)
    }
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            move_state: MoveState::Fall(None),
            action_state: ActionState::Idle(None),
        }
    }
}

impl Component for CharacterState {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct OverrideState;
impl Component for OverrideState {
    type Storage = FlaggedStorage<Self, NullStorage<Self>>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct OverrideAction;
impl Component for OverrideAction {
    type Storage = FlaggedStorage<Self, NullStorage<Self>>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct OverrideMove;
impl Component for OverrideMove {
    type Storage = FlaggedStorage<Self, NullStorage<Self>>;
}
