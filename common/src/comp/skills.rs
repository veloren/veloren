use crate::{
    assets::{self, Asset, AssetExt},
    comp::item::tool::ToolKind,
};
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use tracing::warn;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillTreeMap(HashMap<SkillGroupType, HashSet<Skill>>);

impl Asset for SkillTreeMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillLevelMap(HashMap<Skill, Level>);

impl Asset for SkillLevelMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillPrerequisitesMap(HashMap<Skill, HashMap<Skill, Level>>);

impl Asset for SkillPrerequisitesMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

lazy_static! {
    // Determines the skills that comprise each skill group - this data is used to determine
    // which of a player's skill groups a particular skill should be added to when a skill unlock
    // is requested.
    pub static ref SKILL_GROUP_DEFS: HashMap<SkillGroupType, HashSet<Skill>> = {
        SkillTreeMap::load_expect_cloned(
            "common.skill_trees.skills_skill-groups_manifest",
        ).0
    };
    // Loads the maximum level that a skill can obtain
    pub static ref SKILL_MAX_LEVEL: HashMap<Skill, Level> = {
        SkillLevelMap::load_expect_cloned(
            "common.skill_trees.skill_max_levels",
        ).0
    };
    // Loads the prerequisite skills for a particular skill
    pub static ref SKILL_PREREQUISITES: HashMap<Skill, HashMap<Skill, Level>> = {
        SkillPrerequisitesMap::load_expect_cloned(
            "common.skill_trees.skill_prerequisites",
        ).0
    };
}

