use crate::{
    combat::AttackSource,
    comp::{
        ControlAction, Density, Energy, InputAttr, InputKind, Ori, Pos, Vel, ability::Capability,
        inventory::item::armor::Friction, item::ConsumableKind,
    },
    event::{self, EmitExt, LocalEvent},
    event_emitters,
    resources::Time,
    states::{
        self,
        behavior::{CharacterBehavior, JoinData},
        utils::{AbilityInfo, StageSection},
        *,
    },
    util::Dir,
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
    pub character_activity: CharacterActivity,
}

event_emitters! {
    pub struct CharacterStateEvents[CharacterStateEventEmitters] {
        combo: event::ComboChangeEvent,
        event: event::AuraEvent,
        shoot: event::ShootEvent,
        throw: event::ThrowEvent,
        teleport_to: event::TeleportToEvent,
        shockwave: event::ShockwaveEvent,
        explosion: event::ExplosionEvent,
        buff: event::BuffEvent,
        inventory_manip: event::InventoryManipEvent,
        sprite_summon: event::CreateSpriteEvent,
        beam_pillar_summon: event::SummonBeamPillarsEvent,
        change_stance: event::ChangeStanceEvent,
        create_npc: event::CreateNpcEvent,
        create_object: event::CreateObjectEvent,
        energy_change: event::EnergyChangeEvent,
        knockback: event::KnockbackEvent,
        sprite_light: event::ToggleSpriteLightEvent,
        transform: event::TransformEvent,
        regrow_head: event::RegrowHeadEvent,
        create_aura_entity: event::CreateAuraEntityEvent,
        help_downed: event::HelpDownedEvent,
    }
}

pub struct OutputEvents<'a, 'b> {
    local: &'a mut Vec<LocalEvent>,
    server: &'a mut CharacterStateEventEmitters<'b>,
}

impl<'a, 'b: 'a> OutputEvents<'a, 'b> {
    pub fn new(
        local: &'a mut Vec<LocalEvent>,
        server: &'a mut CharacterStateEventEmitters<'b>,
    ) -> Self {
        Self { local, server }
    }

    pub fn emit_local(&mut self, event: LocalEvent) { self.local.push(event); }

    pub fn emit_server<E>(&mut self, event: E)
    where
        CharacterStateEventEmitters<'b>: EmitExt<E>,
    {
        self.server.emit(event);
    }
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
            character_activity: data.character_activity.clone(),
        }
    }
}
#[derive(Clone, Debug, Display, PartialEq, Serialize, Deserialize)]
pub enum CharacterState {
    Idle(idle::Data),
    Crawl,
    Climb(climb::Data),
    Sit,
    Dance,
    Talk(talk::Data),
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
    /// A state where you progress through multiple melee attacks
    ComboMelee2(combo_melee2::Data),
    /// A leap followed by an explosion and a shockwave
    LeapExplosionShockwave(leap_explosion_shockwave::Data),
    /// A leap followed by a small aoe ground attack
    LeapMelee(leap_melee::Data),
    /// A leap followed by a shockwave
    LeapShockwave(leap_shockwave::Data),
    /// A charged ranged attack (e.g. bow)
    ChargedRanged(charged_ranged::Data),
    /// A charged melee attack
    ChargedMelee(charged_melee::Data),
    /// A repeating ranged attack
    RepeaterRanged(repeater_ranged::Data),
    /// An item throw
    Throw(throw::Data),
    /// A ground shockwave attack
    Shockwave(shockwave::Data),
    /// An explosion attack
    Explosion(explosion::Data),
    /// A continuous attack that affects all creatures in a cone originating
    /// from the source
    BasicBeam(basic_beam::Data),
    /// Creates an aura that persists as long as you are actively casting
    BasicAura(basic_aura::Data),
    /// Creates an aura that is attached to a pseudo entity, so it doesn't move
    /// with you Optionally allows for sprites to be created as well
    StaticAura(static_aura::Data),
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
    /// Handles logic for interacting with a sprite or an entity, e.g. using a
    /// chest, picking a plant, helping a downed entity up
    Interact(interact::Data),
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
    /// Transforms an entity into another
    Transform(transform::Data),
    /// Regrow a missing head
    RegrowHead(regrow_head::Data),
}

