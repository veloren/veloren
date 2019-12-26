use self::ActionState::*;
use super::states::*;
use crate::{
    comp::{Body, ControllerInputs, Ori, PhysicsState, Pos, Stats, Vel},
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
};
use specs::LazyUpdate;
use specs::{Component, Entity, FlaggedStorage, HashMapStorage, NullStorage};
use sphynx::Uid;
use std::time::Duration;

pub struct ECSStateData<'a> {
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

pub struct ECSStateUpdate {
    pub character: CharacterState,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum MoveState {
    Stand(StandHandler),
    Run(RunHandler),
    Sit(SitHandler),
    Jump(JumpHandler),
    Fall(FallHandler),
    Glide(GlideHandler),
    Swim(SwimHandler),
    Climb(ClimbHandler),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum ActionState {
    Idle,
    Wield(WieldHandler),
    Attack(AttackKind),
    Block(BlockKind),
    Dodge(DodgeKind),
    // Interact,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum AttackKind {
    BasicAttack(BasicAttackHandler),
    Charge(ChargeAttackHandler),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum BlockKind {
    BasicBlock(BasicBlockHandler),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum DodgeKind {
    Roll(RollHandler),
}

impl ActionState {
    pub fn is_equip_finished(&self) -> bool {
        match self {
            Wield(WieldHandler { equip_delay }) => *equip_delay == Duration::default(),
            _ => true,
        }
    }
    pub fn get_delay(&self) -> Duration {
        match self {
            Wield(WieldHandler { equip_delay }) => *equip_delay,
            _ => Duration::default(),
        }
    }

    pub fn is_attacking(&self) -> bool {
        match self {
            Block(_) => true,
            _ => false,
        }
    }

    pub fn is_blocking(&self) -> bool {
        match self {
            Attack(_) => true,
            _ => false,
        }
    }

    pub fn is_dodging(&self) -> bool {
        match self {
            Dodge(_) => true,
            _ => false,
        }
    }

    pub fn is_wielding(&self) -> bool {
        if let Wield(_) = self {
            true
        } else {
            false
        }
    }
    pub fn is_idling(&self) -> bool {
        if let Idle = self {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct CharacterState {
    pub move_state: MoveState,
    pub action_state: ActionState,
}

impl CharacterState {
    pub fn is_same_move_state(&self, other: &Self) -> bool {
        // Check if state is the same without looking at the inner data
        std::mem::discriminant(&self.move_state) == std::mem::discriminant(&other.move_state)
    }
    pub fn is_same_action_state(&self, other: &Self) -> bool {
        // Check if state is the same without looking at the inner data
        std::mem::discriminant(&self.action_state) == std::mem::discriminant(&other.action_state)
    }
    pub fn is_same_state(&self, other: &Self) -> bool {
        self.is_same_move_state(other) && self.is_same_action_state(other)
    }
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            move_state: MoveState::Fall(FallHandler),
            action_state: ActionState::Idle,
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
