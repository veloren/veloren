use crate::{
    assets::{self, Asset},
    combat::{self, CombatEffect, DamageKind, Knockback},
    comp::{
        self, aura, beam, buff,
        character_state::AttackFilters,
        inventory::{
            item::{
                tool::{
                    AbilityContext, AbilityItem, AbilityKind, ContextualIndex, Stats, ToolKind,
                },
                ItemKind,
            },
            slot::EquipSlot,
            Inventory,
        },
        melee::{CustomCombo, MeleeConstructor, MeleeConstructorKind},
        projectile::ProjectileConstructor,
        skillset::{
            skills::{self, Skill, SKILL_MODIFIERS},
            SkillSet,
        },
        Body, CharacterState, LightEmitter, StateUpdate,
    },
    resources::Secs,
    states::{
        behavior::JoinData,
        sprite_summon::SpriteSummonAnchor,
        utils::{AbilityInfo, ComboConsumption, ScalingKind, StageSection},
        *,
    },
    terrain::SpriteKind,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::{borrow::Cow, time::Duration};

use super::shockwave::ShockwaveDodgeable;

pub const BASE_ABILITY_LIMIT: usize = 5;

// NOTE: different AbilitySpec on same ToolKind share the same key
/// Descriptor to pick the right (auxiliary) ability set
pub type AuxiliaryKey = (Option<ToolKind>, Option<ToolKind>);

// TODO: Potentially look into storing previous ability sets for weapon
// combinations and automatically reverting back to them on switching to that
// set of weapons. Consider after UI is set up and people weigh in on memory
// considerations.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveAbilities {
    pub guard: GuardAbility,
    pub primary: PrimaryAbility,
    pub secondary: SecondaryAbility,
    pub movement: MovementAbility,
    pub limit: Option<usize>,
    pub auxiliary_sets: HashMap<AuxiliaryKey, Vec<AuxiliaryAbility>>,
}

impl Component for ActiveAbilities {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

impl Default for ActiveAbilities {
    fn default() -> Self {
        Self {
            guard: GuardAbility::Tool,
            primary: PrimaryAbility::Tool,
            secondary: SecondaryAbility::Tool,
            movement: MovementAbility::Species,
            limit: None,
            auxiliary_sets: HashMap::new(),
        }
    }
}

// make it pub, for UI stuff, if you want
enum AbilitySource {
    Weapons,
    Glider,
}

impl AbilitySource {
    // Get all needed data here and pick the right ability source
    //
    // make it pub, for UI stuff, if you want
    fn determine(char_state: Option<&CharacterState>) -> Self {
        if char_state.is_some_and(|c| c.is_glide_wielded()) {
            Self::Glider
        } else {
            Self::Weapons
        }
    }
}

impl ActiveAbilities {
    pub fn from_auxiliary(
        auxiliary_sets: HashMap<AuxiliaryKey, Vec<AuxiliaryAbility>>,
        limit: Option<usize>,
    ) -> Self {
        // Discard any sets that exceed the limit
        ActiveAbilities {
            auxiliary_sets: auxiliary_sets
                .into_iter()
                .filter(|(_, set)| limit.map_or(true, |limit| set.len() == limit))
                .collect(),
            limit,
            ..Self::default()
        }
    }

    pub fn default_limited(limit: usize) -> Self {
        ActiveAbilities {
            limit: Some(limit),
            ..Default::default()
        }
    }

    pub fn change_ability(
        &mut self,
        slot: usize,
        auxiliary_key: AuxiliaryKey,
        new_ability: AuxiliaryAbility,
        inventory: Option<&Inventory>,
        skill_set: Option<&SkillSet>,
    ) {
        let auxiliary_set = self
            .auxiliary_sets
            .entry(auxiliary_key)
            .or_insert(Self::default_ability_set(inventory, skill_set, self.limit));
        if let Some(ability) = auxiliary_set.get_mut(slot) {
            *ability = new_ability;
        }
    }

    pub fn active_auxiliary_key(inv: Option<&Inventory>) -> AuxiliaryKey {
        let tool_kind = |slot| {
            inv.and_then(|inv| inv.equipped(slot))
                .and_then(|item| match &*item.kind() {
                    ItemKind::Tool(tool) => Some(tool.kind),
                    _ => None,
                })
        };

        (
            tool_kind(EquipSlot::ActiveMainhand),
            tool_kind(EquipSlot::ActiveOffhand),
        )
    }

    pub fn auxiliary_set(
        &self,
        inv: Option<&Inventory>,
        skill_set: Option<&SkillSet>,
    ) -> Cow<Vec<AuxiliaryAbility>> {
        let aux_key = Self::active_auxiliary_key(inv);

        self.auxiliary_sets
            .get(&aux_key)
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(Self::default_ability_set(inv, skill_set, self.limit)))
    }

    pub fn get_ability(
        &self,
        input: AbilityInput,
        inventory: Option<&Inventory>,
        skill_set: Option<&SkillSet>,
        stats: Option<&comp::Stats>,
    ) -> Ability {
        match input {
            AbilityInput::Guard => self.guard.into(),
            AbilityInput::Primary => self.primary.into(),
            AbilityInput::Secondary => self.secondary.into(),
            AbilityInput::Movement => self.movement.into(),
            AbilityInput::Auxiliary(index) => {
                if stats.map_or(false, |s| s.disable_auxiliary_abilities) {
                    Ability::Empty
                } else {
                    self.auxiliary_set(inventory, skill_set)
                        .get(index)
                        .copied()
                        .map(|a| a.into())
                        .unwrap_or(Ability::Empty)
                }
            },
        }
    }

    /// Returns the CharacterAbility from an ability input, and also whether the
    /// ability was from a weapon wielded in the offhand
    pub fn activate_ability(
        &self,
        input: AbilityInput,
        inv: Option<&Inventory>,
        skill_set: &SkillSet,
        body: Option<&Body>,
        char_state: Option<&CharacterState>,
        context: &AbilityContext,
        stats: Option<&comp::Stats>,
        // bool is from_offhand
    ) -> Option<(CharacterAbility, bool, SpecifiedAbility)> {
        let ability = self.get_ability(input, inv, Some(skill_set), stats);

        let ability_set = |equip_slot| {
            inv.and_then(|inv| inv.equipped(equip_slot))
                .and_then(|i| i.item_config().map(|c| &c.abilities))
        };

        let scale_ability = |ability: CharacterAbility, equip_slot| {
            let tool_kind = inv
                .and_then(|inv| inv.equipped(equip_slot))
                .and_then(|item| match &*item.kind() {
                    ItemKind::Tool(tool) => Some(tool.kind),
                    _ => None,
                });
            ability.adjusted_by_skills(skill_set, tool_kind)
        };

        let spec_ability = |context_index| SpecifiedAbility {
            ability,
            context_index,
        };

        // This function is an attempt to generalize ability handling
        let inst_ability = |slot: EquipSlot, offhand: bool| {
            ability_set(slot).and_then(|abilities| {
                // We use AbilityInput here as an object to match on, which
                // roughly corresponds to all needed data we need to know about
                // ability.
                use AbilityInput as I;

                // Also we don't provide `ability`, nor `ability_input` as an
                // argument to the closure, and that wins us a bit of code
                // duplication we would need to do otherwise, but it's
                // important that we can and do re-create all needed Ability
                // information here to make decisions.
                //
                // For example, we should't take `input` argument provided to
                // activate_abilities, because in case of Auxiliary abilities,
                // it has wrong index.
                //
                // We could alternatively just take `ability`, but it works too.
                let dispatched = match ability.try_ability_set_key()? {
                    I::Guard => abilities.guard(Some(skill_set), context),
                    I::Primary => abilities.primary(Some(skill_set), context),
                    I::Secondary => abilities.secondary(Some(skill_set), context),
                    I::Auxiliary(index) => abilities.auxiliary(index, Some(skill_set), context),
                    I::Movement => return None,
                };

                dispatched
                    .map(|(a, i)| (a.ability.clone(), i))
                    .map(|(a, i)| (scale_ability(a, slot), offhand, spec_ability(i)))
            })
        };

        let source = AbilitySource::determine(char_state);

        match ability {
            Ability::ToolGuard => match source {
                AbilitySource::Weapons => {
                    let equip_slot = combat::get_equip_slot_by_block_priority(inv);
                    inst_ability(equip_slot, matches!(equip_slot, EquipSlot::ActiveOffhand))
                },
                AbilitySource::Glider => None,
            },
            Ability::ToolPrimary => match source {
                AbilitySource::Weapons => inst_ability(EquipSlot::ActiveMainhand, false),
                AbilitySource::Glider => inst_ability(EquipSlot::Glider, false),
            },
            Ability::ToolSecondary => match source {
                AbilitySource::Weapons => inst_ability(EquipSlot::ActiveOffhand, true)
                    .or_else(|| inst_ability(EquipSlot::ActiveMainhand, false)),
                AbilitySource::Glider => inst_ability(EquipSlot::Glider, false),
            },
            Ability::MainWeaponAux(_) => inst_ability(EquipSlot::ActiveMainhand, false),
            Ability::OffWeaponAux(_) => inst_ability(EquipSlot::ActiveOffhand, true),
            Ability::GliderAux(_) => inst_ability(EquipSlot::Glider, false),
            Ability::Empty => None,
            Ability::SpeciesMovement => matches!(body, Some(Body::Humanoid(_)))
                .then(|| CharacterAbility::default_roll(char_state))
                .map(|ability| {
                    (
                        ability.adjusted_by_skills(skill_set, None),
                        false,
                        spec_ability(None),
                    )
                }),
        }
    }

