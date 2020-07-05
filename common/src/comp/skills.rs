use lazy_static::lazy_static;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};
use tracing::warn;

lazy_static! {
    // Determines the skills that comprise each skill group - this data is used to determine
    // which of a player's skill groups a particular skill should be added to when a skill unlock
    // is requested. TODO: Externalise this data in a RON file for ease of modification
    pub static ref SKILL_GROUP_DEFS: HashMap<SkillGroupType, HashSet<Skill>> = {
        let mut defs = HashMap::new();
        defs.insert(SkillGroupType::T1, [ Skill::TestT1Skill1,
                                         Skill::TestT1Skill2,
                                         Skill::TestT1Skill3,
                                         Skill::TestT1Skill4,
                                         Skill::TestT1Skill5]
                                         .iter().cloned().collect::<HashSet<Skill>>());

        defs.insert(SkillGroupType::Swords, [ Skill::TestSwordSkill1,
                                         Skill::TestSwordSkill2,
                                         Skill::TestSwordSkill3]
                                         .iter().cloned().collect::<HashSet<Skill>>());

        defs.insert(SkillGroupType::Axes, [ Skill::TestAxeSkill1,
                                         Skill::TestAxeSkill2,
                                         Skill::TestAxeSkill3]
                                         .iter().cloned().collect::<HashSet<Skill>>());

        defs
    };
}

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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillGroup {
    pub skill_group_type: SkillGroupType,
    pub exp: u32,
    pub available_sp: u8,
}

impl SkillGroup {
    fn new(skill_group_type: SkillGroupType) -> SkillGroup {
        SkillGroup {
            skill_group_type,
            exp: 0,
            available_sp: 0,
        }
    }
}

/// Contains all of a player's skill groups and skills. Provides methods for
/// manipulating assigned skills and skill groups including unlocking skills,
/// refunding skills etc.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillSet {
    pub skill_groups: Vec<SkillGroup>,
    pub skills: HashSet<Skill>,
}

impl Default for SkillSet {
    /// Instantiate a new skill set with the default skill groups with no
    /// unlocked skills in them - used when adding a skill set to a new
    /// player
    fn default() -> Self {
        // TODO: Default skill groups for new players?
        Self {
            skill_groups: Vec::new(),
            skills: HashSet::new(),
        }
    }
}

impl SkillSet {
    pub fn new() -> Self {
        Self {
            skill_groups: Vec::new(),
            skills: HashSet::new(),
        }
    }

    // TODO: Game design to determine how skill groups are unlocked
    ///  Unlocks a skill group for a player. It starts with 0 exp and 0 skill
    ///  points.
    ///
    /// ```
    /// use veloren_common::comp::skills::{SkillGroupType, SkillSet};
    ///
    /// let mut skillset = SkillSet::new();
    /// skillset.unlock_skill_group(SkillGroupType::Axes);
    ///
    /// assert_eq!(skillset.skill_groups.len(), 1);
    /// ```
    pub fn unlock_skill_group(&mut self, skill_group_type: SkillGroupType) {
        if !self
            .skill_groups
            .iter()
            .any(|x| x.skill_group_type == skill_group_type)
        {
            self.skill_groups.push(SkillGroup::new(skill_group_type));
        } else {
            warn!("Tried to unlock already known skill group");
        }
    }

    /// Unlocks a skill for a player, assuming they have the relevant skill
    /// group unlocked and available SP in that skill group.
    ///
    /// ```
    /// use veloren_common::comp::skills::{Skill, SkillGroupType, SkillSet};
    ///
    /// let mut skillset = SkillSet::new();
    /// skillset.unlock_skill_group(SkillGroupType::Axes);
    /// skillset.add_skill_points(SkillGroupType::Axes, 1);
    ///
    /// skillset.unlock_skill(Skill::TestAxeSkill2);
    ///
    /// assert_eq!(skillset.skills.len(), 1);
    /// ```
    pub fn unlock_skill(&mut self, skill: Skill) {
        if !self.skills.contains(&skill) {
            if let Some(skill_group_type) = SkillSet::get_skill_group_type_for_skill(&skill) {
                if let Some(mut skill_group) = self
                    .skill_groups
                    .iter_mut()
                    .find(|x| x.skill_group_type == skill_group_type)
                {
                    if skill_group.available_sp > 0 {
                        skill_group.available_sp -= 1;
                        self.skills.insert(skill);
                    } else {
                        warn!("Tried to unlock skill for skill group with no available SP");
                    }
                } else {
                    warn!("Tried to unlock skill for a skill group that player does not have");
                }
            } else {
                warn!(
                    ?skill,
                    "Tried to unlock skill that does not exist in any skill group!"
                );
            }
        } else {
            warn!("Tried to unlock already unlocked skill");
        }
    }

