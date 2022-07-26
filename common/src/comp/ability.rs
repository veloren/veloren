use crate::{
    assets::{self, Asset},
    combat::{self, CombatEffect, DamageKind, Knockback},
    comp::{
        self, aura, beam, buff,
        inventory::{
            item::{
                tool::{AbilityItem, Stats, ToolKind},
                ItemKind,
            },
            slot::EquipSlot,
            Inventory,
        },
        melee::{MeleeConstructor, MeleeConstructorKind},
        projectile::ProjectileConstructor,
        skillset::{
            skills::{self, Skill, SKILL_MODIFIERS},
            SkillSet,
        },
        Body, CharacterState, LightEmitter, StateUpdate,
    },
    states::{
        behavior::JoinData,
        utils::{AbilityInfo, StageSection},
        *,
    },
    terrain::SpriteKind,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::{convert::TryFrom, time::Duration};

pub const MAX_ABILITIES: usize = 5;
pub type AuxiliaryKey = (Option<ToolKind>, Option<ToolKind>);

// TODO: Potentially look into storing previous ability sets for weapon
// combinations and automatically reverting back to them on switching to that
// set of weapons. Consider after UI is set up and people weigh in on memory
// considerations.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveAbilities {
    pub primary: PrimaryAbility,
    pub secondary: SecondaryAbility,
    pub movement: MovementAbility,
    pub auxiliary_sets: HashMap<AuxiliaryKey, [AuxiliaryAbility; MAX_ABILITIES]>,
}

