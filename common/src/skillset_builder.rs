use crate::comp::{
    item::{tool::ToolKind, Item, ItemKind},
    skills::{
        AxeSkill, BowSkill, HammerSkill, Skill, SkillGroupType, SkillSet, StaffSkill, SwordSkill,
    },
};
use tracing::warn;

#[derive(Copy, Clone)]
pub enum SkillSetConfig {
    Guard,
    Villager,
    Outcast,
    Highwayman,
    Bandit,
    CultistNovice,
    CultistAcolyte,
    Warlord,
    Warlock,
}

pub struct SkillSetBuilder(SkillSet);

impl Default for SkillSetBuilder {
    fn default() -> Self { Self(SkillSet::default()) }
}

impl SkillSetBuilder {
    pub fn build_skillset(main_tool: &Option<Item>, config: Option<SkillSetConfig>) -> Self {
        let active_item = main_tool.as_ref().and_then(|ic| {
            if let ItemKind::Tool(tool) = &ic.kind() {
                Some(tool.kind)
            } else {
                None
            }
        });

        use SkillSetConfig::*;
        match config {
            Some(Guard) => {
                if let Some(ToolKind::Sword) = active_item {
                    // Sword
                    Self::default()
                        .with_skill_group(SkillGroupType::Weapon(ToolKind::Sword))
                        .with_skill(Skill::Sword(SwordSkill::TsCombo))
                        .with_skill(Skill::Sword(SwordSkill::TsDamage))
                        .with_skill(Skill::Sword(SwordSkill::TsRegen))
                        .with_skill(Skill::Sword(SwordSkill::TsSpeed))
                        .with_skill(Skill::Sword(SwordSkill::DDamage))
                        .with_skill(Skill::Sword(SwordSkill::DCost))
                        .with_skill(Skill::Sword(SwordSkill::DDrain))
                        .with_skill(Skill::Sword(SwordSkill::DScaling))
                        .with_skill(Skill::Sword(SwordSkill::DSpeed))
                        .with_skill(Skill::Sword(SwordSkill::DInfinite))
                        .with_skill(Skill::Sword(SwordSkill::UnlockSpin))
                        .with_skill(Skill::Sword(SwordSkill::SDamage))
                        .with_skill(Skill::Sword(SwordSkill::SSpeed))
                        .with_skill(Skill::Sword(SwordSkill::SSpins))
                        .with_skill(Skill::Sword(SwordSkill::SCost))
                } else {
                    Self::default()
                }
            },
            Some(Outcast) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo))
                            .with_skill(Skill::Sword(SwordSkill::DDamage))
                            .with_skill(Skill::Sword(SwordSkill::DCost))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite))
                            .with_skill(Skill::Axe(AxeSkill::SSpeed))
                            .with_skill(Skill::Axe(AxeSkill::SCost))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::CSpeed))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::BDamage))
                            .with_skill(Skill::Bow(BowSkill::ProjSpeed))
                            .with_skill(Skill::Bow(BowSkill::CDamage))
                            .with_skill(Skill::Bow(BowSkill::CKnockback))
                            .with_skill(Skill::Bow(BowSkill::CProjSpeed))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::FDamage))
                            .with_skill(Skill::Staff(StaffSkill::FDrain))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity))
                    },
                    _ => Self::default(),
                }
            },
            Some(Highwayman) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo))
                            .with_skill(Skill::Sword(SwordSkill::TsDamage))
                            .with_skill(Skill::Sword(SwordSkill::DDamage))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin))
                            .with_skill(Skill::Sword(SwordSkill::SSpins))
                            .with_skill(Skill::Sword(SwordSkill::SCost))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo))
                            .with_skill(Skill::Axe(AxeSkill::DsDamage))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite))
                            .with_skill(Skill::Axe(AxeSkill::SDamage))
                            .with_skill(Skill::Axe(AxeSkill::SSpeed))
                            .with_skill(Skill::Axe(AxeSkill::SCost))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                            .with_skill(Skill::Hammer(HammerSkill::LKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::LRange))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::BDamage))
                            .with_skill(Skill::Bow(BowSkill::CDamage))
                            .with_skill(Skill::Bow(BowSkill::CKnockback))
                            .with_skill(Skill::Bow(BowSkill::CSpeed))
                            .with_skill(Skill::Bow(BowSkill::CMove))
                            .with_skill(Skill::Bow(BowSkill::UnlockRepeater))
                            .with_skill(Skill::Bow(BowSkill::RArrows))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BExplosion))
                            .with_skill(Skill::Staff(StaffSkill::BRegen))
                            .with_skill(Skill::Staff(StaffSkill::BRadius))
                            .with_skill(Skill::Staff(StaffSkill::FDamage))
                            .with_skill(Skill::Staff(StaffSkill::FRange))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave))
                    },
                    _ => Self::default(),
                }
            },
            Some(Bandit) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo))
                            .with_skill(Skill::Sword(SwordSkill::TsDamage))
                            .with_skill(Skill::Sword(SwordSkill::DDamage))
                            .with_skill(Skill::Sword(SwordSkill::DCost))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin))
                            .with_skill(Skill::Sword(SwordSkill::SDamage))
                            .with_skill(Skill::Sword(SwordSkill::SSpins))
                            .with_skill(Skill::Sword(SwordSkill::SCost))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo))
                            .with_skill(Skill::Axe(AxeSkill::DsSpeed))
                            .with_skill(Skill::Axe(AxeSkill::DsRegen))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite))
                            .with_skill(Skill::Axe(AxeSkill::SDamage))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap))
                            .with_skill(Skill::Axe(AxeSkill::LKnockback))
                            .with_skill(Skill::Axe(AxeSkill::LCost))
                            .with_skill(Skill::Axe(AxeSkill::LDistance))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                            .with_skill(Skill::Hammer(HammerSkill::LDamage))
                            .with_skill(Skill::Hammer(HammerSkill::LCost))
                            .with_skill(Skill::Hammer(HammerSkill::LDistance))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::BDamage))
                            .with_skill(Skill::Bow(BowSkill::ProjSpeed))
                            .with_skill(Skill::Bow(BowSkill::BRegen))
                            .with_skill(Skill::Bow(BowSkill::CDamage))
                            .with_skill(Skill::Bow(BowSkill::CDrain))
                            .with_skill(Skill::Bow(BowSkill::CSpeed))
                            .with_skill(Skill::Bow(BowSkill::UnlockRepeater))
                            .with_skill(Skill::Bow(BowSkill::RGlide))
                            .with_skill(Skill::Bow(BowSkill::RCost))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BExplosion))
                            .with_skill(Skill::Staff(StaffSkill::FDamage))
                            .with_skill(Skill::Staff(StaffSkill::FRange))
                            .with_skill(Skill::Staff(StaffSkill::FDrain))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave))
                            .with_skill(Skill::Staff(StaffSkill::SDamage))
                            .with_skill(Skill::Staff(StaffSkill::SRange))
                    },
                    _ => Self::default(),
                }
            },
            Some(CultistNovice) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo))
                            .with_skill(Skill::Sword(SwordSkill::TsRegen))
                            .with_skill(Skill::Sword(SwordSkill::TsSpeed))
                            .with_skill(Skill::Sword(SwordSkill::DDamage))
                            .with_skill(Skill::Sword(SwordSkill::DCost))
                            .with_skill(Skill::Sword(SwordSkill::DDrain))
                            .with_skill(Skill::Sword(SwordSkill::DScaling))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin))
                            .with_skill(Skill::Sword(SwordSkill::SSpeed))
                            .with_skill(Skill::Sword(SwordSkill::SSpins))
                            .with_skill(Skill::Sword(SwordSkill::SCost))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite))
                            .with_skill(Skill::Axe(AxeSkill::SHelicopter))
                            .with_skill(Skill::Axe(AxeSkill::SDamage))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap))
                            .with_skill(Skill::Axe(AxeSkill::LKnockback))
                            .with_skill(Skill::Axe(AxeSkill::LDistance))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage))
                            .with_skill(Skill::Hammer(HammerSkill::CDrain))
                            .with_skill(Skill::Hammer(HammerSkill::CSpeed))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                            .with_skill(Skill::Hammer(HammerSkill::LDamage))
                            .with_skill(Skill::Hammer(HammerSkill::LKnockback))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::BDamage))
                            .with_skill(Skill::Bow(BowSkill::ProjSpeed))
                            .with_skill(Skill::Bow(BowSkill::CDamage))
                            .with_skill(Skill::Bow(BowSkill::CKnockback))
                            .with_skill(Skill::Bow(BowSkill::CProjSpeed))
                            .with_skill(Skill::Bow(BowSkill::CDrain))
                            .with_skill(Skill::Bow(BowSkill::UnlockRepeater))
                            .with_skill(Skill::Bow(BowSkill::RDamage))
                            .with_skill(Skill::Bow(BowSkill::RGlide))
                            .with_skill(Skill::Bow(BowSkill::RArrows))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BExplosion))
                            .with_skill(Skill::Staff(StaffSkill::BDamage))
                            .with_skill(Skill::Staff(StaffSkill::BRadius))
                            .with_skill(Skill::Staff(StaffSkill::FDamage))
                            .with_skill(Skill::Staff(StaffSkill::FRange))
                            .with_skill(Skill::Staff(StaffSkill::FDrain))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave))
                            .with_skill(Skill::Staff(StaffSkill::SDamage))
                            .with_skill(Skill::Staff(StaffSkill::SRange))
                    },
                    _ => Self::default(),
                }
            },
            Some(CultistAcolyte) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo))
                            .with_skill(Skill::Sword(SwordSkill::TsDamage))
                            .with_skill(Skill::Sword(SwordSkill::TsSpeed))
                            .with_skill(Skill::Sword(SwordSkill::DDamage))
                            .with_skill(Skill::Sword(SwordSkill::DScaling))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin))
                            .with_skill(Skill::Sword(SwordSkill::SDamage))
                            .with_skill(Skill::Sword(SwordSkill::SSpeed))
                            .with_skill(Skill::Sword(SwordSkill::SSpins))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo))
                            .with_skill(Skill::Axe(AxeSkill::DsDamage))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite))
                            .with_skill(Skill::Axe(AxeSkill::SDamage))
                            .with_skill(Skill::Axe(AxeSkill::SCost))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap))
                            .with_skill(Skill::Axe(AxeSkill::LDamage))
                            .with_skill(Skill::Axe(AxeSkill::LKnockback))
                            .with_skill(Skill::Axe(AxeSkill::LCost))
                            .with_skill(Skill::Axe(AxeSkill::LDistance))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage))
                            .with_skill(Skill::Hammer(HammerSkill::CDrain))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                            .with_skill(Skill::Hammer(HammerSkill::LDamage))
                            .with_skill(Skill::Hammer(HammerSkill::LRange))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::BDamage))
                            .with_skill(Skill::Bow(BowSkill::CDamage))
                            .with_skill(Skill::Bow(BowSkill::CKnockback))
                            .with_skill(Skill::Bow(BowSkill::CProjSpeed))
                            .with_skill(Skill::Bow(BowSkill::CDrain))
                            .with_skill(Skill::Bow(BowSkill::UnlockRepeater))
                            .with_skill(Skill::Bow(BowSkill::RDamage))
                            .with_skill(Skill::Bow(BowSkill::RGlide))
                            .with_skill(Skill::Bow(BowSkill::RArrows))
                            .with_skill(Skill::Bow(BowSkill::RCost))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BExplosion))
                            .with_skill(Skill::Staff(StaffSkill::BDamage))
                            .with_skill(Skill::Staff(StaffSkill::BRadius))
                            .with_skill(Skill::Staff(StaffSkill::FDamage))
                            .with_skill(Skill::Staff(StaffSkill::FRange))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave))
                            .with_skill(Skill::Staff(StaffSkill::SDamage))
                            .with_skill(Skill::Staff(StaffSkill::SKnockback))
                            .with_skill(Skill::Staff(StaffSkill::SRange))
                    },
                    _ => Self::default(),
                }
            },
            Some(Warlord) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo))
                            .with_skill(Skill::Sword(SwordSkill::TsDamage))
                            .with_skill(Skill::Sword(SwordSkill::TsRegen))
                            .with_skill(Skill::Sword(SwordSkill::DDamage))
                            .with_skill(Skill::Sword(SwordSkill::DCost))
                            .with_skill(Skill::Sword(SwordSkill::DDrain))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin))
                            .with_skill(Skill::Sword(SwordSkill::SDamage))
                            .with_skill(Skill::Sword(SwordSkill::SSpins))
                            .with_skill(Skill::Sword(SwordSkill::SSpins))
                            .with_skill(Skill::Sword(SwordSkill::SCost))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo))
                            .with_skill(Skill::Axe(AxeSkill::DsDamage))
                            .with_skill(Skill::Axe(AxeSkill::DsSpeed))
                            .with_skill(Skill::Axe(AxeSkill::DsRegen))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite))
                            .with_skill(Skill::Axe(AxeSkill::SHelicopter))
                            .with_skill(Skill::Axe(AxeSkill::SDamage))
                            .with_skill(Skill::Axe(AxeSkill::SSpeed))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap))
                            .with_skill(Skill::Axe(AxeSkill::LDamage))
                            .with_skill(Skill::Axe(AxeSkill::LKnockback))
                            .with_skill(Skill::Axe(AxeSkill::LDistance))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage))
                            .with_skill(Skill::Hammer(HammerSkill::CDrain))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                            .with_skill(Skill::Hammer(HammerSkill::LDamage))
                            .with_skill(Skill::Hammer(HammerSkill::LDistance))
                            .with_skill(Skill::Hammer(HammerSkill::LKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::LRange))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::BDamage))
                            .with_skill(Skill::Bow(BowSkill::BRegen))
                            .with_skill(Skill::Bow(BowSkill::CDamage))
                            .with_skill(Skill::Bow(BowSkill::CKnockback))
                            .with_skill(Skill::Bow(BowSkill::CProjSpeed))
                            .with_skill(Skill::Bow(BowSkill::CSpeed))
                            .with_skill(Skill::Bow(BowSkill::CMove))
                            .with_skill(Skill::Bow(BowSkill::UnlockRepeater))
                            .with_skill(Skill::Bow(BowSkill::RDamage))
                            .with_skill(Skill::Bow(BowSkill::RGlide))
                            .with_skill(Skill::Bow(BowSkill::RArrows))
                            .with_skill(Skill::Bow(BowSkill::RCost))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BExplosion))
                            .with_skill(Skill::Staff(StaffSkill::BDamage))
                            .with_skill(Skill::Staff(StaffSkill::BRegen))
                            .with_skill(Skill::Staff(StaffSkill::BRadius))
                            .with_skill(Skill::Staff(StaffSkill::FDamage))
                            .with_skill(Skill::Staff(StaffSkill::FDrain))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave))
                            .with_skill(Skill::Staff(StaffSkill::SDamage))
                            .with_skill(Skill::Staff(StaffSkill::SKnockback))
                            .with_skill(Skill::Staff(StaffSkill::SCost))
                    },
                    _ => Self::default(),
                }
            },
            Some(Warlock) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo))
                            .with_skill(Skill::Sword(SwordSkill::TsDamage))
                            .with_skill(Skill::Sword(SwordSkill::TsRegen))
                            .with_skill(Skill::Sword(SwordSkill::TsSpeed))
                            .with_skill(Skill::Sword(SwordSkill::DDamage))
                            .with_skill(Skill::Sword(SwordSkill::DDamage))
                            .with_skill(Skill::Sword(SwordSkill::DCost))
                            .with_skill(Skill::Sword(SwordSkill::DDrain))
                            .with_skill(Skill::Sword(SwordSkill::DScaling))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin))
                            .with_skill(Skill::Sword(SwordSkill::SDamage))
                            .with_skill(Skill::Sword(SwordSkill::SSpeed))
                            .with_skill(Skill::Sword(SwordSkill::SSpins))
                            .with_skill(Skill::Sword(SwordSkill::SSpins))
                            .with_skill(Skill::Sword(SwordSkill::SCost))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo))
                            .with_skill(Skill::Axe(AxeSkill::DsDamage))
                            .with_skill(Skill::Axe(AxeSkill::DsSpeed))
                            .with_skill(Skill::Axe(AxeSkill::DsRegen))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite))
                            .with_skill(Skill::Axe(AxeSkill::SHelicopter))
                            .with_skill(Skill::Axe(AxeSkill::SDamage))
                            .with_skill(Skill::Axe(AxeSkill::SSpeed))
                            .with_skill(Skill::Axe(AxeSkill::SCost))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap))
                            .with_skill(Skill::Axe(AxeSkill::LDamage))
                            .with_skill(Skill::Axe(AxeSkill::LKnockback))
                            .with_skill(Skill::Axe(AxeSkill::LCost))
                            .with_skill(Skill::Axe(AxeSkill::LDistance))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage))
                            .with_skill(Skill::Hammer(HammerSkill::CDrain))
                            .with_skill(Skill::Hammer(HammerSkill::CSpeed))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                            .with_skill(Skill::Hammer(HammerSkill::LDamage))
                            .with_skill(Skill::Hammer(HammerSkill::LCost))
                            .with_skill(Skill::Hammer(HammerSkill::LDistance))
                            .with_skill(Skill::Hammer(HammerSkill::LKnockback))
                            .with_skill(Skill::Hammer(HammerSkill::LRange))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::BDamage))
                            .with_skill(Skill::Bow(BowSkill::ProjSpeed))
                            .with_skill(Skill::Bow(BowSkill::BRegen))
                            .with_skill(Skill::Bow(BowSkill::CDamage))
                            .with_skill(Skill::Bow(BowSkill::CKnockback))
                            .with_skill(Skill::Bow(BowSkill::CProjSpeed))
                            .with_skill(Skill::Bow(BowSkill::CDrain))
                            .with_skill(Skill::Bow(BowSkill::CSpeed))
                            .with_skill(Skill::Bow(BowSkill::CMove))
                            .with_skill(Skill::Bow(BowSkill::UnlockRepeater))
                            .with_skill(Skill::Bow(BowSkill::RDamage))
                            .with_skill(Skill::Bow(BowSkill::RGlide))
                            .with_skill(Skill::Bow(BowSkill::RArrows))
                            .with_skill(Skill::Bow(BowSkill::RCost))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupType::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BExplosion))
                            .with_skill(Skill::Staff(StaffSkill::BDamage))
                            .with_skill(Skill::Staff(StaffSkill::BRegen))
                            .with_skill(Skill::Staff(StaffSkill::BRadius))
                            .with_skill(Skill::Staff(StaffSkill::FDamage))
                            .with_skill(Skill::Staff(StaffSkill::FRange))
                            .with_skill(Skill::Staff(StaffSkill::FDrain))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave))
                            .with_skill(Skill::Staff(StaffSkill::SDamage))
                            .with_skill(Skill::Staff(StaffSkill::SKnockback))
                            .with_skill(Skill::Staff(StaffSkill::SRange))
                            .with_skill(Skill::Staff(StaffSkill::SCost))
                    },
                    _ => Self::default(),
                }
            },
            Some(Villager) | None => Self::default(),
        }
    }

    pub fn with_skill(mut self, skill: Skill) -> Self {
        if let Some(skill_group) = skill.get_skill_group_type() {
            self.0
                .add_skill_points(skill_group, self.0.skill_point_cost(skill));
            self.0.unlock_skill(skill);
            if !self.0.skills.contains_key(&skill) {
                warn!(
                    "Failed to add skill: {:?}. Verify that it has the appropriate skill group \
                     available and meets all prerequisite skills.",
                    skill
                );
            }
        } else {
            warn!(
                "Tried to add skill: {:?} which does not have an associated skill group.",
                skill
            );
        }
        self
    }

    pub fn with_skill_group(mut self, skill_group: SkillGroupType) -> Self {
        self.0.unlock_skill_group(skill_group);
        self
    }

    pub fn build(self) -> SkillSet { self.0 }
}
