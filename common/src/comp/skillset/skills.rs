use crate::comp::skillset::{
    SkillGroupKind, SKILL_GROUP_LOOKUP, SKILL_MAX_LEVEL, SKILL_PREREQUISITES,
};
use serde::{Deserialize, Serialize};

/// Represents a skill that a player can unlock, that either grants them some
/// kind of active ability, or a passive effect etc. Obviously because this is
/// an enum it doesn't describe what the skill actually -does-, this will be
/// handled by dedicated ECS systems.
// NOTE: if skill does use some constant, add it to corresponding
// SkillTree Modifiers below.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum Skill {
    General(GeneralSkill),
    Sword(SwordSkill),
    Axe(AxeSkill),
    Hammer(HammerSkill),
    Bow(BowSkill),
    Staff(StaffSkill),
    Sceptre(SceptreSkill),
    Roll(RollSkill),
    Climb(ClimbSkill),
    Swim(SwimSkill),
    Pick(MiningSkill),
    // TODO: Don't do this, maybe Sharp has idea?
    UnlockGroup(SkillGroupKind),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum SwordSkill {
    // Sword passives
    InterruptingAttacks,
    // Triple strike upgrades
    TsCombo,
    TsDamage,
    TsRegen,
    TsSpeed,
    // Dash upgrades
    DCost,
    DDrain,
    DDamage,
    DScaling,
    DSpeed,
    DInfinite, // Represents charge through, not migrated because laziness
    // Spin upgrades
    UnlockSpin,
    SDamage,
    SSpeed,
    SCost,
    SSpins,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum AxeSkill {
    // Double strike upgrades
    DsCombo,
    DsDamage,
    DsSpeed,
    DsRegen,
    // Spin upgrades
    SInfinite,
    SHelicopter,
    SDamage,
    SSpeed,
    SCost,
    // Leap upgrades
    UnlockLeap,
    LDamage,
    LKnockback,
    LCost,
    LDistance,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum HammerSkill {
    // Single strike upgrades
    SsKnockback,
    SsDamage,
    SsSpeed,
    SsRegen,
    // Charged melee upgrades
    CDamage,
    CKnockback,
    CDrain,
    CSpeed,
    // Leap upgrades
    UnlockLeap,
    LDamage,
    LCost,
    LDistance,
    LKnockback,
    LRange,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum BowSkill {
    // Passives
    ProjSpeed,
    // Charged upgrades
    CDamage,
    CRegen,
    CKnockback,
    CSpeed,
    CMove,
    // Repeater upgrades
    RDamage,
    RCost,
    RSpeed,
    // Shotgun upgrades
    UnlockShotgun,
    SDamage,
    SCost,
    SArrows,
    SSpread,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum StaffSkill {
    // Basic ranged upgrades
    BDamage,
    BRegen,
    BRadius,
    // Flamethrower upgrades
    FDamage,
    FRange,
    FDrain,
    FVelocity,
    // Shockwave upgrades
    UnlockShockwave,
    SDamage,
    SKnockback,
    SRange,
    SCost,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum SceptreSkill {
    // Lifesteal beam upgrades
    LDamage,
    LRange,
    LLifesteal,
    LRegen,
    // Healing aura upgrades
    HHeal,
    HRange,
    HDuration,
    HCost,
    // Warding aura upgrades
    UnlockAura,
    AStrength,
    ADuration,
    ARange,
    ACost,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum GeneralSkill {
    HealthIncrease,
    EnergyIncrease,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum RollSkill {
    Cost,
    Strength,
    Duration,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum ClimbSkill {
    Cost,
    Speed,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum SwimSkill {
    Speed,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum MiningSkill {
    Speed,
    OreGain,
    GemGain,
}

impl Skill {
    /// Returns a vec of prerequisite skills (it should only be necessary to
    /// note direct prerequisites)
    pub fn prerequisite_skills(&self) -> impl Iterator<Item = (Skill, Option<u16>)> {
        SKILL_PREREQUISITES
            .get(self)
            .into_iter()
            .flatten()
            .map(|(skill, level)| (*skill, *level))
    }

    /// Returns the cost in skill points of unlocking a particular skill
    pub fn skill_cost(&self, level: Option<u16>) -> u16 {
        // TODO: Better balance the costs later
        level.unwrap_or(1)
    }

    /// Returns the maximum level a skill can reach, returns None if the skill
    /// doesn't level
    pub fn max_level(&self) -> Option<u16> { SKILL_MAX_LEVEL.get(self).copied().flatten() }

    /// Returns the skill group type for a skill from the static skill group
    /// definitions.
    pub fn skill_group_kind(&self) -> Option<SkillGroupKind> {
        SKILL_GROUP_LOOKUP.get(self).copied()
    }
}

/// Tree of modifiers that represent how stats are
/// changed per each skill level.
///
/// It's used as bridge between ECS systems
/// and voxygen Diary for skill descriptions and helps to sync them.
///
/// NOTE: Just adding constant does nothing, you need to use it in both
/// ECS systems and Diary.
// TODO: make it lazy_static and move to .ron?
pub const SKILL_MODIFIERS: SkillTreeModifiers = SkillTreeModifiers::get();

pub struct SkillTreeModifiers {
    pub sword_tree: SwordTreeModifiers,
    pub axe_tree: AxeTreeModifiers,
    pub hammer_tree: HammerTreeModifiers,
    pub bow_tree: BowTreeModifiers,
    pub staff_tree: StaffTreeModifiers,
    pub sceptre_tree: SceptreTreeModifiers,
    pub mining_tree: MiningTreeModifiers,
    pub general_tree: GeneralTreeModifiers,
}

impl SkillTreeModifiers {
    const fn get() -> Self {
        Self {
            sword_tree: SwordTreeModifiers::get(),
            axe_tree: AxeTreeModifiers::get(),
            hammer_tree: HammerTreeModifiers::get(),
            bow_tree: BowTreeModifiers::get(),
            staff_tree: StaffTreeModifiers::get(),
            sceptre_tree: SceptreTreeModifiers::get(),
            mining_tree: MiningTreeModifiers::get(),
            general_tree: GeneralTreeModifiers::get(),
        }
    }
}

pub struct SwordTreeModifiers {
    pub dash: SwordDashModifiers,
    pub spin: SwordSpinModifiers,
}

pub struct SwordDashModifiers {
    pub energy_cost: f32,
    pub energy_drain: f32,
    pub base_damage: f32,
    pub scaled_damage: f32,
    pub forward_speed: f32,
}

pub struct SwordSpinModifiers {
    pub base_damage: f32,
    pub swing_duration: f32,
    pub energy_cost: f32,
    pub num: u32,
}

impl SwordTreeModifiers {
    const fn get() -> Self {
        Self {
            dash: SwordDashModifiers {
                energy_cost: 0.9,
                energy_drain: 0.9,
                base_damage: 1.1,
                scaled_damage: 1.1,
                forward_speed: 1.05,
            },
            spin: SwordSpinModifiers {
                base_damage: 1.2,
                swing_duration: 0.9,
                energy_cost: 0.9,
                num: 1,
            },
        }
    }
}

pub struct AxeTreeModifiers {
    pub spin: AxeSpinModifiers,
    pub leap: AxeLeapModifiers,
}

pub struct AxeSpinModifiers {
    pub base_damage: f32,
    pub swing_duration: f32,
    pub energy_cost: f32,
}

pub struct AxeLeapModifiers {
    pub base_damage: f32,
    pub knockback: f32,
    pub energy_cost: f32,
    // TODO: split to forward and vertical?
    pub leap_strength: f32,
}

impl AxeTreeModifiers {
    const fn get() -> Self {
        Self {
            spin: AxeSpinModifiers {
                base_damage: 1.2,
                swing_duration: 0.85,
                energy_cost: 0.85,
            },
            leap: AxeLeapModifiers {
                base_damage: 1.2,
                knockback: 1.2,
                energy_cost: 0.75,
                leap_strength: 1.1,
            },
        }
    }
}

pub struct HammerTreeModifiers {
    pub single_strike: HammerStrikeModifiers,
    pub charged: HammerChargedModifers,
    pub leap: HammerLeapModifiers,
}

pub struct HammerStrikeModifiers {
    pub knockback: f32,
}

pub struct HammerChargedModifers {
    pub scaled_damage: f32,
    pub scaled_knockback: f32,
    pub energy_drain: f32,
    pub charge_rate: f32,
}

pub struct HammerLeapModifiers {
    pub base_damage: f32,
    pub knockback: f32,
    pub energy_cost: f32,
    pub leap_strength: f32,
    pub range: f32,
}

impl HammerTreeModifiers {
    const fn get() -> Self {
        Self {
            single_strike: HammerStrikeModifiers { knockback: 1.25 },
            charged: HammerChargedModifers {
                scaled_damage: 1.2,
                scaled_knockback: 1.3,
                energy_drain: 0.85,
                charge_rate: 1.15,
            },
            leap: HammerLeapModifiers {
                base_damage: 1.25,
                knockback: 1.3,
                energy_cost: 0.75,
                leap_strength: 1.1,
                range: 0.5,
            },
        }
    }
}

pub struct BowTreeModifiers {
    pub universal: BowUniversalModifiers,
    pub charged: BowChargedModifiers,
    pub repeater: BowRepeaterModifiers,
    pub shotgun: BowShotgunModifiers,
}

pub struct BowUniversalModifiers {
    // TODO: split per abilities?
    pub projectile_speed: f32,
}

pub struct BowChargedModifiers {
    pub damage_scaling: f32,
    pub regen_scaling: f32,
    pub knockback_scaling: f32,
    pub charge_rate: f32,
    pub move_speed: f32,
}

pub struct BowRepeaterModifiers {
    pub power: f32,
    pub energy_cost: f32,
    pub max_speed: f32,
}

pub struct BowShotgunModifiers {
    pub power: f32,
    pub energy_cost: f32,
    pub num_projectiles: u32,
    pub spread: f32,
}

impl BowTreeModifiers {
    const fn get() -> Self {
        Self {
            universal: BowUniversalModifiers {
                projectile_speed: 1.1,
            },
            charged: BowChargedModifiers {
                damage_scaling: 1.1,
                regen_scaling: 1.1,
                knockback_scaling: 1.1,
                charge_rate: 1.1,
                move_speed: 1.1,
            },
            repeater: BowRepeaterModifiers {
                power: 1.1,
                energy_cost: 0.9,
                max_speed: 1.2,
            },
            shotgun: BowShotgunModifiers {
                power: 1.1,
                energy_cost: 0.9,
                num_projectiles: 1,
                spread: 0.9,
            },
        }
    }
}

pub struct StaffTreeModifiers {
    pub fireball: StaffFireballModifiers,
    pub flamethrower: StaffFlamethrowerModifiers,
    pub shockwave: StaffShockwaveModifiers,
}

pub struct StaffFireballModifiers {
    pub power: f32,
    pub regen: f32,
    pub range: f32,
}

pub struct StaffFlamethrowerModifiers {
    pub damage: f32,
    pub range: f32,
    pub energy_drain: f32,
    pub velocity: f32,
}

pub struct StaffShockwaveModifiers {
    pub damage: f32,
    pub knockback: f32,
    pub duration: f32,
    pub energy_cost: f32,
}

impl StaffTreeModifiers {
    const fn get() -> Self {
        Self {
            fireball: StaffFireballModifiers {
                power: 1.1,
                regen: 1.1,
                range: 1.1,
            },
            flamethrower: StaffFlamethrowerModifiers {
                damage: 1.2,
                range: 1.1,
                energy_drain: 0.9,
                velocity: 1.1,
            },
            shockwave: StaffShockwaveModifiers {
                damage: 1.15,
                knockback: 1.15,
                duration: 1.1,
                energy_cost: 0.9,
            },
        }
    }
}

pub struct SceptreTreeModifiers {
    pub beam: SceptreBeamModifiers,
    pub healing_aura: SceptreHealingAuraModifiers,
    pub warding_aura: SceptreWardingAuraModifiers,
}

pub struct SceptreBeamModifiers {
    pub damage: f32,
    pub range: f32,
    pub energy_regen: f32,
    pub lifesteal: f32,
}

pub struct SceptreHealingAuraModifiers {
    pub strength: f32,
    pub duration: f32,
    pub range: f32,
    pub energy_cost: f32,
}

pub struct SceptreWardingAuraModifiers {
    pub strength: f32,
    pub duration: f32,
    pub range: f32,
    pub energy_cost: f32,
}

impl SceptreTreeModifiers {
    const fn get() -> Self {
        Self {
            beam: SceptreBeamModifiers {
                damage: 1.1,
                range: 1.1,
                energy_regen: 1.1,
                lifesteal: 1.05,
            },
            healing_aura: SceptreHealingAuraModifiers {
                strength: 1.05,
                duration: 1.1,
                range: 1.1,
                energy_cost: 0.90,
            },
            warding_aura: SceptreWardingAuraModifiers {
                strength: 1.05,
                duration: 1.1,
                range: 1.1,
                energy_cost: 0.95,
            },
        }
    }
}

pub struct MiningTreeModifiers {
    pub speed: f32,
    pub gem_gain: f32,
    pub ore_gain: f32,
}

impl MiningTreeModifiers {
    const fn get() -> Self {
        Self {
            speed: 1.1,
            gem_gain: 0.05,
            ore_gain: 0.05,
        }
    }
}

pub struct GeneralTreeModifiers {
    pub roll: RollTreeModifiers,
    pub swim: SwimTreeModifiers,
    pub climb: ClimbTreeModifiers,
}

pub struct RollTreeModifiers {
    pub energy_cost: f32,
    pub strength: f32,
    pub duration: f32,
}

pub struct SwimTreeModifiers {
    pub speed: f32,
}

pub struct ClimbTreeModifiers {
    pub energy_cost: f32,
    pub speed: f32,
}

impl GeneralTreeModifiers {
    const fn get() -> Self {
        Self {
            roll: RollTreeModifiers {
                energy_cost: 0.95,
                strength: 1.05,
                duration: 1.05,
            },
            swim: SwimTreeModifiers { speed: 1.25 },
            climb: ClimbTreeModifiers {
                energy_cost: 0.8,
                speed: 1.2,
            },
        }
    }
}
