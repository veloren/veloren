use crate::{
    combat::{
        AttackEffect, AttackSource, AttackedModification, AttackedModifier, CombatBuff,
        CombatBuffStrength, CombatEffect, CombatModification, CombatRequirement, Knockback,
        KnockbackDir, ScalingKind, StatEffect, StatEffectTarget,
    },
    comp::{Mass, Stats, aura::AuraKey, tool::ToolKind},
    link::DynWeakLinkHandle,
    match_some,
    resources::{Secs, Time},
    uid::Uid,
};

use core::cmp::Ordering;
use enum_map::{Enum, EnumMap};
use itertools::Either;
use serde::{Deserialize, Serialize};
use slotmap::{SlotMap, new_key_type};
use specs::{Component, DerefFlaggedStorage, VecStorage};
use strum::EnumIter;

use super::Body;

new_key_type! { pub struct BuffKey; }

/// De/buff Kind.
/// This is used to determine what effects a buff will have
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord, EnumIter, Enum,
)]
pub enum BuffKind {
    // =================
    //       BUFFS
    // =================
    /// Restores health/time for some period.
    /// Strength should be the healing per second.
    Regeneration,
    /// Restores health/time for some period for consumables.
    /// Strength should be the healing per second.
    Saturation,
    /// Applied when drinking a potion.
    /// Strength should be the healing per second.
    Potion,
    /// Increases movement speed and vulnerability to damage as well as
    /// decreases the amount of damage dealt.
    /// Movement speed increases linearly with strength 1.0 is an 100% increase
    /// Damage vulnerability and damage reduction are both hard set to 100%
    Agility,
    /// Applied when resting (sitting at campfire or sleeping).
    /// Strength is fraction of health restored per second.
    RestingHeal,
    /// Restores energy/time for some period.
    /// Strength should be the energy regenerated per second.
    EnergyRegen,
    /// Generates combo over time for some period.
    /// Strength should be the combo generated per second.
    ComboGeneration,
    /// Raises maximum energy.
    /// Strength should be 10x the effect to max energy.
    IncreaseMaxEnergy,
    /// Raises maximum health.
    /// Strength should be the effect to max health.
    IncreaseMaxHealth,
    /// Makes you immune to attacks.
    /// Strength does not affect this buff.
    Invulnerability,
    /// Reduces incoming damage.
    /// Strength scales the damage reduction non-linearly. 0.5 provides 50% DR,
    /// 1.0 provides 67% DR.
    ProtectingWard,
    /// Increases movement speed and gives health regeneration.
    /// Strength scales the movement speed linearly. 0.5 is 150% speed, 1.0 is
    /// 200% speed. Provides regeneration at 10x the value of the strength.
    Frenzied,
    /// Increases movement and attack speed Strength scales strength of both
    /// effects linearly. 0.5 is a 50% increase, 1.0 is a 100% increase.
    Hastened,
    /// Increases resistance to incoming poise, and poise damage dealt as health
    /// is lost.
    /// Strength scales the resistance non-linearly. 0.5 provides 50%, 1.0
    /// provides 67%.
    /// Strength scales the poise damage increase linearly, a strength of 1.0
    /// and n health less from maximum health will cause poise damage to
    /// increase by n%.
    Fortitude,
    /// Increases both attack damage and vulnerability to damage.
    /// Damage increases linearly with strength, 1.0 is a 100% increase.
    /// Damage reduction decreases linearly with strength, 1.0 is a 100%
    /// decrease.
    Reckless,
    /// Provides immunity to burning and increases movement speed in lava.
    /// Movement speed increases linearly with strength, 1.0 is a 100% increase.
    // SalamanderAspect, TODO: Readd in second dwarven mine MR
    /// Your attacks cause targets to receive the burning debuff
    /// Strength of burning debuff is a fraction of the damage, fraction
    /// increases linearly with strength
    Flame,
    /// Your attacks cause targets to receive the frozen debuff
    /// Strength of frozen debuff is equal to the strength of this buff
    Frigid,
    /// Your attacks have lifesteal
    /// Strength increases the fraction of damage restored as life
    Lifesteal,
    /// Your attacks against bleeding targets have lifesteal
    /// Strength increases the fraction of damage restored as life
    Bloodfeast,
    /// Guarantees that the next attack is a precise hit. Does this kind of
    /// hackily by adding 100% to the precision, will need to be adjusted if we
    /// ever allow double precision hits instead of treating 100 as a
    /// ceiling.
    ImminentCritical,
    /// Increases combo gain, every 1 strength increases combo per strike by 1,
    /// rounds to nearest integer
    Fury,
    /// Allows attacks to ignore DR and increases energy reward
    /// DR penetration is non-linear, 0.5 is 50% penetration and 1.0 is a 67%
    /// penetration. Energy reward is increased linearly to strength, 1.0 is a
    /// 150 % increase.
    Sunderer,
    /// Generates combo when damaged and decreases movement speed.
    /// Movement speed is decreased to 50%. Combo generation is linear with
    /// strength, 1.0 is 5 combo generated on being hit.
    Defiance,
    /// Increases both attack damage, vulnerability to damage, attack speed, and
    /// movement speed Damage increases linearly with strength, 1.0 is a
    /// 100% increase. Damage reduction decreases linearly with strength,
    /// 1.0 is a 100% Attack speed increases non-linearly with strength, 0.5
    /// is a 25% increase, 1.0 is a 33% increase Movement speed increases
    /// non-linearly with strength, 0.5 is a 12.5% increase, 1.0 is a 16.7%
    /// increase decrease.
    Berserk,
    /// Increases poise resistance and energy reward. However if killed, buffs
    /// killer with Reckless buff. Poise resistance scales non-linearly with
    /// strength, 0.5 is 50% and 1.0 is 67%. Energy reward scales linearly with
    /// strength, 0.5 is +50% and 1.0 is +100% strength. Reckless buff reward
    /// strength is equal to scornful taunt buff strength.
    ScornfulTaunt,
    /// Increases damage resistance, causes energy to be generated when damaged,
    /// and decreases movement speed. Damage resistance increases non-linearly
    /// with strength, 0.5 is 25% and 1.0 is 34%. Energy generation is linear
    /// with strength, 1.0 is 10 energy per hit. Movement speed is decreased to
    /// 70%.
    Tenacity,
    /// Applies to some debuffs that have strong CC effects. Automatically
    /// gained upon receiving those debuffs, and causes future instances of
    /// those debuffs to be applied with reduced duration.
    /// Strength linearly decreases the duration of newly applied, affected
    /// debuffs, 0.5 is a 50% reduction.
    Resilience,
    /// Causes the next attack to have precision of 1.0 if the target is not
    /// wielding their weapon, and also generally increases damage.
    /// Strength linearly increases the damage increase.
    OwlTalon,
    /// Causes the next projectile fired to have more knockback and poise
    /// damage.
    /// Strength linearly increases the knockback and poise damage applied to
    /// the next projectile.
    HeavyNock,
    /// Causes the next projectile to both gain precision and restore more
    /// energy.
    /// Strength linearly increases the precision override and energy restored.
    Heartseeker,
    /// Causes projectile attacks to have more precision power, and to guarantee
    /// a minimum precision multiplier.
    /// Strength linearly increases both. The minimum precision power is
    /// equivalent to the buff strength, and the additional precision power is
    /// 50% of the buff strength.
    EagleEye,
    /// Causes the next projectile fired to debuff the target with ArdentHunted.
    /// Projectiles fired at the target generate additional combo, and
    /// increase energy reward by a percentage.
    /// Strength linearly increases the amount of additional combo generated and
    /// the additional energy reward.
    ArdentHunter,
    /// Causes the next projectile fired to do additional damage for every
    /// debuff the target has that had been inflicted by the attacker when using
    /// a bow.
    /// Strength linearly increases the amount of additional damage.
    SepticShot,
    // =================
    //      DEBUFFS
    // =================
    /// Does damage to a creature over time.
    /// Strength should be the DPS of the debuff.
    /// Provides immunity against Frozen.
    Burning,
    /// Lowers health over time for some duration.
    /// Strength should be the DPS of the debuff.
    Bleeding,
    /// Lower a creature's max health over time.
    /// Strength only affects the target max health, 0.5 targets 50% of base
    /// max, 1.0 targets 100% of base max.
    Cursed,
    /// Reduces movement speed and causes bleeding damage.
    /// Strength scales the movement speed debuff non-linearly. 0.5 is 50%
    /// speed, 1.0 is 33% speed. Bleeding is at 4x the value of the strength.
    Crippled,
    /// Slows movement and attack speed and increases poise damage received.
    /// Strength scales the attack speed debuff non-linearly. 0.5 is ~50%
    /// speed, 1.0 is 33% speed. Movement speed debuff is scaled to be slightly
    /// smaller than attack speed debuff. Received poise damage scales linearly,
    /// 1.0 is a 100% increase.
    /// Provides immunity against Heatstroke and Chilled.
    Frozen,
    /// Makes you wet and causes you to have reduced friction on the ground.
    /// Strength scales the friction you ignore non-linearly. 0.5 is 50% ground
    /// friction, 1.0 is 33% ground friction.
    /// Provides immunity against Burning.
    Wet,
    /// Makes you move slower.
    /// Strength scales the movement speed debuff non-linearly. 0.5 is 50%
    /// speed, 1.0 is 33% speed.
    Ensnared,
    /// Drain stamina to a creature over time.
    /// Strength should be the energy per second of the debuff.
    Poisoned,
    /// Results from having an attack parried.
    /// Causes your attack speed to be slower to emulate the recover duration of
    /// an ability being lengthened.
    Parried,
    /// Results from drinking a potion.
    /// Decreases the health gained from subsequent potions.
    PotionSickness,
    /// Slows movement speed and reduces energy reward.
    /// Both scales non-linearly to strength, 0.5 lead to movespeed reduction
    /// by 25% and energy reward reduced by 150%, 1.0 lead to MS reduction by
    /// 33.3% and energy reward reduced by 200%. Energy reward can't be
    /// reduced by more than 200%, to a minimum value of -100%.
    Heatstroke,
    /// Reduces movement speed to 0.
    /// Strength increases the relative mass of the creature that can be
    /// targeted. A strength of 1.0 means that a creature of the same mass gets
    /// rooted for the full duration. A strength of 2.0 means a creature of
    /// twice the mass gets rooted for the full duration. If the target's mass
    /// is higher than the strength allows for, duration gets reduced using a
    /// mutiplier from the ratio of masses.
    Rooted,
    /// Slows movement speed and reduces energy reward
    /// Both scale non-linearly with strength, 0.5 leads to 50% reduction of
    /// energy reward and 33% reduction of move speed. 1.0 leads to 67%
    /// reduction of energy reward and 50% reduction of move speed.
    Winded,
    /// Prevents use of auxiliary abilities.
    /// Does not scale with strength
    Amnesia,
    /// Increases amount of poise damage received
    /// Scales linearly with strength, 1.0 leads to 100% more poise damage
    /// received
    OffBalance,
    /// Decreases movement speed and increases amount of poise damage received.
    /// Movement speed decreases non-linearly with strength, 0.5 leads to a 25%
    /// reduction, 1.0 leads to a 33% reduction. Poise damage received scales
    /// linearly with strength, 1.0 leads to 100% more poise damage.
    /// Provides immunity to Heatstroke.
    Chilled,
    /// Increases combo generation and energy reward when hit with projectiles.
    /// Strength linearly increases the amount of additional combo generated and
    /// the additional energy reward.
    ArdentHunted,
    // =================
    //      COMPLEX
    // =================
    /// Changed into another body.
    Polymorphed,
}