/// Represents a skill that a player can unlock, that either grants them some
/// kind of active ability, or a passive effect etc. Obviously because this is
/// an enum it doesn't describe what the skill actually -does-, this will be
/// handled by dedicated ECS systems.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Skill {
    General(GeneralSkill),
    Sword(SwordSkill),
    Axe(AxeSkill),
    Hammer(HammerSkill),
    Bow(BowSkill),
    Staff(StaffSkill),
    Sceptre(SceptreSkill),
    UnlockGroup(SkillGroupType),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
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
    DInfinite,
    // Spin upgrades
    SUnlockSpin,
    SDamage,
    SSpeed,
    SCost,
    SSpins,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
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
    LUnlockLeap,
    LDamage,
    LKnockback,
    LCost,
    LDistance,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
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
    LUnlockLeap,
    LDamage,
    LCost,
    LDistance,
    LKnockback,
    LRange,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum BowSkill {
    // Passives
    ProjSpeed,
    // Basic ranged upgrades
    BDamage,
    BRegen,
    // Charged ranged upgrades
    CDamage,
    CKnockback,
    CProjSpeed,
    CDrain,
    CSpeed,
    CMove,
    // Repeater upgrades
    UnlockRepeater,
    RDamage,
    RLeap,
    RArrows,
    RCost,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaffSkill {
    // Basic ranged upgrades
    BExplosion,
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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceptreSkill {
    Unlock404,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralSkill {
    HealthIncrease,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillGroupType {
    General,
    Weapon(ToolKind),
}

/// A group of skills that have been unlocked by a player. Each skill group has
/// independent exp and skill points which are used to unlock skills in that
/// skill group.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SkillGroup {
    pub skill_group_type: SkillGroupType,
    pub exp: u16,
    pub available_sp: u16,
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
    pub skills: HashMap<Skill, Level>,
}

pub type Level = Option<u16>;

impl Default for SkillSet {
    /// Instantiate a new skill set with the default skill groups with no
    /// unlocked skills in them - used when adding a skill set to a new
    /// player
    fn default() -> Self {
        Self {
            skill_groups: vec![SkillGroup::new(SkillGroupType::General)],
            skills: HashMap::new(),
        }
    }
}

impl SkillSet {
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
        if let Some(skill_group_type) = SkillSet::get_skill_group_type_for_skill(&skill) {
            let next_level = if self.skills.contains_key(&skill) {
                self.skills.get(&skill).copied().flatten().map(|l| l + 1)
            } else {
                skill.get_max_level().map(|_| 1)
            };
            let prerequisites_met = self.prerequisites_met(skill, next_level);
            if !matches!(self.skills.get(&skill), Some(&None)) {
                if let Some(mut skill_group) = self
                    .skill_groups
                    .iter_mut()
                    .find(|x| x.skill_group_type == skill_group_type)
                {
                    if prerequisites_met {
                        if skill_group.available_sp >= skill.skill_cost(next_level) {
                            skill_group.available_sp -= skill.skill_cost(next_level);
                            if let Skill::UnlockGroup(group) = skill {
                                self.unlock_skill_group(group);
                            }
                            self.skills.insert(skill, next_level);
                        } else {
                            warn!("Tried to unlock skill for skill group with insufficient SP");
                        }
                    } else {
                        warn!("Tried to unlock skill without meeting prerequisite skills");
                    }
                } else {
                    warn!("Tried to unlock skill for a skill group that player does not have");
                }
            } else {
                warn!("Tried to unlock skill the player already has")
            }
        } else {
            warn!(
                ?skill,
                "Tried to unlock skill that does not exist in any skill group!"
            );
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
        if self.skills.contains_key(&skill) {
            if let Some(skill_group_type) = SkillSet::get_skill_group_type_for_skill(&skill) {
                if let Some(mut skill_group) = self
                    .skill_groups
                    .iter_mut()
                    .find(|x| x.skill_group_type == skill_group_type)
                {
                    // We know key is already contained, so unwrap is safe
                    let level = *(self.skills.get(&skill).unwrap());
                    skill_group.available_sp += skill.skill_cost(level);
                    if level.map_or(false, |l| l > 1) {
                        self.skills.insert(skill, level.map(|l| l - 1));
                    } else {
                        self.skills.remove(&skill);
                    }
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
        number_of_skill_points: u16,
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

    /// Checks if the skill set of an entity contains a particular skill group
    /// type
    pub fn contains_skill_group(&self, skill_group_type: SkillGroupType) -> bool {
        self.skill_groups
            .iter()
            .any(|x| x.skill_group_type == skill_group_type)
    }

    /// Adds/subtracts experience to the skill group within an entity's skill
    /// set
    pub fn change_experience(&mut self, skill_group_type: SkillGroupType, amount: i32) {
        if let Some(mut skill_group) = self
            .skill_groups
            .iter_mut()
            .find(|x| x.skill_group_type == skill_group_type)
        {
            skill_group.exp = (skill_group.exp as i32 + amount) as u16;
        } else {
            warn!("Tried to add experience to a skill group that player does not have");
        }
    }

    /// Checks that the skill set contains all prerequisite skills for a
    /// particular skill
    pub fn prerequisites_met(&self, skill: Skill, level: Level) -> bool {
        skill.prerequisite_skills(level).iter().all(|(s, l)| {
            self.skills.contains_key(s) && self.skills.get(s).map_or(false, |l_b| l_b >= l)
        })
    }
}

impl Skill {
    /// Returns a vec of prerequisite skills (it should only be necessary to
    /// note direct prerequisites)
    pub fn prerequisite_skills(self, level: Level) -> HashMap<Skill, Level> {
        let mut prerequisites = HashMap::new();
        if let Some(level) = level {
            if level > self.get_max_level().unwrap_or(0) {
                // Sets a prerequisite of itself for skills beyond the max level
                prerequisites.insert(self, Some(level));
            } else if level > 1 {
                // For skills above level 1, sets prerequisite of skill of lower level
                prerequisites.insert(self, Some(level - 1));
            }
        }
        if let Some(skills) = SKILL_PREREQUISITES.get(&self) {
            prerequisites.extend(skills);
        }
        prerequisites
    }

    /// Returns the cost in skill points of unlocking a particular skill
    pub fn skill_cost(self, level: Level) -> u16 {
        use Skill::*;
        match self {
            General(GeneralSkill::HealthIncrease) => 1,
            _ => level.unwrap_or(1),
        }
    }

    /// Returns the maximum level a skill can reach, returns None if the skill
    /// doesn't level
    pub fn get_max_level(self) -> Option<u16> { SKILL_MAX_LEVEL.get(&self).copied().flatten() }
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
