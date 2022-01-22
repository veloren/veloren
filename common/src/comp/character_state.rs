use crate::{
    comp::{
        item::ConsumableKind, ControlAction, Density, Energy, InputAttr, InputKind, Ori, Pos, Vel,
    },
    event::{LocalEvent, ServerEvent},
    states::{
        self,
        behavior::{CharacterBehavior, JoinData},
        utils::StageSection,
        *,
    },
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::collections::BTreeMap;
use strum_macros::Display;

/// Data returned from character behavior fn's to Character Behavior System.
pub struct StateUpdate {
    pub character: CharacterState,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
    pub density: Density,
    pub energy: Energy,
    pub swap_equipped_weapons: bool,
    pub should_strafe: bool,
    pub queued_inputs: BTreeMap<InputKind, InputAttr>,
    pub removed_inputs: Vec<InputKind>,
}

pub struct OutputEvents<'a> {
    local: &'a mut Vec<LocalEvent>,
    server: &'a mut Vec<ServerEvent>,
}

impl<'a> OutputEvents<'a> {
    pub fn new(local: &'a mut Vec<LocalEvent>, server: &'a mut Vec<ServerEvent>) -> Self {
        Self { local, server }
    }

    pub fn emit_local(&mut self, event: LocalEvent) { self.local.push(event); }

    pub fn emit_server(&mut self, event: ServerEvent) { self.server.push(event); }
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
            should_strafe: data.inputs.strafing,
            character: data.character.clone(),
            queued_inputs: BTreeMap::new(),
            removed_inputs: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, Display, PartialEq, Serialize, Deserialize)]
pub enum CharacterState {
    Idle(idle::Data),
    Climb(climb::Data),
    Sit,
    Dance,
    Talk,
    Glide(glide::Data),
    GlideWield(glide_wield::Data),
    /// A stunned state
    Stunned(stunned::Data),
    /// A basic blocking state
    BasicBlock(basic_block::Data),
    /// Player is busy equipping or unequipping weapons
    Equipping(equipping::Data),
    /// Player is holding a weapon and can perform other actions
    Wielding(wielding::Data),
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
    /// A short teleport that targets either a position or entity
    Blink(blink::Data),
    /// Summons creatures that fight for the caster
    BasicSummon(basic_summon::Data),
    /// Inserts a buff on the caster
    SelfBuff(self_buff::Data),
    /// Creates sprites around the caster
    SpriteSummon(sprite_summon::Data),
    /// Handles logic for using an item so it is not simply instant
    UseItem(use_item::Data),
    /// Handles logic for interacting with a sprite, e.g. using a chest or
    /// picking a plant
    SpriteInteract(sprite_interact::Data),
}

impl CharacterState {
    pub fn is_wield(&self) -> bool {
        matches!(
            self,
            CharacterState::Wielding(_)
                | CharacterState::BasicMelee(_)
                | CharacterState::BasicRanged(_)
                | CharacterState::DashMelee(_)
                | CharacterState::ComboMelee(_)
                | CharacterState::BasicBlock(_)
                | CharacterState::LeapMelee(_)
                | CharacterState::SpinMelee(_)
                | CharacterState::ChargedMelee(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
                | CharacterState::Shockwave(_)
                | CharacterState::BasicBeam(_)
                | CharacterState::BasicAura(_)
                | CharacterState::SelfBuff(_)
                | CharacterState::Blink(_)
                | CharacterState::BasicSummon(_)
                | CharacterState::SpriteSummon(_)
        )
    }

    pub fn is_stealthy(&self) -> bool {
        matches!(
            self,
            CharacterState::Idle(idle::Data { is_sneaking: true })
                | CharacterState::Wielding(wielding::Data {
                    is_sneaking: true,
                    ..
                })
                | CharacterState::Roll(roll::Data {
                    is_sneaking: true,
                    ..
                })
        )
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
                | CharacterState::SelfBuff(_)
                | CharacterState::Blink(_)
                | CharacterState::BasicSummon(_)
                | CharacterState::SpriteSummon(_)
        )
    }