/// Tells a little more about the buff kind than simple buff/debuff
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuffDescriptor {
    /// Simple positive buffs, like `BuffKind::Saturation`
    SimplePositive,
    /// Simple negative buffs, like `BuffKind::Bleeding`
    SimpleNegative,
    /// Buffs that require unusual data that can't be governed just by strength
    /// and duration, like `BuffKind::Polymorhped`
    Complex,
    // For future additions, we may want to tell about non-obvious buffs,
    // like Agility.
    // Also maybe extend Complex to differentiate between Positive, Negative
    // and Neutral buffs?
    // For now, Complex is assumed to be neutral/non-obvious.
}

impl BuffKind {
    /// Tells a little more about buff kind than simple buff/debuff
    ///
    /// Read more in [BuffDescriptor].
    pub fn differentiate(self) -> BuffDescriptor {
        match self {
            BuffKind::Regeneration
            | BuffKind::Saturation
            | BuffKind::Potion
            | BuffKind::Agility
            | BuffKind::RestingHeal
            | BuffKind::Frenzied
            | BuffKind::EnergyRegen
            | BuffKind::ComboGeneration
            | BuffKind::IncreaseMaxEnergy
            | BuffKind::IncreaseMaxHealth
            | BuffKind::Invulnerability
            | BuffKind::ProtectingWard
            | BuffKind::Hastened
            | BuffKind::Fortitude
            | BuffKind::Reckless
            | BuffKind::Flame
            | BuffKind::Frigid
            | BuffKind::Lifesteal
            //| BuffKind::SalamanderAspect
            | BuffKind::ImminentCritical
            | BuffKind::Fury
            | BuffKind::Sunderer
            | BuffKind::Defiance
            | BuffKind::Bloodfeast
            | BuffKind::Berserk
            | BuffKind::ScornfulTaunt
            | BuffKind::Tenacity
            | BuffKind::Resilience
            | BuffKind::OwlTalon
            | BuffKind::HeavyNock
            | BuffKind::Heartseeker
            | BuffKind::EagleEye
            | BuffKind::ArdentHunter
            | BuffKind::SepticShot => BuffDescriptor::SimplePositive,
            BuffKind::Bleeding
            | BuffKind::Cursed
            | BuffKind::Burning
            | BuffKind::Crippled
            | BuffKind::Frozen
            | BuffKind::Wet
            | BuffKind::Ensnared
            | BuffKind::Poisoned
            | BuffKind::Parried
            | BuffKind::PotionSickness
            | BuffKind::Heatstroke
            | BuffKind::Rooted
            | BuffKind::Winded
            | BuffKind::Amnesia
            | BuffKind::OffBalance
            | BuffKind::Chilled
            | BuffKind::ArdentHunted => BuffDescriptor::SimpleNegative,
            BuffKind::Polymorphed => BuffDescriptor::Complex,
        }
    }

