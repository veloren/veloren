use crate::{
    assets::{self, Asset, AssetExt},
    comp::{
        item::tool::ToolKind,
        skills::{GeneralSkill, Skill},
    },
};
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::hash::Hash;
use tracing::{trace, warn};

pub mod skills;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillTreeMap(HashMap<SkillGroupKind, HashSet<Skill>>);

impl Asset for SkillTreeMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

pub struct SkillGroupDef {
    pub skills: HashSet<Skill>,
    pub total_skill_point_cost: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillLevelMap(HashMap<Skill, Option<u16>>);

impl Asset for SkillLevelMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillPrerequisitesMap(HashMap<Skill, HashMap<Skill, Option<u16>>>);

impl Asset for SkillPrerequisitesMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

lazy_static! {
    // Determines the skills that comprise each skill group.
    //
    // This data is used to determine which of a player's skill groups a
    // particular skill should be added to when a skill unlock is requested.
    pub static ref SKILL_GROUP_DEFS: HashMap<SkillGroupKind, SkillGroupDef> = {
        let map = SkillTreeMap::load_expect_cloned(
            "common.skill_trees.skills_skill-groups_manifest",
        ).0;
        map.iter().map(|(sgk, skills)|
            (*sgk, SkillGroupDef { skills: skills.clone(),
                total_skill_point_cost: skills
                    .iter()
                    .map(|skill| {
                        if let Some(max_level) = skill.max_level() {
                            (1..=max_level)
                                .into_iter()
                                .map(|level| skill.skill_cost(Some(level)))
                                .sum()
                        } else {
                            skill.skill_cost(None)
                        }
                    })
                    .sum()
            })
        )
        .collect()
    };
    // Creates a hashmap for the reverse lookup of skill groups from a skill
    pub static ref SKILL_GROUP_LOOKUP: HashMap<Skill, SkillGroupKind> = {
        let map = SkillTreeMap::load_expect_cloned(
            "common.skill_trees.skills_skill-groups_manifest",
        ).0;
        map.iter().flat_map(|(sgk, skills)| skills.into_iter().map(move |s| (*s, *sgk))).collect()
    };
    // Loads the maximum level that a skill can obtain
    pub static ref SKILL_MAX_LEVEL: HashMap<Skill, Option<u16>> = {
        SkillLevelMap::load_expect_cloned(
            "common.skill_trees.skill_max_levels",
        ).0
    };
    // Loads the prerequisite skills for a particular skill
    pub static ref SKILL_PREREQUISITES: HashMap<Skill, HashMap<Skill, Option<u16>>> = {
        SkillPrerequisitesMap::load_expect_cloned(
            "common.skill_trees.skill_prerequisites",
        ).0
    };
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillGroupKind {
    General,
    Weapon(ToolKind),
}

impl SkillGroupKind {
    /// Gets the cost in experience of earning a skill point
    pub fn skill_point_cost(self, level: u16) -> u32 {
        const EXP_INCREMENT: f32 = 10.0;
        const STARTING_EXP: f32 = 70.0;
        const EXP_CEILING: f32 = 1000.0;
        const SCALING_FACTOR: f32 = 0.125;
        (EXP_INCREMENT
            * (EXP_CEILING
                / EXP_INCREMENT
                / (1.0
                    + std::f32::consts::E.powf(-SCALING_FACTOR * level as f32)
                        * (EXP_CEILING / STARTING_EXP - 1.0)))
                .floor()) as u32
    }

    /// Gets the total amount of skill points that can be spent in a particular
    /// skill group
    pub fn total_skill_point_cost(self) -> u16 {
        if let Some(SkillGroupDef {
            total_skill_point_cost,
            ..
        }) = SKILL_GROUP_DEFS.get(&self)
        {
            *total_skill_point_cost
        } else {
            0
        }
    }
}

/// A group of skills that have been unlocked by a player. Each skill group has
/// independent exp and skill points which are used to unlock skills in that
/// skill group.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SkillGroup {
    pub skill_group_kind: SkillGroupKind,
    // How much exp has been used for skill points
    pub spent_exp: u32,
    // How much exp has been earned in total
    pub earned_exp: u32,
    pub available_sp: u16,
    pub earned_sp: u16,
    // Used for persistence
    pub ordered_skills: Vec<Skill>,
}

impl SkillGroup {
    fn new(skill_group_kind: SkillGroupKind) -> SkillGroup {
        SkillGroup {
            skill_group_kind,
            spent_exp: 0,
            earned_exp: 0,
            available_sp: 0,
            earned_sp: 0,
            ordered_skills: Vec::new(),
        }
    }