impl CharacterState {
    pub fn is_wield(&self) -> bool {
        match self {
            CharacterState::Wallrun(wallrun::Data { was_wielded })
            | CharacterState::Climb(climb::Data { was_wielded, .. })
            | CharacterState::Roll(roll::Data { was_wielded, .. })
            | CharacterState::Stunned(stunned::Data { was_wielded, .. }) => *was_wielded,
            CharacterState::Wielding(_)
            | CharacterState::BasicMelee(_)
            | CharacterState::BasicRanged(_)
            | CharacterState::Throw(_)
            | CharacterState::DashMelee(_)
            | CharacterState::ComboMelee2(_)
            | CharacterState::BasicBlock(_)
            | CharacterState::LeapMelee(_)
            | CharacterState::LeapShockwave(_)
            | CharacterState::LeapExplosionShockwave(_)
            | CharacterState::Explosion(_)
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
            | CharacterState::FinisherMelee(_)
            | CharacterState::DiveMelee(_)
            | CharacterState::RiposteMelee(_)
            | CharacterState::RapidMelee(_)
            | CharacterState::StaticAura(_) => true,
            CharacterState::Idle(_)
            | CharacterState::Crawl
            | CharacterState::Sit
            | CharacterState::Dance
            | CharacterState::Talk(_)
            | CharacterState::Glide(_)
            | CharacterState::GlideWield(_)
            | CharacterState::Equipping(_)
            | CharacterState::Boost(_)
            | CharacterState::UseItem(_)
            | CharacterState::Interact(_)
            | CharacterState::Skate(_)
            | CharacterState::Transform(_)
            | CharacterState::RegrowHead(_) => false,
        }
    }