    /// Checks if buff is buff or debuff.
    pub fn is_buff(self) -> bool {
        match self.differentiate() {
            BuffDescriptor::SimplePositive => true,
            BuffDescriptor::SimpleNegative | BuffDescriptor::Complex => false,
        }
    }

    pub fn is_simple(self) -> bool {
        match self.differentiate() {
            BuffDescriptor::SimplePositive | BuffDescriptor::SimpleNegative => true,
            BuffDescriptor::Complex => false,
        }
    }

    /// Checks if buff should queue.
    pub fn queues(self) -> bool { matches!(self, BuffKind::Saturation) }

    /// Checks if the buff can affect other buff effects applied in the same
    /// tick.
    pub fn affects_subsequent_buffs(self) -> bool {
        matches!(
            self,
            BuffKind::PotionSickness /* | BuffKind::SalamanderAspect */
        )
    }

    /// Checks if multiple instances of the buff should be processed, instead of
    /// only the strongest.
    pub fn stacks(self) -> bool { matches!(self, BuffKind::PotionSickness | BuffKind::Resilience) }

    pub fn effects(&self, data: &BuffData, source_entity: Option<Uid>) -> Vec<BuffEffect> {
        // Normalized nonlinear scaling
        // TODO: Do we want to make denominator term parameterized. Come back to if we
        // add nn_scaling3.
        let nn_scaling = |a: f32| a.abs() / (a.abs() + 0.5) * a.signum();
        let nn_scaling2 = |a: f32| a.abs() / (a.abs() + 1.0) * a.signum();
        let instance = rand::random();
        match self {
            BuffKind::Bleeding => vec![BuffEffect::HealthChangeOverTime {
                rate: -data.strength,
                kind: ModifierKind::Additive,
                instance,
                tick_dur: Secs(0.5),
            }],
            BuffKind::Regeneration => vec![BuffEffect::HealthChangeOverTime {
                rate: data.strength,
                kind: ModifierKind::Additive,
                instance,
                tick_dur: Secs(1.0),
            }],
            BuffKind::Saturation => vec![BuffEffect::HealthChangeOverTime {
                rate: data.strength,
                kind: ModifierKind::Additive,
                instance,
                tick_dur: Secs(3.0),
            }],
            BuffKind::Potion => {
                vec![BuffEffect::HealthChangeOverTime {
                    rate: data.strength,
                    kind: ModifierKind::Additive,
                    instance,
                    tick_dur: Secs(0.1),
                }]
            },
            BuffKind::Agility => vec![
                BuffEffect::MovementSpeed(1.0 + data.strength),
                BuffEffect::DamageReduction(-1.0),
                BuffEffect::AttackDamage(0.0),
            ],
            BuffKind::RestingHeal => vec![BuffEffect::HealthChangeOverTime {
                rate: data.strength,
                kind: ModifierKind::Multiplicative,
                instance,
                tick_dur: Secs(2.0),
            }],
            BuffKind::Cursed => vec![
                BuffEffect::MaxHealthChangeOverTime {
                    rate: -1.0,
                    kind: ModifierKind::Additive,
                    target_fraction: 1.0 - data.strength,
                },
                BuffEffect::HealthChangeOverTime {
                    rate: -1.0,
                    kind: ModifierKind::Additive,
                    instance,
                    tick_dur: Secs(0.5),
                },
            ],
            BuffKind::EnergyRegen => vec![BuffEffect::EnergyChangeOverTime {
                rate: data.strength,
                kind: ModifierKind::Additive,
                tick_dur: Secs(0.25),
                reset_rate_on_tick: false,
            }],
            BuffKind::ComboGeneration => {
                let target_tick_dur = 0.25;
                // Combo per tick must be an integer
                let nearest_valid_tick_dur =
                    (data.strength as f64 * target_tick_dur).round() / data.strength as f64;

                vec![BuffEffect::ComboChangeOverTime {
                    rate: data.strength,
                    tick_dur: Secs(nearest_valid_tick_dur),
                }]
            },
            BuffKind::IncreaseMaxEnergy => vec![BuffEffect::MaxEnergyModifier {
                value: data.strength,
                kind: ModifierKind::Additive,
            }],
            BuffKind::IncreaseMaxHealth => vec![BuffEffect::MaxHealthModifier {
                value: data.strength,
                kind: ModifierKind::Additive,
            }],
            BuffKind::Invulnerability => vec![BuffEffect::DamageReduction(1.0)],
            BuffKind::ProtectingWard => vec![BuffEffect::DamageReduction(
                // Causes non-linearity in effect strength, but necessary
                // to allow for tool power and other things to affect the
                // strength. 0.5 also still provides 50% damage reduction.
                nn_scaling(data.strength),
            )],
            BuffKind::Burning => vec![
                BuffEffect::HealthChangeOverTime {
                    rate: -data.strength,
                    kind: ModifierKind::Additive,
                    instance,
                    tick_dur: Secs(0.25),
                },
                BuffEffect::BuffImmunity(BuffKind::Frozen),
            ],
            BuffKind::Poisoned => vec![BuffEffect::EnergyChangeOverTime {
                rate: -data.strength,
                kind: ModifierKind::Additive,
                tick_dur: Secs(0.5),
                reset_rate_on_tick: true,
            }],
            BuffKind::Crippled => vec![
                BuffEffect::MovementSpeed(1.0 - nn_scaling(data.strength)),
                BuffEffect::HealthChangeOverTime {
                    rate: -data.strength * 4.0,
                    kind: ModifierKind::Additive,
                    instance,
                    tick_dur: Secs(0.5),
                },
            ],
            BuffKind::Frenzied => vec![
                BuffEffect::MovementSpeed(1.0 + data.strength),
                BuffEffect::HealthChangeOverTime {
                    rate: data.strength * 10.0,
                    kind: ModifierKind::Additive,
                    instance,
                    tick_dur: Secs(1.0),
                },
            ],
            BuffKind::Frozen => vec![
                BuffEffect::MovementSpeed(f32::powf(1.0 - nn_scaling(data.strength), 1.1)),
                BuffEffect::AttackSpeed(1.0 - nn_scaling(data.strength)),
                BuffEffect::PoiseReduction(-data.strength),
                BuffEffect::BuffImmunity(BuffKind::Heatstroke),
                BuffEffect::BuffImmunity(BuffKind::Chilled),
            ],
            BuffKind::Chilled => vec![
                BuffEffect::MovementSpeed(1.0 - 0.5 * nn_scaling(data.strength)),
                BuffEffect::PoiseReduction(-data.strength),
                BuffEffect::BuffImmunity(BuffKind::Heatstroke),
            ],
            BuffKind::Wet => vec![
                BuffEffect::GroundFriction(1.0 - nn_scaling(data.strength)),
                BuffEffect::BuffImmunity(BuffKind::Burning),
            ],
            BuffKind::Ensnared => vec![BuffEffect::MovementSpeed(1.0 - nn_scaling(data.strength))],
            BuffKind::Hastened => vec![
                BuffEffect::MovementSpeed(1.0 + data.strength),
                BuffEffect::AttackSpeed(1.0 + data.strength),
            ],
            BuffKind::Fortitude => vec![
                BuffEffect::PoiseReduction(nn_scaling(data.strength)),
                BuffEffect::PoiseDamageFromLostHealth(data.strength),
            ],
            BuffKind::Parried => vec![BuffEffect::PrecisionVulnerabilityOverride(0.75)],
            BuffKind::PotionSickness => vec![BuffEffect::ItemEffectReduction(data.strength)],
            BuffKind::Reckless => vec![
                BuffEffect::DamageReduction(-data.strength),
                BuffEffect::AttackDamage(1.0 + data.strength),
            ],
            BuffKind::Polymorphed => {
                let mut effects = Vec::new();
                if let Some(MiscBuffData::Body(body)) = data.misc_data {
                    effects.push(BuffEffect::BodyChange(body));
                }
                effects
            },
            BuffKind::Flame => vec![BuffEffect::AttackEffect(AttackEffect::new(
                None,
                CombatEffect::Buff(CombatBuff {
                    kind: BuffKind::Burning,
                    dur_secs: data.secondary_duration.unwrap_or(Secs(5.0)),
                    strength: CombatBuffStrength::DamageFraction(data.strength),
                    chance: 1.0,
                }),
            ))],
            BuffKind::Frigid => vec![BuffEffect::AttackEffect(AttackEffect::new(
                None,
                CombatEffect::Buff(CombatBuff {
                    kind: BuffKind::Frozen,
                    dur_secs: data.secondary_duration.unwrap_or(Secs(5.0)),
                    strength: CombatBuffStrength::Value(data.strength),
                    chance: 1.0,
                }),
            ))],
            BuffKind::Lifesteal => vec![BuffEffect::AttackEffect(AttackEffect::new(
                None,
                CombatEffect::Lifesteal(data.strength),
            ))],
            /*BuffKind::SalamanderAspect => vec![
                BuffEffect::BuffImmunity(BuffKind::Burning),
                BuffEffect::SwimSpeed(1.0 + data.strength),
            ],*/
            BuffKind::Bloodfeast => vec![BuffEffect::AttackEffect(
                AttackEffect::new(None, CombatEffect::Lifesteal(data.strength))
                    .with_requirement(CombatRequirement::TargetHasBuff(BuffKind::Bleeding)),
            )],
            BuffKind::ImminentCritical => vec![BuffEffect::PrecisionModifier(None, 1.0, false)],
            BuffKind::Fury => vec![BuffEffect::AttackEffect(
                AttackEffect::new(None, CombatEffect::Combo(data.strength.round() as i32))
                    .with_requirement(CombatRequirement::AnyDamage),
            )],
            BuffKind::Sunderer => vec![
                BuffEffect::MitigationsPenetration(nn_scaling(data.strength)),
                BuffEffect::EnergyReward(1.0 + 1.5 * data.strength),
            ],
            BuffKind::Defiance => vec![
                BuffEffect::MovementSpeed(0.5),
                BuffEffect::DamagedEffect(StatEffect::new(
                    StatEffectTarget::Target,
                    CombatEffect::Combo((data.strength * 5.0).round() as i32),
                )),
            ],
            BuffKind::Berserk => vec![
                BuffEffect::DamageReduction(-data.strength),
                BuffEffect::AttackDamage(1.0 + data.strength),
                BuffEffect::AttackSpeed(1.0 + nn_scaling(data.strength) / 2.0),
                BuffEffect::MovementSpeed(1.0 + nn_scaling(data.strength) / 4.0),
            ],
            BuffKind::Heatstroke => vec![
                BuffEffect::MovementSpeed(1.0 - nn_scaling(data.strength) * 0.5),
                BuffEffect::EnergyReward((1.0 - nn_scaling(data.strength) * 3.0).max(-1.0)),
            ],
            BuffKind::ScornfulTaunt => vec![
                BuffEffect::PoiseReduction(nn_scaling(data.strength)),
                BuffEffect::EnergyReward(1.0 + data.strength),
                BuffEffect::DeathEffect(StatEffect::new(
                    StatEffectTarget::Attacker,
                    CombatEffect::Buff(CombatBuff {
                        kind: BuffKind::Reckless,
                        dur_secs: data.duration.unwrap_or(Secs(10.0)),
                        strength: CombatBuffStrength::Value(data.strength),
                        chance: 1.0,
                    }),
                )),
            ],
            BuffKind::Rooted => vec![BuffEffect::MovementSpeed(0.0)],
            BuffKind::Winded => vec![
                BuffEffect::MovementSpeed(1.0 - nn_scaling2(data.strength)),
                BuffEffect::EnergyReward(1.0 - nn_scaling(data.strength)),
            ],
            BuffKind::Amnesia => vec![BuffEffect::DisableAuxiliaryAbilities],
            BuffKind::OffBalance => vec![BuffEffect::PoiseReduction(-data.strength)],
            BuffKind::Tenacity => vec![
                BuffEffect::DamageReduction(nn_scaling(data.strength) / 2.0),
                BuffEffect::MovementSpeed(0.7),
                BuffEffect::DamagedEffect(StatEffect::new(
                    StatEffectTarget::Target,
                    CombatEffect::Energy(data.strength * 10.0),
                )),
            ],
            BuffKind::Resilience => vec![BuffEffect::CrowdControlResistance(data.strength)],
            BuffKind::OwlTalon => vec![
                BuffEffect::PrecisionModifier(Some(CombatRequirement::TargetUnwielded), 0.8, false),
                BuffEffect::AttackDamage(1.0 + data.strength),
            ],
            BuffKind::HeavyNock => {
                let range_mod = CombatModification::RangeWeakening {
                    start_dist: 5.0,
                    end_dist: 50.0,
                    min_str: 0.3,
                };
                let knockback = AttackEffect::new(
                    None,
                    CombatEffect::Knockback(Knockback {
                        direction: KnockbackDir::Away,
                        strength: 20.0 * data.strength,
                    }),
                )
                .with_requirement(CombatRequirement::AnyDamage)
                .with_requirement(CombatRequirement::AttackSource(AttackSource::Projectile))
                .with_modification(range_mod);
                let poise = AttackEffect::new(None, CombatEffect::Poise(35.0 * data.strength))
                    .with_requirement(CombatRequirement::AnyDamage)
                    .with_requirement(CombatRequirement::AttackSource(AttackSource::Projectile))
                    .with_modification(range_mod);
                vec![
                    BuffEffect::AttackEffect(knockback),
                    BuffEffect::AttackEffect(poise),
                    // BuffEffect::AttackDamage(0.5 * data.strength), // TODO: has no effect on
                    // damage?
                ]
            },
            BuffKind::Heartseeker => {
                let energy =
                    AttackEffect::new(None, CombatEffect::EnergyReward(30.0 * data.strength))
                        .with_requirement(CombatRequirement::AnyDamage)
                        .with_requirement(CombatRequirement::AttackSource(
                            AttackSource::Projectile,
                        ));
                vec![
                    BuffEffect::PrecisionModifier(
                        Some(CombatRequirement::AttackSource(AttackSource::Projectile)),
                        data.strength * 0.4,
                        false,
                    ),
                    BuffEffect::AttackEffect(energy),
                ]
            },
            BuffKind::EagleEye => {
                vec![
                    BuffEffect::PrecisionModifier(
                        Some(CombatRequirement::AttackSource(AttackSource::Projectile)),
                        data.strength,
                        false,
                    ),
                    BuffEffect::PrecisionPowerMult(1.0 + data.strength * 0.5),
                ]
            },
            BuffKind::ArdentHunter => vec![BuffEffect::AttackEffect(
                AttackEffect::new(
                    None,
                    CombatEffect::Buff(CombatBuff {
                        kind: BuffKind::ArdentHunted,
                        dur_secs: data.secondary_duration.unwrap_or(Secs(60.0)),
                        strength: CombatBuffStrength::Value(data.strength),
                        chance: 1.0,
                    }),
                )
                .with_requirement(CombatRequirement::AttackSource(AttackSource::Projectile)),
            )],
            BuffKind::ArdentHunted => {
                let projectile_req = CombatRequirement::AttackSource(AttackSource::Projectile);
                let mut energy_reward_effect =
                    AttackedModification::new(AttackedModifier::EnergyReward(data.strength))
                        .with_requirement(projectile_req);
                let mut damage_mult_effect =
                    AttackedModification::new(AttackedModifier::DamageMultiplier(data.strength))
                        .with_requirement(projectile_req);
                if let Some(uid) = source_entity {
                    let attacker_req = CombatRequirement::Attacker(uid);
                    energy_reward_effect = energy_reward_effect.with_requirement(attacker_req);
                    damage_mult_effect = damage_mult_effect.with_requirement(attacker_req);
                }
                vec![
                    BuffEffect::AttackedModification(energy_reward_effect),
                    BuffEffect::AttackedModification(damage_mult_effect),
                ]
            },
            BuffKind::SepticShot => vec![BuffEffect::AttackEffect(
                AttackEffect::new(None, CombatEffect::DebuffsVulnerable {
                    mult: data.strength,
                    scaling: ScalingKind::Sqrt,
                    filter_attacker: true,
                    filter_weapon: Some(ToolKind::Bow),
                })
                .with_requirement(CombatRequirement::AttackSource(AttackSource::Projectile)),
            )],
        }
    }

