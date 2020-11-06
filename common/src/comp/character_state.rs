use crate::{
    comp::{Energy, Ori, Pos, Vel},
    event::{LocalEvent, ServerEvent},
    states::*,
    sys::character_behavior::JoinData,
    Damage, GroupTarget, Knockback,
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage, VecStorage};
use specs_idvs::IdvStorage;
use std::collections::VecDeque;

/// Data returned from character behavior fn's to Character Behavior System.
pub struct StateUpdate {
    pub character: CharacterState,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
    pub energy: Energy,
    pub swap_loadout: bool,
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
            swap_loadout: false,
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
    Dance,
    Sneak,
    Glide,
    GlideWield,
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
    /// A force will boost you into a direction for some duration
    Boost(boost::Data),
    /// Dash forward and then attack
    DashMelee(dash_melee::Data),
    /// A three-stage attack where each attack pushes player forward
    /// and successive attacks increase in damage, while player holds button.
    ComboMelee(combo_melee::Data),
    /// A leap followed by a small aoe ground attack
    LeapMelee(leap_melee::Data),
    /// Spin around, dealing damage to enemies surrounding you
    SpinMelee(spin_melee::Data),
    /// A charged ranged attack (e.g. bow)
    ChargedRanged(charged_ranged::Data),
    /// A charged melee attack
    ChargedMelee(charged_melee::Data),
    /// A repeating ranged attack
    RepeaterRanged(repeater_ranged::Data),
    /// A ground shockwave attack
    Shockwave(shockwave::Data),
    /// A continuous attack that affects all creatures in a cone originating
    /// from the source
    BasicBeam(basic_beam::Data),
}

impl CharacterState {
    pub fn is_wield(&self) -> bool {
        matches!(self,
            CharacterState::Wielding
            | CharacterState::BasicMelee(_)
            | CharacterState::BasicRanged(_)
            | CharacterState::DashMelee(_)
            | CharacterState::ComboMelee(_)
            | CharacterState::BasicBlock
            | CharacterState::LeapMelee(_)
            | CharacterState::SpinMelee(_)
            | CharacterState::ChargedMelee(_)
            | CharacterState::ChargedRanged(_)
            | CharacterState::RepeaterRanged(_)
            | CharacterState::Shockwave(_)
            | CharacterState::BasicBeam(_)
        )
    }

    pub fn is_stealthy(&self) -> bool {
        matches!(self, CharacterState::Sneak | CharacterState::Roll(_))
    }

    pub fn is_attack(&self) -> bool {
        matches!(self,
            CharacterState::BasicMelee(_)
            | CharacterState::BasicRanged(_)
            | CharacterState::DashMelee(_)
            | CharacterState::ComboMelee(_)
            | CharacterState::LeapMelee(_)
            | CharacterState::SpinMelee(_)
            | CharacterState::ChargedMelee(_)
            | CharacterState::ChargedRanged(_)
            | CharacterState::RepeaterRanged(_)
            | CharacterState::Shockwave(_)
            | CharacterState::BasicBeam(_)
        )
    }

    pub fn is_aimed(&self) -> bool {
        matches!(self,
            CharacterState::BasicMelee(_)
            | CharacterState::BasicRanged(_)
            | CharacterState::DashMelee(_)
            | CharacterState::ComboMelee(_)
            | CharacterState::BasicBlock
            | CharacterState::LeapMelee(_)
            | CharacterState::ChargedMelee(_)
            | CharacterState::ChargedRanged(_)
            | CharacterState::RepeaterRanged(_)
            | CharacterState::Shockwave(_)
            | CharacterState::BasicBeam(_)
        )
    }

    pub fn is_block(&self) -> bool { matches!(self, CharacterState::BasicBlock) }

    pub fn is_dodge(&self) -> bool { matches!(self, CharacterState::Roll(_)) }

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
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attacking {
    pub damages: Vec<(Option<GroupTarget>, Damage)>,
    pub range: f32,
    pub max_angle: f32,
    pub applied: bool,
    pub hit_count: u32,
    pub knockback: Knockback,
}

impl Component for Attacking {
    type Storage = VecStorage<Self>;
}