    /// Returns the available experience that could be used to earn another
    /// skill point in a particular skill group.
    pub fn available_experience(&self) -> u32 { self.earned_exp - self.spent_exp }

    /// Adds a skill point while subtracting the necessary amount of experience
    pub fn earn_skill_point(&mut self) -> Result<(), SpRewardError> {
        let sp_cost = self.skill_group_kind.skill_point_cost(self.earned_sp);
        if self.available_experience() >= sp_cost {
            self.spent_exp = self.spent_exp.saturating_add(sp_cost);
            self.available_sp = self.available_sp.saturating_add(1);
            self.earned_sp = self.earned_sp.saturating_add(1);
            Ok(())
        } else {
            Err(SpRewardError::InsufficientExp)
        }
    }
}

/// Contains all of a player's skill groups and skills. Provides methods for
/// manipulating assigned skills and skill groups including unlocking skills,
/// refunding skills etc.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillSet {
    pub skill_groups: Vec<SkillGroup>,
    pub skills: HashMap<Skill, Option<u16>>,
    pub modify_health: bool,
    pub modify_energy: bool,
}

impl Component for SkillSet {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

impl Default for SkillSet {
    /// Instantiate a new skill set with the default skill groups with no
    /// unlocked skills in them - used when adding a skill set to a new
    /// player
    fn default() -> Self {
        Self {
            skill_groups: vec![
                SkillGroup::new(SkillGroupKind::General),
                SkillGroup::new(SkillGroupKind::Weapon(ToolKind::Pick)),
            ],
            skills: HashMap::new(),
            modify_health: false,
            modify_energy: false,
        }
    }
}

impl SkillSet {
    /// Checks if the skill set of an entity contains a particular skill group
    /// type
    pub fn contains_skill_group(&self, skill_group_kind: SkillGroupKind) -> bool {
        self.skill_groups
            .iter()
            .any(|x| x.skill_group_kind == skill_group_kind)
    }

    ///  Unlocks a skill group for a player. It starts with 0 exp and 0 skill
    ///  points.
    pub fn unlock_skill_group(&mut self, skill_group_kind: SkillGroupKind) {
        if !self.contains_skill_group(skill_group_kind) {
            self.skill_groups.push(SkillGroup::new(skill_group_kind));
        } else {
            warn!("Tried to unlock already known skill group");
        }
    }

    /// Returns a reference to a particular skill group in a skillset
    fn skill_group(&self, skill_group: SkillGroupKind) -> Option<&SkillGroup> {
        self.skill_groups
            .iter()
            .find(|s_g| s_g.skill_group_kind == skill_group)
    }

    /// Returns a mutable reference to a particular skill group in a skillset
    fn skill_group_mut(&mut self, skill_group: SkillGroupKind) -> Option<&mut SkillGroup> {
        self.skill_groups
            .iter_mut()
            .find(|s_g| s_g.skill_group_kind == skill_group)
    }

    /// Adds experience to the skill group within an entity's skill set
    pub fn add_experience(&mut self, skill_group_kind: SkillGroupKind, amount: u32) {
        if let Some(mut skill_group) = self.skill_group_mut(skill_group_kind) {
            skill_group.earned_exp = skill_group.earned_exp.saturating_add(amount);
        } else {
            warn!("Tried to add experience to a skill group that player does not have");
        }
    }

    /// Gets the available experience for a particular skill group
    pub fn available_experience(&self, skill_group: SkillGroupKind) -> u32 {
        self.skill_group(skill_group)
            .map_or(0, |s_g| s_g.available_experience())
    }

    /// Checks how much experience is needed for the next skill point in a tree
    pub fn skill_point_cost(&self, skill_group: SkillGroupKind) -> u32 {
        if let Some(level) = self.skill_group(skill_group).map(|sg| sg.earned_sp) {
            skill_group.skill_point_cost(level)
        } else {
            skill_group.skill_point_cost(0)
        }
    }