    fn extend_cat_ids(&self, mut cat_ids: Vec<BuffCategory>) -> Vec<BuffCategory> {
        // TODO: Remove clippy allow after another buff needs this
        #[allow(clippy::single_match)]
        match self {
            BuffKind::PotionSickness => {
                cat_ids.push(BuffCategory::PersistOnDowned);
            },
            _ => {},
        }
        cat_ids
    }

    fn modify_data(
        &self,
        mut data: BuffData,
        source_mass: Option<&Mass>,
        dest_info: DestInfo,
        source: BuffSource,
    ) -> BuffData {
        // TODO: Remove clippy allow after another buff needs this
        #[expect(clippy::single_match)]
        match self {
            BuffKind::Rooted => {
                let source_mass = source_mass.map_or(50.0, |m| m.0);
                let dest_mass = dest_info.mass.map_or(50.0, |m| m.0);
                let low_clamp = (0.25 + data.strength * 0.25).clamp(0.0, 1.0);
                let high_clamp = (1.0 + data.strength * 0.5).max(1.0);
                let ratio = (source_mass / dest_mass).clamp(low_clamp, high_clamp);
                data.duration = data.duration.map(|dur| Secs(dur.0 * ratio as f64));
            },
            _ => {},
        }
        if self.resilience_ccr_strength(data).is_some() {
            let dur_mult = dest_info
                .stats
                .map_or(1.0, |s| (1.0 - s.crowd_control_resistance).max(0.0));
            data.duration = data.duration.map(|dur| dur * dur_mult as f64);
        }
        self.apply_item_effect_reduction(&mut data, source, dest_info);
        data
    }

