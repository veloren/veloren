use crate::{
    combat::AttackSource,
    comp::{
        ability::Capability, inventory::item::armor::Friction, item::ConsumableKind, ControlAction,
        Density, Energy, InputAttr, InputKind, Ori, Pos, Vel,
    },
    event::{LocalEvent, ServerEvent},
    resources::Time,
    states::{
        self,
        behavior::{CharacterBehavior, JoinData},
        utils::{AbilityInfo, StageSection},
        *,
    },
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::{collections::BTreeMap, time::Duration};
use strum::Display;

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
    /// A state where you progress through multiple melee attacks
    ComboMelee2(combo_melee2::Data),
    /// A leap followed by a small aoe ground attack
    LeapMelee(leap_melee::Data),
    /// A leap followed by a shockwave
    LeapShockwave(leap_shockwave::Data),
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
    /// Runs on the wall
    Wallrun(wallrun::Data),
    /// Ice skating or skiing
    Skate(skate::Data),
    /// Play music instrument
    Music(music::Data),
    /// Melee attack that scales off and consumes combo
    FinisherMelee(finisher_melee::Data),
    /// State entered when diving, melee attack triggered upon landing on the
    /// ground
    DiveMelee(dive_melee::Data),
    /// Attack that attempts to parry, and if it parries moves to an attack
    RiposteMelee(riposte_melee::Data),
    /// A series of consecutive, identical attacks that only go through buildup
    /// and recover once for the entire state
    RapidMelee(rapid_melee::Data),
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
                | CharacterState::ComboMelee2(_)
                | CharacterState::BasicBlock(_)
                | CharacterState::LeapMelee(_)
                | CharacterState::LeapShockwave(_)
                | CharacterState::SpinMelee(_)
                | CharacterState::ChargedMelee(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
                | CharacterState::Shockwave(_)
                | CharacterState::BasicBeam(_)
                | CharacterState::BasicAura(_)
                | CharacterState::SelfBuff(_)
                | CharacterState::Blink(_)
                | CharacterState::Music(_)
                | CharacterState::BasicSummon(_)
                | CharacterState::SpriteSummon(_)
                | CharacterState::Roll(roll::Data {
                    was_wielded: true,
                    ..
                })
                | CharacterState::Stunned(stunned::Data {
                    was_wielded: true,
                    ..
                })
                | CharacterState::FinisherMelee(_)
                | CharacterState::DiveMelee(_)
                | CharacterState::RiposteMelee(_)
                | CharacterState::RapidMelee(_)
        )
    }

    pub fn was_wielded(&self) -> bool {
        match self {
            CharacterState::Roll(data) => data.was_wielded,
            CharacterState::Stunned(data) => data.was_wielded,
            CharacterState::SpriteInteract(data) => data.static_data.was_wielded,
            CharacterState::UseItem(data) => data.static_data.was_wielded,
            _ => false,
        }
    }

    pub fn is_stealthy(&self) -> bool {
        matches!(
            self,
            CharacterState::Idle(idle::Data {
                is_sneaking: true,
                footwear: _,
                time_entered: _,
            }) | CharacterState::Wielding(wielding::Data {
                is_sneaking: true,
                ..
            }) | CharacterState::Roll(roll::Data {
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
                | CharacterState::ComboMelee2(_)
                | CharacterState::LeapMelee(_)
                | CharacterState::LeapShockwave(_)
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
                | CharacterState::FinisherMelee(_)
                | CharacterState::DiveMelee(_)
                | CharacterState::RiposteMelee(_)
                | CharacterState::RapidMelee(_)
        )
    }

    pub fn is_aimed(&self) -> bool {
        matches!(
            self,
            CharacterState::BasicMelee(_)
                | CharacterState::BasicRanged(_)
                | CharacterState::DashMelee(_)
                | CharacterState::ComboMelee(_)
                | CharacterState::ComboMelee2(_)
                | CharacterState::BasicBlock(_)
                | CharacterState::LeapMelee(_)
                | CharacterState::LeapShockwave(_)
                | CharacterState::ChargedMelee(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
                | CharacterState::Shockwave(_)
                | CharacterState::BasicBeam(_)
                | CharacterState::Stunned(_)
                | CharacterState::UseItem(_)
                | CharacterState::Wielding(_)
                | CharacterState::Talk
                | CharacterState::FinisherMelee(_)
                | CharacterState::DiveMelee(_)
                | CharacterState::RiposteMelee(_)
                | CharacterState::RapidMelee(_)
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

    pub fn block_strength(&self, attack_source: AttackSource) -> Option<f32> {
        let from_capability = if let AttackSource::Melee = attack_source {
            if let Some(capabilities) = self
                .ability_info()
                .map(|a| a.ability_meta)
                .map(|m| m.capabilities)
            {
                (capabilities.contains(Capability::BUILDUP_BLOCKS)
                    && matches!(self.stage_section(), Some(StageSection::Buildup)))
                .then_some(0.5)
            } else {
                None
            }
        } else {
            None
        };
        let from_state = match self {
            CharacterState::BasicBlock(c) => c
                .static_data
                .blocked_attacks
                .applies(attack_source)
                .then_some(c.static_data.block_strength),
            _ => None,
        };
        match (from_capability, from_state) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        }
    }

    pub fn is_parry(&self, attack_source: AttackSource) -> bool {
        let melee = matches!(attack_source, AttackSource::Melee);
        let from_capability = melee
            && self
                .ability_info()
                .map(|a| a.ability_meta.capabilities)
                .map_or(false, |c| {
                    c.contains(Capability::BUILDUP_PARRIES)
                        && matches!(self.stage_section(), Some(StageSection::Buildup))
                });
        let from_state = match self {
            CharacterState::BasicBlock(c) => c.is_parry(attack_source),
            CharacterState::RiposteMelee(c) => {
                melee && matches!(c.stage_section, StageSection::Buildup)
            },
            _ => false,
        };
        from_capability || from_state
    }

    /// In radians
    pub fn block_angle(&self) -> f32 {
        match self {
            CharacterState::BasicBlock(c) => c.static_data.max_angle.to_radians(),
            CharacterState::ComboMelee2(c) => {
                let strike_data =
                    c.static_data.strikes[c.completed_strikes % c.static_data.strikes.len()];
                strike_data.melee_constructor.angle.to_radians()
            },
            CharacterState::RiposteMelee(c) => c.static_data.melee_constructor.angle.to_radians(),
            // TODO: Add more here as needed, maybe look into having character state return the
            // melee constructor if it has one and using that?
            _ => 0.0,
        }
    }

    pub fn is_dodge(&self) -> bool { matches!(self, CharacterState::Roll(_)) }

    pub fn is_glide(&self) -> bool { matches!(self, CharacterState::Glide(_)) }

    pub fn is_skate(&self) -> bool { matches!(self, CharacterState::Skate(_)) }

    pub fn is_music(&self) -> bool { matches!(self, CharacterState::Music(_)) }

    pub fn attack_immunities(&self) -> Option<AttackFilters> {
        if let CharacterState::Roll(c) = self {
            Some(c.static_data.attack_immunities)
        } else {
            None
        }
    }

    pub fn is_stunned(&self) -> bool { matches!(self, CharacterState::Stunned(_)) }

    pub fn is_forced_movement(&self) -> bool {
        matches!(self,
            CharacterState::ComboMelee(s) if s.stage_section == StageSection::Action)
            || matches!(self, CharacterState::ComboMelee2(s) if s.stage_section == StageSection::Action)
            || matches!(self, CharacterState::DashMelee(s) if s.stage_section == StageSection::Charge)
            || matches!(self, CharacterState::LeapMelee(s) if s.stage_section == StageSection::Movement)
            || matches!(self, CharacterState::SpinMelee(s) if s.stage_section == StageSection::Action)
            || matches!(self, CharacterState::Roll(s) if s.stage_section == StageSection::Movement)
    }

    pub fn is_melee_attack(&self) -> bool {
        matches!(self.attack_kind(), Some(AttackSource::Melee))
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
                | CharacterState::ComboMelee2(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
                | CharacterState::BasicBeam(_)
                | CharacterState::BasicAura(_)
                | CharacterState::BasicSummon(_)
                | CharacterState::SelfBuff(_)
                | CharacterState::SpriteSummon(_)
                | CharacterState::UseItem(_)
                | CharacterState::SpriteInteract(_)
                | CharacterState::Music(_)
                | CharacterState::RiposteMelee(_)
                | CharacterState::RapidMelee(_)
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
            CharacterState::Talk => talk::Data.behavior(j, output_events),
            CharacterState::Climb(data) => data.behavior(j, output_events),
            CharacterState::Wallrun(data) => data.behavior(j, output_events),
            CharacterState::Glide(data) => data.behavior(j, output_events),
            CharacterState::GlideWield(data) => data.behavior(j, output_events),
            CharacterState::Stunned(data) => data.behavior(j, output_events),
            CharacterState::Sit => sit::Data::behavior(&sit::Data, j, output_events),
            CharacterState::Dance => dance::Data::behavior(&dance::Data, j, output_events),
            CharacterState::BasicBlock(data) => data.behavior(j, output_events),
            CharacterState::Roll(data) => data.behavior(j, output_events),
            CharacterState::Wielding(data) => data.behavior(j, output_events),
            CharacterState::Equipping(data) => data.behavior(j, output_events),
            CharacterState::ComboMelee(data) => data.behavior(j, output_events),
            CharacterState::ComboMelee2(data) => data.behavior(j, output_events),
            CharacterState::BasicMelee(data) => data.behavior(j, output_events),
            CharacterState::BasicRanged(data) => data.behavior(j, output_events),
            CharacterState::Boost(data) => data.behavior(j, output_events),
            CharacterState::DashMelee(data) => data.behavior(j, output_events),
            CharacterState::LeapMelee(data) => data.behavior(j, output_events),
            CharacterState::LeapShockwave(data) => data.behavior(j, output_events),
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
            CharacterState::Skate(data) => data.behavior(j, output_events),
            CharacterState::Music(data) => data.behavior(j, output_events),
            CharacterState::FinisherMelee(data) => data.behavior(j, output_events),
            CharacterState::DiveMelee(data) => data.behavior(j, output_events),
            CharacterState::RiposteMelee(data) => data.behavior(j, output_events),
            CharacterState::RapidMelee(data) => data.behavior(j, output_events),
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
            CharacterState::Talk => talk::Data.handle_event(j, output_events, action),
            CharacterState::Climb(data) => data.handle_event(j, output_events, action),
            CharacterState::Wallrun(data) => data.handle_event(j, output_events, action),
            CharacterState::Glide(data) => data.handle_event(j, output_events, action),
            CharacterState::GlideWield(data) => data.handle_event(j, output_events, action),
            CharacterState::Stunned(data) => data.handle_event(j, output_events, action),
            CharacterState::Sit => {
                states::sit::Data::handle_event(&sit::Data, j, output_events, action)
            },
            CharacterState::Dance => {
                states::dance::Data::handle_event(&dance::Data, j, output_events, action)
            },
            CharacterState::BasicBlock(data) => data.handle_event(j, output_events, action),
            CharacterState::Roll(data) => data.handle_event(j, output_events, action),
            CharacterState::Wielding(data) => data.handle_event(j, output_events, action),
            CharacterState::Equipping(data) => data.handle_event(j, output_events, action),
            CharacterState::ComboMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::ComboMelee2(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicRanged(data) => data.handle_event(j, output_events, action),
            CharacterState::Boost(data) => data.handle_event(j, output_events, action),
            CharacterState::DashMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::LeapMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::LeapShockwave(data) => data.handle_event(j, output_events, action),
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
            CharacterState::Skate(data) => data.handle_event(j, output_events, action),
            CharacterState::Music(data) => data.handle_event(j, output_events, action),
            CharacterState::FinisherMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::DiveMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::RiposteMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::RapidMelee(data) => data.handle_event(j, output_events, action),
        }
    }

    pub fn footwear(&self) -> Option<Friction> {
        match &self {
            CharacterState::Idle(data) => data.footwear,
            CharacterState::Skate(data) => Some(data.footwear),
            _ => None,
        }
    }

    pub fn ability_info(&self) -> Option<AbilityInfo> {
        match &self {
            CharacterState::Idle(_) => None,
            CharacterState::Talk => None,
            CharacterState::Climb(_) => None,
            CharacterState::Wallrun(_) => None,
            CharacterState::Skate(_) => None,
            CharacterState::Glide(_) => None,
            CharacterState::GlideWield(_) => None,
            CharacterState::Stunned(_) => None,
            CharacterState::Sit => None,
            CharacterState::Dance => None,
            CharacterState::BasicBlock(data) => Some(data.static_data.ability_info),
            CharacterState::Roll(data) => Some(data.static_data.ability_info),
            CharacterState::Wielding(_) => None,
            CharacterState::Equipping(_) => None,
            CharacterState::ComboMelee(data) => Some(data.static_data.ability_info),
            CharacterState::ComboMelee2(data) => Some(data.static_data.ability_info),
            CharacterState::BasicMelee(data) => Some(data.static_data.ability_info),
            CharacterState::BasicRanged(data) => Some(data.static_data.ability_info),
            CharacterState::Boost(data) => Some(data.static_data.ability_info),
            CharacterState::DashMelee(data) => Some(data.static_data.ability_info),
            CharacterState::LeapMelee(data) => Some(data.static_data.ability_info),
            CharacterState::LeapShockwave(data) => Some(data.static_data.ability_info),
            CharacterState::SpinMelee(data) => Some(data.static_data.ability_info),
            CharacterState::ChargedMelee(data) => Some(data.static_data.ability_info),
            CharacterState::ChargedRanged(data) => Some(data.static_data.ability_info),
            CharacterState::RepeaterRanged(data) => Some(data.static_data.ability_info),
            CharacterState::Shockwave(data) => Some(data.static_data.ability_info),
            CharacterState::BasicBeam(data) => Some(data.static_data.ability_info),
            CharacterState::BasicAura(data) => Some(data.static_data.ability_info),
            CharacterState::Blink(data) => Some(data.static_data.ability_info),
            CharacterState::BasicSummon(data) => Some(data.static_data.ability_info),
            CharacterState::SelfBuff(data) => Some(data.static_data.ability_info),
            CharacterState::SpriteSummon(data) => Some(data.static_data.ability_info),
            CharacterState::UseItem(_) => None,
            CharacterState::SpriteInteract(_) => None,
            CharacterState::FinisherMelee(data) => Some(data.static_data.ability_info),
            CharacterState::Music(data) => Some(data.static_data.ability_info),
            CharacterState::DiveMelee(data) => Some(data.static_data.ability_info),
            CharacterState::RiposteMelee(data) => Some(data.static_data.ability_info),
            CharacterState::RapidMelee(data) => Some(data.static_data.ability_info),
        }
    }

    pub fn stage_section(&self) -> Option<StageSection> {
        match &self {
            CharacterState::Idle(_) => None,
            CharacterState::Talk => None,
            CharacterState::Climb(_) => None,
            CharacterState::Wallrun(_) => None,
            CharacterState::Skate(_) => None,
            CharacterState::Glide(_) => None,
            CharacterState::GlideWield(_) => None,
            CharacterState::Stunned(data) => Some(data.stage_section),
            CharacterState::Sit => None,
            CharacterState::Dance => None,
            CharacterState::BasicBlock(data) => Some(data.stage_section),
            CharacterState::Roll(data) => Some(data.stage_section),
            CharacterState::Equipping(_) => Some(StageSection::Buildup),
            CharacterState::Wielding(_) => None,
            CharacterState::ComboMelee(data) => Some(data.stage_section),
            CharacterState::ComboMelee2(data) => Some(data.stage_section),
            CharacterState::BasicMelee(data) => Some(data.stage_section),
            CharacterState::BasicRanged(data) => Some(data.stage_section),
            CharacterState::Boost(_) => None,
            CharacterState::DashMelee(data) => Some(data.stage_section),
            CharacterState::LeapMelee(data) => Some(data.stage_section),
            CharacterState::LeapShockwave(data) => Some(data.stage_section),
            CharacterState::SpinMelee(data) => Some(data.stage_section),
            CharacterState::ChargedMelee(data) => Some(data.stage_section),
            CharacterState::ChargedRanged(data) => Some(data.stage_section),
            CharacterState::RepeaterRanged(data) => Some(data.stage_section),
            CharacterState::Shockwave(data) => Some(data.stage_section),
            CharacterState::BasicBeam(data) => Some(data.stage_section),
            CharacterState::BasicAura(data) => Some(data.stage_section),
            CharacterState::Blink(data) => Some(data.stage_section),
            CharacterState::BasicSummon(data) => Some(data.stage_section),
            CharacterState::SelfBuff(data) => Some(data.stage_section),
            CharacterState::SpriteSummon(data) => Some(data.stage_section),
            CharacterState::UseItem(data) => Some(data.stage_section),
            CharacterState::SpriteInteract(data) => Some(data.stage_section),
            CharacterState::FinisherMelee(data) => Some(data.stage_section),
            CharacterState::Music(data) => Some(data.stage_section),
            CharacterState::DiveMelee(data) => Some(data.stage_section),
            CharacterState::RiposteMelee(data) => Some(data.stage_section),
            CharacterState::RapidMelee(data) => Some(data.stage_section),
        }
    }

    pub fn durations(&self) -> Option<DurationsInfo> {
        match &self {
            CharacterState::Idle(_) => None,
            CharacterState::Talk => None,
            CharacterState::Climb(_) => None,
            CharacterState::Wallrun(_) => None,
            CharacterState::Skate(_) => None,
            CharacterState::Glide(_) => None,
            CharacterState::GlideWield(_) => None,
            CharacterState::Stunned(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::Sit => None,
            CharacterState::Dance => None,
            CharacterState::BasicBlock(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::Roll(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                recover: Some(data.static_data.recover_duration),
                movement: Some(data.static_data.movement_duration),
                ..Default::default()
            }),
            CharacterState::Wielding(_) => None,
            CharacterState::Equipping(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                ..Default::default()
            }),
            CharacterState::ComboMelee(data) => {
                let stage_index = data.stage_index();
                let stage = data.static_data.stage_data[stage_index];
                Some(DurationsInfo {
                    buildup: Some(stage.base_buildup_duration),
                    action: Some(stage.base_swing_duration),
                    recover: Some(stage.base_recover_duration),
                    ..Default::default()
                })
            },
            CharacterState::ComboMelee2(data) => {
                let strike = data.strike_data();
                Some(DurationsInfo {
                    buildup: Some(strike.buildup_duration),
                    action: Some(strike.swing_duration),
                    recover: Some(strike.recover_duration),
                    ..Default::default()
                })
            },
            CharacterState::BasicMelee(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::BasicRanged(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::Boost(data) => Some(DurationsInfo {
                movement: Some(data.static_data.movement_duration),
                ..Default::default()
            }),
            CharacterState::DashMelee(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                charge: Some(data.static_data.charge_duration),
                ..Default::default()
            }),
            CharacterState::LeapMelee(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                movement: Some(data.static_data.movement_duration),
                ..Default::default()
            }),
            CharacterState::LeapShockwave(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                movement: Some(data.static_data.movement_duration),
                ..Default::default()
            }),
            CharacterState::SpinMelee(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::ChargedMelee(data) => Some(DurationsInfo {
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                charge: Some(data.static_data.charge_duration),
                ..Default::default()
            }),
            CharacterState::ChargedRanged(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                recover: Some(data.static_data.recover_duration),
                charge: Some(data.static_data.charge_duration),
                ..Default::default()
            }),
            CharacterState::RepeaterRanged(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.shoot_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::Shockwave(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::BasicBeam(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::BasicAura(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.cast_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::Blink(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::BasicSummon(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.cast_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::SelfBuff(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.cast_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::SpriteSummon(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.cast_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::UseItem(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.use_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::SpriteInteract(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.use_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::FinisherMelee(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::Music(data) => Some(DurationsInfo {
                action: Some(data.static_data.play_duration),
                ..Default::default()
            }),
            CharacterState::DiveMelee(data) => Some(DurationsInfo {
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                movement: Some(data.static_data.movement_duration),
                ..Default::default()
            }),
            CharacterState::RiposteMelee(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::RapidMelee(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
        }
    }

    pub fn timer(&self) -> Option<Duration> {
        match &self {
            CharacterState::Idle(_) => None,
            CharacterState::Talk => None,
            CharacterState::Climb(_) => None,
            CharacterState::Wallrun(_) => None,
            CharacterState::Skate(_) => None,
            CharacterState::Glide(data) => Some(data.timer),
            CharacterState::GlideWield(_) => None,
            CharacterState::Stunned(data) => Some(data.timer),
            CharacterState::Sit => None,
            CharacterState::Dance => None,
            CharacterState::BasicBlock(data) => Some(data.timer),
            CharacterState::Roll(data) => Some(data.timer),
            CharacterState::Wielding(_) => None,
            CharacterState::Equipping(data) => Some(data.timer),
            CharacterState::ComboMelee(data) => Some(data.timer),
            CharacterState::ComboMelee2(data) => Some(data.timer),
            CharacterState::BasicMelee(data) => Some(data.timer),
            CharacterState::BasicRanged(data) => Some(data.timer),
            CharacterState::Boost(data) => Some(data.timer),
            CharacterState::DashMelee(data) => Some(data.timer),
            CharacterState::LeapMelee(data) => Some(data.timer),
            CharacterState::LeapShockwave(data) => Some(data.timer),
            CharacterState::SpinMelee(data) => Some(data.timer),
            CharacterState::ChargedMelee(data) => Some(data.timer),
            CharacterState::ChargedRanged(data) => Some(data.timer),
            CharacterState::RepeaterRanged(data) => Some(data.timer),
            CharacterState::Shockwave(data) => Some(data.timer),
            CharacterState::BasicBeam(data) => Some(data.timer),
            CharacterState::BasicAura(data) => Some(data.timer),
            CharacterState::Blink(data) => Some(data.timer),
            CharacterState::BasicSummon(data) => Some(data.timer),
            CharacterState::SelfBuff(data) => Some(data.timer),
            CharacterState::SpriteSummon(data) => Some(data.timer),
            CharacterState::UseItem(data) => Some(data.timer),
            CharacterState::SpriteInteract(data) => Some(data.timer),
            CharacterState::FinisherMelee(data) => Some(data.timer),
            CharacterState::Music(data) => Some(data.timer),
            CharacterState::DiveMelee(data) => Some(data.timer),
            CharacterState::RiposteMelee(data) => Some(data.timer),
            CharacterState::RapidMelee(data) => Some(data.timer),
        }
    }

    pub fn attack_kind(&self) -> Option<AttackSource> {
        match self {
            CharacterState::Idle(_) => None,
            CharacterState::Talk => None,
            CharacterState::Climb(_) => None,
            CharacterState::Wallrun(_) => None,
            CharacterState::Skate(_) => None,
            CharacterState::Glide(_) => None,
            CharacterState::GlideWield(_) => None,
            CharacterState::Stunned(_) => None,
            CharacterState::Sit => None,
            CharacterState::Dance => None,
            CharacterState::BasicBlock(_) => None,
            CharacterState::Roll(_) => None,
            CharacterState::Wielding(_) => None,
            CharacterState::Equipping(_) => None,
            CharacterState::ComboMelee(_) => Some(AttackSource::Melee),
            CharacterState::ComboMelee2(_) => Some(AttackSource::Melee),
            CharacterState::BasicMelee(_) => Some(AttackSource::Melee),
            CharacterState::BasicRanged(data) => {
                Some(if data.static_data.projectile.is_explosive() {
                    AttackSource::Explosion
                } else {
                    AttackSource::Projectile
                })
            },
            CharacterState::Boost(_) => None,
            CharacterState::DashMelee(_) => Some(AttackSource::Melee),
            CharacterState::LeapMelee(_) => Some(AttackSource::Melee),
            CharacterState::SpinMelee(_) => Some(AttackSource::Melee),
            CharacterState::ChargedMelee(_) => Some(AttackSource::Melee),
            // TODO: When charged ranged not only arrow make this check projectile type
            CharacterState::ChargedRanged(_) => Some(AttackSource::Projectile),
            CharacterState::RepeaterRanged(data) => {
                Some(if data.static_data.projectile.is_explosive() {
                    AttackSource::Explosion
                } else {
                    AttackSource::Projectile
                })
            },
            CharacterState::Shockwave(data) => Some(if data.static_data.requires_ground {
                AttackSource::GroundShockwave
            } else {
                AttackSource::AirShockwave
            }),
            CharacterState::LeapShockwave(data) => Some(if data.static_data.requires_ground {
                AttackSource::GroundShockwave
            } else {
                AttackSource::AirShockwave
            }),
            CharacterState::BasicBeam(_) => Some(AttackSource::Beam),
            CharacterState::BasicAura(_) => None,
            CharacterState::Blink(_) => None,
            CharacterState::BasicSummon(_) => None,
            CharacterState::SelfBuff(_) => None,
            CharacterState::SpriteSummon(_) => None,
            CharacterState::UseItem(_) => None,
            CharacterState::SpriteInteract(_) => None,
            CharacterState::FinisherMelee(_) => Some(AttackSource::Melee),
            CharacterState::Music(_) => None,
            CharacterState::DiveMelee(_) => Some(AttackSource::Melee),
            CharacterState::RiposteMelee(_) => Some(AttackSource::Melee),
            CharacterState::RapidMelee(_) => Some(AttackSource::Melee),
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct DurationsInfo {
    pub buildup: Option<Duration>,
    pub action: Option<Duration>,
    pub recover: Option<Duration>,
    pub movement: Option<Duration>,
    pub charge: Option<Duration>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq)]
pub struct AttackFilters {
    pub melee: bool,
    pub projectiles: bool,
    pub beams: bool,
    pub ground_shockwaves: bool,
    pub air_shockwaves: bool,
    pub explosions: bool,
}

impl AttackFilters {
    pub fn applies(&self, attack: AttackSource) -> bool {
        match attack {
            AttackSource::Melee => self.melee,
            AttackSource::Projectile => self.projectiles,
            AttackSource::Beam => self.beams,
            AttackSource::GroundShockwave => self.ground_shockwaves,
            AttackSource::AirShockwave => self.air_shockwaves,
            AttackSource::Explosion => self.explosions,
        }
    }
}

impl Default for CharacterState {
    fn default() -> Self {
        Self::Idle(idle::Data {
            is_sneaking: false,
            footwear: None,
            time_entered: Time(0.0),
        })
    }
}

impl Component for CharacterState {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
