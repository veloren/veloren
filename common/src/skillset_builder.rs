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
        let mut skillset = Self::default();
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
                    skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                    skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                    skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                    skillset.with_skill(Skill::Sword(SwordSkill::TsRegen));
                    skillset.with_skill(Skill::Sword(SwordSkill::TsSpeed));
                    skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                    skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                    skillset.with_skill(Skill::Sword(SwordSkill::DDrain));
                    skillset.with_skill(Skill::Sword(SwordSkill::DScaling));
                    skillset.with_skill(Skill::Sword(SwordSkill::DSpeed));
                    skillset.with_skill(Skill::Sword(SwordSkill::DInfinite));
                    skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                    skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                    skillset.with_skill(Skill::Sword(SwordSkill::SSpeed));
                    skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                    skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                }
            },
            Some(Outcast) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset.with_skill(Skill::Axe(AxeSkill::SSpeed));
                        skillset.with_skill(Skill::Axe(AxeSkill::SCost));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CSpeed));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::ProjSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                    },
                    _ => {},
                }
            },
            Some(Highwayman) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::SSpeed));
                        skillset.with_skill(Skill::Axe(AxeSkill::SCost));
                        skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LRange));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset.with_skill(Skill::Bow(BowSkill::CSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::CMove));
                        skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset.with_skill(Skill::Staff(StaffSkill::BRegen));
                        skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                    },
                    _ => {},
                }
            },
            Some(Bandit) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                        skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsSpeed));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsRegen));
                        skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset.with_skill(Skill::Axe(AxeSkill::LCost));
                        skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LCost));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LDistance));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::ProjSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::BRegen));
                        skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::CDrain));
                        skillset.with_skill(Skill::Bow(BowSkill::CSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset.with_skill(Skill::Bow(BowSkill::RCost));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::SRange));
                    },
                    _ => {},
                }
            },
            Some(CultistNovice) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsRegen));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsSpeed));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDrain));
                        skillset.with_skill(Skill::Sword(SwordSkill::DScaling));
                        skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpeed));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset.with_skill(Skill::Axe(AxeSkill::SHelicopter));
                        skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CDrain));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CSpeed));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LKnockback));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::ProjSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::CDrain));
                        skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset.with_skill(Skill::Bow(BowSkill::RDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset.with_skill(Skill::Staff(StaffSkill::BDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::SRange));
                    },
                    _ => {},
                }
            },
            Some(CultistAcolyte) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsSpeed));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::DScaling));
                        skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpeed));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::SCost));
                        skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Axe(AxeSkill::LDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset.with_skill(Skill::Axe(AxeSkill::LCost));
                        skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CDrain));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LRange));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::CDrain));
                        skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset.with_skill(Skill::Bow(BowSkill::RDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                        skillset.with_skill(Skill::Bow(BowSkill::RCost));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset.with_skill(Skill::Staff(StaffSkill::BDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::SKnockback));
                        skillset.with_skill(Skill::Staff(StaffSkill::SRange));
                    },
                    _ => {},
                }
            },
            Some(Warlord) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsRegen));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDrain));
                        skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsSpeed));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsRegen));
                        skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset.with_skill(Skill::Axe(AxeSkill::SHelicopter));
                        skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::SSpeed));
                        skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Axe(AxeSkill::LDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CDrain));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LDistance));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LRange));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::BRegen));
                        skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::CSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::CMove));
                        skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset.with_skill(Skill::Bow(BowSkill::RDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                        skillset.with_skill(Skill::Bow(BowSkill::RCost));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset.with_skill(Skill::Staff(StaffSkill::BDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::BRegen));
                        skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::SKnockback));
                        skillset.with_skill(Skill::Staff(StaffSkill::SCost));
                    },
                    _ => {},
                }
            },
            Some(Warlock) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsRegen));
                        skillset.with_skill(Skill::Sword(SwordSkill::TsSpeed));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                        skillset.with_skill(Skill::Sword(SwordSkill::DDrain));
                        skillset.with_skill(Skill::Sword(SwordSkill::DScaling));
                        skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpeed));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsSpeed));
                        skillset.with_skill(Skill::Axe(AxeSkill::DsRegen));
                        skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset.with_skill(Skill::Axe(AxeSkill::SHelicopter));
                        skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::SSpeed));
                        skillset.with_skill(Skill::Axe(AxeSkill::SCost));
                        skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Axe(AxeSkill::LDamage));
                        skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset.with_skill(Skill::Axe(AxeSkill::LCost));
                        skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CDrain));
                        skillset.with_skill(Skill::Hammer(HammerSkill::CSpeed));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LCost));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LDistance));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LKnockback));
                        skillset.with_skill(Skill::Hammer(HammerSkill::LRange));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::ProjSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::BRegen));
                        skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::CDrain));
                        skillset.with_skill(Skill::Bow(BowSkill::CSpeed));
                        skillset.with_skill(Skill::Bow(BowSkill::CMove));
                        skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset.with_skill(Skill::Bow(BowSkill::RDamage));
                        skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                        skillset.with_skill(Skill::Bow(BowSkill::RCost));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset.with_skill(Skill::Staff(StaffSkill::BDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::BRegen));
                        skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset.with_skill(Skill::Staff(StaffSkill::SKnockback));
                        skillset.with_skill(Skill::Staff(StaffSkill::SRange));
                        skillset.with_skill(Skill::Staff(StaffSkill::SCost));
                    },
                    _ => {},
                }
            },
            _ => {},
        }

        skillset
    }

    pub fn with_skill(&mut self, skill: Skill) {
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
        }
    }

    pub fn with_skill_group(&mut self, skill_group: SkillGroupType) {
        self.0.unlock_skill_group(skill_group);
    }

    pub fn build(self) -> SkillSet { self.0 }
}