    pub fn is_ranged(&self) -> bool {
        matches!(
            self,
            CharacterState::BasicRanged(_)
                | CharacterState::Throw(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
        )
    }

    /// If this state can manipulate loadout, interact with sprites etc.
    pub fn can_interact(&self) -> bool {
        match self {
            CharacterState::Idle(_)
            | CharacterState::Sit
            | CharacterState::Dance
            | CharacterState::Talk(_)
            | CharacterState::Equipping(_)
            | CharacterState::Wielding(_)
            | CharacterState::GlideWield(_) => true,
            CharacterState::Crawl
            | CharacterState::Climb(_)
            | CharacterState::Glide(_)
            | CharacterState::Stunned(_)
            | CharacterState::BasicBlock(_)
            | CharacterState::Roll(_)
            | CharacterState::BasicMelee(_)
            | CharacterState::BasicRanged(_)
            | CharacterState::Throw(_)
            | CharacterState::Boost(_)
            | CharacterState::DashMelee(_)
            | CharacterState::ComboMelee2(_)
            | CharacterState::LeapExplosionShockwave(_)
            | CharacterState::LeapMelee(_)
            | CharacterState::LeapShockwave(_)
            | CharacterState::ChargedRanged(_)
            | CharacterState::ChargedMelee(_)
            | CharacterState::RepeaterRanged(_)
            | CharacterState::Shockwave(_)
            | CharacterState::Explosion(_)
            | CharacterState::BasicBeam(_)
            | CharacterState::BasicAura(_)
            | CharacterState::StaticAura(_)
            | CharacterState::Blink(_)
            | CharacterState::BasicSummon(_)
            | CharacterState::SelfBuff(_)
            | CharacterState::SpriteSummon(_)
            | CharacterState::UseItem(_)
            | CharacterState::Interact(_)
            | CharacterState::Wallrun(_)
            | CharacterState::Skate(_)
            | CharacterState::Music(_)
            | CharacterState::FinisherMelee(_)
            | CharacterState::DiveMelee(_)
            | CharacterState::RiposteMelee(_)
            | CharacterState::RapidMelee(_)
            | CharacterState::Transform(_)
            | CharacterState::RegrowHead(_) => false,
        }
    }

    pub fn was_wielded(&self) -> bool {
        match self {
            CharacterState::Roll(data) => data.was_wielded,
            CharacterState::Stunned(data) => data.was_wielded,
            CharacterState::Interact(data) => data.static_data.was_wielded,
            CharacterState::UseItem(data) => data.static_data.was_wielded,
            CharacterState::Wallrun(data) => data.was_wielded,
            CharacterState::Climb(data) => data.was_wielded,
            CharacterState::Idle(_)
            | CharacterState::Crawl
            | CharacterState::Sit
            | CharacterState::Dance
            | CharacterState::Talk(_)
            | CharacterState::Glide(_)
            | CharacterState::GlideWield(_)
            | CharacterState::BasicBlock(_)
            | CharacterState::Equipping(_)
            | CharacterState::Wielding(_)
            | CharacterState::BasicMelee(_)
            | CharacterState::BasicRanged(_)
            | CharacterState::Throw(_)
            | CharacterState::Boost(_)
            | CharacterState::DashMelee(_)
            | CharacterState::ComboMelee2(_)
            | CharacterState::LeapMelee(_)
            | CharacterState::LeapShockwave(_)
            | CharacterState::LeapExplosionShockwave(_)
            | CharacterState::Explosion(_)
            | CharacterState::ChargedRanged(_)
            | CharacterState::ChargedMelee(_)
            | CharacterState::RepeaterRanged(_)
            | CharacterState::Shockwave(_)
            | CharacterState::BasicBeam(_)
            | CharacterState::BasicAura(_)
            | CharacterState::StaticAura(_)
            | CharacterState::Blink(_)
            | CharacterState::BasicSummon(_)
            | CharacterState::SelfBuff(_)
            | CharacterState::SpriteSummon(_)
            | CharacterState::Skate(_)
            | CharacterState::Music(_)
            | CharacterState::FinisherMelee(_)
            | CharacterState::DiveMelee(_)
            | CharacterState::RiposteMelee(_)
            | CharacterState::RapidMelee(_)
            | CharacterState::Transform(_)
            | CharacterState::RegrowHead(_) => false,
        }
    }

    pub fn is_glide_wielded(&self) -> bool {
        matches!(
            self,
            CharacterState::Glide { .. } | CharacterState::GlideWield { .. }
        )
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

    pub fn should_follow_look(&self) -> bool {
        matches!(self, CharacterState::Boost(_)) || self.is_attack()
    }

    pub fn is_attack(&self) -> bool {
        matches!(
            self,
            CharacterState::BasicMelee(_)
                | CharacterState::BasicRanged(_)
                | CharacterState::DashMelee(_)
                | CharacterState::ComboMelee2(_)
                | CharacterState::LeapExplosionShockwave(_)
                | CharacterState::LeapMelee(_)
                | CharacterState::LeapShockwave(_)
                | CharacterState::ChargedMelee(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
                | CharacterState::Throw(_)
                | CharacterState::Shockwave(_)
                | CharacterState::Explosion(_)
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
                | CharacterState::StaticAura(_)
        )
    }

    pub fn is_blockable(&self) -> Option<bool> {
        match self {
            CharacterState::BasicMelee(data) => Some(data.static_data.melee_constructor.blockable),
            CharacterState::BasicRanged(data) => data
                .static_data
                .projectile
                .attack
                .map(|projectile_attack| projectile_attack.blockable),
            CharacterState::DashMelee(data) => Some(data.static_data.melee_constructor.blockable),
            CharacterState::ComboMelee2(data) => data
                .static_data
                .strikes
                .get(data.completed_strikes)
                .map(|strike| strike.melee_constructor.blockable),
            CharacterState::LeapMelee(data) => Some(data.static_data.melee_constructor.blockable),
            CharacterState::ChargedRanged(data) => data
                .static_data
                .projectile
                .attack
                .map(|projectile_attack| projectile_attack.blockable),
            CharacterState::ChargedMelee(data) => Some(
                data.static_data.melee_constructor.blockable
                    && data
                        .static_data
                        .buildup_strike
                        .is_none_or(|(_, buildup_strike)| buildup_strike.blockable),
            ),
            CharacterState::RepeaterRanged(data) => data
                .static_data
                .projectile
                .attack
                .map(|projectile_attack| projectile_attack.blockable),
            CharacterState::BasicBeam(data) => Some(data.static_data.blockable),
            CharacterState::FinisherMelee(data) => {
                Some(data.static_data.melee_constructor.blockable)
            },
            CharacterState::DiveMelee(data) => Some(data.static_data.melee_constructor.blockable),
            CharacterState::RiposteMelee(data) => {
                Some(data.static_data.melee_constructor.blockable)
            },
            CharacterState::RapidMelee(data) => Some(data.static_data.melee_constructor.blockable),
            CharacterState::Idle(_)
            | CharacterState::Crawl
            | CharacterState::Climb(_)
            | CharacterState::Sit
            | CharacterState::Dance
            | CharacterState::Talk(_)
            | CharacterState::Glide(_)
            | CharacterState::GlideWield(_)
            | CharacterState::Stunned(_)
            | CharacterState::BasicBlock(_)
            | CharacterState::Equipping(_)
            | CharacterState::Wielding(_)
            | CharacterState::Roll(_)
            | CharacterState::Boost(_)
            | CharacterState::Explosion(_)
            | CharacterState::LeapExplosionShockwave(_)
            | CharacterState::LeapShockwave(_)
            | CharacterState::Shockwave(_)
            | CharacterState::Throw(_)
            | CharacterState::BasicAura(_)
            | CharacterState::StaticAura(_)
            | CharacterState::Blink(_)
            | CharacterState::BasicSummon(_)
            | CharacterState::SelfBuff(_)
            | CharacterState::SpriteSummon(_)
            | CharacterState::UseItem(_)
            | CharacterState::Interact(_)
            | CharacterState::Wallrun(_)
            | CharacterState::Skate(_)
            | CharacterState::Music(_)
            | CharacterState::Transform(_)
            | CharacterState::RegrowHead(_) => None,
        }
    }

    pub fn is_aimed(&self) -> bool {
        matches!(
            self,
            CharacterState::BasicMelee(_)
                | CharacterState::BasicRanged(_)
                | CharacterState::DashMelee(_)
                | CharacterState::ComboMelee2(_)
                | CharacterState::BasicBlock(_)
                | CharacterState::LeapExplosionShockwave(_)
                | CharacterState::LeapMelee(_)
                | CharacterState::LeapShockwave(_)
                | CharacterState::ChargedMelee(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
                | CharacterState::Throw(_)
                | CharacterState::Shockwave(_)
                | CharacterState::BasicBeam(_)
                | CharacterState::Stunned(_)
                | CharacterState::Wielding(_)
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
                | CharacterState::Talk(_)
                | CharacterState::Roll(_),
        )
    }

    pub fn is_parry(&self, attack_source: AttackSource) -> bool {
        let melee = matches!(attack_source, AttackSource::Melee);
        let from_capability_melee = melee
            && self
                .ability_info()
                .map(|a| a.ability_meta.capabilities)
                .is_some_and(|c| {
                    c.contains(Capability::PARRIES_MELEE)
                        && matches!(
                            self.stage_section(),
                            Some(StageSection::Buildup | StageSection::Action)
                        )
                });
        let from_capability = matches!(
            attack_source,
            AttackSource::Melee
                | AttackSource::Projectile
                | AttackSource::Beam
                | AttackSource::AirShockwave
                | AttackSource::Explosion
        ) && self
            .ability_info()
            .map(|a| a.ability_meta.capabilities)
            .is_some_and(|c| {
                c.contains(Capability::PARRIES)
                    && matches!(
                        self.stage_section(),
                        Some(StageSection::Buildup | StageSection::Action)
                    )
            });
        let from_state = match self {
            CharacterState::BasicBlock(c) => c.is_parry(attack_source),
            CharacterState::RiposteMelee(c) => {
                melee
                    && matches!(
                        c.stage_section,
                        StageSection::Buildup | StageSection::Action
                    )
            },
            _ => false,
        };
        from_capability_melee || from_capability || from_state
    }

    pub fn is_block(&self, attack_source: AttackSource) -> bool {
        match self {
            CharacterState::BasicBlock(data) => {
                data.static_data.blocked_attacks.applies(attack_source)
                    && matches!(
                        self.stage_section(),
                        Some(StageSection::Buildup | StageSection::Action)
                    )
            },
            _ => self
                .ability_info()
                .map(|ability| ability.ability_meta.capabilities)
                .is_some_and(|capabilities| {
                    capabilities.contains(Capability::BLOCKS)
                        && matches!(
                            self.stage_section(),
                            Some(StageSection::Buildup | StageSection::Action)
                        )
                        && matches!(attack_source, AttackSource::Melee)
                }),
        }
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

    pub fn is_dodge(&self) -> bool {
        if let CharacterState::Roll(c) = self {
            matches!(
                c.stage_section,
                StageSection::Buildup | StageSection::Movement
            )
        } else {
            false
        }
    }

    pub fn is_glide(&self) -> bool { matches!(self, CharacterState::Glide(_)) }

    pub fn is_skate(&self) -> bool { matches!(self, CharacterState::Skate(_)) }

    pub fn is_music(&self) -> bool { matches!(self, CharacterState::Music(_)) }

    pub fn roll_attack_immunities(&self) -> Option<AttackFilters> {
        if self.is_dodge()
            && let CharacterState::Roll(c) = self
        {
            Some(c.static_data.attack_immunities)
        } else {
            None
        }
    }

    pub fn is_stunned(&self) -> bool { matches!(self, CharacterState::Stunned(_)) }

    pub fn is_forced_movement(&self) -> bool {
        matches!(self, CharacterState::ComboMelee2(s) if s.stage_section == StageSection::Action)
            || matches!(self, CharacterState::DashMelee(s) if s.stage_section == StageSection::Charge)
            || matches!(self, CharacterState::LeapMelee(s) if s.stage_section == StageSection::Movement)
            || matches!(self, CharacterState::Roll(s) if s.stage_section == StageSection::Movement)
    }

    pub fn is_melee_attack(&self) -> bool { self.attack_kind().contains(&AttackSource::Melee) }

    pub fn is_beam_attack(&self) -> bool { self.attack_kind().contains(&AttackSource::Beam) }

    pub fn can_perform_mounted(&self) -> bool {
        matches!(
            self,
            CharacterState::Idle(_)
                | CharacterState::Sit
                | CharacterState::Dance
                | CharacterState::Talk(_)
                | CharacterState::Stunned(_)
                | CharacterState::BasicBlock(_)
                | CharacterState::Equipping(_)
                | CharacterState::Wielding(_)
                | CharacterState::BasicMelee(_)
                | CharacterState::BasicRanged(_)
                | CharacterState::ComboMelee2(_)
                | CharacterState::ChargedRanged(_)
                | CharacterState::RepeaterRanged(_)
                | CharacterState::Throw(_)
                | CharacterState::BasicBeam(_)
                | CharacterState::BasicAura(_)
                | CharacterState::BasicSummon(_)
                | CharacterState::SelfBuff(_)
                | CharacterState::SpriteSummon(_)
                | CharacterState::UseItem(_)
                | CharacterState::Interact(_)
                | CharacterState::Music(_)
                | CharacterState::RiposteMelee(_)
                | CharacterState::RapidMelee(_)
        )
    }

    /// A subset of `can_perform_mounted` actions that allow the character
    /// to change their orientation towards their look dir when mounted.
    pub fn can_look_while_mounted(&self) -> bool {
        !matches!(
            self,
            CharacterState::Idle(_)
                | CharacterState::Sit
                | CharacterState::Dance
                | CharacterState::Talk(_)
                | CharacterState::Stunned(_)
                | CharacterState::Equipping(_)
                | CharacterState::SelfBuff(_)
                | CharacterState::BasicAura(_)
        ) && self.can_perform_mounted()
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
            CharacterState::Talk(data) => data.behavior(j, output_events),
            CharacterState::Climb(data) => data.behavior(j, output_events),
            CharacterState::Wallrun(data) => data.behavior(j, output_events),
            CharacterState::Glide(data) => data.behavior(j, output_events),
            CharacterState::GlideWield(data) => data.behavior(j, output_events),
            CharacterState::Stunned(data) => data.behavior(j, output_events),
            CharacterState::Sit => sit::Data::behavior(&sit::Data, j, output_events),
            CharacterState::Crawl => crawl::Data::behavior(&crawl::Data, j, output_events),
            CharacterState::Dance => dance::Data::behavior(&dance::Data, j, output_events),
            CharacterState::BasicBlock(data) => data.behavior(j, output_events),
            CharacterState::Roll(data) => data.behavior(j, output_events),
            CharacterState::Wielding(data) => data.behavior(j, output_events),
            CharacterState::Equipping(data) => data.behavior(j, output_events),
            CharacterState::ComboMelee2(data) => data.behavior(j, output_events),
            CharacterState::BasicMelee(data) => data.behavior(j, output_events),
            CharacterState::BasicRanged(data) => data.behavior(j, output_events),
            CharacterState::Boost(data) => data.behavior(j, output_events),
            CharacterState::DashMelee(data) => data.behavior(j, output_events),
            CharacterState::LeapExplosionShockwave(data) => data.behavior(j, output_events),
            CharacterState::LeapMelee(data) => data.behavior(j, output_events),
            CharacterState::LeapShockwave(data) => data.behavior(j, output_events),
            CharacterState::ChargedMelee(data) => data.behavior(j, output_events),
            CharacterState::ChargedRanged(data) => data.behavior(j, output_events),
            CharacterState::RepeaterRanged(data) => data.behavior(j, output_events),
            CharacterState::Throw(data) => data.behavior(j, output_events),
            CharacterState::Shockwave(data) => data.behavior(j, output_events),
            CharacterState::Explosion(data) => data.behavior(j, output_events),
            CharacterState::BasicBeam(data) => data.behavior(j, output_events),
            CharacterState::BasicAura(data) => data.behavior(j, output_events),
            CharacterState::Blink(data) => data.behavior(j, output_events),
            CharacterState::BasicSummon(data) => data.behavior(j, output_events),
            CharacterState::SelfBuff(data) => data.behavior(j, output_events),
            CharacterState::SpriteSummon(data) => data.behavior(j, output_events),
            CharacterState::UseItem(data) => data.behavior(j, output_events),
            CharacterState::Interact(data) => data.behavior(j, output_events),
            CharacterState::Skate(data) => data.behavior(j, output_events),
            CharacterState::Music(data) => data.behavior(j, output_events),
            CharacterState::FinisherMelee(data) => data.behavior(j, output_events),
            CharacterState::DiveMelee(data) => data.behavior(j, output_events),
            CharacterState::RiposteMelee(data) => data.behavior(j, output_events),
            CharacterState::RapidMelee(data) => data.behavior(j, output_events),
            CharacterState::Transform(data) => data.behavior(j, output_events),
            CharacterState::RegrowHead(data) => data.behavior(j, output_events),
            CharacterState::StaticAura(data) => data.behavior(j, output_events),
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
            CharacterState::Talk(data) => data.handle_event(j, output_events, action),
            CharacterState::Climb(data) => data.handle_event(j, output_events, action),
            CharacterState::Wallrun(data) => data.handle_event(j, output_events, action),
            CharacterState::Glide(data) => data.handle_event(j, output_events, action),
            CharacterState::GlideWield(data) => data.handle_event(j, output_events, action),
            CharacterState::Stunned(data) => data.handle_event(j, output_events, action),
            CharacterState::Sit => {
                states::sit::Data::handle_event(&sit::Data, j, output_events, action)
            },
            CharacterState::Crawl => {
                states::crawl::Data::handle_event(&crawl::Data, j, output_events, action)
            },
            CharacterState::Dance => {
                states::dance::Data::handle_event(&dance::Data, j, output_events, action)
            },
            CharacterState::BasicBlock(data) => data.handle_event(j, output_events, action),
            CharacterState::Roll(data) => data.handle_event(j, output_events, action),
            CharacterState::Wielding(data) => data.handle_event(j, output_events, action),
            CharacterState::Equipping(data) => data.handle_event(j, output_events, action),
            CharacterState::ComboMelee2(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicRanged(data) => data.handle_event(j, output_events, action),
            CharacterState::Boost(data) => data.handle_event(j, output_events, action),
            CharacterState::DashMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::LeapExplosionShockwave(data) => {
                data.handle_event(j, output_events, action)
            },
            CharacterState::LeapMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::LeapShockwave(data) => data.handle_event(j, output_events, action),
            CharacterState::ChargedMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::ChargedRanged(data) => data.handle_event(j, output_events, action),
            CharacterState::RepeaterRanged(data) => data.handle_event(j, output_events, action),
            CharacterState::Throw(data) => data.handle_event(j, output_events, action),
            CharacterState::Shockwave(data) => data.handle_event(j, output_events, action),
            CharacterState::Explosion(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicBeam(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicAura(data) => data.handle_event(j, output_events, action),
            CharacterState::Blink(data) => data.handle_event(j, output_events, action),
            CharacterState::BasicSummon(data) => data.handle_event(j, output_events, action),
            CharacterState::SelfBuff(data) => data.handle_event(j, output_events, action),
            CharacterState::SpriteSummon(data) => data.handle_event(j, output_events, action),
            CharacterState::UseItem(data) => data.handle_event(j, output_events, action),
            CharacterState::Interact(data) => data.handle_event(j, output_events, action),
            CharacterState::Skate(data) => data.handle_event(j, output_events, action),
            CharacterState::Music(data) => data.handle_event(j, output_events, action),
            CharacterState::FinisherMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::DiveMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::RiposteMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::RapidMelee(data) => data.handle_event(j, output_events, action),
            CharacterState::Transform(data) => data.handle_event(j, output_events, action),
            CharacterState::RegrowHead(data) => data.handle_event(j, output_events, action),
            CharacterState::StaticAura(data) => data.handle_event(j, output_events, action),
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
            CharacterState::Talk(_) => None,
            CharacterState::Climb(_) => None,
            CharacterState::Wallrun(_) => None,
            CharacterState::Skate(_) => None,
            CharacterState::Glide(_) => None,
            CharacterState::GlideWield(_) => None,
            CharacterState::Stunned(_) => None,
            CharacterState::Sit => None,
            CharacterState::Crawl => None,
            CharacterState::Dance => None,
            CharacterState::BasicBlock(data) => Some(data.static_data.ability_info),
            CharacterState::Roll(data) => Some(data.static_data.ability_info),
            CharacterState::Wielding(_) => None,
            CharacterState::Equipping(_) => None,
            CharacterState::ComboMelee2(data) => Some(data.static_data.ability_info),
            CharacterState::BasicMelee(data) => Some(data.static_data.ability_info),
            CharacterState::BasicRanged(data) => Some(data.static_data.ability_info),
            CharacterState::Boost(data) => Some(data.static_data.ability_info),
            CharacterState::DashMelee(data) => Some(data.static_data.ability_info),
            CharacterState::LeapExplosionShockwave(data) => Some(data.static_data.ability_info),
            CharacterState::LeapMelee(data) => Some(data.static_data.ability_info),
            CharacterState::LeapShockwave(data) => Some(data.static_data.ability_info),
            CharacterState::ChargedMelee(data) => Some(data.static_data.ability_info),
            CharacterState::ChargedRanged(data) => Some(data.static_data.ability_info),
            CharacterState::RepeaterRanged(data) => Some(data.static_data.ability_info),
            CharacterState::Throw(data) => Some(data.static_data.ability_info),
            CharacterState::Shockwave(data) => Some(data.static_data.ability_info),
            CharacterState::Explosion(data) => Some(data.static_data.ability_info),
            CharacterState::BasicBeam(data) => Some(data.static_data.ability_info),
            CharacterState::BasicAura(data) => Some(data.static_data.ability_info),
            CharacterState::Blink(data) => Some(data.static_data.ability_info),
            CharacterState::BasicSummon(data) => Some(data.static_data.ability_info),
            CharacterState::SelfBuff(data) => Some(data.static_data.ability_info),
            CharacterState::SpriteSummon(data) => Some(data.static_data.ability_info),
            CharacterState::UseItem(_) => None,
            CharacterState::Interact(_) => None,
            CharacterState::FinisherMelee(data) => Some(data.static_data.ability_info),
            CharacterState::Music(data) => Some(data.static_data.ability_info),
            CharacterState::DiveMelee(data) => Some(data.static_data.ability_info),
            CharacterState::RiposteMelee(data) => Some(data.static_data.ability_info),
            CharacterState::RapidMelee(data) => Some(data.static_data.ability_info),
            CharacterState::Transform(data) => Some(data.static_data.ability_info),
            CharacterState::RegrowHead(data) => Some(data.static_data.ability_info),
            CharacterState::StaticAura(data) => Some(data.static_data.ability_info),
        }
    }

    pub fn stage_section(&self) -> Option<StageSection> {
        match &self {
            CharacterState::Idle(_) => None,
            CharacterState::Talk(_) => None,
            CharacterState::Climb(_) => None,
            CharacterState::Wallrun(_) => None,
            CharacterState::Skate(_) => None,
            CharacterState::Glide(_) => None,
            CharacterState::GlideWield(_) => None,
            CharacterState::Stunned(data) => Some(data.stage_section),
            CharacterState::Sit => None,
            CharacterState::Crawl => None,
            CharacterState::Dance => None,
            CharacterState::BasicBlock(data) => Some(data.stage_section),
            CharacterState::Roll(data) => Some(data.stage_section),
            CharacterState::Equipping(_) => Some(StageSection::Buildup),
            CharacterState::Wielding(_) => None,
            CharacterState::ComboMelee2(data) => Some(data.stage_section),
            CharacterState::BasicMelee(data) => Some(data.stage_section),
            CharacterState::BasicRanged(data) => Some(data.stage_section),
            CharacterState::Boost(_) => None,
            CharacterState::DashMelee(data) => Some(data.stage_section),
            CharacterState::LeapExplosionShockwave(data) => Some(data.stage_section),
            CharacterState::LeapMelee(data) => Some(data.stage_section),
            CharacterState::LeapShockwave(data) => Some(data.stage_section),
            CharacterState::ChargedMelee(data) => Some(data.stage_section),
            CharacterState::ChargedRanged(data) => Some(data.stage_section),
            CharacterState::RepeaterRanged(data) => Some(data.stage_section),
            CharacterState::Throw(data) => Some(data.stage_section),
            CharacterState::Shockwave(data) => Some(data.stage_section),
            CharacterState::Explosion(data) => Some(data.stage_section),
            CharacterState::BasicBeam(data) => Some(data.stage_section),
            CharacterState::BasicAura(data) => Some(data.stage_section),
            CharacterState::Blink(data) => Some(data.stage_section),
            CharacterState::BasicSummon(data) => Some(data.stage_section),
            CharacterState::SelfBuff(data) => Some(data.stage_section),
            CharacterState::SpriteSummon(data) => Some(data.stage_section),
            CharacterState::UseItem(data) => Some(data.stage_section),
            CharacterState::Interact(data) => Some(data.stage_section),
            CharacterState::FinisherMelee(data) => Some(data.stage_section),
            CharacterState::Music(data) => Some(data.stage_section),
            CharacterState::DiveMelee(data) => Some(data.stage_section),
            CharacterState::RiposteMelee(data) => Some(data.stage_section),
            CharacterState::RapidMelee(data) => Some(data.stage_section),
            CharacterState::Transform(data) => Some(data.stage_section),
            CharacterState::RegrowHead(data) => Some(data.stage_section),
            CharacterState::StaticAura(data) => Some(data.stage_section),
        }
    }

    pub fn durations(&self) -> Option<DurationsInfo> {
        match &self {
            CharacterState::Idle(_) => None,
            CharacterState::Talk(_) => None,
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
            CharacterState::Crawl => None,
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
            CharacterState::LeapExplosionShockwave(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                movement: Some(data.static_data.movement_duration),
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
            CharacterState::ChargedMelee(data) => Some(DurationsInfo {
                buildup: data.static_data.buildup_strike.map(|x| x.0),
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
            CharacterState::Throw(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                charge: Some(data.static_data.charge_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::Shockwave(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::Explosion(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.action_duration),
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
            CharacterState::Interact(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: data.static_data.use_duration,
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
                buildup: data.static_data.buildup_duration,
                movement: Some(data.static_data.movement_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::RiposteMelee(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(if data.whiffed {
                    data.static_data.whiffed_recover_duration
                } else {
                    data.static_data.recover_duration
                }),
                ..Default::default()
            }),
            CharacterState::RapidMelee(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.swing_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::Transform(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::RegrowHead(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
            CharacterState::StaticAura(data) => Some(DurationsInfo {
                buildup: Some(data.static_data.buildup_duration),
                action: Some(data.static_data.cast_duration),
                recover: Some(data.static_data.recover_duration),
                ..Default::default()
            }),
        }
    }

    pub fn timer(&self) -> Option<Duration> {
        match &self {
            CharacterState::Idle(_) => None,
            CharacterState::Crawl => None,
            CharacterState::Talk(_) => None,
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
            CharacterState::ComboMelee2(data) => Some(data.timer),
            CharacterState::BasicMelee(data) => Some(data.timer),
            CharacterState::BasicRanged(data) => Some(data.timer),
            CharacterState::Boost(data) => Some(data.timer),
            CharacterState::DashMelee(data) => Some(data.timer),
            CharacterState::LeapExplosionShockwave(data) => Some(data.timer),
            CharacterState::LeapMelee(data) => Some(data.timer),
            CharacterState::LeapShockwave(data) => Some(data.timer),
            CharacterState::ChargedMelee(data) => Some(data.timer),
            CharacterState::ChargedRanged(data) => Some(data.timer),
            CharacterState::RepeaterRanged(data) => Some(data.timer),
            CharacterState::Throw(data) => Some(data.timer),
            CharacterState::Shockwave(data) => Some(data.timer),
            CharacterState::Explosion(data) => Some(data.timer),
            CharacterState::BasicBeam(data) => Some(data.timer),
            CharacterState::BasicAura(data) => Some(data.timer),
            CharacterState::Blink(data) => Some(data.timer),
            CharacterState::BasicSummon(data) => Some(data.timer),
            CharacterState::SelfBuff(data) => Some(data.timer),
            CharacterState::SpriteSummon(data) => Some(data.timer),
            CharacterState::UseItem(data) => Some(data.timer),
            CharacterState::Interact(data) => Some(data.timer),
            CharacterState::FinisherMelee(data) => Some(data.timer),
            CharacterState::Music(data) => Some(data.timer),
            CharacterState::DiveMelee(data) => Some(data.timer),
            CharacterState::RiposteMelee(data) => Some(data.timer),
            CharacterState::RapidMelee(data) => Some(data.timer),
            CharacterState::Transform(data) => Some(data.timer),
            CharacterState::RegrowHead(data) => Some(data.timer),
            CharacterState::StaticAura(data) => Some(data.timer),
        }
    }

    pub fn attack_kind(&self) -> &[AttackSource] {
        match self {
            CharacterState::Idle(_) => &[],
            CharacterState::Crawl => &[],
            CharacterState::Talk(_) => &[],
            CharacterState::Climb(_) => &[],
            CharacterState::Wallrun(_) => &[],
            CharacterState::Skate(_) => &[],
            CharacterState::Glide(_) => &[],
            CharacterState::GlideWield(_) => &[],
            CharacterState::Stunned(_) => &[],
            CharacterState::Sit => &[],
            CharacterState::Dance => &[],
            CharacterState::BasicBlock(_) => &[],
            CharacterState::Roll(_) => &[],
            CharacterState::Wielding(_) => &[],
            CharacterState::Equipping(_) => &[],
            CharacterState::ComboMelee2(_) => &[AttackSource::Melee],
            CharacterState::BasicMelee(_) => &[AttackSource::Melee],
            CharacterState::BasicRanged(data) => {
                if data.static_data.projectile.is_explosive() {
                    &[AttackSource::Explosion]
                } else {
                    &[AttackSource::Projectile]
                }
            },
            CharacterState::Boost(_) => &[],
            CharacterState::DashMelee(_) => &[AttackSource::Melee],
            CharacterState::LeapMelee(_) => &[AttackSource::Melee],
            CharacterState::ChargedMelee(_) => &[AttackSource::Melee],
            // TODO: When charged ranged not only arrow make this check projectile type
            CharacterState::ChargedRanged(_) => &[AttackSource::Projectile],
            CharacterState::RepeaterRanged(data) => {
                if data.static_data.projectile.is_explosive() {
                    &[AttackSource::Explosion]
                } else {
                    &[AttackSource::Projectile]
                }
            },
            CharacterState::LeapExplosionShockwave(data) => data
                .static_data
                .shockwave_dodgeable
                .explosion_shockwave_attack_source_slice(),
            CharacterState::Throw(_) => &[AttackSource::Projectile],
            CharacterState::Shockwave(data) => {
                data.static_data.dodgeable.shockwave_attack_source_slice()
            },
            CharacterState::LeapShockwave(data) => {
                data.static_data.dodgeable.shockwave_attack_source_slice()
            },
            CharacterState::Explosion(_) => &[AttackSource::Explosion],
            CharacterState::BasicBeam(_) => &[AttackSource::Beam],
            CharacterState::BasicAura(_) => &[],
            CharacterState::Blink(_) => &[],
            CharacterState::BasicSummon(_) => &[],
            CharacterState::SelfBuff(_) => &[],
            CharacterState::SpriteSummon(_) => &[],
            CharacterState::UseItem(_) => &[],
            CharacterState::Interact(_) => &[],
            CharacterState::FinisherMelee(_) => &[AttackSource::Melee],
            CharacterState::Music(_) => &[],
            CharacterState::DiveMelee(_) => &[AttackSource::Melee],
            CharacterState::RiposteMelee(_) => &[AttackSource::Melee],
            CharacterState::RapidMelee(_) => &[AttackSource::Melee],
            CharacterState::Transform(_) => &[],
            CharacterState::RegrowHead(_) => &[],
            CharacterState::StaticAura(_) => &[],
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
            AttackSource::UndodgeableShockwave => false,
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

/// Contains information about the visual activity of a character.
///
/// For now this only includes the direction they're looking in, but later it
/// might include markers indicating that they're available for
/// trade/interaction, more details about their stance or appearance, facial
/// expression, etc.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterActivity {
    /// `None` means that the look direction should be derived from the
    /// orientation
    pub look_dir: Option<Dir>,
    /// If the character is using a Helm, this is the y direction the
    /// character steering. If the character is not steering this is
    /// a stale value.
    pub steer_dir: f32,
    /// If true, the owner has set this pet to stay at a fixed location and
    /// to not engage in combat
    pub is_pet_staying: bool,
}

impl Component for CharacterActivity {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
