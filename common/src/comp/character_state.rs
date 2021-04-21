use crate::{
    combat::Attack,
    comp::{tool::ToolKind, Density, Energy, InputAttr, InputKind, Ori, Pos, Vel},
    event::{LocalEvent, ServerEvent},
    states::{behavior::JoinData, *},
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, VecStorage};
use specs_idvs::IdvStorage;
use std::collections::{BTreeMap, VecDeque};
use vek::*;

/// Data returned from character behavior fn's to Character Behavior System.
pub struct StateUpdate {
    pub character: CharacterState,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
    pub density: Density,
    pub energy: Energy,
    pub swap_equipped_weapons: bool,
    pub queued_inputs: BTreeMap<InputKind, InputAttr>,
    pub removed_inputs: Vec<InputKind>,
    pub local_events: VecDeque<LocalEvent>,
    pub server_events: VecDeque<ServerEvent>,
}

impl From<&JoinData<'_>> for StateUpdate {
    fn from(data: &JoinData) -> Self {
        StateUpdate {
            pos: *data.pos,
            vel: *data.vel,
            ori: *data.ori,
            density: *data.density,
            energy: *data.energy,
            swap_equipped_weapons: false,
            character: data.character.clone(),
            queued_inputs: BTreeMap::new(),
            removed_inputs: Vec::new(),
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CharacterState {
    Idle,
    Climb(climb::Data),
    Sit,
    Dance,
    Talk,
    Sneak,
    Glide,
    GlideWield,
    /// A stunned state
    Stunned(stunned::Data),
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
    /// Creates an aura that persists as long as you are actively casting
    BasicAura(basic_aura::Data),
    /// A directed beam that heals targets in range. This is separate from basic
    /// beam as a large amount of functionality needed to be special cased
    /// specifically for the healing beam. There was also functionality present
    /// on basic beam which was unnecessary for the healing beam.
    HealingBeam(healing_beam::Data),
    /// A short teleport that targets either a position or entity
    Blink(blink::Data),
    /// Summons creatures that fight for the caster
    BasicSummon(basic_summon::Data),
}

impl CharacterState {
    pub fn is_wield(&self) -> bool {
        matches!(
            self,
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
                | CharacterState::BasicAura(_)
                | CharacterState::HealingBeam(_)
        )
    }

    pub fn is_stealthy(&self) -> bool {
        matches!(self, CharacterState::Sneak | CharacterState::Roll(_))
    }

    pub fn is_attack(&self) -> bool {
        matches!(
            self,
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
                | CharacterState::BasicAura(_)
                | CharacterState::HealingBeam(_)
        )
    }

    pub fn is_aimed(&self) -> bool {
        matches!(
            self,
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
                | CharacterState::Stunned(_)
                | CharacterState::Wielding
                | CharacterState::Talk
                | CharacterState::HealingBeam(_)
        )
    }

    pub fn is_using_hands(&self) -> bool {
        matches!(
            self,
            CharacterState::Climb(_)
                | CharacterState::Equipping(_)
                | CharacterState::Dance
                | CharacterState::Glide
                | CharacterState::GlideWield
                | CharacterState::Roll(_),
        )
    }

    pub fn is_block(&self) -> bool { matches!(self, CharacterState::BasicBlock) }

    pub fn is_dodge(&self) -> bool { matches!(self, CharacterState::Roll(_)) }

    pub fn is_melee_dodge(&self) -> bool {
        matches!(self, CharacterState::Roll(d) if d.static_data.immune_melee)
    }

    pub fn is_stunned(&self) -> bool { matches!(self, CharacterState::Stunned(_)) }

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
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Melee {
    pub attack: Attack,
    pub range: f32,
    pub max_angle: f32,
    pub applied: bool,
    pub hit_count: u32,
    pub break_block: Option<(Vec3<i32>, Option<ToolKind>)>,
}

impl Component for Melee {
    type Storage = VecStorage<Self>;
}