    /// If a buff kind should also give resilience when applied, return the
    /// strength that resilience should have, otherwise return None
    pub fn resilience_ccr_strength(&self, data: BuffData) -> Option<f32> {
        match_some!(self,
            BuffKind::Amnesia => 0.3,
            BuffKind::Frozen => data.strength,
            BuffKind::Winded => data.strength / 3.0,
            BuffKind::Rooted => data.duration.map_or(0.1, |dur| dur.0 as f32 / 10.0),
        )
    }

    pub fn apply_item_effect_reduction(
        &self,
        data: &mut BuffData,
        source: BuffSource,
        dest_info: DestInfo,
    ) {
        if !matches!(source, BuffSource::Item) {
            return;
        }
        let item_effect_reduction = dest_info.stats.map_or(1.0, |s| s.item_effect_reduction);
        match self {
            BuffKind::Potion | BuffKind::Agility => {
                data.strength *= item_effect_reduction;
            },
            BuffKind::Burning | BuffKind::Frozen | BuffKind::Resilience => {
                data.duration = data.duration.map(|dur| dur * item_effect_reduction as f64);
            },
            _ => {},
        };
    }
}

// Struct used to store data relevant to a buff
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct BuffData {
    pub strength: f32,
    #[serde(default)]
    pub duration: Option<Secs>,
    #[serde(default)]
    pub delay: Option<Secs>,
    /// Used for buffs that have rider buffs (e.g. Flame, Frigid)
    #[serde(default)]
    pub secondary_duration: Option<Secs>,
    /// Used to add random data to buffs if needed (e.g. polymorphed)
    #[serde(default)]
    pub misc_data: Option<MiscBuffData>,
}