    /// Adds skill points to a skill group as long as the player has that skill
    /// group type.
    pub fn add_skill_points(
        &mut self,
        skill_group_kind: SkillGroupKind,
        number_of_skill_points: u16,
    ) {
        if let Some(mut skill_group) = self.skill_group_mut(skill_group_kind) {
            skill_group.available_sp = skill_group
                .available_sp
                .saturating_add(number_of_skill_points);
            skill_group.earned_sp = skill_group.earned_sp.saturating_add(number_of_skill_points);
        } else {
            warn!("Tried to add skill points to a skill group that player does not have");
        }
    }

    /// Adds a skill point while subtracting the necessary amount of experience
    pub fn earn_skill_point(
        &mut self,
        skill_group_kind: SkillGroupKind,
    ) -> Result<(), SpRewardError> {
        if let Some(skill_group) = self.skill_group_mut(skill_group_kind) {
            skill_group.earn_skill_point()
        } else {
            Err(SpRewardError::UnavailableSkillGroup)
        }
    }

    /// Gets the available points for a particular skill group
    pub fn available_sp(&self, skill_group: SkillGroupKind) -> u16 {
        self.skill_group(skill_group)
            .map_or(0, |s_g| s_g.available_sp)
    }

    /// Gets the total earned points for a particular skill group
    pub fn earned_sp(&self, skill_group: SkillGroupKind) -> u16 {
        self.skill_group(skill_group).map_or(0, |s_g| s_g.earned_sp)
    }

    /// Checks that the skill set contains all prerequisite skills of the required level for a particular skill
    pub fn prerequisites_met(&self, skill: Skill) -> bool {
        skill
            .prerequisite_skills()
            .all(|(s, l)| self.skill_level(s).map_or(false, |l_b| l_b >= l))
    }

    /// Gets skill point cost to purchase skill of next level
    pub fn skill_cost(&self, skill: Skill) -> u16 {
        let next_level = self.next_skill_level(skill);
        skill.skill_cost(next_level)
    }