impl Component for ActiveAbilities {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

impl Default for ActiveAbilities {
    fn default() -> Self {
        Self {
            primary: PrimaryAbility::Tool,
            secondary: SecondaryAbility::Tool,
            movement: MovementAbility::Species,
            auxiliary_sets: HashMap::new(),
        }
    }
}

impl ActiveAbilities {
    pub fn new(auxiliary_sets: HashMap<AuxiliaryKey, [AuxiliaryAbility; MAX_ABILITIES]>) -> Self {
        ActiveAbilities {
            auxiliary_sets,
            ..Self::default()
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
            .or_insert(Self::default_ability_set(inventory, skill_set));
        if let Some(ability) = auxiliary_set.get_mut(slot) {
            *ability = new_ability;
        }
    }

    pub fn auxiliary_set(
        &self,
        inv: Option<&Inventory>,
        skill_set: Option<&SkillSet>,
    ) -> [AuxiliaryAbility; MAX_ABILITIES] {
        let tool_kind = |slot| {
            inv.and_then(|inv| inv.equipped(slot))
                .and_then(|item| match &*item.kind() {
                    ItemKind::Tool(tool) => Some(tool.kind),
                    _ => None,
                })
        };

        let aux_key = (
            tool_kind(EquipSlot::ActiveMainhand),
            tool_kind(EquipSlot::ActiveOffhand),
        );

        self.auxiliary_sets
            .get(&aux_key)
            .copied()
            .unwrap_or_else(|| Self::default_ability_set(inv, skill_set))
    }

    pub fn get_ability(
        &self,
        input: AbilityInput,
        inventory: Option<&Inventory>,
        skill_set: Option<&SkillSet>,
    ) -> Ability {
        match input {
            AbilityInput::Primary => self.primary.into(),
            AbilityInput::Secondary => self.secondary.into(),
            AbilityInput::Movement => self.movement.into(),
            AbilityInput::Auxiliary(index) => self
                .auxiliary_set(inventory, skill_set)
                .get(index)
                .copied()
                .map(|a| a.into())
                .unwrap_or(Ability::Empty),
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
        // bool is from_offhand
    ) -> Option<(CharacterAbility, bool)> {
        let ability = self.get_ability(input, inv, Some(skill_set));

        let ability_set = |equip_slot| {
            inv.and_then(|inv| inv.equipped(equip_slot))
                .map(|i| &i.item_config_expect().abilities)
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

        let unwrap_ability = |(skill_req, ability): &(Option<Skill>, AbilityItem)| {
            (*skill_req, ability.ability.clone())
        };

        let unlocked = |(s, a): (Option<Skill>, CharacterAbility)| {
            // If there is a skill requirement and the skillset does not contain the
            // required skill, return None
            s.map_or(true, |s| skill_set.has_skill(s)).then_some(a)
        };

        match ability {
            Ability::ToolPrimary => ability_set(EquipSlot::ActiveMainhand)
                .map(|abilities| abilities.primary.ability.clone())
                .map(|ability| (scale_ability(ability, EquipSlot::ActiveMainhand), false)),
            Ability::ToolSecondary => ability_set(EquipSlot::ActiveOffhand)
                .map(|abilities| abilities.secondary.ability.clone())
                .map(|ability| (scale_ability(ability, EquipSlot::ActiveOffhand), true))
                .or_else(|| {
                    ability_set(EquipSlot::ActiveMainhand)
                        .map(|abilities| abilities.secondary.ability.clone())
                        .map(|ability| (scale_ability(ability, EquipSlot::ActiveMainhand), false))
                }),
            Ability::SpeciesMovement => matches!(body, Some(Body::Humanoid(_)))
                .then(CharacterAbility::default_roll)
                .map(|ability| (ability.adjusted_by_skills(skill_set, None), false)),
            Ability::MainWeaponAux(index) => ability_set(EquipSlot::ActiveMainhand)
                .and_then(|abilities| abilities.abilities.get(index).map(unwrap_ability))
                .and_then(unlocked)
                .map(|ability| (scale_ability(ability, EquipSlot::ActiveMainhand), false)),
            Ability::OffWeaponAux(index) => ability_set(EquipSlot::ActiveOffhand)
                .and_then(|abilities| abilities.abilities.get(index).map(unwrap_ability))
                .and_then(unlocked)
                .map(|ability| (scale_ability(ability, EquipSlot::ActiveOffhand), true)),
            Ability::Empty => None,
        }
    }

    pub fn iter_unlocked_abilities<'a>(
        inv: Option<&'a Inventory>,
        skill_set: Option<&'a SkillSet>,
        equip_slot: EquipSlot,
    ) -> impl Iterator<Item = usize> + 'a {
        inv.and_then(|inv| inv.equipped(equip_slot))
            .into_iter()
            .flat_map(|i| &i.item_config_expect().abilities.abilities)
            .enumerate()
            .filter_map(move |(i, (skill, _))| {
                skill
                    .map_or(true, |s| skill_set.map_or(false, |ss| ss.has_skill(s)))
                    .then_some(i)
            })
    }

    fn default_ability_set<'a>(
        inv: Option<&'a Inventory>,
        skill_set: Option<&'a SkillSet>,
    ) -> [AuxiliaryAbility; MAX_ABILITIES] {
        let mut iter = Self::iter_unlocked_abilities(inv, skill_set, EquipSlot::ActiveMainhand)
            .map(AuxiliaryAbility::MainWeapon)
            .chain(
                Self::iter_unlocked_abilities(inv, skill_set, EquipSlot::ActiveOffhand)
                    .map(AuxiliaryAbility::OffWeapon),
            );

        [(); MAX_ABILITIES].map(|()| iter.next().unwrap_or(AuxiliaryAbility::Empty))
    }
}

pub enum AbilityInput {
    Primary,
    Secondary,
    Movement,
    Auxiliary(usize),
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum Ability {
    ToolPrimary,
    ToolSecondary,
    SpeciesMovement,
    MainWeaponAux(usize),
    OffWeaponAux(usize),
    Empty,
    /* For future use
     * ArmorAbility(usize), */
}

impl Ability {
    pub fn ability_id(self, inv: Option<&Inventory>) -> Option<&str> {
        let ability_set = |equip_slot| {
            inv.and_then(|inv| inv.equipped(equip_slot))
                .map(|i| &i.item_config_expect().abilities)
        };

        match self {
            Ability::ToolPrimary => ability_set(EquipSlot::ActiveMainhand)
                .map(|abilities| abilities.primary.id.as_str()),
            Ability::ToolSecondary => ability_set(EquipSlot::ActiveOffhand)
                .map(|abilities| abilities.secondary.id.as_str())
                .or_else(|| {
                    ability_set(EquipSlot::ActiveMainhand)
                        .map(|abilities| abilities.secondary.id.as_str())
                }),
            Ability::SpeciesMovement => None, // TODO: Make not None
            Ability::MainWeaponAux(index) => {
                ability_set(EquipSlot::ActiveMainhand).and_then(|abilities| {
                    abilities
                        .abilities
                        .get(index)
                        .map(|(_, ability)| ability.id.as_str())
                })
            },
            Ability::OffWeaponAux(index) => {
                ability_set(EquipSlot::ActiveOffhand).and_then(|abilities| {
                    abilities
                        .abilities
                        .get(index)
                        .map(|(_, ability)| ability.id.as_str())
                })
            },
            Ability::Empty => None,
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

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum AuxiliaryAbility {
    MainWeapon(usize),
    OffWeapon(usize),
    Empty,
}

impl From<AuxiliaryAbility> for Ability {
    fn from(primary: AuxiliaryAbility) -> Self {
        match primary {
            AuxiliaryAbility::MainWeapon(i) => Ability::MainWeaponAux(i),
            AuxiliaryAbility::OffWeapon(i) => Ability::OffWeaponAux(i),
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
    ComboMelee(StageSection, u32),
    LeapMelee(StageSection),
    SpinMelee(StageSection),
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
            CharacterState::ComboMelee(data) => Self::ComboMelee(data.stage_section, data.stage),
            CharacterState::SpinMelee(data) => Self::SpinMelee(data.stage_section),
            CharacterState::ChargedMelee(data) => Self::ChargedMelee(data.stage_section),
            CharacterState::ChargedRanged(_) => Self::ChargedRanged,
            CharacterState::Shockwave(_) => Self::Shockwave,
            CharacterState::BasicBeam(_) => Self::BasicBeam,
            CharacterState::RepeaterRanged(_) => Self::RepeaterRanged,
            CharacterState::BasicAura(_) => Self::BasicAura,
            CharacterState::SelfBuff(_) => Self::SelfBuff,
            CharacterState::Idle(_)
            | CharacterState::Climb(_)
            | CharacterState::Sit
            | CharacterState::Dance
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
            | CharacterState::Wallrun(_) => Self::Other,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CharacterAbility {
    BasicMelee {
        energy_cost: f32,
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        ori_modifier: f32,
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
    },
    Boost {
        movement_duration: f32,
        only_up: bool,
        speed: f32,
        max_exit_velocity: f32,
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
        charge_through: bool,
        is_interruptible: bool,
    },
    BasicBlock {
        buildup_duration: f32,
        recover_duration: f32,
        max_angle: f32,
        block_strength: f32,
        energy_cost: f32,
    },
    Roll {
        energy_cost: f32,
        buildup_duration: f32,
        movement_duration: f32,
        recover_duration: f32,
        roll_strength: f32,
        immune_melee: bool,
    },
    ComboMelee {
        stage_data: Vec<combo_melee::Stage<f32>>,
        initial_energy_gain: f32,
        max_energy_gain: f32,
        energy_increase: f32,
        speed_increase: f32,
        max_speed_increase: f32,
        scales_from_combo: u32,
        is_interruptible: bool,
        ori_modifier: f32,
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
    },
    SpinMelee {
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        energy_cost: f32,
        is_infinite: bool,
        movement_behavior: spin_melee::MovementBehavior,
        is_interruptible: bool,
        forward_speed: f32,
        num_spins: u32,
        specifier: Option<spin_melee::FrontendSpecifier>,
        melee_constructor: MeleeConstructor,
    },
    ChargedMelee {
        energy_cost: f32,
        energy_drain: f32,
        charge_duration: f32,
        swing_duration: f32,
        hit_timing: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        specifier: Option<charged_melee::FrontendSpecifier>,
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
        move_speed: f32,
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
        requires_ground: bool,
        move_efficiency: f32,
        damage_kind: DamageKind,
        specifier: comp::shockwave::FrontendSpecifier,
        damage_effect: Option<CombatEffect>,
    },
    BasicBeam {
        buildup_duration: f32,
        recover_duration: f32,
        beam_duration: f32,
        damage: f32,
        tick_rate: f32,
        range: f32,
        max_angle: f32,
        damage_effect: Option<CombatEffect>,
        energy_regen: f32,
        energy_drain: f32,
        ori_rate: f32,
        specifier: beam::FrontendSpecifier,
    },
    BasicAura {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        targets: combat::GroupTarget,
        auras: Vec<aura::AuraBuffConstructor>,
        aura_duration: f32,
        range: f32,
        energy_cost: f32,
        scales_with_combo: bool,
        specifier: Option<aura::Specifier>,
    },
    Blink {
        buildup_duration: f32,
        recover_duration: f32,
        max_range: f32,
    },
    BasicSummon {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        summon_amount: u32,
        summon_distance: (f32, f32),
        summon_info: basic_summon::SummonInfo,
        duration: Option<Duration>,
    },
    SelfBuff {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        buff_kind: buff::BuffKind,
        buff_strength: f32,
        buff_duration: Option<f32>,
        energy_cost: f32,
    },
    SpriteSummon {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        sprite: SpriteKind,
        summon_distance: (f32, f32),
        sparseness: f64,
    },
}

impl Default for CharacterAbility {
    fn default() -> Self {
        CharacterAbility::BasicMelee {
            energy_cost: 0.0,
            buildup_duration: 0.25,
            swing_duration: 0.25,
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
                damage_effect: None,
            },
            ori_modifier: 1.0,
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
        match self {
            CharacterAbility::Roll { energy_cost, .. } => {
                data.physics.on_ground.is_some()
                    && data.inputs.move_dir.magnitude_squared() > 0.25
                    && update.energy.try_change_by(-*energy_cost).is_ok()
            },
            CharacterAbility::DashMelee { energy_cost, .. }
            | CharacterAbility::BasicMelee { energy_cost, .. }
            | CharacterAbility::BasicRanged { energy_cost, .. }
            | CharacterAbility::SpinMelee { energy_cost, .. }
            | CharacterAbility::ChargedRanged { energy_cost, .. }
            | CharacterAbility::ChargedMelee { energy_cost, .. }
            | CharacterAbility::Shockwave { energy_cost, .. }
            | CharacterAbility::BasicBlock { energy_cost, .. }
            | CharacterAbility::SelfBuff { energy_cost, .. } => {
                update.energy.try_change_by(-*energy_cost).is_ok()
            },
            // Consumes energy within state, so value only checked before entering state
            CharacterAbility::RepeaterRanged { energy_cost, .. } => {
                update.energy.current() >= *energy_cost
            },
            CharacterAbility::LeapMelee { energy_cost, .. } => {
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
            CharacterAbility::ComboMelee { .. }
            | CharacterAbility::Boost { .. }
            | CharacterAbility::BasicBeam { .. }
            | CharacterAbility::Blink { .. }
            | CharacterAbility::BasicSummon { .. }
            | CharacterAbility::SpriteSummon { .. } => true,
        }
    }

    pub fn default_roll() -> CharacterAbility {
        CharacterAbility::Roll {
            energy_cost: 12.0,
            buildup_duration: 0.05,
            movement_duration: 0.33,
            recover_duration: 0.125,
            roll_strength: 2.0,
            immune_melee: true,
        }
    }

    pub fn default_block() -> CharacterAbility {
        CharacterAbility::BasicBlock {
            buildup_duration: 0.25,
            recover_duration: 0.2,
            max_angle: 60.0,
            block_strength: 0.5,
            energy_cost: 2.5,
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
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats, 1.0);
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
                charge_through: _,
                is_interruptible: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *energy_drain /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats, 1.0);
            },
            BasicBlock {
                ref mut buildup_duration,
                ref mut recover_duration,
                // Do we want angle to be adjusted by range?
                max_angle: _,
                // Block strength explicitly not modified by power, that will be a separate stat
                block_strength: _,
                ref mut energy_cost,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
            },
            Roll {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut movement_duration,
                ref mut recover_duration,
                roll_strength: _,
                immune_melee: _,
            } => {
                *buildup_duration /= stats.speed;
                *movement_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
            },
            ComboMelee {
                ref mut stage_data,
                initial_energy_gain: _,
                max_energy_gain: _,
                energy_increase: _,
                speed_increase: _,
                max_speed_increase: _,
                scales_from_combo: _,
                is_interruptible: _,
                ori_modifier: _,
            } => {
                *stage_data = stage_data
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
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats, 1.0);
            },
            SpinMelee {
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut energy_cost,
                ref mut melee_constructor,
                is_infinite: _,
                movement_behavior: _,
                is_interruptible: _,
                forward_speed: _,
                num_spins: _,
                specifier: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats, 1.0);
            },
            ChargedMelee {
                ref mut energy_cost,
                ref mut energy_drain,
                charge_duration: _,
                ref mut swing_duration,
                hit_timing: _,
                ref mut recover_duration,
                ref mut melee_constructor,
                specifier: _,
            } => {
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *energy_drain /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats, 1.0);
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
                charge_duration: _,
                ref mut recover_duration,
                projectile_body: _,
                projectile_light: _,
                ref mut initial_projectile_speed,
                ref mut scaled_projectile_speed,
                move_speed: _,
            } => {
                *initial_damage *= stats.power;
                *scaled_damage *= stats.power;
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *initial_projectile_speed *= stats.range;
                *scaled_projectile_speed *= stats.range;
                *energy_cost /= stats.energy_efficiency;
                *energy_drain /= stats.energy_efficiency;
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
                requires_ground: _,
                move_efficiency: _,
                damage_kind: _,
                specifier: _,
                ref mut damage_effect,
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
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *damage *= stats.power;
                *tick_rate *= stats.speed;
                *range *= stats.range;
                // Duration modified to keep velocity constant
                *beam_duration *= stats.range;
                *energy_drain /= stats.energy_efficiency;
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
            Blink {
                ref mut buildup_duration,
                ref mut recover_duration,
                ref mut max_range,
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
                summon_distance: (ref mut inner_dist, ref mut outer_dist),
                sparseness: _,
            } => {
                // TODO: Figure out how/if power should affect this
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *inner_dist *= stats.range;
                *outer_dist *= stats.range;
            },
        }
        self
    }

    pub fn get_energy_cost(&self) -> f32 {
        use CharacterAbility::*;
        match self {
            BasicMelee { energy_cost, .. }
            | BasicRanged { energy_cost, .. }
            | RepeaterRanged { energy_cost, .. }
            | DashMelee { energy_cost, .. }
            | Roll { energy_cost, .. }
            | LeapMelee { energy_cost, .. }
            | SpinMelee { energy_cost, .. }
            | ChargedMelee { energy_cost, .. }
            | ChargedRanged { energy_cost, .. }
            | Shockwave { energy_cost, .. }
            | BasicAura { energy_cost, .. }
            | BasicBlock { energy_cost, .. }
            | SelfBuff { energy_cost, .. } => *energy_cost,
            BasicBeam { energy_drain, .. } => {
                if *energy_drain > f32::EPSILON {
                    1.0
                } else {
                    0.0
                }
            },
            Boost { .. }
            | ComboMelee { .. }
            | Blink { .. }
            | BasicSummon { .. }
            | SpriteSummon { .. } => 0.0,
        }
    }

    #[must_use = "method returns new ability and doesn't mutate the original value"]
    pub fn adjusted_by_skills(mut self, skillset: &SkillSet, tool: Option<ToolKind>) -> Self {
        match tool {
            Some(ToolKind::Sword) => self.adjusted_by_sword_skills(skillset),
            Some(ToolKind::Axe) => self.adjusted_by_axe_skills(skillset),
            Some(ToolKind::Hammer) => self.adjusted_by_hammer_skills(skillset),
            Some(ToolKind::Bow) => self.adjusted_by_bow_skills(skillset),
            Some(ToolKind::Staff) => self.adjusted_by_staff_skills(skillset),
            Some(ToolKind::Sceptre) => self.adjusted_by_sceptre_skills(skillset),
            Some(ToolKind::Pick) => self.adjusted_by_mining_skills(skillset),
            None => self.adjusted_by_general_skills(skillset),
            Some(_) => {},
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

    fn adjusted_by_general_skills(&mut self, skillset: &SkillSet) {
        if let CharacterAbility::Roll {
            ref mut energy_cost,
            ref mut roll_strength,
            ref mut movement_duration,
            ..
        } = self
        {
            use skills::RollSkill::{Cost, Duration, Strength};

            let modifiers = SKILL_MODIFIERS.general_tree.roll;

            if let Ok(level) = skillset.skill_level(Skill::Roll(Cost)) {
                *energy_cost *= modifiers.energy_cost.powi(level.into());
            }
            if let Ok(level) = skillset.skill_level(Skill::Roll(Strength)) {
                *roll_strength *= modifiers.strength.powi(level.into());
            }
            if let Ok(level) = skillset.skill_level(Skill::Roll(Duration)) {
                *movement_duration *= modifiers.duration.powi(level.into());
            }
        }
    }

    fn adjusted_by_sword_skills(&mut self, skillset: &SkillSet) {
        use skills::{Skill::Sword, SwordSkill::*};

        match self {
            CharacterAbility::ComboMelee {
                ref mut is_interruptible,
                ref mut speed_increase,
                ref mut max_speed_increase,
                ref stage_data,
                ref mut max_energy_gain,
                ref mut scales_from_combo,
                ..
            } => {
                *is_interruptible = skillset.has_skill(Sword(InterruptingAttacks));

                if skillset.has_skill(Sword(TsCombo)) {
                    let speed_segments = f32::from(Sword(TsSpeed).max_level()) + 1.0;
                    let speed_level = f32::from(skillset.skill_level(Sword(TsSpeed)).unwrap_or(0));
                    *speed_increase = (speed_level + 1.0) / speed_segments;
                    *max_speed_increase = (speed_level + 1.0) / speed_segments;
                } else {
                    *speed_increase = 0.0;
                    *max_speed_increase = 0.0;
                }

                let energy_level = skillset.skill_level(Sword(TsRegen)).unwrap_or(0);

                let stages = u16::try_from(stage_data.len())
                    .expect("number of stages can't be more than u16");

                *max_energy_gain *= f32::from((energy_level + 1) * stages - 1)
                    * f32::from(stages - 1)
                    / f32::from(Sword(TsRegen).max_level() + 1);
                *scales_from_combo = skillset.skill_level(Sword(TsDamage)).unwrap_or(0).into();
            },
            CharacterAbility::DashMelee {
                ref mut is_interruptible,
                ref mut energy_cost,
                ref mut energy_drain,
                ref mut forward_speed,
                ref mut charge_through,
                ref mut melee_constructor,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.sword_tree.dash;
                *is_interruptible = skillset.has_skill(Sword(InterruptingAttacks));
                if let Ok(level) = skillset.skill_level(Sword(DCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Sword(DDrain)) {
                    *energy_drain *= modifiers.energy_drain.powi(level.into());
                }
                if let MeleeConstructorKind::Slash { ref mut damage, .. } = melee_constructor.kind {
                    if let Ok(level) = skillset.skill_level(Sword(DDamage)) {
                        *damage *= modifiers.base_damage.powi(level.into());
                    }
                }
                if let Some(MeleeConstructorKind::Slash { ref mut damage, .. }) =
                    melee_constructor.scaled
                {
                    if let Ok(level) = skillset.skill_level(Sword(DScaling)) {
                        *damage *= modifiers.scaled_damage.powi(level.into());
                    }
                }
                if skillset.has_skill(Sword(DSpeed)) {
                    *forward_speed *= modifiers.forward_speed;
                }
                *charge_through = skillset.has_skill(Sword(DChargeThrough));
            },
            CharacterAbility::SpinMelee {
                ref mut is_interruptible,
                ref mut swing_duration,
                ref mut energy_cost,
                ref mut num_spins,
                ref mut melee_constructor,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.sword_tree.spin;
                *is_interruptible = skillset.has_skill(Sword(InterruptingAttacks));
                if let MeleeConstructorKind::Slash { ref mut damage, .. } = melee_constructor.kind {
                    if let Ok(level) = skillset.skill_level(Sword(SDamage)) {
                        *damage *= modifiers.base_damage.powi(level.into());
                    }
                }
                if let Ok(level) = skillset.skill_level(Sword(SSpeed)) {
                    *swing_duration *= modifiers.swing_duration.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Sword(SCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
                let spin_level = skillset.skill_level(Sword(SSpins)).unwrap_or(0);
                *num_spins = u32::from(spin_level) * modifiers.num + 1;
            },
            _ => {},
        }
    }

    fn adjusted_by_axe_skills(&mut self, skillset: &SkillSet) {
        #![allow(clippy::enum_glob_use)]
        use skills::{AxeSkill::*, Skill::Axe};

        match self {
            CharacterAbility::ComboMelee {
                ref mut speed_increase,
                ref mut max_speed_increase,
                ref mut stage_data,
                ref mut max_energy_gain,
                ref mut scales_from_combo,
                ..
            } => {
                if !skillset.has_skill(Axe(DsCombo)) {
                    stage_data.pop();
                }
                let speed_segments = f32::from(Axe(DsSpeed).max_level());
                let speed_level = f32::from(skillset.skill_level(Axe(DsSpeed)).unwrap_or(0));
                *speed_increase *= speed_level / speed_segments;
                *max_speed_increase *= speed_level / speed_segments;

                let energy_level = skillset.skill_level(Axe(DsRegen)).unwrap_or(0);

                let stages = u16::try_from(stage_data.len())
                    .expect("number of stages can't be more than u16");

                *max_energy_gain *= f32::from((energy_level + 1) * stages - 1).max(1.0)
                    * f32::from(stages - 1).max(1.0)
                    / f32::from(Axe(DsRegen).max_level() + 1);
                *scales_from_combo = skillset.skill_level(Axe(DsDamage)).unwrap_or(0).into();
            },
            CharacterAbility::SpinMelee {
                ref mut swing_duration,
                ref mut energy_cost,
                ref mut is_infinite,
                ref mut movement_behavior,
                ref mut melee_constructor,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.axe_tree.spin;

                *is_infinite = skillset.has_skill(Axe(SInfinite));
                *movement_behavior = if skillset.has_skill(Axe(SHelicopter)) {
                    spin_melee::MovementBehavior::AxeHover
                } else {
                    spin_melee::MovementBehavior::Walking
                };
                if let MeleeConstructorKind::Slash { ref mut damage, .. } = melee_constructor.kind {
                    if let Ok(level) = skillset.skill_level(Axe(SDamage)) {
                        *damage *= modifiers.base_damage.powi(level.into());
                    }
                }
                if let Ok(level) = skillset.skill_level(Axe(SSpeed)) {
                    *swing_duration *= modifiers.swing_duration.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Axe(SCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
            },
            CharacterAbility::LeapMelee {
                ref mut melee_constructor,
                ref mut energy_cost,
                ref mut forward_leap_strength,
                ref mut vertical_leap_strength,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.axe_tree.leap;
                if let MeleeConstructorKind::Slash {
                    ref mut damage,
                    ref mut knockback,
                    ..
                } = melee_constructor.kind
                {
                    if let Ok(level) = skillset.skill_level(Axe(LDamage)) {
                        *damage *= modifiers.base_damage.powi(level.into());
                    }
                    if let Ok(level) = skillset.skill_level(Axe(LKnockback)) {
                        *knockback *= modifiers.knockback.powi(level.into());
                    }
                }
                if let Ok(level) = skillset.skill_level(Axe(LCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Axe(LDistance)) {
                    let strength = modifiers.leap_strength;
                    *forward_leap_strength *= strength.powi(level.into());
                    *vertical_leap_strength *= strength.powi(level.into());
                }
            },
            _ => {},
        }
    }

    fn adjusted_by_hammer_skills(&mut self, skillset: &SkillSet) {
        #![allow(clippy::enum_glob_use)]
        use skills::{HammerSkill::*, Skill::Hammer};

        match self {
            CharacterAbility::ComboMelee {
                ref mut speed_increase,
                ref mut max_speed_increase,
                ref mut stage_data,
                ref mut max_energy_gain,
                ref mut scales_from_combo,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.hammer_tree.single_strike;

                if let Ok(level) = skillset.skill_level(Hammer(SsKnockback)) {
                    *stage_data = (*stage_data)
                        .iter()
                        .map(|s| s.modify_strike(modifiers.knockback.powi(level.into())))
                        .collect::<Vec<_>>();
                }
                let speed_segments = f32::from(Hammer(SsSpeed).max_level());
                let speed_level = f32::from(skillset.skill_level(Hammer(SsSpeed)).unwrap_or(0));
                *speed_increase *= speed_level / speed_segments;
                *max_speed_increase *= speed_level / speed_segments;

                let energy_level = skillset.skill_level(Hammer(SsRegen)).unwrap_or(0);

                let stages = u16::try_from(stage_data.len())
                    .expect("number of stages can't be more than u16");

                *max_energy_gain *= f32::from((energy_level + 1) * stages)
                    / f32::from((Hammer(SsRegen).max_level() + 1) * stages);

                *scales_from_combo = skillset.skill_level(Hammer(SsDamage)).unwrap_or(0).into();
            },
            CharacterAbility::ChargedMelee {
                ref mut energy_drain,
                ref mut charge_duration,
                ref mut melee_constructor,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.hammer_tree.charged;

                if let Some(MeleeConstructorKind::Bash {
                    ref mut damage,
                    ref mut knockback,
                    ..
                }) = melee_constructor.scaled
                {
                    if let Ok(level) = skillset.skill_level(Hammer(CDamage)) {
                        *damage *= modifiers.scaled_damage.powi(level.into());
                    }
                    if let Ok(level) = skillset.skill_level(Hammer(CKnockback)) {
                        *knockback *= modifiers.scaled_knockback.powi(level.into());
                    }
                }
                if let Ok(level) = skillset.skill_level(Hammer(CDrain)) {
                    *energy_drain *= modifiers.energy_drain.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Hammer(CSpeed)) {
                    let charge_time = 1.0 / modifiers.charge_rate;
                    *charge_duration *= charge_time.powi(level.into());
                }
            },
            CharacterAbility::LeapMelee {
                ref mut energy_cost,
                ref mut forward_leap_strength,
                ref mut vertical_leap_strength,
                ref mut melee_constructor,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.hammer_tree.leap;
                if let MeleeConstructorKind::Bash {
                    ref mut damage,
                    ref mut knockback,
                    ..
                } = melee_constructor.kind
                {
                    if let Ok(level) = skillset.skill_level(Hammer(LDamage)) {
                        *damage *= modifiers.base_damage.powi(level.into());
                    }
                    if let Ok(level) = skillset.skill_level(Hammer(LKnockback)) {
                        *knockback *= modifiers.knockback.powi(level.into());
                    }
                }
                if let Ok(level) = skillset.skill_level(Hammer(LCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Hammer(LDistance)) {
                    let strength = modifiers.leap_strength;
                    *forward_leap_strength *= strength.powi(level.into());
                    *vertical_leap_strength *= strength.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Hammer(LRange)) {
                    melee_constructor.range += modifiers.range * f32::from(level);
                }
            },
            _ => {},
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
                    *beam_duration *= range_mod;
                }
                if let Ok(level) = skillset.skill_level(Staff(FDrain)) {
                    *energy_drain *= modifiers.energy_drain.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Staff(FVelocity)) {
                    let velocity_increase = modifiers.velocity.powi(level.into());
                    let duration_mod = 1.0 / (1.0 + velocity_increase);
                    *beam_duration *= duration_mod;
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
                    *beam_duration *= range_mod;
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
                            *duration *= modifiers.duration.powi(level.into());
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
                            *duration *= modifiers.duration.powi(level.into());
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

impl From<(&CharacterAbility, AbilityInfo, &JoinData<'_>)> for CharacterState {
    fn from((ability, ability_info, data): (&CharacterAbility, AbilityInfo, &JoinData)) -> Self {
        match ability {
            CharacterAbility::BasicMelee {
                buildup_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                ori_modifier,
                energy_cost: _,
            } => CharacterState::BasicMelee(basic_melee::Data {
                static_data: basic_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ori_modifier: *ori_modifier,
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
                charge_through,
                is_interruptible,
            } => CharacterState::DashMelee(dash_melee::Data {
                static_data: dash_melee::StaticData {
                    energy_drain: *energy_drain,
                    forward_speed: *forward_speed,
                    charge_through: *charge_through,
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    charge_duration: Duration::from_secs_f32(*charge_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ori_modifier: *ori_modifier,
                    is_interruptible: *is_interruptible,
                    ability_info,
                },
                auto_charge: false,
                timer: Duration::default(),
                charge_end_timer: Duration::from_secs_f32(*charge_duration),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::BasicBlock {
                buildup_duration,
                recover_duration,
                max_angle,
                block_strength,
                energy_cost,
            } => CharacterState::BasicBlock(basic_block::Data {
                static_data: basic_block::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    max_angle: *max_angle,
                    block_strength: *block_strength,
                    energy_cost: *energy_cost,
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
                immune_melee,
            } => CharacterState::Roll(roll::Data {
                static_data: roll::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    roll_strength: *roll_strength,
                    immune_melee: *immune_melee,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                was_wielded: false, // false by default. utils might set it to true
                is_sneaking: false,
                was_combo: None,
            }),
            CharacterAbility::ComboMelee {
                stage_data,
                initial_energy_gain,
                max_energy_gain,
                energy_increase,
                speed_increase,
                max_speed_increase,
                scales_from_combo,
                is_interruptible,
                ori_modifier,
            } => CharacterState::ComboMelee(combo_melee::Data {
                static_data: combo_melee::StaticData {
                    num_stages: stage_data.len() as u32,
                    stage_data: stage_data.iter().map(|stage| stage.to_duration()).collect(),
                    initial_energy_gain: *initial_energy_gain,
                    max_energy_gain: *max_energy_gain,
                    energy_increase: *energy_increase,
                    speed_increase: 1.0 - *speed_increase,
                    max_speed_increase: *max_speed_increase,
                    scales_from_combo: *scales_from_combo,
                    is_interruptible: *is_interruptible,
                    ori_modifier: *ori_modifier as f32,
                    ability_info,
                },
                exhausted: false,
                stage: 1,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
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
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::SpinMelee {
                buildup_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                energy_cost,
                is_infinite,
                movement_behavior,
                is_interruptible,
                forward_speed,
                num_spins,
                specifier,
            } => CharacterState::SpinMelee(spin_melee::Data {
                static_data: spin_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    energy_cost: *energy_cost,
                    is_infinite: *is_infinite,
                    movement_behavior: *movement_behavior,
                    is_interruptible: *is_interruptible,
                    forward_speed: *forward_speed,
                    num_spins: *num_spins,
                    ability_info,
                    specifier: *specifier,
                },
                timer: Duration::default(),
                consecutive_spins: 1,
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::ChargedMelee {
                energy_cost,
                energy_drain,
                charge_duration,
                swing_duration,
                hit_timing,
                recover_duration,
                melee_constructor,
                specifier,
            } => CharacterState::ChargedMelee(charged_melee::Data {
                static_data: charged_melee::StaticData {
                    energy_cost: *energy_cost,
                    energy_drain: *energy_drain,
                    charge_duration: Duration::from_secs_f32(*charge_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    hit_timing: *hit_timing,
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ability_info,
                    specifier: *specifier,
                },
                stage_section: StageSection::Charge,
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
                move_speed,
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
                requires_ground,
                move_efficiency,
                damage_kind,
                specifier,
                damage_effect,
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
                    requires_ground: *requires_ground,
                    move_efficiency: *move_efficiency,
                    damage_effect: *damage_effect,
                    ability_info,
                    damage_kind: *damage_kind,
                    specifier: *specifier,
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
            } => CharacterState::BasicBeam(basic_beam::Data {
                static_data: basic_beam::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    beam_duration: Duration::from_secs_f32(*beam_duration),
                    damage: *damage,
                    tick_rate: *tick_rate,
                    range: *range,
                    max_angle: *max_angle,
                    damage_effect: *damage_effect,
                    energy_regen: *energy_regen,
                    energy_drain: *energy_drain,
                    ability_info,
                    ori_rate: *ori_rate,
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
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
            } => CharacterState::BasicAura(basic_aura::Data {
                static_data: basic_aura::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    targets: *targets,
                    auras: auras.clone(),
                    aura_duration: Duration::from_secs_f32(*aura_duration),
                    range: *range,
                    ability_info,
                    scales_with_combo: *scales_with_combo,
                    combo_at_cast: data.combo.map_or(0, |c| c.counter()),
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::Blink {
                buildup_duration,
                recover_duration,
                max_range,
            } => CharacterState::Blink(blink::Data {
                static_data: blink::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    max_range: *max_range,
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
            } => CharacterState::SelfBuff(self_buff::Data {
                static_data: self_buff::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    buff_kind: *buff_kind,
                    buff_strength: *buff_strength,
                    buff_duration: buff_duration.map(Duration::from_secs_f32),
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::SpriteSummon {
                buildup_duration,
                cast_duration,
                recover_duration,
                sprite,
                summon_distance,
                sparseness,
            } => CharacterState::SpriteSummon(sprite_summon::Data {
                static_data: sprite_summon::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    sprite: *sprite,
                    summon_distance: *summon_distance,
                    sparseness: *sparseness,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                achieved_radius: summon_distance.0.floor() as i32 - 1,
            }),
        }
    }
}
