use crate::states::*;
use crate::{
    comp::{Body, ControllerInputs, Ori, PhysicsState, Pos, Stats, Vel},
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    sync::Uid,
};
use serde::Deserialize;
use serde::Serialize;
use specs::LazyUpdate;
use specs::{Component, Entity, FlaggedStorage, HashMapStorage};

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

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum CharacterState {
    Idle(Option<idle::State>),
    Climb(Option<climb::State>),
    Sit(Option<sit::State>),
    Wielding(Option<wielding::State>),
    Wielded(Option<wielded::State>),
    //BasicAttack(Option<basic_attack::State>),
    //BasicBlock(Option<basic_block::State>),
    //Charge(Option<charge_attack::State>),
    Roll(Option<roll::State>),
    Glide(Option<glide::State>),
}

impl CharacterState {
    pub fn is_attack(&self) -> bool {
        match self {
            //CharacterState::BasicAttack(_) => true,
            _ => false,
        }
    }

    pub fn is_block(&self) -> bool {
        match self {
            //CharacterState::BasicBlock(_) => true,
            _ => false,
        }
    }

    pub fn is_dodge(&self) -> bool {
        match self {
            CharacterState::Roll(_) => true,
            _ => false,
        }
    }

    /// Compares `action_state`s for shallow equality (does not check internal struct equality)
    pub fn equals(&self, other: &Self) -> bool {
        // Check if state is the same without looking at the inner data
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }

    /// Passes data to variant or subvariant handlers
    /// States contain `Option<StateHandler Implementor>`s, and will be
    /// `None` if state data has not been initialized. So we have to
    /// check and intialize new state data if so.
    pub fn update(&self, ecs_data: &EcsStateData) -> StateUpdate {
        match self {
            CharacterState::Idle(opt_state) => opt_state
                .unwrap_or_else(|| idle::State::new(ecs_data))
                .handle(ecs_data),
            CharacterState::Climb(opt_state) => opt_state
                .unwrap_or_else(|| climb::State::new(ecs_data))
                .handle(ecs_data),
            CharacterState::Sit(opt_state) => opt_state
                .unwrap_or_else(|| sit::State::new(ecs_data))
                .handle(ecs_data),
            CharacterState::Wielding(opt_state) => opt_state
                .unwrap_or_else(|| wielding::State::new(ecs_data))
                .handle(ecs_data),
            CharacterState::Wielded(opt_state) => opt_state
                .unwrap_or_else(|| wielded::State::new(ecs_data))
                .handle(ecs_data),
            /*CharacterState::BasicAttack(opt_state) => opt_state
                // If data hasn't been initialized, initialize a new one
                .unwrap_or_else(|| basic_attack::State::new(ecs_data))
                // Call handler
                .handle(ecs_data),
            CharacterState::Charge(opt_state) => opt_state
                .unwrap_or_else(|| charge_attack::State::new(ecs_data))
                .handle(ecs_data),
            CharacterState::BasicBlock(opt_state) => opt_state
                .unwrap_or_else(|| basic_block::State::new(ecs_data))
                .handle(ecs_data),*/
            CharacterState::Roll(opt_state) => opt_state
                .unwrap_or_else(|| roll::State::new(ecs_data))
                .handle(ecs_data),
            CharacterState::Glide(opt_state) => opt_state
                .unwrap_or_else(|| glide::State::new(ecs_data))
                .handle(ecs_data),
            // All states should be explicitly handled
            // Do not use default match: _ => {},
        }
    }
}

impl Default for CharacterState {
    fn default() -> Self {
        Self::Idle(None)
    }
}

impl Component for CharacterState {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