    /// Checks if player has sufficient skill points to purchase a skill
    pub fn sufficient_skill_points(&self, skill: Skill) -> bool {
        if let Some(skill_group_kind) = skill.skill_group_kind() {
            if let Some(skill_group) = self
                .skill_groups
                .iter()
                .find(|x| x.skill_group_kind == skill_group_kind)
            {
                let needed_sp = self.skill_cost(skill);
                skill_group.available_sp >= needed_sp
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Checks the next level of a skill
    fn next_skill_level(&self, skill: Skill) -> Option<u16> {
        if let Ok(level) = self.skill_level(skill) {
            // If already has skill, and that skill has levels, level + 1
            level.map(|l| l + 1)
        } else {
            // Else if the skill has levels, 1
            skill.max_level().map(|_| 1)
        }
    }

    /// Unlocks a skill for a player, assuming they have the relevant skill
    /// group unlocked and available SP in that skill group.
    pub fn unlock_skill(&mut self, skill: Skill) {
        if let Some(skill_group_kind) = skill.skill_group_kind() {
            let next_level = self.next_skill_level(skill);
            let prerequisites_met = self.prerequisites_met(skill);
            // Check that skill is not yet at max level
            if !matches!(self.skills.get(&skill), Some(level) if *level == skill.max_level()) {
                if let Some(mut skill_group) = self.skill_group_mut(skill_group_kind) {
                    if prerequisites_met {
                        if skill_group.available_sp >= skill.skill_cost(next_level) {
                            skill_group.available_sp -= skill.skill_cost(next_level);
                            skill_group.ordered_skills.push(skill);
                            match skill {
                                Skill::UnlockGroup(group) => {
                                    self.unlock_skill_group(group);
                                },
                                Skill::General(GeneralSkill::HealthIncrease) => {
                                    self.modify_health = true;
                                },
                                Skill::General(GeneralSkill::EnergyIncrease) => {
                                    self.modify_energy = true;
                                },
                                _ => {},
                            }
                            self.skills.insert(skill, next_level);
                        } else {
                            trace!("Tried to unlock skill for skill group with insufficient SP");
                        }
                    } else {
                        trace!("Tried to unlock skill without meeting prerequisite skills");
                    }
                } else {
                    trace!("Tried to unlock skill for a skill group that player does not have");
                }
            } else {
                trace!("Tried to unlock skill the player already has")
            }
        } else {
            warn!(
                ?skill,
                "Tried to unlock skill that does not exist in any skill group!"
            );
        }
    }

    /// Checks if the player has available SP to spend
    pub fn has_available_sp(&self) -> bool {
        self.skill_groups.iter().any(|sg| {
            sg.available_sp > 0
                && (sg.earned_sp - sg.available_sp) < sg.skill_group_kind.total_skill_point_cost()
        })
    }

    /// Checks if the skill is at max level in a skill set
    pub fn is_at_max_level(&self, skill: Skill) -> bool {
        if let Ok(level) = self.skill_level(skill) {
            level == skill.max_level()
        } else {
            false
        }
    }

    /// Checks if skill set contains a skill
    pub fn has_skill(&self, skill: Skill) -> bool { self.skills.contains_key(&skill) }

    /// Returns the level of the skill
    pub fn skill_level(&self, skill: Skill) -> Result<Option<u16>, SkillError> {
        if let Some(level) = self.skills.get(&skill).copied() {
            Ok(level)
        } else {
            Err(SkillError::MissingSkill)
        }
    }

    /// Returns the level of the skill or passed value as default
    pub fn skill_level_or(&self, skill: Skill, default: u16) -> u16 {
        if let Ok(Some(level)) = self.skill_level(skill) {
            level
        } else {
            default
        }
    }
}

pub enum SkillError {
    MissingSkill,
}

pub enum SpRewardError {
    InsufficientExp,
    UnavailableSkillGroup,
}

#[cfg(test)]
mod tests {
    use super::*;
    // Code reviewers: Open a comment here, I want to refactor these tests

    #[test]
    fn test_refund_skill() {
        let mut skillset = SkillSet::default();
        skillset.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Axe));
        skillset.add_skill_points(SkillGroupKind::Weapon(ToolKind::Axe), 1);
        skillset.unlock_skill(Skill::Axe(AxeSkill::UnlockLeap));

        assert_eq!(skillset.skill_groups[2].available_sp, 0);
        assert_eq!(skillset.skills.len(), 1);
        assert!(skillset.has_skill(Skill::Axe(AxeSkill::UnlockLeap)));

        skillset.refund_skill(Skill::Axe(AxeSkill::UnlockLeap));

        assert_eq!(skillset.skill_groups[2].available_sp, 1);
        assert_eq!(skillset.skills.get(&Skill::Axe(AxeSkill::UnlockLeap)), None);
    }

    #[test]
    fn test_unlock_skillgroup() {
        let mut skillset = SkillSet::default();
        skillset.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Axe));

        assert_eq!(skillset.skill_groups.len(), 3);
        assert_eq!(
            skillset.skill_groups[2],
            SkillGroup::new(SkillGroupKind::Weapon(ToolKind::Axe))
        );
    }

    #[test]
    fn test_unlock_skill() {
        let mut skillset = SkillSet::default();

        skillset.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Axe));
        skillset.add_skill_points(SkillGroupKind::Weapon(ToolKind::Axe), 1);

        assert_eq!(skillset.skill_groups[2].available_sp, 1);
        assert_eq!(skillset.skills.len(), 0);

        // Try unlocking a skill with enough skill points
        skillset.unlock_skill(Skill::Axe(AxeSkill::UnlockLeap));

        assert_eq!(skillset.skill_groups[2].available_sp, 0);
        assert_eq!(skillset.skills.len(), 1);
        assert!(skillset.has_skill(Skill::Axe(AxeSkill::UnlockLeap)));

        // Try unlocking a skill without enough skill points
        skillset.unlock_skill(Skill::Axe(AxeSkill::DsCombo));

        assert_eq!(skillset.skills.len(), 1);
        assert_eq!(skillset.skills.get(&Skill::Axe(AxeSkill::DsCombo)), None);
    }

    #[test]
    fn test_add_skill_points() {
        let mut skillset = SkillSet::default();
        skillset.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Axe));
        skillset.add_skill_points(SkillGroupKind::Weapon(ToolKind::Axe), 1);

        assert_eq!(skillset.skill_groups[2].available_sp, 1);
    }
}
