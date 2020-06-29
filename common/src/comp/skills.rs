use lazy_static::lazy_static;
use std::collections::HashMap;
use tracing::warn;

/// Represents a skill that a player can unlock, that either grants them some
/// kind of active ability, or a passive effect etc. Obviously because this is
/// an enum it doesn't describe what the skill actually -does-, this will be
/// handled by dedicated ECS systems.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Skill {
    TestT1Skill1,
    TestT1Skill2,
    TestT1Skill3,
    TestT1Skill4,
    TestT1Skill5,
    TestSwordSkill1,
    TestSwordSkill2,
    TestSwordSkill3,
    TestAxeSkill1,
    TestAxeSkill2,
    TestAxeSkill3,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillGroupType {
    T1,
    Swords,
    Axes,
}

/// A group of skills that have been unlocked by a player. Each skill group has
/// independent exp and skill points which are used to unlock skills in that
/// skill group.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillGroup {
    pub skills: Vec<Skill>,
    pub exp: u32,
    pub available_sp: u8,
}

impl Default for SkillGroup {
    fn default() -> Self {
        Self {
            skills: Vec::new(),
            exp: 0,
            available_sp: 0,
        }
    }
}

lazy_static! {
    // Determines the skills that comprise each skill group - this data is used to determine
    // which of a player's skill groups a particular skill should be added to when a skill unlock
    // is requested. TODO: Externalise this data in a RON file for ease of modification
    static ref SKILL_GROUP_DEFS: HashMap<SkillGroupType, Vec<Skill>> = {
        let mut defs = HashMap::new();
        defs.insert(SkillGroupType::T1, vec![
            Skill::TestT1Skill1,
            Skill::TestT1Skill2,
            Skill::TestT1Skill3,
            Skill::TestT1Skill4,
            Skill::TestT1Skill5]);

        defs.insert(SkillGroupType::Swords, vec![
            Skill::TestSwordSkill1,
            Skill::TestSwordSkill2,
            Skill::TestSwordSkill3]);

        defs.insert(SkillGroupType::Axes, vec![
            Skill::TestAxeSkill1,
            Skill::TestAxeSkill2,
            Skill::TestAxeSkill3]);

        defs
    };
}

/// Contains all of a player's skill groups and provides methods for
/// manipulating assigned skills including unlocking skills, refunding skills
/// etc.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillSet {
    pub skill_groups: HashMap<SkillGroupType, SkillGroup>,
}

impl Default for SkillSet {
    /// Instantiate a new skill set with the default skill groups with no
    /// unlocked skills in them - used when adding a skill set to a new
    /// player
    fn default() -> Self {
        let mut skill_groups = HashMap::new();
        skill_groups.insert(SkillGroupType::T1, SkillGroup::default());
        skill_groups.insert(SkillGroupType::Swords, SkillGroup::default());
        skill_groups.insert(SkillGroupType::Axes, SkillGroup::default());
        Self { skill_groups }
    }
}

impl SkillSet {
    pub fn refund_skill(&mut self, _skill: Skill) {
        // TODO: check player has skill, remove skill and increase SP in skill group by 1
    }

    pub fn unlock_skill(&mut self, skill: Skill) {
        // Find the skill group type for the skill from the static skill definitions
        let skill_group_type = SKILL_GROUP_DEFS.iter().find_map(|(key, val)| {
            if val.contains(&skill) {
                Some(*key)
            } else {
                None
            }
        });

        // Find the skill group for the skill on the player, check that the skill is not
        // already unlocked and that they have available SP in that group, and then
        // allocate the skill and reduce the player's SP in that skill group by 1.
        if let Some(skill_group_type) = skill_group_type {
            if let Some(skill_group) = self.skill_groups.get_mut(&skill_group_type) {
                if !skill_group.skills.contains(&skill) {
                    if skill_group.available_sp > 0 {
                        skill_group.skills.push(skill);
                        skill_group.available_sp -= 1;
                    } else {
                        warn!("Tried to unlock skill for skill group with no available SP");
                    }
                } else {
                    warn!("Tried to unlock already unlocked skill");
                }
            } else {
                warn!("Tried to unlock skill for a skill group that player does not have");
            }
        } else {
            warn!("Tried to unlock skill that does not exist in any skill group!");
        }
    }
}
