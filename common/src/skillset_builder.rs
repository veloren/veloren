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
                    skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsRegen));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsSpeed));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDrain));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::DScaling));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::DSpeed));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::DInfinite));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpeed));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                    skillset = skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                }
            },
            Some(Outcast) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SSpeed));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SCost));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CSpeed));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::ProjSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                    },
                    _ => {},
                }
            },
            Some(Highwayman) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SSpeed));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SCost));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LRange));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CMove));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BRegen));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                    },
                    _ => {},
                }
            },
            Some(Bandit) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsSpeed));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsRegen));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LCost));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LCost));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LDistance));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::ProjSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BRegen));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDrain));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RCost));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SRange));
                    },
                    _ => {},
                }
            },
            Some(CultistNovice) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsRegen));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsSpeed));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDrain));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DScaling));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpeed));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SHelicopter));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CDrain));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CSpeed));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LKnockback));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::ProjSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDrain));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SRange));
                    },
                    _ => {},
                }
            },
            Some(CultistAcolyte) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsSpeed));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DScaling));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpeed));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SCost));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LCost));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CDrain));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LRange));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDrain));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RCost));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SKnockback));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SRange));
                    },
                    _ => {},
                }
            },
            Some(Warlord) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsRegen));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDrain));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsSpeed));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsRegen));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SHelicopter));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SSpeed));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CDrain));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LDistance));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LRange));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BRegen));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CMove));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RCost));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BRegen));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SKnockback));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SCost));
                    },
                    _ => {},
                }
            },
            Some(Warlock) => {
                match active_item {
                    Some(ToolKind::Sword) => {
                        // Sword
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsCombo));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsRegen));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::TsSpeed));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DCost));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DDrain));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::DScaling));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpeed));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SSpins));
                        skillset = skillset.with_skill(Skill::Sword(SwordSkill::SCost));
                    },
                    Some(ToolKind::Axe) => {
                        // Axe
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsCombo));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsSpeed));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::DsRegen));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SInfinite));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SHelicopter));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SSpeed));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::SCost));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LDamage));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LKnockback));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LCost));
                        skillset = skillset.with_skill(Skill::Axe(AxeSkill::LDistance));
                    },
                    Some(ToolKind::Hammer) => {
                        // Hammer
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Hammer));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsSpeed));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::SsRegen));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CDrain));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::CSpeed));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LUnlockLeap));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LDamage));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LCost));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LDistance));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LKnockback));
                        skillset = skillset.with_skill(Skill::Hammer(HammerSkill::LRange));
                    },
                    Some(ToolKind::Bow) => {
                        // Bow
                        skillset = skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Bow));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::ProjSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::BRegen));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CKnockback));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CProjSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CDrain));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CSpeed));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::CMove));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::UnlockRepeater));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RDamage));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RGlide));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RArrows));
                        skillset = skillset.with_skill(Skill::Bow(BowSkill::RCost));
                    },
                    Some(ToolKind::Staff) => {
                        // Staff
                        skillset =
                            skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Staff));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BExplosion));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BRegen));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::BRadius));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FRange));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FDrain));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::FVelocity));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::UnlockShockwave));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SDamage));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SKnockback));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SRange));
                        skillset = skillset.with_skill(Skill::Staff(StaffSkill::SCost));
                    },
                    _ => {},
                }
            },
            Some(Villager) | None => {},
        }

        skillset
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