impl Default for BuffData {
    fn default() -> Self { Self::new(0.0, Some(Secs(0.0))) }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MiscBuffData {
    Body(Body),
}

impl BuffData {
    pub fn new(strength: f32, duration: Option<Secs>) -> Self {
        Self {
            strength,
            duration,
            delay: None,
            secondary_duration: None,
            misc_data: None,
        }
    }

    pub fn with_delay(mut self, delay: Secs) -> Self {
        self.delay = Some(delay);
        self
    }

    pub fn with_secondary_duration(mut self, sec_dur: Secs) -> Self {
        self.secondary_duration = Some(sec_dur);
        self
    }

    pub fn with_misc_data(mut self, misc_data: MiscBuffData) -> Self {
        self.misc_data = Some(misc_data);
        self
    }
}

/// De/buff category ID.
/// Similar to `BuffKind`, but to mark a category (for more generic usage, like
/// positive/negative buffs).
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum BuffCategory {
    Natural,
    Physical,
    Magical,
    Divine,
    PersistOnDowned,
    PersistOnDeath,
    FromActiveAura(Uid, AuraKey),
    FromLink(DynWeakLinkHandle),
    RemoveOnAttack,
    RemoveOnShoot,
    RemoveOnLoadoutChange,
    SelfBuff,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ModifierKind {
    Additive,
    Multiplicative,
}

/// Data indicating and configuring behaviour of a de/buff.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BuffEffect {
    /// Periodically damages or heals entity
    HealthChangeOverTime {
        rate: f32,
        kind: ModifierKind,
        instance: u64,
        tick_dur: Secs,
    },
    /// Periodically consume entity energy
    EnergyChangeOverTime {
        rate: f32,
        kind: ModifierKind,
        tick_dur: Secs,
        reset_rate_on_tick: bool,
    },
    /// Periodically change entity combo
    ComboChangeOverTime {
        rate: f32,
        tick_dur: Secs,
    },
    /// Changes maximum health by a certain amount
    MaxHealthModifier {
        value: f32,
        kind: ModifierKind,
    },
    /// Changes maximum energy by a certain amount
    MaxEnergyModifier {
        value: f32,
        kind: ModifierKind,
    },
    /// Reduces damage after armor is accounted for by this fraction
    DamageReduction(f32),
    /// Gradually changes an entities max health over time
    MaxHealthChangeOverTime {
        rate: f32,
        kind: ModifierKind,
        target_fraction: f32,
    },
    /// Modifies move speed of target
    MovementSpeed(f32),
    /// Modifies attack speed of target
    AttackSpeed(f32),
    /// Modifies recovery speed of target
    RecoverySpeed(f32),
    /// Modifies ground friction of target
    GroundFriction(f32),
    /// Reduces poise damage taken after armor is accounted for by this fraction
    PoiseReduction(f32),
    /// Increases poise damage dealt when health is lost
    PoiseDamageFromLostHealth(f32),
    /// Modifier to the amount of damage dealt with attacks
    AttackDamage(f32),
    /// Adds a precision modifier applied to an attack if the condition
    /// is met, also allows for the modifier to optionally override other
    /// precision bonuses
    PrecisionModifier(Option<CombatRequirement>, f32, bool),
    /// Overrides the precision multiplier applied to an incoming attack
    PrecisionVulnerabilityOverride(f32),
    /// Changes body.
    BodyChange(Body),
    BuffImmunity(BuffKind),
    SwimSpeed(f32),
    /// Add an attack effect to attacks made while buff is active
    AttackEffect(AttackEffect),
    /// Increases poise damage dealt by attacks
    AttackPoise(f32),
    /// Ignores some damage reduction on target
    MitigationsPenetration(f32),
    /// Modifies energy rewarded on successful strikes
    EnergyReward(f32),
    /// Add an effect to the entity when damaged by an attack
    DamagedEffect(StatEffect),
    /// Add an effect to the entity when killed
    DeathEffect(StatEffect),
    /// Prevents use of auxiliary abilities
    DisableAuxiliaryAbilities,
    /// Reduces duration of crowd control debuffs
    CrowdControlResistance(f32),
    /// Reduces the strength or duration of item buff
    ItemEffectReduction(f32),
    /// Adds an effect that modifies how attacks are applied to this entity
    AttackedModification(AttackedModification),
    /// Multiplies the precision damage applied to attacks made
    PrecisionPowerMult(f32),
}

/// Actual de/buff.
/// Buff can timeout after some time if `time` is Some. If `time` is None,
/// Buff will last indefinitely, until removed manually (by some action, like
/// uncursing).
///
/// Buff has a kind, which is used to determine the effects in a builder
/// function.
///
/// To provide more classification info when needed,
/// buff can be in one or more buff category.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Buff {
    pub kind: BuffKind,
    pub data: BuffData,
    pub cat_ids: Vec<BuffCategory>,
    pub end_time: Option<Time>,
    pub start_time: Time,
    pub effects: Vec<BuffEffect>,
    pub source: BuffSource,
}

/// Information about whether buff addition or removal was requested.
/// This to implement "on_add" and "on_remove" hooks for constant buffs.
#[derive(Clone, Debug)]
pub enum BuffChange {
    /// Adds this buff.
    Add(Buff),
    /// Removes all buffs with this ID.
    RemoveByKind(BuffKind),
    /// Removes all buffs with this ID, but not debuffs.
    RemoveFromController(BuffKind),
    /// Removes buffs of these indices, should only be called when buffs expire
    RemoveByKey(Vec<BuffKey>),
    /// Removes buffs of these categories (first vec is of categories of which
    /// all are required, second vec is of categories of which at least one is
    /// required, third vec is of categories that will not be removed)
    RemoveByCategory {
        all_required: Vec<BuffCategory>,
        any_required: Vec<BuffCategory>,
        none_required: Vec<BuffCategory>,
    },
    /// Refreshes durations of all buffs with this kind.
    Refresh(BuffKind),
}

