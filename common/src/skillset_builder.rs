use crate::comp::{
    item::tool::ToolKind,
    skills::{Skill, SkillGroupType, SkillSet, SwordSkill},
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
    pub fn build_skillset(config: SkillSetConfig) -> Self {
        let mut skillset = Self::default();
        use SkillSetConfig::*;
        match config {
            Guard => {
                skillset.with_skill_group(SkillGroupType::Weapon(ToolKind::Sword));
                skillset.with_skill(Skill::Sword(SwordSkill::SUnlockSpin));
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
                    "Failed to add skill. Verify that it has the appropriate skill group \
                     available."
                );
            }
        }
    }

    pub fn with_skill_group(&mut self, skill_group: SkillGroupType) {
        self.0.unlock_skill_group(skill_group);
    }

    pub fn build(self) -> SkillSet { self.0 }
}
