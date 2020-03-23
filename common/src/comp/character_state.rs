use crate::{
    comp::{Energy, Loadout, Ori, Pos, Vel},
    event::{LocalEvent, ServerEvent},
    states::*,
    sys::character_behavior::JoinData,
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage, HashMapStorage, VecStorage};
use std::collections::VecDeque;

/// Data returned from character behavior fn's to Character Behavior System.
pub struct StateUpdate {
    pub character: CharacterState,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
    pub energy: Energy,
    pub loadout: Loadout,
    pub local_events: VecDeque<LocalEvent>,
    pub server_events: VecDeque<ServerEvent>,
}

impl From<&JoinData<'_>> for StateUpdate {
    fn from(data: &JoinData) -> Self {
        StateUpdate {
            pos: *data.pos,
            vel: *data.vel,
            ori: *data.ori,
            energy: *data.energy,
            loadout: data.loadout.clone(),
            character: data.character.clone(),
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CharacterState {
    Idle,
    Climb,
    Sit,
    Glide,
    /// A basic blocking state
    BasicBlock,
    /// Player is busy equipping or unequipping weapons
    Equipping(equipping::Data),
    /// Player is holding a weapon and can perform other actions
    Wielding,
    /// A dodge where player can roll
    Roll(roll::Data),
    /// A basic melee attack (e.g. sword)
    BasicMelee(basic_melee::Data),
    /// A basic ranged attack (e.g. bow)
    BasicRanged(basic_ranged::Data),
    /// Cast a fireball
    CastFireball(cast_fireball::Data),
    /// A force will boost you into a direction for some duration
    Boost(boost::Data),
    /// Dash forward and then attack
    DashMelee(dash_melee::Data),
    /// A three-stage attack where play must click at appropriate times
    /// to continue attack chain.
    TimedCombo(timed_combo::Data),
    /// A three-stage attack where each attack pushes player forward
    /// and successive attacks increase in damage, while player holds button.
    TripleStrike(triple_strike::Data),
}

impl CharacterState {
    pub fn is_wield(&self) -> bool {
        match self {
            CharacterState::Wielding
            | CharacterState::BasicMelee(_)
            | CharacterState::BasicRanged(_)
            | CharacterState::CastFireball(_)
            | CharacterState::DashMelee(_)
            | CharacterState::TripleStrike(_)
            | CharacterState::TimedCombo(_)
            | CharacterState::BasicBlock => true,
            _ => false,
        }
    }

    pub fn can_swap(&self) -> bool {
        match self {
            CharacterState::Wielding => true,
            _ => false,
        }
    }

    pub fn is_attack(&self) -> bool {
        match self {
            CharacterState::BasicMelee(_)
            | CharacterState::BasicRanged(_)
            | CharacterState::CastFireball(_)
            | CharacterState::TimedCombo(_)
            | CharacterState::DashMelee(_)
            | CharacterState::TripleStrike(_) => true,
            _ => false,
        }
    }

    pub fn is_block(&self) -> bool {
        match self {
            CharacterState::BasicBlock => true,
            _ => false,
        }
    }

    pub fn is_dodge(&self) -> bool {
        match self {
            CharacterState::Roll(_) => true,
            _ => false,
        }
    }

    /// Compares for shallow equality (does not check internal struct equality)
    pub fn same_variant(&self, other: &Self) -> bool {
        // Check if state is the same without looking at the inner data
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Default for CharacterState {
    fn default() -> Self { Self::Idle }
}

impl Component for CharacterState {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attacking {
    pub base_damage: u32,
    pub range: f32,
    pub max_angle: f32,
    pub applied: bool,
    pub hit_count: u32,
}

impl Component for Attacking {
    type Storage = VecStorage<Self>;
}