impl Buff {
    /// Builder function for buffs
    pub fn new(
        kind: BuffKind,
        data: BuffData,
        cat_ids: Vec<BuffCategory>,
        source: BuffSource,
        time: Time,
        dest_info: DestInfo,
        // Create source_info if we need more parameters from source
        source_mass: Option<&Mass>,
    ) -> Self {
        let data = kind.modify_data(data, source_mass, dest_info, source);
        let source_uid = if let BuffSource::Character { by, .. } = source {
            Some(by)
        } else {
            None
        };
        let effects = kind.effects(&data, source_uid);
        let cat_ids = kind.extend_cat_ids(cat_ids);
        let start_time = Time(time.0 + data.delay.map_or(0.0, |delay| delay.0));
        let end_time = if cat_ids.iter().any(|cat_id| {
            matches!(
                cat_id,
                BuffCategory::FromActiveAura(..) | BuffCategory::FromLink(_)
            )
        }) {
            None
        } else {
            data.duration.map(|dur| Time(start_time.0 + dur.0))
        };
        Buff {
            kind,
            data,
            cat_ids,
            start_time,
            end_time,
            effects,
            source,
        }
    }

    /// Calculate how much time has elapsed since the buff was applied
    pub fn elapsed(&self, time: Time) -> Secs { Secs(time.0 - self.start_time.0) }
}

impl PartialOrd for Buff {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if self.data.strength > other.data.strength {
            Some(Ordering::Greater)
        } else if self.data.strength < other.data.strength {
            Some(Ordering::Less)
        } else if self.data.delay.is_none() && other.data.delay.is_some() {
            Some(Ordering::Greater)
        } else if self.data.delay.is_some() && other.data.delay.is_none() {
            Some(Ordering::Less)
        } else if compare_end_time(self.end_time, other.end_time) {
            Some(Ordering::Greater)
        } else if compare_end_time(other.end_time, self.end_time) {
            Some(Ordering::Less)
        } else {
            None
        }
    }
}

fn compare_end_time(a: Option<Time>, b: Option<Time>) -> bool {
    a.is_none_or(|time_a| b.is_some_and(|time_b| time_a.0 > time_b.0))
}

impl PartialEq for Buff {
    fn eq(&self, other: &Self) -> bool {
        self.data.strength == other.data.strength
            && self.end_time == other.end_time
            && self.start_time == other.start_time
    }
}

/// Source of the de/buff
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BuffSource {
    /// Applied by a character
    Character {
        by: Uid,
        tool_kind: Option<ToolKind>,
    },
    /// Applied by world, like a poisonous fumes from a swamp
    World,
    /// Applied by command
    Command,
    /// Applied by an item
    Item,
    /// Applied by another buff (like an after-effect)
    Buff,
    /// Applied by a block
    Block,
    /// Some other source
    Unknown,
}

/// Component holding all de/buffs that gets resolved each tick.
/// On each tick, remaining time of buffs get lowered and
/// buff effect of each buff is applied or not, depending on the `BuffEffect`
/// (specs system will decide based on `BuffEffect`, to simplify
/// implementation). TODO: Something like `once` flag for `Buff` to remove the
/// dependence on `BuffEffect` enum?
///
/// In case of one-time buffs, buff effects will be applied on addition
/// and undone on removal of the buff (by the specs system).
/// Example could be decreasing max health, which, if repeated each tick,
/// would be probably an undesired effect).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Buffs {
    /// Maps kinds of buff to currently applied buffs of that kind and
    /// the time that the first buff was added (time gets reset if entity no
    /// longer has buffs of that kind)
    pub kinds: EnumMap<BuffKind, Option<(Vec<BuffKey>, Time)>>,
    // All buffs currently present on an entity
    pub buffs: SlotMap<BuffKey, Buff>,
}

impl Buffs {
    fn sort_kind(&mut self, kind: BuffKind) {
        if let Some(buff_order) = self.kinds[kind].as_mut() {
            if buff_order.0.is_empty() {
                self.kinds[kind] = None;
            } else {
                let buffs = &self.buffs;
                // Intentionally sorted in reverse so that the strongest buffs are earlier in
                // the vector
                buff_order
                    .0
                    .sort_by(|a, b| buffs[*b].partial_cmp(&buffs[*a]).unwrap_or(Ordering::Equal));
            }
        }
    }

    pub fn remove_kind(&mut self, kind: BuffKind) {
        if let Some((buff_keys, _)) = self.kinds[kind].as_ref() {
            for key in buff_keys {
                self.buffs.remove(*key);
            }
            self.kinds[kind] = None;
        }
    }

    pub fn insert(&mut self, buff: Buff, current_time: Time) -> BuffKey {
        let kind = buff.kind;
        // Try to find another overlaping non-queueable buff with same data, cat_ids and
        // source.
        let other_key = if kind.queues() {
            None
        } else {
            self.kinds[kind].as_ref().and_then(|(keys, _)| {
                keys.iter()
                    .find(|key| {
                        self.buffs.get(**key).is_some_and(|other_buff| {
                            other_buff.data == buff.data
                                && other_buff.cat_ids == buff.cat_ids
                                && other_buff.source == buff.source
                                && other_buff
                                    .end_time
                                    .is_none_or(|end_time| end_time.0 >= buff.start_time.0)
                        })
                    })
                    .copied()
            })
        };

        // If another buff with the same fields is found, update end_time and effects
        let key = if !kind.stacks()
            && let Some((other_buff, key)) =
                other_key.and_then(|key| Some((self.buffs.get_mut(key)?, key)))
        {
            other_buff.end_time = buff.end_time;
            other_buff.effects = buff.effects;
            key
        // Otherwise, insert a new buff
        } else {
            let key = self.buffs.insert(buff);
            self.kinds[kind]
                .get_or_insert_with(|| (Vec::new(), current_time))
                .0
                .push(key);
            key
        };

        self.sort_kind(kind);
        if kind.queues() {
            self.delay_queueable_buffs(kind, current_time);
        }
        key
    }

    pub fn contains(&self, kind: BuffKind) -> bool { self.kinds[kind].is_some() }

    pub fn contains_any(&self, kinds: &[BuffKind]) -> bool {
        kinds.iter().any(|kind| self.contains(*kind))
    }