    pub fn is_aimed(&self) -> bool {
        matches!(
            self,
            CharacterState::BasicMelee(_)
                | CharacterState::BasicRanged(_)
                | CharacterState::DashMelee(_)
                | CharacterState::ComboMelee(_)
                | CharacterState::BasicBlock(_)
                | CharacterState::LeapMelee(_)
                | CharacterState::ChargedMelee(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
                | CharacterState::Shockwave(_)
                | CharacterState::BasicBeam(_)
                | CharacterState::Stunned(_)
                | CharacterState::UseItem(_)
                | CharacterState::Wielding(_)
                | CharacterState::Talk
        )
    }

    pub fn is_using_hands(&self) -> bool {
        matches!(
            self,
            CharacterState::Climb(_)
                | CharacterState::Equipping(_)
                | CharacterState::Dance
                | CharacterState::Glide(_)
                | CharacterState::GlideWield(_)
                | CharacterState::Talk
                | CharacterState::Roll(_),
        )
    }

    pub fn is_block(&self) -> bool { matches!(self, CharacterState::BasicBlock(_)) }

    pub fn is_dodge(&self) -> bool { matches!(self, CharacterState::Roll(_)) }

    pub fn is_glide(&self) -> bool { matches!(self, CharacterState::Glide(_)) }

    pub fn is_melee_dodge(&self) -> bool {
        matches!(self, CharacterState::Roll(d) if d.static_data.immune_melee)
    }

    pub fn is_stunned(&self) -> bool { matches!(self, CharacterState::Stunned(_)) }

    pub fn is_forced_movement(&self) -> bool {
        matches!(self,
            CharacterState::ComboMelee(s) if s.stage_section == StageSection::Action)
            || matches!(self, CharacterState::DashMelee(s) if s.stage_section == StageSection::Charge)
            || matches!(self, CharacterState::LeapMelee(s) if s.stage_section == StageSection::Movement)
            || matches!(self, CharacterState::SpinMelee(s) if s.stage_section == StageSection::Action)
            || matches!(self, CharacterState::Roll(s) if s.stage_section == StageSection::Movement)
    }

    pub fn can_perform_mounted(&self) -> bool {
        matches!(
            self,
            CharacterState::Idle(_)
                | CharacterState::Sit
                | CharacterState::Dance
                | CharacterState::Talk
                | CharacterState::Stunned(_)
                | CharacterState::BasicBlock(_)
                | CharacterState::Equipping(_)
                | CharacterState::Wielding(_)
                | CharacterState::BasicMelee(_)
                | CharacterState::BasicRanged(_)
                | CharacterState::ComboMelee(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
                | CharacterState::BasicBeam(_)
                | CharacterState::BasicAura(_)
                | CharacterState::BasicSummon(_)
                | CharacterState::SelfBuff(_)
                | CharacterState::SpriteSummon(_)
                | CharacterState::UseItem(_)
                | CharacterState::SpriteInteract(_)
        )
    }

    pub fn is_sitting(&self) -> bool {
        use use_item::{Data, ItemUseKind, StaticData};
        matches!(
            self,
            CharacterState::Sit
                | CharacterState::UseItem(Data {
                    static_data: StaticData {
                        item_kind: ItemUseKind::Consumable(
                            ConsumableKind::ComplexFood | ConsumableKind::Food
                        ),
                        ..
                    },
                    ..
                })
        )
    }

    /// Compares for shallow equality (does not check internal struct equality)
    pub fn same_variant(&self, other: &Self) -> bool {
        // Check if state is the same without looking at the inner data
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub fn behavior(&self, j: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        match &self {
            CharacterState::Idle(data) => data.behavior(j, output_events),
            CharacterState::Talk => states::talk::Data.behavior(j, output_events),
            CharacterState::Climb(data) => data.behavior(j, output_events),
            CharacterState::Glide(data) => data.behavior(j, output_events),
            CharacterState::GlideWield(data) => data.behavior(j, output_events),
            CharacterState::Stunned(data) => data.behavior(j, output_events),
            CharacterState::Sit => {
                states::sit::Data::behavior(&states::sit::Data, j, output_events)
            },
            CharacterState::Dance => {
                states::dance::Data::behavior(&states::dance::Data, j, output_events)
            },
            CharacterState::BasicBlock(data) => data.behavior(j, output_events),
            CharacterState::Roll(data) => data.behavior(j, output_events),
            CharacterState::Wielding(data) => data.behavior(j, output_events),
            CharacterState::Equipping(data) => data.behavior(j, output_events),
            CharacterState::ComboMelee(data) => data.behavior(j, output_events),
            CharacterState::BasicMelee(data) => data.behavior(j, output_events),
            CharacterState::BasicRanged(data) => data.behavior(j, output_events),
            CharacterState::Boost(data) => data.behavior(j, output_events),
            CharacterState::DashMelee(data) => data.behavior(j, output_events),
            CharacterState::LeapMelee(data) => data.behavior(j, output_events),
            CharacterState::SpinMelee(data) => data.behavior(j, output_events),
            CharacterState::ChargedMelee(data) => data.behavior(j, output_events),
            CharacterState::ChargedRanged(data) => data.behavior(j, output_events),
            CharacterState::RepeaterRanged(data) => data.behavior(j, output_events),
            CharacterState::Shockwave(data) => data.behavior(j, output_events),
            CharacterState::BasicBeam(data) => data.behavior(j, output_events),
            CharacterState::BasicAura(data) => data.behavior(j, output_events),
            CharacterState::Blink(data) => data.behavior(j, output_events),
            CharacterState::BasicSummon(data) => data.behavior(j, output_events),
            CharacterState::SelfBuff(data) => data.behavior(j, output_events),
            CharacterState::SpriteSummon(data) => data.behavior(j, output_events),
            CharacterState::UseItem(data) => data.behavior(j, output_events),
            CharacterState::SpriteInteract(data) => data.behavior(j, output_events),
        }
    }

    pub fn handle_event(
        &self,
        j: &JoinData,
        output_events: &mut OutputEvents,
        action: ControlAction,
    ) -> StateUpdate {
        match &self {
            CharacterState::Idle(data) => data.handle_event(j, output_events, action),
            CharacterState::Talk => states::talk::Data.handle_event(j, output_events, action),
            CharacterState::Climb(data) => data.handle_event(j, output_events, action),
            CharacterState::Glide(data) => data.handle_event(j, output_events, action),
            CharacterState::GlideWield(data) => data.handle_event(j, output_events, action),
            CharacterState::Stunned(data) => data.handle_event(j, output_events, action),
            CharacterState::Sit => {
                states::sit::Data::handle_event(&states::sit::Data, j, output_events, action)
            },
            CharacterState::Dance => {
                states::dance::Data::handle_event(&states::dance::Data, j, output_events, action)
            },
            CharacterState::BasicBlock(data) => data.handle_event(j, output_events, action),
            CharacterState::Roll(data) => data.handle_event(j, output_events, action),
            CharacterState::Wielding(data) => data.handle_event(j, output_events, action),
            CharacterState::Equipping(data) => data.handle_event(j, output_events, action),
            CharacterState::ComboMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicRanged(data) => data.handle_event(j, output_events, action),
            CharacterState::Boost(data) => data.handle_event(j, output_events, action),
            CharacterState::DashMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::LeapMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::SpinMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::ChargedMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::ChargedRanged(data) => data.handle_event(j, output_events, action),
            CharacterState::RepeaterRanged(data) => data.handle_event(j, output_events, action),
            CharacterState::Shockwave(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicBeam(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicAura(data) => data.handle_event(j, output_events, action),
            CharacterState::Blink(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicSummon(data) => data.handle_event(j, output_events, action),
            CharacterState::SelfBuff(data) => data.handle_event(j, output_events, action),
            CharacterState::SpriteSummon(data) => data.handle_event(j, output_events, action),
            CharacterState::UseItem(data) => data.handle_event(j, output_events, action),
            CharacterState::SpriteInteract(data) => data.handle_event(j, output_events, action),
        }
    }
}

impl Default for CharacterState {
    fn default() -> Self { Self::Idle(idle::Data { is_sneaking: false }) }
}

impl Component for CharacterState {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