    /// Removes a skill from a player and refunds 1 skill point in the relevant
    /// skill group.
    ///
    /// ```
    /// use veloren_common::comp::skills::{Skill, SkillGroupType, SkillSet};
    ///
    /// let mut skillset = SkillSet::new();
    /// skillset.unlock_skill_group(SkillGroupType::Axes);
    /// skillset.add_skill_points(SkillGroupType::Axes, 1);
    /// skillset.unlock_skill(Skill::TestAxeSkill2);
    ///
    /// skillset.refund_skill(Skill::TestAxeSkill2);
    ///
    /// assert_eq!(skillset.skills.len(), 0);
    /// ```
    pub fn refund_skill(&mut self, skill: Skill) {
        if self.skills.contains(&skill) {
            if let Some(skill_group_type) = SkillSet::get_skill_group_type_for_skill(&skill) {
                if let Some(mut skill_group) = self
                    .skill_groups
                    .iter_mut()
                    .find(|x| x.skill_group_type == skill_group_type)
                {
                    skill_group.available_sp += 1;
                    self.skills.remove(&skill);
                } else {
                    warn!("Tried to refund skill for a skill group that player does not have");
                }
            } else {
                warn!(
                    ?skill,
                    "Tried to refund skill that does not exist in any skill group"
                )
            }
        } else {
            warn!("Tried to refund skill that has not been unlocked");
        }
    }

    /// Returns the skill group type for a skill from the static skill group
    /// definitions.
    fn get_skill_group_type_for_skill(skill: &Skill) -> Option<SkillGroupType> {
        SKILL_GROUP_DEFS.iter().find_map(|(key, val)| {
            if val.contains(&skill) {
                Some(*key)
            } else {
                None
            }
        })
    }

    /// Adds skill points to a skill group as long as the player has that skill
    /// group type.
    ///
    /// ```
    /// use veloren_common::comp::skills::{SkillGroupType, SkillSet};
    ///
    /// let mut skillset = SkillSet::new();
    /// skillset.unlock_skill_group(SkillGroupType::Axes);
    /// skillset.add_skill_points(SkillGroupType::Axes, 1);
    ///
    /// assert_eq!(skillset.skill_groups[0].available_sp, 1);
    /// ```
    pub fn add_skill_points(
        &mut self,
        skill_group_type: SkillGroupType,
        number_of_skill_points: u8,
    ) {
        if let Some(mut skill_group) = self
            .skill_groups
            .iter_mut()
            .find(|x| x.skill_group_type == skill_group_type)
        {
            skill_group.available_sp += number_of_skill_points;
        } else {
            warn!("Tried to add skill points to a skill group that player does not have");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refund_skill() {
        let mut skillset = SkillSet::new();
        skillset.unlock_skill_group(SkillGroupType::Axes);
        skillset.add_skill_points(SkillGroupType::Axes, 1);
        skillset.unlock_skill(Skill::TestAxeSkill2);

        assert_eq!(skillset.skill_groups[0].available_sp, 0);
        assert_eq!(skillset.skills.len(), 1);
        assert_eq!(
            skillset.skills.get(&Skill::TestAxeSkill2),
            Some(&Skill::TestAxeSkill2)
        );

        skillset.refund_skill(Skill::TestAxeSkill2);

        assert_eq!(skillset.skill_groups[0].available_sp, 1);
        assert_eq!(skillset.skills.get(&Skill::TestAxeSkill2), None);
    }

    #[test]
    fn test_unlock_skillgroup() {
        let mut skillset = SkillSet::new();
        skillset.unlock_skill_group(SkillGroupType::Axes);

        assert_eq!(skillset.skill_groups.len(), 1);
        assert_eq!(
            skillset.skill_groups[0],
            SkillGroup::new(SkillGroupType::Axes)
        );
    }

    #[test]
    fn test_unlock_skill() {
        let mut skillset = SkillSet::new();

        skillset.unlock_skill_group(SkillGroupType::Axes);
        skillset.add_skill_points(SkillGroupType::Axes, 1);

        assert_eq!(skillset.skill_groups[0].available_sp, 1);
        assert_eq!(skillset.skills.len(), 0);

        // Try unlocking a skill with enough skill points
        skillset.unlock_skill(Skill::TestAxeSkill2);

        assert_eq!(skillset.skill_groups[0].available_sp, 0);
        assert_eq!(skillset.skills.len(), 1);
        assert_eq!(
            skillset.skills.get(&Skill::TestAxeSkill2),
            Some(&Skill::TestAxeSkill2)
        );

        // Try unlocking a skill without enough skill points
        skillset.unlock_skill(Skill::TestAxeSkill1);

        assert_eq!(skillset.skills.len(), 1);
        assert_eq!(skillset.skills.get(&Skill::TestAxeSkill1), None);
    }

    #[test]
    fn test_add_skill_points() {
        let mut skillset = SkillSet::new();
        skillset.unlock_skill_group(SkillGroupType::Axes);
        skillset.add_skill_points(SkillGroupType::Axes, 1);

        assert_eq!(skillset.skill_groups[0].available_sp, 1);
    }
}