    // Iterate through buffs of a given kind in effect order (most powerful first)
    pub fn iter_kind(&self, kind: BuffKind) -> impl Iterator<Item = (BuffKey, &Buff)> + '_ {
        self.kinds[kind]
            .as_ref()
            .map(|keys| keys.0.iter())
            .unwrap_or_else(|| [].iter())
            .map(move |&key| (key, &self.buffs[key]))
    }

    // Iterates through all active buffs (the most powerful buff of each
    // non-stacking kind, and all of the stacking ones)
    pub fn iter_active(&self) -> impl Iterator<Item = impl Iterator<Item = &Buff>> + '_ {
        self.kinds
            .iter()
            .filter_map(|(kind, keys)| keys.as_ref().map(|keys| (kind, keys)))
            .map(move |(kind, keys)| {
                if kind.stacks() {
                    // Iterate stackable buffs in reverse order to show the timer of the soonest one
                    // to expire
                    Either::Left(keys.0.iter().filter_map(|key| self.buffs.get(*key)).rev())
                } else {
                    Either::Right(self.buffs.get(keys.0[0]).into_iter())
                }
            })
    }

    // Gets most powerful buff of a given kind
    pub fn remove(&mut self, buff_key: BuffKey) {
        if let Some(buff) = self.buffs.remove(buff_key) {
            let kind = buff.kind;
            self.kinds[kind]
                .as_mut()
                .map(|keys| keys.0.retain(|key| *key != buff_key));
            self.sort_kind(kind);
        }
    }

    fn delay_queueable_buffs(&mut self, kind: BuffKind, current_time: Time) {
        let mut next_start_time: Option<Time> = None;
        debug_assert!(kind.queues());
        if let Some(buffs) = self.kinds[kind].as_mut() {
            buffs.0.iter().for_each(|key| {
                if let Some(buff) = self.buffs.get_mut(*key) {
                    // End time only being updated when there is some next_start_time will
                    // technically cause buffs to "end early" if they have a weaker strength than a
                    // buff with an infinite duration, but this is fine since those buffs wouldn't
                    // matter anyways
                    if let Some(next_start_time) = next_start_time {
                        // Delays buff so that it has the same progress it has now at the time the
                        // previous buff would end.
                        //
                        // Shift should be relative to current time, unless the buff is delayed and
                        // hasn't started yet
                        let reference_time = current_time.0.max(buff.start_time.0);
                        // If buff has a delay, ensure that queueables shuffling queue does not
                        // potentially allow skipping delay
                        buff.start_time = Time(next_start_time.0.max(buff.start_time.0));
                        buff.end_time = buff.end_time.map(|end| {
                            Time(end.0 + next_start_time.0.max(reference_time) - reference_time)
                        });
                    }
                    next_start_time = buff.end_time;
                }
            })
        }
    }
}

impl Component for Buffs {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Default, Copy, Clone)]
pub struct DestInfo<'a> {
    pub stats: Option<&'a Stats>,
    pub mass: Option<&'a Mass>,
}

#[cfg(test)]
pub mod tests {
    use crate::comp::buff::*;

    #[cfg(test)]
    fn create_test_queueable_buff(buff_data: BuffData, time: Time) -> Buff {
        // Change to another buff that queues if we ever add one and remove saturation,
        // otherwise maybe add a test buff kind?
        debug_assert!(BuffKind::Saturation.queues());
        Buff::new(
            BuffKind::Saturation,
            buff_data,
            Vec::new(),
            BuffSource::Unknown,
            time,
            DestInfo::default(),
            None,
        )
    }

    #[test]
    /// Tests a number of buffs with various progresses that queue to ensure
    /// queue has correct total duration
    fn test_queueable_buffs_three() {
        let mut buff_comp: Buffs = Default::default();
        let buff_data = BuffData::new(1.0, Some(Secs(10.0)));
        let time_a = Time(0.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_a), time_a);
        let time_b = Time(6.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_b), time_b);
        let time_c = Time(11.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_c), time_c);
        // Check that all buffs have an end_time less than or equal to 30, and that at
        // least one has an end_time greater than or equal to 30.
        //
        // This should be true because 3 buffs that each lasted for 10 seconds were
        // inserted at various times, so the total duration should be 30 seconds.
        assert!(
            buff_comp
                .buffs
                .values()
                .all(|b| b.end_time.unwrap().0 < 30.01)
        );
        assert!(
            buff_comp
                .buffs
                .values()
                .any(|b| b.end_time.unwrap().0 > 29.99)
        );
    }

    #[test]
    /// Tests that if a buff had a delay but will start soon, and an immediate
    /// queueable buff is added, delayed buff has correct start time
    fn test_queueable_buff_delay_start() {
        let mut buff_comp: Buffs = Default::default();
        let queued_buff_data = BuffData::new(1.0, Some(Secs(10.0))).with_delay(Secs(10.0));
        let buff_data = BuffData::new(1.0, Some(Secs(10.0)));
        let time_a = Time(0.0);
        buff_comp.insert(create_test_queueable_buff(queued_buff_data, time_a), time_a);
        let time_b = Time(6.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_b), time_b);
        // Check that all buffs have an end_time less than or equal to 26, and that at
        // least one has an end_time greater than or equal to 26.
        //
        // This should be true because the first buff added had a delay of 10 seconds
        // and a duration of 10 seconds, the second buff added at 6 seconds had no
        // delay, and a duration of 10 seconds. When it finishes at 16 seconds the first
        // buff is past the delay time so should finish at 26 seconds.
        assert!(
            buff_comp
                .buffs
                .values()
                .all(|b| b.end_time.unwrap().0 < 26.01)
        );
        assert!(
            buff_comp
                .buffs
                .values()
                .any(|b| b.end_time.unwrap().0 > 25.99)
        );
    }

    #[test]
    /// Tests that if a buff had a long delay, a short immediate queueable buff
    /// does not move delayed buff start or end times
    fn test_queueable_buff_long_delay() {
        let mut buff_comp: Buffs = Default::default();
        let queued_buff_data = BuffData::new(1.0, Some(Secs(10.0))).with_delay(Secs(50.0));
        let buff_data = BuffData::new(1.0, Some(Secs(10.0)));
        let time_a = Time(0.0);
        buff_comp.insert(create_test_queueable_buff(queued_buff_data, time_a), time_a);
        let time_b = Time(10.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_b), time_b);
        // Check that all buffs have either an end time less than or equal to 20 seconds
        // XOR a start time greater than or equal to 50 seconds, that all buffs have a
        // start time less than or equal to 50 seconds, that all buffs have an end time
        // less than or equal to 60 seconds, and that at least one buff has an end time
        // greater than or equal to 60 seconds
        //
        // This should be true because the first buff has a delay of 50 seconds, the
        // second buff added has no delay at 10 seconds and lasts 10 seconds, so should
        // end at 20 seconds and not affect the start time of the delayed buff, and
        // since the delayed buff was not affected the end time should be 10 seconds
        // after the start time: 60 seconds != used here to emulate xor
        assert!(
            buff_comp
                .buffs
                .values()
                .all(|b| (b.end_time.unwrap().0 < 20.01) != (b.start_time.0 > 49.99))
        );
        assert!(buff_comp.buffs.values().all(|b| b.start_time.0 < 50.01));
        assert!(
            buff_comp
                .buffs
                .values()
                .all(|b| b.end_time.unwrap().0 < 60.01)
        );
        assert!(
            buff_comp
                .buffs
                .values()
                .any(|b| b.end_time.unwrap().0 > 59.99)
        );
    }
}