    pub fn iter_available_abilities_on<'a>(
        inv: Option<&'a Inventory>,
        skill_set: Option<&'a SkillSet>,
        equip_slot: EquipSlot,
    ) -> impl Iterator<Item = usize> + 'a {
        inv.and_then(|inv| inv.equipped(equip_slot).and_then(|i| i.item_config()))
            .into_iter()
            .flat_map(|config| &config.abilities.abilities)
            .enumerate()
            .filter_map(move |(i, a)| match a {
                AbilityKind::Simple(skill, _) => skill
                    .map_or(true, |s| skill_set.map_or(false, |ss| ss.has_skill(s)))
                    .then_some(i),
                AbilityKind::Contextualized {
                    pseudo_id: _,
                    abilities,
                } => abilities
                    .iter()
                    .any(|(_contexts, (skill, _))| {
                        skill.map_or(true, |s| skill_set.map_or(false, |ss| ss.has_skill(s)))
                    })
                    .then_some(i),
            })
    }

    pub fn all_available_abilities(
        inv: Option<&Inventory>,
        skill_set: Option<&SkillSet>,
    ) -> Vec<AuxiliaryAbility> {
        let mut ability_buff = vec![];
        // Check if uses combo of two "equal" weapons
        let paired = inv
            .and_then(|inv| {
                let a = inv.equipped(EquipSlot::ActiveMainhand)?;
                let b = inv.equipped(EquipSlot::ActiveOffhand)?;

                if let (ItemKind::Tool(tool_a), ItemKind::Tool(tool_b)) = (&*a.kind(), &*b.kind()) {
                    Some((a.ability_spec(), tool_a.kind, b.ability_spec(), tool_b.kind))
                } else {
                    None
                }
            })
            .is_some_and(|(a_spec, a_kind, b_spec, b_kind)| (a_spec, a_kind) == (b_spec, b_kind));

        // Push main weapon abilities
        Self::iter_available_abilities_on(inv, skill_set, EquipSlot::ActiveMainhand)
            .map(AuxiliaryAbility::MainWeapon)
            .for_each(|a| ability_buff.push(a));

        // Push secondary weapon abilities, if different
        // If equal, just take the first
        if !paired {
            Self::iter_available_abilities_on(inv, skill_set, EquipSlot::ActiveOffhand)
                .map(AuxiliaryAbility::OffWeapon)
                .for_each(|a| ability_buff.push(a));
        }
        // Push glider abilities
        Self::iter_available_abilities_on(inv, skill_set, EquipSlot::Glider)
            .map(AuxiliaryAbility::Glider)
            .for_each(|a| ability_buff.push(a));

        ability_buff
    }

    fn default_ability_set<'a>(
        inv: Option<&'a Inventory>,
        skill_set: Option<&'a SkillSet>,
        limit: Option<usize>,
    ) -> Vec<AuxiliaryAbility> {
        let mut iter = Self::iter_available_abilities_on(inv, skill_set, EquipSlot::ActiveMainhand)
            .map(AuxiliaryAbility::MainWeapon)
            .chain(
                Self::iter_available_abilities_on(inv, skill_set, EquipSlot::ActiveOffhand)
                    .map(AuxiliaryAbility::OffWeapon),
            );

        if let Some(limit) = limit {
            (0..limit)
                .map(|_| iter.next().unwrap_or(AuxiliaryAbility::Empty))
                .collect()
        } else {
            iter.collect()
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum AbilityInput {
    Guard,
    Primary,
    Secondary,
    Movement,
    Auxiliary(usize),
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Ability {
    ToolGuard,
    ToolPrimary,
    ToolSecondary,
    SpeciesMovement,
    MainWeaponAux(usize),
    OffWeaponAux(usize),
    GliderAux(usize),
    Empty,
    /* For future use
     * ArmorAbility(usize), */
}

impl Ability {
    // Used for generic ability dispatch (inst_ability) in this file
    //
    // It does use AbilityInput to avoid creating just another enum, but it is
    // semantically different.
    fn try_ability_set_key(&self) -> Option<AbilityInput> {
        let input = match self {
            Self::ToolGuard => AbilityInput::Guard,
            Self::ToolPrimary => AbilityInput::Primary,
            Self::ToolSecondary => AbilityInput::Secondary,
            Self::SpeciesMovement => AbilityInput::Movement,
            Self::GliderAux(idx) | Self::OffWeaponAux(idx) | Self::MainWeaponAux(idx) => {
                AbilityInput::Auxiliary(*idx)
            },
            Self::Empty => return None,
        };

        Some(input)
    }

    pub fn ability_id<'a>(
        self,
        char_state: Option<&CharacterState>,
        inv: Option<&'a Inventory>,
        skill_set: Option<&'a SkillSet>,
        context: &AbilityContext,
    ) -> Option<&'a str> {
        let ability_set = |equip_slot| {
            inv.and_then(|inv| inv.equipped(equip_slot))
                .and_then(|i| i.item_config().map(|c| &c.abilities))
        };

        let contextual_id = |kind: Option<&'a AbilityKind<_>>| -> Option<&'a str> {
            if let Some(AbilityKind::Contextualized {
                pseudo_id,
                abilities: _,
            }) = kind
            {
                Some(pseudo_id.as_str())
            } else {
                None
            }
        };

        let inst_ability = |slot: EquipSlot| {
            ability_set(slot).and_then(|abilities| {
                use AbilityInput as I;

                let dispatched = match self.try_ability_set_key()? {
                    I::Guard => abilities.guard(skill_set, context),
                    I::Primary => abilities.primary(skill_set, context),
                    I::Secondary => abilities.secondary(skill_set, context),
                    I::Auxiliary(index) => abilities.auxiliary(index, skill_set, context),
                    I::Movement => return None,
                };

                dispatched.map(|(a, _)| a.id.as_str()).or_else(|| {
                    match self.try_ability_set_key()? {
                        I::Guard => abilities
                            .guard
                            .as_ref()
                            .and_then(|g| contextual_id(Some(g))),
                        I::Primary => contextual_id(Some(&abilities.primary)),
                        I::Secondary => contextual_id(Some(&abilities.secondary)),
                        I::Auxiliary(index) => contextual_id(abilities.abilities.get(index)),
                        I::Movement => None,
                    }
                })
            })
        };

        let source = AbilitySource::determine(char_state);
        match source {
            AbilitySource::Glider => match self {
                Ability::ToolGuard => None,
                Ability::ToolPrimary => inst_ability(EquipSlot::Glider),
                Ability::ToolSecondary => inst_ability(EquipSlot::Glider),
                Ability::SpeciesMovement => None, // TODO: Make not None
                Ability::MainWeaponAux(_) => inst_ability(EquipSlot::ActiveMainhand),
                Ability::OffWeaponAux(_) => inst_ability(EquipSlot::ActiveOffhand),
                Ability::GliderAux(_) => inst_ability(EquipSlot::Glider),
                Ability::Empty => None,
            },
            AbilitySource::Weapons => match self {
                Ability::ToolGuard => {
                    let equip_slot = combat::get_equip_slot_by_block_priority(inv);
                    inst_ability(equip_slot)
                },
                Ability::ToolPrimary => inst_ability(EquipSlot::ActiveMainhand),
                Ability::ToolSecondary => inst_ability(EquipSlot::ActiveOffhand)
                    .or_else(|| inst_ability(EquipSlot::ActiveMainhand)),
                Ability::SpeciesMovement => None, // TODO: Make not None
                Ability::MainWeaponAux(_) => inst_ability(EquipSlot::ActiveMainhand),
                Ability::OffWeaponAux(_) => inst_ability(EquipSlot::ActiveOffhand),
                Ability::GliderAux(_) => inst_ability(EquipSlot::Glider),
                Ability::Empty => None,
            },
        }
    }

    pub fn is_from_wielded(&self) -> bool {
        match self {
            Ability::ToolPrimary
            | Ability::ToolSecondary
            | Ability::MainWeaponAux(_)
            | Ability::GliderAux(_)
            | Ability::OffWeaponAux(_)
            | Ability::ToolGuard => true,
            Ability::SpeciesMovement | Ability::Empty => false,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum GuardAbility {
    Tool,
    Empty,
}

impl From<GuardAbility> for Ability {
    fn from(guard: GuardAbility) -> Self {
        match guard {
            GuardAbility::Tool => Ability::ToolGuard,
            GuardAbility::Empty => Ability::Empty,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SpecifiedAbility {
    pub ability: Ability,
    pub context_index: Option<ContextualIndex>,
}

impl SpecifiedAbility {
    pub fn ability_id<'a>(
        self,
        char_state: Option<&CharacterState>,
        inv: Option<&'a Inventory>,
    ) -> Option<&'a str> {
        let ability_set = |equip_slot| {
            inv.and_then(|inv| inv.equipped(equip_slot))
                .and_then(|i| i.item_config().map(|c| &c.abilities))
        };

        fn ability_id(spec_ability: SpecifiedAbility, ability: &AbilityKind<AbilityItem>) -> &str {
            match ability {
                AbilityKind::Simple(_, a) => a.id.as_str(),
                AbilityKind::Contextualized {
                    pseudo_id,
                    abilities,
                } => spec_ability
                    .context_index
                    .and_then(|i| abilities.get(i.0))
                    .map_or(pseudo_id.as_str(), |(_, (_, a))| a.id.as_str()),
            }
        }

        let inst_ability = |slot: EquipSlot| {
            ability_set(slot).and_then(|abilities| {
                use AbilityInput as I;

                let dispatched = match self.ability.try_ability_set_key()? {
                    I::Guard => abilities.guard.as_ref(),
                    I::Primary => Some(&abilities.primary),
                    I::Secondary => Some(&abilities.secondary),
                    I::Auxiliary(index) => abilities.abilities.get(index),
                    I::Movement => return None,
                };
                dispatched.map(|a| ability_id(self, a))
            })
        };

        let source = AbilitySource::determine(char_state);
        match source {
            AbilitySource::Glider => match self.ability {
                Ability::ToolGuard => None,
                Ability::ToolPrimary => inst_ability(EquipSlot::Glider),
                Ability::ToolSecondary => inst_ability(EquipSlot::Glider),
                Ability::SpeciesMovement => None,
                Ability::MainWeaponAux(_) => inst_ability(EquipSlot::ActiveMainhand),
                Ability::OffWeaponAux(_) => inst_ability(EquipSlot::ActiveOffhand),
                Ability::GliderAux(_) => inst_ability(EquipSlot::Glider),
                Ability::Empty => None,
            },
            AbilitySource::Weapons => match self.ability {
                Ability::ToolGuard => inst_ability(combat::get_equip_slot_by_block_priority(inv)),
                Ability::ToolPrimary => inst_ability(EquipSlot::ActiveMainhand),
                Ability::ToolSecondary => inst_ability(EquipSlot::ActiveOffhand)
                    .or_else(|| inst_ability(EquipSlot::ActiveMainhand)),
                Ability::SpeciesMovement => None, // TODO: Make not None
                Ability::MainWeaponAux(_) => inst_ability(EquipSlot::ActiveMainhand),
                Ability::OffWeaponAux(_) => inst_ability(EquipSlot::ActiveOffhand),
                Ability::GliderAux(_) => inst_ability(EquipSlot::Glider),
                Ability::Empty => None,
            },
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum PrimaryAbility {
    Tool,
    Empty,
}

impl From<PrimaryAbility> for Ability {
    fn from(primary: PrimaryAbility) -> Self {
        match primary {
            PrimaryAbility::Tool => Ability::ToolPrimary,
            PrimaryAbility::Empty => Ability::Empty,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum SecondaryAbility {
    Tool,
    Empty,
}

impl From<SecondaryAbility> for Ability {
    fn from(primary: SecondaryAbility) -> Self {
        match primary {
            SecondaryAbility::Tool => Ability::ToolSecondary,
            SecondaryAbility::Empty => Ability::Empty,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum MovementAbility {
    Species,
    Empty,
}

impl From<MovementAbility> for Ability {
    fn from(primary: MovementAbility) -> Self {
        match primary {
            MovementAbility::Species => Ability::SpeciesMovement,
            MovementAbility::Empty => Ability::Empty,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum AuxiliaryAbility {
    MainWeapon(usize),
    OffWeapon(usize),
    Glider(usize),
    Empty,
}

impl From<AuxiliaryAbility> for Ability {
    fn from(primary: AuxiliaryAbility) -> Self {
        match primary {
            AuxiliaryAbility::MainWeapon(i) => Ability::MainWeaponAux(i),
            AuxiliaryAbility::OffWeapon(i) => Ability::OffWeaponAux(i),
            AuxiliaryAbility::Glider(i) => Ability::GliderAux(i),
            AuxiliaryAbility::Empty => Ability::Empty,
        }
    }
}

/// A lighter form of character state to pass around as needed for frontend
/// purposes
// Only add to this enum as needed for frontends, not necessary to immediately
// add a variant here when adding a new character state
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum CharacterAbilityType {
    BasicMelee(StageSection),
    BasicRanged,
    Boost,
    ChargedMelee(StageSection),
    ChargedRanged,
    DashMelee(StageSection),
    BasicBlock,
    ComboMelee2(StageSection),
    FinisherMelee(StageSection),
    DiveMelee(StageSection),
    RiposteMelee(StageSection),
    RapidMelee(StageSection),
    LeapMelee(StageSection),
    LeapShockwave(StageSection),
    Music(StageSection),
    Shockwave,
    BasicBeam,
    RepeaterRanged,
    BasicAura,
    SelfBuff,
    Other,
}

impl From<&CharacterState> for CharacterAbilityType {
    fn from(state: &CharacterState) -> Self {
        match state {
            CharacterState::BasicMelee(data) => Self::BasicMelee(data.stage_section),
            CharacterState::BasicRanged(_) => Self::BasicRanged,
            CharacterState::Boost(_) => Self::Boost,
            CharacterState::DashMelee(data) => Self::DashMelee(data.stage_section),
            CharacterState::BasicBlock(_) => Self::BasicBlock,
            CharacterState::LeapMelee(data) => Self::LeapMelee(data.stage_section),
            CharacterState::LeapShockwave(data) => Self::LeapShockwave(data.stage_section),
            CharacterState::ComboMelee2(data) => Self::ComboMelee2(data.stage_section),
            CharacterState::FinisherMelee(data) => Self::FinisherMelee(data.stage_section),
            CharacterState::DiveMelee(data) => Self::DiveMelee(data.stage_section),
            CharacterState::RiposteMelee(data) => Self::RiposteMelee(data.stage_section),
            CharacterState::RapidMelee(data) => Self::RapidMelee(data.stage_section),
            CharacterState::ChargedMelee(data) => Self::ChargedMelee(data.stage_section),
            CharacterState::ChargedRanged(_) => Self::ChargedRanged,
            CharacterState::Shockwave(_) => Self::Shockwave,
            CharacterState::BasicBeam(_) => Self::BasicBeam,
            CharacterState::RepeaterRanged(_) => Self::RepeaterRanged,
            CharacterState::BasicAura(_) => Self::BasicAura,
            CharacterState::SelfBuff(_) => Self::SelfBuff,
            CharacterState::Music(data) => Self::Music(data.stage_section),
            CharacterState::Idle(_)
            | CharacterState::Climb(_)
            | CharacterState::Sit
            | CharacterState::Dance
            | CharacterState::Pet(_)
            | CharacterState::Talk
            | CharacterState::Glide(_)
            | CharacterState::GlideWield(_)
            | CharacterState::Stunned(_)
            | CharacterState::Equipping(_)
            | CharacterState::Wielding(_)
            | CharacterState::Roll(_)
            | CharacterState::Blink(_)
            | CharacterState::BasicSummon(_)
            | CharacterState::SpriteSummon(_)
            | CharacterState::UseItem(_)
            | CharacterState::SpriteInteract(_)
            | CharacterState::Skate(_)
            | CharacterState::Transform(_)
            | CharacterState::Wallrun(_)
            | CharacterState::StaticAura(_) => Self::Other,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// For documentation on individual fields, see the corresponding character
/// state file in 'common/src/states/'
pub enum CharacterAbility {
    BasicMelee {
        energy_cost: f32,
        buildup_duration: f32,
        swing_duration: f32,
        hit_timing: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        ori_modifier: f32,
        frontend_specifier: Option<basic_melee::FrontendSpecifier>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicRanged {
        energy_cost: f32,
        buildup_duration: f32,
        recover_duration: f32,
        projectile: ProjectileConstructor,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_speed: f32,
        num_projectiles: u32,
        projectile_spread: f32,
        damage_effect: Option<CombatEffect>,
        move_efficiency: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    RepeaterRanged {
        energy_cost: f32,
        buildup_duration: f32,
        shoot_duration: f32,
        recover_duration: f32,
        max_speed: f32,
        half_speed_at: u32,
        projectile: ProjectileConstructor,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_speed: f32,
        damage_effect: Option<CombatEffect>,
        properties_of_aoe: Option<repeater_ranged::ProjectileOffset>,
        specifier: Option<repeater_ranged::FrontendSpecifier>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Boost {
        movement_duration: f32,
        only_up: bool,
        speed: f32,
        max_exit_velocity: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    GlideBoost {
        booster: glide::Boost,
        #[serde(default)]
        meta: AbilityMeta,
    },
    DashMelee {
        energy_cost: f32,
        energy_drain: f32,
        forward_speed: f32,
        buildup_duration: f32,
        charge_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        ori_modifier: f32,
        auto_charge: bool,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicBlock {
        buildup_duration: f32,
        recover_duration: f32,
        max_angle: f32,
        block_strength: f32,
        parry_window: basic_block::ParryWindow,
        energy_cost: f32,
        energy_regen: f32,
        can_hold: bool,
        blocked_attacks: AttackFilters,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Roll {
        energy_cost: f32,
        buildup_duration: f32,
        movement_duration: f32,
        recover_duration: f32,
        roll_strength: f32,
        attack_immunities: AttackFilters,
        #[serde(default)]
        meta: AbilityMeta,
    },
    ComboMelee2 {
        strikes: Vec<combo_melee2::Strike<f32>>,
        energy_cost_per_strike: f32,
        specifier: Option<combo_melee2::FrontendSpecifier>,
        #[serde(default)]
        auto_progress: bool,
        #[serde(default)]
        meta: AbilityMeta,
    },
    LeapMelee {
        energy_cost: f32,
        buildup_duration: f32,
        movement_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        forward_leap_strength: f32,
        vertical_leap_strength: f32,
        damage_effect: Option<CombatEffect>,
        specifier: Option<leap_melee::FrontendSpecifier>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    LeapShockwave {
        energy_cost: f32,
        buildup_duration: f32,
        movement_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        damage: f32,
        poise_damage: f32,
        knockback: Knockback,
        shockwave_angle: f32,
        shockwave_vertical_angle: f32,
        shockwave_speed: f32,
        shockwave_duration: f32,
        dodgeable: ShockwaveDodgeable,
        move_efficiency: f32,
        damage_kind: DamageKind,
        specifier: comp::shockwave::FrontendSpecifier,
        damage_effect: Option<CombatEffect>,
        forward_leap_strength: f32,
        vertical_leap_strength: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    ChargedMelee {
        energy_cost: f32,
        energy_drain: f32,
        buildup_strike: Option<(f32, MeleeConstructor)>,
        charge_duration: f32,
        swing_duration: f32,
        hit_timing: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        specifier: Option<charged_melee::FrontendSpecifier>,
        damage_effect: Option<CombatEffect>,
        custom_combo: Option<CustomCombo>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    ChargedRanged {
        energy_cost: f32,
        energy_drain: f32,
        initial_regen: f32,
        scaled_regen: f32,
        initial_damage: f32,
        scaled_damage: f32,
        initial_knockback: f32,
        scaled_knockback: f32,
        buildup_duration: f32,
        charge_duration: f32,
        recover_duration: f32,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        initial_projectile_speed: f32,
        scaled_projectile_speed: f32,
        damage_effect: Option<CombatEffect>,
        move_speed: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Shockwave {
        energy_cost: f32,
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        damage: f32,
        poise_damage: f32,
        knockback: Knockback,
        shockwave_angle: f32,
        shockwave_vertical_angle: f32,
        shockwave_speed: f32,
        shockwave_duration: f32,
        dodgeable: ShockwaveDodgeable,
        move_efficiency: f32,
        damage_kind: DamageKind,
        specifier: comp::shockwave::FrontendSpecifier,
        ori_rate: f32,
        damage_effect: Option<CombatEffect>,
        timing: shockwave::Timing,
        emit_outcome: bool,
        #[serde(default)]
        minimum_combo: u32,
        #[serde(default)]
        combo_consumption: ComboConsumption,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicBeam {
        buildup_duration: f32,
        recover_duration: f32,
        beam_duration: f64,
        damage: f32,
        tick_rate: f32,
        range: f32,
        max_angle: f32,
        damage_effect: Option<CombatEffect>,
        energy_regen: f32,
        energy_drain: f32,
        ori_rate: f32,
        specifier: beam::FrontendSpecifier,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicAura {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        targets: combat::GroupTarget,
        auras: Vec<aura::AuraBuffConstructor>,
        aura_duration: Option<Secs>,
        range: f32,
        energy_cost: f32,
        scales_with_combo: bool,
        specifier: Option<aura::Specifier>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    StaticAura {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        energy_cost: f32,
        targets: combat::GroupTarget,
        auras: Vec<aura::AuraBuffConstructor>,
        aura_duration: Option<Secs>,
        range: f32,
        sprite_info: Option<static_aura::SpriteInfo>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Blink {
        buildup_duration: f32,
        recover_duration: f32,
        max_range: f32,
        frontend_specifier: Option<blink::FrontendSpecifier>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicSummon {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        summon_amount: u32,
        summon_distance: (f32, f32),
        summon_info: basic_summon::SummonInfo,
        duration: Option<Duration>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    SelfBuff {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        buff_kind: buff::BuffKind,
        buff_strength: f32,
        buff_duration: Option<Secs>,
        energy_cost: f32,
        #[serde(default = "default_true")]
        enforced_limit: bool,
        #[serde(default)]
        combo_cost: u32,
        combo_scaling: Option<ScalingKind>,
        #[serde(default)]
        meta: AbilityMeta,
        specifier: Option<self_buff::FrontendSpecifier>,
    },
    SpriteSummon {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        sprite: SpriteKind,
        del_timeout: Option<(f32, f32)>,
        summon_distance: (f32, f32),
        sparseness: f64,
        angle: f32,
        #[serde(default)]
        anchor: SpriteSummonAnchor,
        #[serde(default)]
        move_efficiency: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Music {
        play_duration: f32,
        ori_modifier: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    FinisherMelee {
        energy_cost: f32,
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        minimum_combo: u32,
        scaling: Option<finisher_melee::Scaling>,
        #[serde(default)]
        combo_consumption: ComboConsumption,
        #[serde(default)]
        meta: AbilityMeta,
    },
    DiveMelee {
        energy_cost: f32,
        vertical_speed: f32,
        buildup_duration: Option<f32>,
        movement_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        max_scaling: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    RiposteMelee {
        energy_cost: f32,
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        block_strength: f32,
        melee_constructor: MeleeConstructor,
        #[serde(default)]
        meta: AbilityMeta,
    },
    RapidMelee {
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        energy_cost: f32,
        max_strikes: Option<u32>,
        melee_constructor: MeleeConstructor,
        move_modifier: f32,
        ori_modifier: f32,
        frontend_specifier: Option<rapid_melee::FrontendSpecifier>,
        #[serde(default)]
        minimum_combo: u32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Transform {
        buildup_duration: f32,
        recover_duration: f32,
        target: String,
        #[serde(default)]
        specifier: Option<transform::FrontendSpecifier>,
        /// Only set to `true` for admin only abilities since this disables
        /// persistence and is not intended to be used by regular players
        #[serde(default)]
        allow_players: bool,
        #[serde(default)]
        meta: AbilityMeta,
    },
}

impl Default for CharacterAbility {
    fn default() -> Self {
        CharacterAbility::BasicMelee {
            energy_cost: 0.0,
            buildup_duration: 0.25,
            swing_duration: 0.25,
            hit_timing: 0.5,
            recover_duration: 0.5,
            melee_constructor: MeleeConstructor {
                kind: MeleeConstructorKind::Slash {
                    damage: 1.0,
                    knockback: 0.0,
                    poise: 0.0,
                    energy_regen: 0.0,
                },
                scaled: None,
                range: 3.5,
                angle: 15.0,
                multi_target: None,
                damage_effect: None,
                attack_effect: None,
                simultaneous_hits: 1,
                custom_combo: None,
                precision_flank_multipliers: Default::default(),
                precision_flank_invert: false,
            },
            ori_modifier: 1.0,
            frontend_specifier: None,
            meta: Default::default(),
        }
    }
}

impl Asset for CharacterAbility {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl CharacterAbility {
    /// Attempts to fulfill requirements, mutating `update` (taking energy) if
    /// applicable.
    pub fn requirements_paid(&self, data: &JoinData, update: &mut StateUpdate) -> bool {
        let from_meta = {
            let AbilityMeta { requirements, .. } = self.ability_meta();
            requirements.requirements_met(data.stance)
        };
        from_meta
            && match self {
                CharacterAbility::Roll { energy_cost, .. }
                | CharacterAbility::StaticAura {
                    energy_cost,
                    sprite_info: Some(_),
                    ..
                } => {
                    data.physics.on_ground.is_some()
                        && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::DashMelee { energy_cost, .. }
                | CharacterAbility::BasicMelee { energy_cost, .. }
                | CharacterAbility::BasicRanged { energy_cost, .. }
                | CharacterAbility::ChargedRanged { energy_cost, .. }
                | CharacterAbility::ChargedMelee { energy_cost, .. }
                | CharacterAbility::BasicBlock { energy_cost, .. }
                | CharacterAbility::RiposteMelee { energy_cost, .. }
                | CharacterAbility::ComboMelee2 {
                    energy_cost_per_strike: energy_cost,
                    ..
                }
                | CharacterAbility::StaticAura {
                    energy_cost,
                    sprite_info: None,
                    ..
                } => update.energy.try_change_by(-*energy_cost).is_ok(),
                // Consumes energy within state, so value only checked before entering state
                CharacterAbility::RepeaterRanged { energy_cost, .. } => {
                    update.energy.current() >= *energy_cost
                },
                CharacterAbility::LeapMelee { energy_cost, .. }
                | CharacterAbility::LeapShockwave { energy_cost, .. } => {
                    update.vel.0.z >= 0.0 && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::BasicAura {
                    energy_cost,
                    scales_with_combo,
                    ..
                } => {
                    ((*scales_with_combo && data.combo.map_or(false, |c| c.counter() > 0))
                        | !*scales_with_combo)
                        && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::FinisherMelee {
                    energy_cost,
                    minimum_combo,
                    ..
                }
                | CharacterAbility::RapidMelee {
                    energy_cost,
                    minimum_combo,
                    ..
                }
                | CharacterAbility::SelfBuff {
                    energy_cost,
                    combo_cost: minimum_combo,
                    ..
                }
                | CharacterAbility::Shockwave {
                    energy_cost,
                    minimum_combo,
                    ..
                } => {
                    data.combo.map_or(false, |c| c.counter() >= *minimum_combo)
                        && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::DiveMelee {
                    buildup_duration,
                    energy_cost,
                    ..
                } => {
                    // If either in the air or is on ground and able to be activated from
                    // ground.
                    //
                    // NOTE: there is a check in CharacterState::from below that must be kept in
                    // sync with the conditions here (it determines whether this starts in a
                    // movement or buildup stage).
                    (data.physics.on_ground.is_none() || buildup_duration.is_some())
                        && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::Boost { .. }
                | CharacterAbility::GlideBoost { .. }
                | CharacterAbility::BasicBeam { .. }
                | CharacterAbility::Blink { .. }
                | CharacterAbility::Music { .. }
                | CharacterAbility::BasicSummon { .. }
                | CharacterAbility::SpriteSummon { .. }
                | CharacterAbility::Transform { .. } => true,
            }
    }

    pub fn default_roll(current_state: Option<&CharacterState>) -> CharacterAbility {
        let remaining_recover = if let Some(char_state) = current_state {
            if matches!(char_state.stage_section(), Some(StageSection::Recover)) {
                let timer = char_state.timer().map_or(0.0, |t| t.as_secs_f32());
                let recover_duration = char_state
                    .durations()
                    .and_then(|durs| durs.recover)
                    .map_or(timer, |rec| rec.as_secs_f32());
                recover_duration - timer
            } else {
                0.0
            }
        } else {
            0.0
        }
        .max(0.0);
        CharacterAbility::Roll {
            energy_cost: 10.85,
            // Remaining recover flows into buildup
            buildup_duration: 0.05 + remaining_recover,
            movement_duration: 0.36,
            recover_duration: 0.125,
            roll_strength: 3.3075,
            attack_immunities: AttackFilters {
                melee: true,
                projectiles: false,
                beams: true,
                ground_shockwaves: false,
                air_shockwaves: true,
                explosions: true,
            },
            meta: Default::default(),
        }
    }

    #[must_use]
    pub fn adjusted_by_stats(mut self, stats: Stats) -> Self {
        use CharacterAbility::*;
        match self {
            BasicMelee {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                ori_modifier: _,
                hit_timing: _,
                frontend_specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            BasicRanged {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut recover_duration,
                ref mut projectile,
                projectile_body: _,
                projectile_light: _,
                ref mut projectile_speed,
                num_projectiles: _,
                projectile_spread: _,
                damage_effect: _,
                move_efficiency: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *projectile = projectile.modified_projectile(stats.power, 1_f32, 1_f32);
                *projectile_speed *= stats.range;
                *energy_cost /= stats.energy_efficiency;
            },
            RepeaterRanged {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut shoot_duration,
                ref mut recover_duration,
                max_speed: _,
                half_speed_at: _,
                ref mut projectile,
                projectile_body: _,
                projectile_light: _,
                ref mut projectile_speed,
                damage_effect: _,
                properties_of_aoe: _,
                specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *shoot_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *projectile = projectile.modified_projectile(stats.power, 1_f32, 1_f32);
                *projectile_speed *= stats.range;
                *energy_cost /= stats.energy_efficiency;
            },
            Boost {
                ref mut movement_duration,
                only_up: _,
                speed: ref mut boost_speed,
                max_exit_velocity: _,
                meta: _,
            } => {
                *movement_duration /= stats.speed;
                *boost_speed *= stats.power;
            },
            DashMelee {
                ref mut energy_cost,
                ref mut energy_drain,
                forward_speed: _,
                ref mut buildup_duration,
                charge_duration: _,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                ori_modifier: _,
                auto_charge: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *energy_drain /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            BasicBlock {
                ref mut buildup_duration,
                ref mut recover_duration,
                // Do we want angle to be adjusted by range?
                max_angle: _,
                ref mut block_strength,
                parry_window: _,
                ref mut energy_cost,
                energy_regen: _,
                can_hold: _,
                blocked_attacks: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *block_strength *= stats.power;
            },
            Roll {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut movement_duration,
                ref mut recover_duration,
                roll_strength: _,
                attack_immunities: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *movement_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
            },
            ComboMelee2 {
                ref mut strikes,
                ref mut energy_cost_per_strike,
                specifier: _,
                auto_progress: _,
                meta: _,
            } => {
                *energy_cost_per_strike /= stats.energy_efficiency;
                *strikes = strikes
                    .iter_mut()
                    .map(|s| s.adjusted_by_stats(stats))
                    .collect();
            },
            LeapMelee {
                ref mut energy_cost,
                ref mut buildup_duration,
                movement_duration: _,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                forward_leap_strength: _,
                vertical_leap_strength: _,
                ref mut damage_effect,
                specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
                if let Some(CombatEffect::Buff(combat::CombatBuff {
                    kind: _,
                    dur_secs: _,
                    strength,
                    chance: _,
                })) = damage_effect
                {
                    *strength *= stats.buff_strength;
                }
            },
            LeapShockwave {
                ref mut energy_cost,
                ref mut buildup_duration,
                movement_duration: _,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut damage,
                ref mut poise_damage,
                knockback: _,
                shockwave_angle: _,
                shockwave_vertical_angle: _,
                shockwave_speed: _,
                ref mut shockwave_duration,
                dodgeable: _,
                move_efficiency: _,
                damage_kind: _,
                specifier: _,
                ref mut damage_effect,
                forward_leap_strength: _,
                vertical_leap_strength: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *damage *= stats.power;
                *poise_damage *= stats.effect_power;
                *shockwave_duration *= stats.range;
                *energy_cost /= stats.energy_efficiency;
                if let Some(CombatEffect::Buff(combat::CombatBuff {
                    kind: _,
                    dur_secs: _,
                    strength,
                    chance: _,
                })) = damage_effect
                {
                    *strength *= stats.buff_strength;
                }
            },
            ChargedMelee {
                ref mut energy_cost,
                ref mut energy_drain,
                ref mut buildup_strike,
                ref mut charge_duration,
                ref mut swing_duration,
                hit_timing: _,
                ref mut recover_duration,
                ref mut melee_constructor,
                specifier: _,
                ref mut damage_effect,
                meta: _,
                custom_combo: _,
            } => {
                *swing_duration /= stats.speed;
                *buildup_strike = buildup_strike
                    .map(|(dur, strike)| (dur / stats.speed, strike.adjusted_by_stats(stats)));
                *charge_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *energy_drain *= stats.speed / stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
                if let Some(CombatEffect::Buff(combat::CombatBuff {
                    kind: _,
                    dur_secs: _,
                    strength,
                    chance: _,
                })) = damage_effect
                {
                    *strength *= stats.buff_strength;
                }
            },
            ChargedRanged {
                ref mut energy_cost,
                ref mut energy_drain,
                initial_regen: _,
                scaled_regen: _,
                ref mut initial_damage,
                ref mut scaled_damage,
                initial_knockback: _,
                scaled_knockback: _,
                ref mut buildup_duration,
                ref mut charge_duration,
                ref mut recover_duration,
                projectile_body: _,
                projectile_light: _,
                ref mut initial_projectile_speed,
                ref mut scaled_projectile_speed,
                damage_effect: _,
                move_speed: _,
                meta: _,
            } => {
                *initial_damage *= stats.power;
                *scaled_damage *= stats.power;
                *buildup_duration /= stats.speed;
                *charge_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *initial_projectile_speed *= stats.range;
                *scaled_projectile_speed *= stats.range;
                *energy_cost /= stats.energy_efficiency;
                *energy_drain *= stats.speed / stats.energy_efficiency;
            },
            Shockwave {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut damage,
                ref mut poise_damage,
                knockback: _,
                shockwave_angle: _,
                shockwave_vertical_angle: _,
                shockwave_speed: _,
                ref mut shockwave_duration,
                dodgeable: _,
                move_efficiency: _,
                damage_kind: _,
                specifier: _,
                ori_rate: _,
                ref mut damage_effect,
                timing: _,
                emit_outcome: _,
                minimum_combo: _,
                combo_consumption: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *damage *= stats.power;
                *poise_damage *= stats.effect_power;
                *shockwave_duration *= stats.range;
                *energy_cost /= stats.energy_efficiency;
                *damage_effect = damage_effect.map(|de| de.adjusted_by_stats(stats));
            },
            BasicBeam {
                ref mut buildup_duration,
                ref mut recover_duration,
                ref mut beam_duration,
                ref mut damage,
                ref mut tick_rate,
                ref mut range,
                max_angle: _,
                ref mut damage_effect,
                energy_regen: _,
                ref mut energy_drain,
                ori_rate: _,
                specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *damage *= stats.power;
                *tick_rate *= stats.speed;
                *range *= stats.range;
                // Duration modified to keep velocity constant
                *beam_duration *= stats.range as f64;
                *energy_drain /= stats.energy_efficiency;
                *damage_effect = damage_effect.map(|de| de.adjusted_by_stats(stats));
            },
            BasicAura {
                ref mut buildup_duration,
                ref mut cast_duration,
                ref mut recover_duration,
                targets: _,
                ref mut auras,
                aura_duration: _,
                ref mut range,
                ref mut energy_cost,
                scales_with_combo: _,
                specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                auras.iter_mut().for_each(
                    |aura::AuraBuffConstructor {
                         kind: _,
                         ref mut strength,
                         duration: _,
                         category: _,
                     }| {
                        *strength *= stats.diminished_buff_strength();
                    },
                );
                *range *= stats.range;
                *energy_cost /= stats.energy_efficiency;
            },
            StaticAura {
                ref mut buildup_duration,
                ref mut cast_duration,
                ref mut recover_duration,
                targets: _,
                ref mut auras,
                aura_duration: _,
                ref mut range,
                ref mut energy_cost,
                ref mut sprite_info,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                auras.iter_mut().for_each(
                    |aura::AuraBuffConstructor {
                         kind: _,
                         ref mut strength,
                         duration: _,
                         category: _,
                     }| {
                        *strength *= stats.diminished_buff_strength();
                    },
                );
                *range *= stats.range;
                *energy_cost /= stats.energy_efficiency;
                *sprite_info = sprite_info.map(|mut si| {
                    si.summon_distance.0 *= stats.range;
                    si.summon_distance.1 *= stats.range;
                    si
                });
            },
            Blink {
                ref mut buildup_duration,
                ref mut recover_duration,
                ref mut max_range,
                frontend_specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *max_range *= stats.range;
            },
            BasicSummon {
                ref mut buildup_duration,
                ref mut cast_duration,
                ref mut recover_duration,
                summon_amount: _,
                summon_distance: (ref mut inner_dist, ref mut outer_dist),
                summon_info: _,
                duration: _,
                meta: _,
            } => {
                // TODO: Figure out how/if power should affect this
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *inner_dist *= stats.range;
                *outer_dist *= stats.range;
            },
            SelfBuff {
                ref mut buildup_duration,
                ref mut cast_duration,
                ref mut recover_duration,
                buff_kind: _,
                ref mut buff_strength,
                buff_duration: _,
                ref mut energy_cost,
                enforced_limit: _,
                combo_cost: _,
                combo_scaling: _,
                meta: _,
                specifier: _,
            } => {
                *buff_strength *= stats.diminished_buff_strength();
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
            },
            SpriteSummon {
                ref mut buildup_duration,
                ref mut cast_duration,
                ref mut recover_duration,
                sprite: _,
                del_timeout: _,
                summon_distance: (ref mut inner_dist, ref mut outer_dist),
                sparseness: _,
                angle: _,
                anchor: _,
                move_efficiency: _,
                meta: _,
            } => {
                // TODO: Figure out how/if power should affect this
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *inner_dist *= stats.range;
                *outer_dist *= stats.range;
            },
            Music {
                ref mut play_duration,
                ori_modifier: _,
                meta: _,
            } => {
                *play_duration /= stats.speed;
            },
            FinisherMelee {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                minimum_combo: _,
                scaling: _,
                combo_consumption: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            DiveMelee {
                ref mut energy_cost,
                vertical_speed: _,
                movement_duration: _,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                max_scaling: _,
                meta: _,
            } => {
                *buildup_duration = buildup_duration.map(|b| b / stats.speed);
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            RiposteMelee {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut block_strength,
                ref mut melee_constructor,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *block_strength *= stats.power;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            RapidMelee {
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut energy_cost,
                ref mut melee_constructor,
                max_strikes: _,
                move_modifier: _,
                ori_modifier: _,
                minimum_combo: _,
                frontend_specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            Transform {
                ref mut buildup_duration,
                ref mut recover_duration,
                target: _,
                specifier: _,
                allow_players: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
            },
            GlideBoost { .. } => {},
        }
        self
    }

    pub fn energy_cost(&self) -> f32 {
        use CharacterAbility::*;
        match self {
            BasicMelee { energy_cost, .. }
            | BasicRanged { energy_cost, .. }
            | RepeaterRanged { energy_cost, .. }
            | DashMelee { energy_cost, .. }
            | Roll { energy_cost, .. }
            | LeapMelee { energy_cost, .. }
            | LeapShockwave { energy_cost, .. }
            | ChargedMelee { energy_cost, .. }
            | ChargedRanged { energy_cost, .. }
            | Shockwave { energy_cost, .. }
            | BasicAura { energy_cost, .. }
            | BasicBlock { energy_cost, .. }
            | SelfBuff { energy_cost, .. }
            | FinisherMelee { energy_cost, .. }
            | ComboMelee2 {
                energy_cost_per_strike: energy_cost,
                ..
            }
            | DiveMelee { energy_cost, .. }
            | RiposteMelee { energy_cost, .. }
            | RapidMelee { energy_cost, .. }
            | StaticAura { energy_cost, .. } => *energy_cost,
            BasicBeam { energy_drain, .. } => {
                if *energy_drain > f32::EPSILON {
                    1.0
                } else {
                    0.0
                }
            },
            Boost { .. }
            | GlideBoost { .. }
            | Blink { .. }
            | Music { .. }
            | BasicSummon { .. }
            | SpriteSummon { .. }
            | Transform { .. } => 0.0,
        }
    }

    #[allow(clippy::bool_to_int_with_if)]
    pub fn combo_cost(&self) -> u32 {
        use CharacterAbility::*;
        match self {
            BasicAura {
                scales_with_combo, ..
            } => {
                if *scales_with_combo {
                    1
                } else {
                    0
                }
            },
            FinisherMelee {
                minimum_combo: combo,
                ..
            }
            | RapidMelee {
                minimum_combo: combo,
                ..
            }
            | SelfBuff {
                combo_cost: combo, ..
            }
            | Shockwave {
                minimum_combo: combo,
                ..
            } => *combo,
            BasicMelee { .. }
            | BasicRanged { .. }
            | RepeaterRanged { .. }
            | DashMelee { .. }
            | Roll { .. }
            | LeapMelee { .. }
            | LeapShockwave { .. }
            | ChargedMelee { .. }
            | ChargedRanged { .. }
            | BasicBlock { .. }
            | ComboMelee2 { .. }
            | DiveMelee { .. }
            | RiposteMelee { .. }
            | BasicBeam { .. }
            | Boost { .. }
            | GlideBoost { .. }
            | Blink { .. }
            | Music { .. }
            | BasicSummon { .. }
            | SpriteSummon { .. }
            | Transform { .. }
            | StaticAura { .. } => 0,
        }
    }

    // TODO: Maybe consider making CharacterAbility a struct at some point?
    pub fn ability_meta(&self) -> AbilityMeta {
        use CharacterAbility::*;
        match self {
            BasicMelee { meta, .. }
            | BasicRanged { meta, .. }
            | RepeaterRanged { meta, .. }
            | DashMelee { meta, .. }
            | Roll { meta, .. }
            | LeapMelee { meta, .. }
            | LeapShockwave { meta, .. }
            | ChargedMelee { meta, .. }
            | ChargedRanged { meta, .. }
            | Shockwave { meta, .. }
            | BasicAura { meta, .. }
            | BasicBlock { meta, .. }
            | SelfBuff { meta, .. }
            | BasicBeam { meta, .. }
            | Boost { meta, .. }
            | GlideBoost { meta, .. }
            | ComboMelee2 { meta, .. }
            | Blink { meta, .. }
            | BasicSummon { meta, .. }
            | SpriteSummon { meta, .. }
            | FinisherMelee { meta, .. }
            | Music { meta, .. }
            | DiveMelee { meta, .. }
            | RiposteMelee { meta, .. }
            | RapidMelee { meta, .. }
            | Transform { meta, .. }
            | StaticAura { meta, .. } => *meta,
        }
    }

    #[must_use = "method returns new ability and doesn't mutate the original value"]
    pub fn adjusted_by_skills(mut self, skillset: &SkillSet, tool: Option<ToolKind>) -> Self {
        match tool {
            Some(ToolKind::Bow) => self.adjusted_by_bow_skills(skillset),
            Some(ToolKind::Staff) => self.adjusted_by_staff_skills(skillset),
            Some(ToolKind::Sceptre) => self.adjusted_by_sceptre_skills(skillset),
            Some(ToolKind::Pick) => self.adjusted_by_mining_skills(skillset),
            None | Some(_) => {},
        }
        self
    }

    fn adjusted_by_mining_skills(&mut self, skillset: &SkillSet) {
        use skills::MiningSkill::Speed;

        if let CharacterAbility::BasicMelee {
            ref mut buildup_duration,
            ref mut swing_duration,
            ref mut recover_duration,
            ..
        } = self
        {
            if let Ok(level) = skillset.skill_level(Skill::Pick(Speed)) {
                let modifiers = SKILL_MODIFIERS.mining_tree;

                let speed = modifiers.speed.powi(level.into());
                *buildup_duration /= speed;
                *swing_duration /= speed;
                *recover_duration /= speed;
            }
        }
    }

    fn adjusted_by_bow_skills(&mut self, skillset: &SkillSet) {
        #![allow(clippy::enum_glob_use)]
        use skills::{BowSkill::*, Skill::Bow};

        let projectile_speed_modifier = SKILL_MODIFIERS.bow_tree.universal.projectile_speed;
        match self {
            CharacterAbility::ChargedRanged {
                ref mut initial_damage,
                ref mut scaled_damage,
                ref mut initial_regen,
                ref mut scaled_regen,
                ref mut initial_knockback,
                ref mut scaled_knockback,
                ref mut move_speed,
                ref mut initial_projectile_speed,
                ref mut scaled_projectile_speed,
                ref mut charge_duration,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.bow_tree.charged;
                if let Ok(level) = skillset.skill_level(Bow(ProjSpeed)) {
                    let projectile_speed_scaling = projectile_speed_modifier.powi(level.into());
                    *initial_projectile_speed *= projectile_speed_scaling;
                    *scaled_projectile_speed *= projectile_speed_scaling;
                }
                if let Ok(level) = skillset.skill_level(Bow(CDamage)) {
                    let damage_scaling = modifiers.damage_scaling.powi(level.into());
                    *initial_damage *= damage_scaling;
                    *scaled_damage *= damage_scaling;
                }
                if let Ok(level) = skillset.skill_level(Bow(CRegen)) {
                    let regen_scaling = modifiers.regen_scaling.powi(level.into());
                    *initial_regen *= regen_scaling;
                    *scaled_regen *= regen_scaling;
                }
                if let Ok(level) = skillset.skill_level(Bow(CKnockback)) {
                    let knockback_scaling = modifiers.knockback_scaling.powi(level.into());
                    *initial_knockback *= knockback_scaling;
                    *scaled_knockback *= knockback_scaling;
                }
                if let Ok(level) = skillset.skill_level(Bow(CSpeed)) {
                    let charge_time = 1.0 / modifiers.charge_rate;
                    *charge_duration *= charge_time.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(CMove)) {
                    *move_speed *= modifiers.move_speed.powi(level.into());
                }
            },
            CharacterAbility::RepeaterRanged {
                ref mut energy_cost,
                ref mut projectile,
                ref mut max_speed,
                ref mut projectile_speed,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.bow_tree.repeater;
                if let Ok(level) = skillset.skill_level(Bow(ProjSpeed)) {
                    *projectile_speed *= projectile_speed_modifier.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(RDamage)) {
                    let power = modifiers.power.powi(level.into());
                    *projectile = projectile.modified_projectile(power, 1_f32, 1_f32);
                }
                if let Ok(level) = skillset.skill_level(Bow(RCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(RSpeed)) {
                    *max_speed *= modifiers.max_speed.powi(level.into());
                }
            },
            CharacterAbility::BasicRanged {
                ref mut projectile,
                ref mut energy_cost,
                ref mut num_projectiles,
                ref mut projectile_spread,
                ref mut projectile_speed,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.bow_tree.shotgun;
                if let Ok(level) = skillset.skill_level(Bow(ProjSpeed)) {
                    *projectile_speed *= projectile_speed_modifier.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(SDamage)) {
                    let power = modifiers.power.powi(level.into());
                    *projectile = projectile.modified_projectile(power, 1_f32, 1_f32);
                }
                if let Ok(level) = skillset.skill_level(Bow(SCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(SArrows)) {
                    *num_projectiles += u32::from(level) * modifiers.num_projectiles;
                }
                if let Ok(level) = skillset.skill_level(Bow(SSpread)) {
                    *projectile_spread *= modifiers.spread.powi(level.into());
                }
            },
            _ => {},
        }
    }

    fn adjusted_by_staff_skills(&mut self, skillset: &SkillSet) {
        #![allow(clippy::enum_glob_use)]
        use skills::{Skill::Staff, StaffSkill::*};

        match self {
            CharacterAbility::BasicRanged {
                ref mut projectile, ..
            } => {
                let modifiers = SKILL_MODIFIERS.staff_tree.fireball;
                let damage_level = skillset.skill_level(Staff(BDamage)).unwrap_or(0);
                let regen_level = skillset.skill_level(Staff(BRegen)).unwrap_or(0);
                let range_level = skillset.skill_level(Staff(BRadius)).unwrap_or(0);
                let power = modifiers.power.powi(damage_level.into());
                let regen = modifiers.regen.powi(regen_level.into());
                let range = modifiers.range.powi(range_level.into());
                *projectile = projectile.modified_projectile(power, regen, range);
            },
            CharacterAbility::BasicBeam {
                ref mut damage,
                ref mut range,
                ref mut energy_drain,
                ref mut beam_duration,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.staff_tree.flamethrower;
                if let Ok(level) = skillset.skill_level(Staff(FDamage)) {
                    *damage *= modifiers.damage.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Staff(FRange)) {
                    let range_mod = modifiers.range.powi(level.into());
                    *range *= range_mod;
                    // Duration modified to keep velocity constant
                    *beam_duration *= range_mod as f64;
                }
                if let Ok(level) = skillset.skill_level(Staff(FDrain)) {
                    *energy_drain *= modifiers.energy_drain.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Staff(FVelocity)) {
                    let velocity_increase = modifiers.velocity.powi(level.into());
                    let duration_mod = 1.0 / (1.0 + velocity_increase);
                    *beam_duration *= duration_mod as f64;
                }
            },
            CharacterAbility::Shockwave {
                ref mut damage,
                ref mut knockback,
                ref mut shockwave_duration,
                ref mut energy_cost,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.staff_tree.shockwave;
                if let Ok(level) = skillset.skill_level(Staff(SDamage)) {
                    *damage *= modifiers.damage.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Staff(SKnockback)) {
                    let knockback_mod = modifiers.knockback.powi(level.into());
                    *knockback = knockback.modify_strength(knockback_mod);
                }
                if let Ok(level) = skillset.skill_level(Staff(SRange)) {
                    *shockwave_duration *= modifiers.duration.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Staff(SCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
            },
            _ => {},
        }
    }

    fn adjusted_by_sceptre_skills(&mut self, skillset: &SkillSet) {
        #![allow(clippy::enum_glob_use)]
        use skills::{SceptreSkill::*, Skill::Sceptre};

        match self {
            CharacterAbility::BasicBeam {
                ref mut damage,
                ref mut range,
                ref mut beam_duration,
                ref mut damage_effect,
                ref mut energy_regen,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.sceptre_tree.beam;
                if let Ok(level) = skillset.skill_level(Sceptre(LDamage)) {
                    *damage *= modifiers.damage.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Sceptre(LRange)) {
                    let range_mod = modifiers.range.powi(level.into());
                    *range *= range_mod;
                    // Duration modified to keep velocity constant
                    *beam_duration *= range_mod as f64;
                }
                if let Ok(level) = skillset.skill_level(Sceptre(LRegen)) {
                    *energy_regen *= modifiers.energy_regen.powi(level.into());
                }
                if let (Ok(level), Some(CombatEffect::Lifesteal(ref mut lifesteal))) =
                    (skillset.skill_level(Sceptre(LLifesteal)), damage_effect)
                {
                    *lifesteal *= modifiers.lifesteal.powi(level.into());
                }
            },
            CharacterAbility::BasicAura {
                ref mut auras,
                ref mut range,
                ref mut energy_cost,
                specifier: Some(aura::Specifier::HealingAura),
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.sceptre_tree.healing_aura;
                if let Ok(level) = skillset.skill_level(Sceptre(HHeal)) {
                    auras.iter_mut().for_each(|ref mut aura| {
                        aura.strength *= modifiers.strength.powi(level.into());
                    });
                }
                if let Ok(level) = skillset.skill_level(Sceptre(HDuration)) {
                    auras.iter_mut().for_each(|ref mut aura| {
                        if let Some(ref mut duration) = aura.duration {
                            *duration *= modifiers.duration.powi(level.into()) as f64;
                        }
                    });
                }
                if let Ok(level) = skillset.skill_level(Sceptre(HRange)) {
                    *range *= modifiers.range.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Sceptre(HCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
            },
            CharacterAbility::BasicAura {
                ref mut auras,
                ref mut range,
                ref mut energy_cost,
                specifier: Some(aura::Specifier::WardingAura),
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.sceptre_tree.warding_aura;
                if let Ok(level) = skillset.skill_level(Sceptre(AStrength)) {
                    auras.iter_mut().for_each(|ref mut aura| {
                        aura.strength *= modifiers.strength.powi(level.into());
                    });
                }
                if let Ok(level) = skillset.skill_level(Sceptre(ADuration)) {
                    auras.iter_mut().for_each(|ref mut aura| {
                        if let Some(ref mut duration) = aura.duration {
                            *duration *= modifiers.duration.powi(level.into()) as f64;
                        }
                    });
                }
                if let Ok(level) = skillset.skill_level(Sceptre(ARange)) {
                    *range *= modifiers.range.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Sceptre(ACost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
            },
            _ => {},
        }
    }
}

/// Small helper for #[serde(default)] booleans
fn default_true() -> bool { true }

impl From<(&CharacterAbility, AbilityInfo, &JoinData<'_>)> for CharacterState {
    fn from((ability, ability_info, data): (&CharacterAbility, AbilityInfo, &JoinData)) -> Self {
        match ability {
            CharacterAbility::BasicMelee {
                buildup_duration,
                swing_duration,
                hit_timing,
                recover_duration,
                melee_constructor,
                ori_modifier,
                frontend_specifier,
                energy_cost: _,
                meta: _,
            } => CharacterState::BasicMelee(basic_melee::Data {
                static_data: basic_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    hit_timing: hit_timing.clamp(0.0, 1.0),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ori_modifier: *ori_modifier,
                    frontend_specifier: *frontend_specifier,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::BasicRanged {
                buildup_duration,
                recover_duration,
                projectile,
                projectile_body,
                projectile_light,
                projectile_speed,
                energy_cost: _,
                num_projectiles,
                projectile_spread,
                damage_effect,
                move_efficiency,
                meta: _,
            } => CharacterState::BasicRanged(basic_ranged::Data {
                static_data: basic_ranged::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    projectile: *projectile,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_speed: *projectile_speed,
                    num_projectiles: *num_projectiles,
                    projectile_spread: *projectile_spread,
                    ability_info,
                    damage_effect: *damage_effect,
                    move_efficiency: *move_efficiency,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::Boost {
                movement_duration,
                only_up,
                speed,
                max_exit_velocity,
                meta: _,
            } => CharacterState::Boost(boost::Data {
                static_data: boost::StaticData {
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    only_up: *only_up,
                    speed: *speed,
                    max_exit_velocity: *max_exit_velocity,
                    ability_info,
                },
                timer: Duration::default(),
            }),
            CharacterAbility::GlideBoost { booster, meta: _ } => {
                let scale = data.body.dimensions().z.sqrt();
                let mut glide_data = glide::Data::new(scale * 4.5, scale, *data.ori);
                glide_data.booster = Some(*booster);

                CharacterState::Glide(glide_data)
            },
            CharacterAbility::DashMelee {
                energy_cost: _,
                energy_drain,
                forward_speed,
                buildup_duration,
                charge_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                ori_modifier,
                auto_charge,
                meta: _,
            } => CharacterState::DashMelee(dash_melee::Data {
                static_data: dash_melee::StaticData {
                    energy_drain: *energy_drain,
                    forward_speed: *forward_speed,
                    auto_charge: *auto_charge,
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    charge_duration: Duration::from_secs_f32(*charge_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ori_modifier: *ori_modifier,
                    ability_info,
                },
                auto_charge: false,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::BasicBlock {
                buildup_duration,
                recover_duration,
                max_angle,
                block_strength,
                parry_window,
                energy_cost,
                energy_regen,
                can_hold,
                blocked_attacks,
                meta: _,
            } => CharacterState::BasicBlock(basic_block::Data {
                static_data: basic_block::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    max_angle: *max_angle,
                    block_strength: *block_strength,
                    parry_window: *parry_window,
                    energy_cost: *energy_cost,
                    energy_regen: *energy_regen,
                    can_hold: *can_hold,
                    blocked_attacks: *blocked_attacks,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::Roll {
                energy_cost: _,
                buildup_duration,
                movement_duration,
                recover_duration,
                roll_strength,
                attack_immunities,
                meta: _,
            } => CharacterState::Roll(roll::Data {
                static_data: roll::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    roll_strength: *roll_strength,
                    attack_immunities: *attack_immunities,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                was_wielded: false, // false by default. utils might set it to true
                prev_aimed_dir: None,
                is_sneaking: false,
            }),
            CharacterAbility::ComboMelee2 {
                strikes,
                energy_cost_per_strike,
                specifier,
                auto_progress,
                meta: _,
            } => CharacterState::ComboMelee2(combo_melee2::Data {
                static_data: combo_melee2::StaticData {
                    strikes: strikes.iter().map(|s| s.to_duration()).collect(),
                    energy_cost_per_strike: *energy_cost_per_strike,
                    specifier: *specifier,
                    auto_progress: *auto_progress,
                    ability_info,
                },
                exhausted: false,
                start_next_strike: false,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                completed_strikes: 0,
            }),
            CharacterAbility::LeapMelee {
                energy_cost: _,
                buildup_duration,
                movement_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                forward_leap_strength,
                vertical_leap_strength,
                damage_effect,
                specifier,
                meta: _,
            } => CharacterState::LeapMelee(leap_melee::Data {
                static_data: leap_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    forward_leap_strength: *forward_leap_strength,
                    vertical_leap_strength: *vertical_leap_strength,
                    ability_info,
                    damage_effect: *damage_effect,
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::LeapShockwave {
                energy_cost: _,
                buildup_duration,
                movement_duration,
                swing_duration,
                recover_duration,
                damage,
                poise_damage,
                knockback,
                shockwave_angle,
                shockwave_vertical_angle,
                shockwave_speed,
                shockwave_duration,
                dodgeable,
                move_efficiency,
                damage_kind,
                specifier,
                damage_effect,
                forward_leap_strength,
                vertical_leap_strength,
                meta: _,
            } => CharacterState::LeapShockwave(leap_shockwave::Data {
                static_data: leap_shockwave::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    damage: *damage,
                    poise_damage: *poise_damage,
                    knockback: *knockback,
                    shockwave_angle: *shockwave_angle,
                    shockwave_vertical_angle: *shockwave_vertical_angle,
                    shockwave_speed: *shockwave_speed,
                    shockwave_duration: Duration::from_secs_f32(*shockwave_duration),
                    dodgeable: *dodgeable,
                    move_efficiency: *move_efficiency,
                    damage_kind: *damage_kind,
                    specifier: *specifier,
                    damage_effect: *damage_effect,
                    forward_leap_strength: *forward_leap_strength,
                    vertical_leap_strength: *vertical_leap_strength,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::ChargedMelee {
                energy_cost,
                energy_drain,
                buildup_strike,
                charge_duration,
                swing_duration,
                hit_timing,
                recover_duration,
                melee_constructor,
                specifier,
                damage_effect,
                custom_combo,
                meta: _,
            } => CharacterState::ChargedMelee(charged_melee::Data {
                static_data: charged_melee::StaticData {
                    energy_cost: *energy_cost,
                    energy_drain: *energy_drain,
                    buildup_strike: buildup_strike
                        .map(|(dur, strike)| (Duration::from_secs_f32(dur), strike)),
                    charge_duration: Duration::from_secs_f32(*charge_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    hit_timing: *hit_timing,
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ability_info,
                    specifier: *specifier,
                    damage_effect: *damage_effect,
                    custom_combo: *custom_combo,
                },
                stage_section: if buildup_strike.is_some() {
                    StageSection::Buildup
                } else {
                    StageSection::Charge
                },
                timer: Duration::default(),
                exhausted: false,
                charge_amount: 0.0,
            }),
            CharacterAbility::ChargedRanged {
                energy_cost: _,
                energy_drain,
                initial_regen,
                scaled_regen,
                initial_damage,
                scaled_damage,
                initial_knockback,
                scaled_knockback,
                buildup_duration,
                charge_duration,
                recover_duration,
                projectile_body,
                projectile_light,
                initial_projectile_speed,
                scaled_projectile_speed,
                damage_effect,
                move_speed,
                meta: _,
            } => CharacterState::ChargedRanged(charged_ranged::Data {
                static_data: charged_ranged::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    charge_duration: Duration::from_secs_f32(*charge_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    energy_drain: *energy_drain,
                    initial_regen: *initial_regen,
                    scaled_regen: *scaled_regen,
                    initial_damage: *initial_damage,
                    scaled_damage: *scaled_damage,
                    initial_knockback: *initial_knockback,
                    scaled_knockback: *scaled_knockback,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    initial_projectile_speed: *initial_projectile_speed,
                    scaled_projectile_speed: *scaled_projectile_speed,
                    move_speed: *move_speed,
                    ability_info,
                    damage_effect: *damage_effect,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::RepeaterRanged {
                energy_cost,
                buildup_duration,
                shoot_duration,
                recover_duration,
                max_speed,
                half_speed_at,
                projectile,
                projectile_body,
                projectile_light,
                projectile_speed,
                damage_effect,
                properties_of_aoe,
                specifier,
                meta: _,
            } => CharacterState::RepeaterRanged(repeater_ranged::Data {
                static_data: repeater_ranged::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    shoot_duration: Duration::from_secs_f32(*shoot_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    energy_cost: *energy_cost,
                    // 1.0 is subtracted as 1.0 is added in state file
                    max_speed: *max_speed - 1.0,
                    half_speed_at: *half_speed_at,
                    projectile: *projectile,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_speed: *projectile_speed,
                    ability_info,
                    damage_effect: *damage_effect,
                    properties_of_aoe: *properties_of_aoe,
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                projectiles_fired: 0,
                speed: 1.0,
            }),
            CharacterAbility::Shockwave {
                energy_cost: _,
                buildup_duration,
                swing_duration,
                recover_duration,
                damage,
                poise_damage,
                knockback,
                shockwave_angle,
                shockwave_vertical_angle,
                shockwave_speed,
                shockwave_duration,
                dodgeable,
                move_efficiency,
                damage_kind,
                specifier,
                ori_rate,
                damage_effect,
                timing,
                emit_outcome,
                minimum_combo,
                combo_consumption,
                meta: _,
            } => CharacterState::Shockwave(shockwave::Data {
                static_data: shockwave::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    damage: *damage,
                    poise_damage: *poise_damage,
                    knockback: *knockback,
                    shockwave_angle: *shockwave_angle,
                    shockwave_vertical_angle: *shockwave_vertical_angle,
                    shockwave_speed: *shockwave_speed,
                    shockwave_duration: Duration::from_secs_f32(*shockwave_duration),
                    dodgeable: *dodgeable,
                    move_efficiency: *move_efficiency,
                    damage_effect: *damage_effect,
                    ability_info,
                    damage_kind: *damage_kind,
                    specifier: *specifier,
                    ori_rate: *ori_rate,
                    timing: *timing,
                    emit_outcome: *emit_outcome,
                    minimum_combo: *minimum_combo,
                    combo_on_use: data.combo.map_or(0, |c| c.counter()),
                    combo_consumption: *combo_consumption,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::BasicBeam {
                buildup_duration,
                recover_duration,
                beam_duration,
                damage,
                tick_rate,
                range,
                max_angle,
                damage_effect,
                energy_regen,
                energy_drain,
                ori_rate,
                specifier,
                meta: _,
            } => CharacterState::BasicBeam(basic_beam::Data {
                static_data: basic_beam::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    beam_duration: Secs(*beam_duration),
                    damage: *damage,
                    tick_rate: *tick_rate,
                    range: *range,
                    end_radius: max_angle.to_radians().tan() * *range,
                    damage_effect: *damage_effect,
                    energy_regen: *energy_regen,
                    energy_drain: *energy_drain,
                    ability_info,
                    ori_rate: *ori_rate,
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                aim_dir: data.ori.look_dir(),
                beam_offset: data.pos.0,
            }),
            CharacterAbility::BasicAura {
                buildup_duration,
                cast_duration,
                recover_duration,
                targets,
                auras,
                aura_duration,
                range,
                energy_cost: _,
                scales_with_combo,
                specifier,
                meta: _,
            } => CharacterState::BasicAura(basic_aura::Data {
                static_data: basic_aura::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    targets: *targets,
                    auras: auras.clone(),
                    aura_duration: *aura_duration,
                    range: *range,
                    ability_info,
                    scales_with_combo: *scales_with_combo,
                    combo_at_cast: data.combo.map_or(0, |c| c.counter()),
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::StaticAura {
                buildup_duration,
                cast_duration,
                recover_duration,
                targets,
                auras,
                aura_duration,
                range,
                energy_cost: _,
                sprite_info,
                meta: _,
            } => CharacterState::StaticAura(static_aura::Data {
                static_data: static_aura::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    targets: *targets,
                    auras: auras.clone(),
                    aura_duration: *aura_duration,
                    range: *range,
                    ability_info,
                    sprite_info: *sprite_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                achieved_radius: sprite_info.map(|si| si.summon_distance.0.floor() as i32 - 1),
            }),
            CharacterAbility::Blink {
                buildup_duration,
                recover_duration,
                max_range,
                frontend_specifier,
                meta: _,
            } => CharacterState::Blink(blink::Data {
                static_data: blink::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    max_range: *max_range,
                    frontend_specifier: *frontend_specifier,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::BasicSummon {
                buildup_duration,
                cast_duration,
                recover_duration,
                summon_amount,
                summon_distance,
                summon_info,
                duration,
                meta: _,
            } => CharacterState::BasicSummon(basic_summon::Data {
                static_data: basic_summon::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    summon_amount: *summon_amount,
                    summon_distance: *summon_distance,
                    summon_info: *summon_info,
                    ability_info,
                    duration: *duration,
                },
                summon_count: 0,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::SelfBuff {
                buildup_duration,
                cast_duration,
                recover_duration,
                buff_kind,
                buff_strength,
                buff_duration,
                energy_cost: _,
                combo_cost,
                combo_scaling,
                enforced_limit,
                meta: _,
                specifier,
            } => CharacterState::SelfBuff(self_buff::Data {
                static_data: self_buff::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    buff_kind: *buff_kind,
                    buff_strength: *buff_strength,
                    buff_duration: *buff_duration,
                    combo_cost: *combo_cost,
                    combo_scaling: *combo_scaling,
                    combo_on_use: data.combo.map_or(0, |c| c.counter()),
                    enforced_limit: *enforced_limit,
                    ability_info,
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::SpriteSummon {
                buildup_duration,
                cast_duration,
                recover_duration,
                sprite,
                del_timeout,
                summon_distance,
                sparseness,
                angle,
                anchor,
                move_efficiency,
                meta: _,
            } => CharacterState::SpriteSummon(sprite_summon::Data {
                static_data: sprite_summon::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    sprite: *sprite,
                    del_timeout: *del_timeout,
                    summon_distance: *summon_distance,
                    sparseness: *sparseness,
                    angle: *angle,
                    anchor: *anchor,
                    move_efficiency: *move_efficiency,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                achieved_radius: summon_distance.0.floor() as i32 - 1,
            }),
            CharacterAbility::Music {
                play_duration,
                ori_modifier,
                meta: _,
            } => CharacterState::Music(music::Data {
                static_data: music::StaticData {
                    play_duration: Duration::from_secs_f32(*play_duration),
                    ori_modifier: *ori_modifier,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Action,
                exhausted: false,
            }),
            CharacterAbility::FinisherMelee {
                energy_cost: _,
                buildup_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                minimum_combo,
                scaling,
                combo_consumption,
                meta: _,
            } => CharacterState::FinisherMelee(finisher_melee::Data {
                static_data: finisher_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    scaling: *scaling,
                    minimum_combo: *minimum_combo,
                    combo_on_use: data.combo.map_or(0, |c| c.counter()),
                    combo_consumption: *combo_consumption,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::DiveMelee {
                buildup_duration,
                movement_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                energy_cost: _,
                vertical_speed,
                max_scaling,
                meta: _,
            } => CharacterState::DiveMelee(dive_melee::Data {
                static_data: dive_melee::StaticData {
                    buildup_duration: buildup_duration.map(Duration::from_secs_f32),
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    vertical_speed: *vertical_speed,
                    melee_constructor: *melee_constructor,
                    max_scaling: *max_scaling,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: if data.physics.on_ground.is_none() || buildup_duration.is_none() {
                    StageSection::Movement
                } else {
                    StageSection::Buildup
                },
                exhausted: false,
                max_vertical_speed: 0.0,
            }),
            CharacterAbility::RiposteMelee {
                energy_cost: _,
                buildup_duration,
                swing_duration,
                recover_duration,
                block_strength,
                melee_constructor,
                meta: _,
            } => CharacterState::RiposteMelee(riposte_melee::Data {
                static_data: riposte_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    block_strength: *block_strength,
                    melee_constructor: *melee_constructor,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::RapidMelee {
                buildup_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                energy_cost,
                max_strikes,
                move_modifier,
                ori_modifier,
                minimum_combo,
                frontend_specifier,
                meta: _,
            } => CharacterState::RapidMelee(rapid_melee::Data {
                static_data: rapid_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    energy_cost: *energy_cost,
                    max_strikes: *max_strikes,
                    move_modifier: *move_modifier,
                    ori_modifier: *ori_modifier,
                    minimum_combo: *minimum_combo,
                    frontend_specifier: *frontend_specifier,
                    ability_info,
                },
                timer: Duration::default(),
                current_strike: 1,
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::Transform {
                buildup_duration,
                recover_duration,
                target,
                specifier,
                allow_players,
                meta: _,
            } => CharacterState::Transform(transform::Data {
                static_data: transform::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    specifier: *specifier,
                    allow_players: *allow_players,
                    target: target.to_owned(),
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AbilityMeta {
    #[serde(default)]
    pub capabilities: Capability,
    #[serde(default)]
    /// This is an event that gets emitted when the ability is first activated
    pub init_event: Option<AbilityInitEvent>,
    #[serde(default)]
    pub requirements: AbilityRequirements,
    /// Adjusts stats of ability when activated based on context.
    // If we ever add more, I guess change to a vec? Or maybe just an array if we want to keep
    // AbilityMeta small?
    pub contextual_stats: Option<StatAdj>,
}

impl StatAdj {
    pub fn equivalent_stats(&self, data: &JoinData) -> Stats {
        let mut stats = Stats::one();
        let add = match self.context {
            StatContext::PoiseResilience(base) => {
                let poise_res = combat::compute_poise_resilience(data.inventory, data.msm);
                poise_res.unwrap_or(0.0) / base
            },
        };
        match self.field {
            StatField::EffectPower => {
                stats.effect_power += add;
            },
            StatField::BuffStrength => {
                stats.buff_strength += add;
            },
            StatField::Power => {
                stats.power += add;
            },
        }
        stats
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StatAdj {
    pub context: StatContext,
    pub field: StatField,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StatContext {
    PoiseResilience(f32),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StatField {
    EffectPower,
    BuffStrength,
    Power,
}

// TODO: Later move over things like energy and combo into here
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AbilityRequirements {
    pub stance: Option<Stance>,
}

impl AbilityRequirements {
    pub fn requirements_met(&self, stance: Option<&Stance>) -> bool {
        let AbilityRequirements { stance: req_stance } = self;
        req_stance.map_or(true, |req_stance| {
            stance.map_or(false, |char_stance| req_stance == *char_stance)
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub enum SwordStance {
    Crippling,
    Cleaving,
    Defensive,
    Heavy,
    Agile,
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
    // If more are ever needed, first check if any not used anymore, as some were only used in intermediary stages so may be free
    pub struct Capability: u8 {
        // The ability will parry all blockable attacks in the buildup portion
        const PARRIES             = 0b00000001;
        // Allows blocking to interrupt the ability at any point
        const BLOCK_INTERRUPT     = 0b00000010;
        // The ability will block melee attacks in the buildup portion
        const BLOCKS              = 0b00000100;
        // When in the ability, an entity only receives half as much poise damage
        const POISE_RESISTANT     = 0b00001000;
        // WHen in the ability, an entity only receives half as much knockback
        const KNOCKBACK_RESISTANT = 0b00010000;
        // The ability will parry melee attacks in the buildup portion
        const PARRIES_MELEE       = 0b00100000;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub enum Stance {
    None,
    Sword(SwordStance),
}

impl Stance {
    pub fn pseudo_ability_id(&self) -> &str {
        match self {
            Stance::Sword(SwordStance::Heavy) => "veloren.core.pseudo_abilities.sword.heavy_stance",
            Stance::Sword(SwordStance::Agile) => "veloren.core.pseudo_abilities.sword.agile_stance",
            Stance::Sword(SwordStance::Defensive) => {
                "veloren.core.pseudo_abilities.sword.defensive_stance"
            },
            Stance::Sword(SwordStance::Crippling) => {
                "veloren.core.pseudo_abilities.sword.crippling_stance"
            },
            Stance::Sword(SwordStance::Cleaving) => {
                "veloren.core.pseudo_abilities.sword.cleaving_stance"
            },
            Stance::None => "veloren.core.pseudo_abilities.no_stance",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AbilityInitEvent {
    EnterStance(Stance),
    GainBuff {
        kind: buff::BuffKind,
        strength: f32,
        duration: Option<Secs>,
    },
}

impl Default for Stance {
    fn default() -> Self { Self::None }
}

impl Component for Stance {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
