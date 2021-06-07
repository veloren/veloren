use crate::comp::{
    item::{tool::ToolKind, Item, ItemKind},
    skills::{
        AxeSkill, BowSkill, HammerSkill, Skill, SkillGroupKind, SkillSet, StaffSkill, SwordSkill,
    },
};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum SkillSetConfig {
    Adlet,
    Gnarling,
    Sahagin,
    Haniwa,
    Myrmidon,
    Guard,
    Villager,
    Merchant,
    Outcast,
    Highwayman,
    Bandit,
    CultistNovice,
    CultistAcolyte,
    Warlord,
    Warlock,
    Mindflayer,
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
            Some(Adlet) => {
                match active_item {
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CMove), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(Gnarling) => {
                match active_item {
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CMove), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(Sahagin) => {
                match active_item {
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CMove), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(Haniwa) => {
                match active_item {
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CMove), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(Myrmidon) => {
                match active_item {
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CMove), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(Guard) => {
                if let Some(ToolKind::Sword) = active_item {
                    // Sword
                    Self::default()
                        .with_skill_group(SkillGroupKind::Weapon(ToolKind::Sword))
                        .with_skill(Skill::Sword(SwordSkill::TsCombo), None)
                        .with_skill(Skill::Sword(SwordSkill::TsDamage), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::TsRegen), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::TsSpeed), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::DDamage), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::DCost), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::DDrain), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::DScaling), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::DSpeed), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::DInfinite), None)
                        .with_skill(Skill::Sword(SwordSkill::UnlockSpin), None)
                        .with_skill(Skill::Sword(SwordSkill::SDamage), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::SSpeed), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::SSpins), Some(1))
                        .with_skill(Skill::Sword(SwordSkill::SCost), Some(1))
                } else {
                    Self::default()
                }
            },
            Some(Outcast) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo), None)
                            .with_skill(Skill::Sword(SwordSkill::DDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DCost), Some(1))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo), None)
                            .with_skill(Skill::Axe(AxeSkill::SInfinite), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SSpeed), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SCost), Some(1))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CSpeed), Some(1))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::ProjSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RSpeed), Some(1))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::FDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDrain), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(Highwayman) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo), None)
                            .with_skill(Skill::Sword(SwordSkill::TsDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin), None)
                            .with_skill(Skill::Sword(SwordSkill::SSpins), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SCost), Some(1))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo), None)
                            .with_skill(Skill::Axe(AxeSkill::DsDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite), None)
                            .with_skill(Skill::Axe(AxeSkill::SDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SSpeed), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SCost), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap), None)
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap), None)
                            .with_skill(Skill::Hammer(HammerSkill::LKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LRange), Some(1))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CMove), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::UnlockShotgun), None)
                            .with_skill(Skill::Bow(BowSkill::SArrows), Some(1))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BRegen), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::BRadius), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FRange), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave), None)
                    },
                    _ => Self::default(),
                }
            },
            Some(Bandit) | Some(Merchant) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo), None)
                            .with_skill(Skill::Sword(SwordSkill::TsDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DCost), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin), None)
                            .with_skill(Skill::Sword(SwordSkill::SDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SSpins), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SCost), Some(1))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo), None)
                            .with_skill(Skill::Axe(AxeSkill::DsSpeed), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::DsRegen), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite), None)
                            .with_skill(Skill::Axe(AxeSkill::SDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap), None)
                            .with_skill(Skill::Axe(AxeSkill::LKnockback), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LCost), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LDistance), Some(1))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap), None)
                            .with_skill(Skill::Hammer(HammerSkill::LDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LCost), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LDistance), Some(1))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::ProjSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CRegen), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RCost), Some(1))
                            .with_skill(Skill::Bow(BowSkill::UnlockShotgun), None)
                            .with_skill(Skill::Bow(BowSkill::SCost), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RCost), Some(1))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::FDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FRange), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDrain), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave), None)
                            .with_skill(Skill::Staff(StaffSkill::SDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::SRange), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(CultistNovice) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo), None)
                            .with_skill(Skill::Sword(SwordSkill::TsRegen), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::TsSpeed), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DCost), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DDrain), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DScaling), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin), None)
                            .with_skill(Skill::Sword(SwordSkill::SSpeed), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SSpins), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SCost), Some(1))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo), None)
                            .with_skill(Skill::Axe(AxeSkill::SInfinite), None)
                            .with_skill(Skill::Axe(AxeSkill::SHelicopter), None)
                            .with_skill(Skill::Axe(AxeSkill::SDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap), None)
                            .with_skill(Skill::Axe(AxeSkill::LKnockback), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LDistance), Some(1))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CDrain), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CSpeed), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap), None)
                            .with_skill(Skill::Hammer(HammerSkill::LDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LKnockback), Some(1))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::ProjSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RCost), Some(1))
                            .with_skill(Skill::Bow(BowSkill::UnlockShotgun), None)
                            .with_skill(Skill::Bow(BowSkill::SDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SArrows), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SSpread), Some(1))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::BRadius), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FRange), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDrain), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave), None)
                            .with_skill(Skill::Staff(StaffSkill::SDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::SRange), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(CultistAcolyte) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo), None)
                            .with_skill(Skill::Sword(SwordSkill::TsDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::TsSpeed), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DScaling), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin), None)
                            .with_skill(Skill::Sword(SwordSkill::SDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SSpeed), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SSpins), Some(1))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo), None)
                            .with_skill(Skill::Axe(AxeSkill::DsDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SCost), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap), None)
                            .with_skill(Skill::Axe(AxeSkill::LDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LKnockback), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LCost), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LDistance), Some(1))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CDrain), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap), None)
                            .with_skill(Skill::Hammer(HammerSkill::LDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LRange), Some(1))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RCost), Some(1))
                            .with_skill(Skill::Bow(BowSkill::UnlockShotgun), None)
                            .with_skill(Skill::Bow(BowSkill::SDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SSpread), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SArrows), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SCost), Some(1))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::BRadius), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FRange), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave), None)
                            .with_skill(Skill::Staff(StaffSkill::SDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::SKnockback), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::SRange), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(Warlord) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo), None)
                            .with_skill(Skill::Sword(SwordSkill::TsDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::TsRegen), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DCost), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DDrain), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin), None)
                            .with_skill(Skill::Sword(SwordSkill::SDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SSpins), Some(2))
                            .with_skill(Skill::Sword(SwordSkill::SCost), Some(1))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo), None)
                            .with_skill(Skill::Axe(AxeSkill::DsDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::DsSpeed), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::DsRegen), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite), None)
                            .with_skill(Skill::Axe(AxeSkill::SHelicopter), None)
                            .with_skill(Skill::Axe(AxeSkill::SDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SSpeed), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap), None)
                            .with_skill(Skill::Axe(AxeSkill::LDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LKnockback), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LDistance), Some(1))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CDrain), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap), None)
                            .with_skill(Skill::Hammer(HammerSkill::LDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LDistance), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LRange), Some(1))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CRegen), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CMove), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::UnlockShotgun), None)
                            .with_skill(Skill::Bow(BowSkill::SDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SSpread), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SArrows), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SCost), Some(1))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::BRegen), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::BRadius), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDrain), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave), None)
                            .with_skill(Skill::Staff(StaffSkill::SDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::SKnockback), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::SCost), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(Warlock) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Sword))
                            .with_skill(Skill::Sword(SwordSkill::TsCombo), None)
                            .with_skill(Skill::Sword(SwordSkill::TsDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::TsRegen), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::TsSpeed), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DDamage), Some(2))
                            .with_skill(Skill::Sword(SwordSkill::DCost), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DDrain), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::DScaling), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::UnlockSpin), None)
                            .with_skill(Skill::Sword(SwordSkill::SDamage), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SSpeed), Some(1))
                            .with_skill(Skill::Sword(SwordSkill::SSpins), Some(2))
                            .with_skill(Skill::Sword(SwordSkill::SCost), Some(1))
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Axe))
                            .with_skill(Skill::Axe(AxeSkill::DsCombo), None)
                            .with_skill(Skill::Axe(AxeSkill::DsDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::DsSpeed), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::DsRegen), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SInfinite), None)
                            .with_skill(Skill::Axe(AxeSkill::SHelicopter), None)
                            .with_skill(Skill::Axe(AxeSkill::SDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SSpeed), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::SCost), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::UnlockLeap), None)
                            .with_skill(Skill::Axe(AxeSkill::LDamage), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LKnockback), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LCost), Some(1))
                            .with_skill(Skill::Axe(AxeSkill::LDistance), Some(1))
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Hammer))
                            .with_skill(Skill::Hammer(HammerSkill::SsKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsSpeed), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::SsRegen), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CDrain), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::CSpeed), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::UnlockLeap), None)
                            .with_skill(Skill::Hammer(HammerSkill::LDamage), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LCost), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LDistance), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LKnockback), Some(1))
                            .with_skill(Skill::Hammer(HammerSkill::LRange), Some(1))
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Bow))
                            .with_skill(Skill::Bow(BowSkill::ProjSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CRegen), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CKnockback), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::CMove), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RSpeed), Some(1))
                            .with_skill(Skill::Bow(BowSkill::RCost), Some(1))
                            .with_skill(Skill::Bow(BowSkill::UnlockShotgun), None)
                            .with_skill(Skill::Bow(BowSkill::SDamage), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SSpread), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SArrows), Some(1))
                            .with_skill(Skill::Bow(BowSkill::SCost), Some(1))
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        Self::default()
                            .with_skill_group(SkillGroupKind::Weapon(ToolKind::Staff))
                            .with_skill(Skill::Staff(StaffSkill::BDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::BRegen), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::BRadius), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FRange), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FDrain), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::FVelocity), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::UnlockShockwave), None)
                            .with_skill(Skill::Staff(StaffSkill::SDamage), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::SKnockback), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::SRange), Some(1))
                            .with_skill(Skill::Staff(StaffSkill::SCost), Some(1))
                    },
                    _ => Self::default(),
                }
            },
            Some(Mindflayer) => Self::default()
                .with_skill_group(SkillGroupKind::Weapon(ToolKind::Staff))
                .with_skill(Skill::Staff(StaffSkill::BDamage), Some(3))
                .with_skill(Skill::Staff(StaffSkill::BRegen), Some(2))
                .with_skill(Skill::Staff(StaffSkill::BRadius), Some(2))
                .with_skill(Skill::Staff(StaffSkill::FDamage), Some(3))
                .with_skill(Skill::Staff(StaffSkill::FRange), Some(2))
                .with_skill(Skill::Staff(StaffSkill::FDrain), Some(2))
                .with_skill(Skill::Staff(StaffSkill::FVelocity), Some(2))
                .with_skill(Skill::Staff(StaffSkill::UnlockShockwave), None)
                .with_skill(Skill::Staff(StaffSkill::SDamage), Some(2))
                .with_skill(Skill::Staff(StaffSkill::SKnockback), Some(2))
                .with_skill(Skill::Staff(StaffSkill::SRange), Some(2))
                .with_skill(Skill::Staff(StaffSkill::SCost), Some(2)),
            Some(Villager) | None => Self::default(),
        }
    }

    #[must_use]
    /// # Panics
    /// will panic only in tests
    /// 1) If added skill doesn't have any group
    /// 2) If added skill already applied
    /// 3) If added skill wasn't applied at the end
    pub fn with_skill(mut self, skill: Skill, level: Option<u16>) -> Self {
        #![warn(clippy::pedantic)]
        let group = if let Some(skill_group) = skill.skill_group_kind() {
            skill_group
        } else {
            let err = format!(
                "Tried to add skill: {:?} which does not have an associated skill group.",
                skill
            );
            if cfg!(test) {
                panic!("{}", err);
            } else {
                warn!("{}", err);
            }
            return self;
        };

        let SkillSetBuilder(ref mut skill_set) = self;
        if skill_is_applied(skill_set, skill, level) {
            let err = format!(
                "Tried to add skill: {:?} with level {:?} which is already applied",
                skill, level,
            );
            if cfg!(test) {
                panic!("{}", err);
            } else {
                warn!("{}", err);
            }
            return self;
        }
        for _ in 0..level.unwrap_or(1) {
            skill_set.add_skill_points(group, skill_set.skill_cost(skill));
            skill_set.unlock_skill(skill);
        }
        if !skill_is_applied(skill_set, skill, level) {
            let err = format!(
                "Failed to add skill: {:?}. Verify that it has the appropriate skill group \
                 available and meets all prerequisite skills.",
                skill
            );
            if cfg!(test) {
                panic!("{}", err);
            } else {
                warn!("{}", err);
            }
        }
        self
    }

    pub fn with_skill_group(self, skill_group: SkillGroupKind) -> Self {
        self.with_skill(Skill::UnlockGroup(skill_group), None)
    }

    pub fn build(self) -> SkillSet { self.0 }
}

fn skill_is_applied(skill_set: &SkillSet, skill: Skill, level: Option<u16>) -> bool {
    if let Ok(applied_level) = skill_set.skill_level(skill) {
        applied_level == level
    } else {
        false
    }
}
