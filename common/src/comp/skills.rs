use crate::{
    assets::{self, Asset, AssetExt},
    comp::item::tool::ToolKind,
};
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use tracing::{trace, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillTreeMap(HashMap<SkillGroupType, HashSet<Skill>>);

impl Asset for SkillTreeMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
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
    // Determines the skills that comprise each skill group - this data is used to determine
    // which of a player's skill groups a particular skill should be added to when a skill unlock
    // is requested.
    pub static ref SKILL_GROUP_DEFS: HashMap<SkillGroupType, HashSet<Skill>> = {
        SkillTreeMap::load_expect_cloned(
            "common.skill_trees.skills_skill-groups_manifest",
        ).0
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
    Roll(RollSkill),
}

pub enum SkillError {
    MissingSkill,
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
    UnlockSpin,
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
    UnlockLeap,
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
    UnlockLeap,
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
    RGlide,
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
    // Beam  upgrades
    BHeal,
    BDamage,
    BRange,
    BLifesteal,
    BRegen,
    BCost,
    // Projectile upgrades
    PHeal,
    PDamage,
    PRadius,
    PCost,
    PProjSpeed,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralSkill {
    HealthIncrease,
    EnergyIncrease,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollSkill {
    ImmuneMelee,
    Cost,
    Strength,
    Duration,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillGroupType {
    General,
    Weapon(ToolKind),
}

impl SkillGroupType {
    /// Gets the cost in experience of earning a skill point
    #[allow(clippy::many_single_char_names)]
    pub fn skill_point_cost(self, level: u16) -> u16 {
        let exp_increment = 10.0;
        let starting_exp = 100.0;
        let exp_ceiling = 1000.0;
        let scaling_factor = 0.1;
        (exp_increment
            * (exp_ceiling
                / exp_increment
                / (1.0
                    + std::f32::consts::E.powf(-scaling_factor * level as f32)
                        * (exp_ceiling / starting_exp - 1.0)))
                .floor()) as u16
    }

    /// Gets the total amount of skill points that can be spent in a particular
    /// skill group
    pub fn max_skill_points(self) -> u16 {
        if let Some(skill_list) = SKILL_GROUP_DEFS.get(&self) {
            skill_list
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
    pub skill_group_type: SkillGroupType,
    pub exp: u16,
    pub available_sp: u16,
    pub earned_sp: u16,
}

impl SkillGroup {
    fn new(skill_group_type: SkillGroupType) -> SkillGroup {
        SkillGroup {
            skill_group_type,
            exp: 0,
            available_sp: 0,
            earned_sp: 0,
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

impl Default for SkillSet {
    /// Instantiate a new skill set with the default skill groups with no
    /// unlocked skills in them - used when adding a skill set to a new
    /// player
    fn default() -> Self {
        Self {
            skill_groups: vec![SkillGroup::new(SkillGroupType::General)],
            skills: HashMap::new(),
            modify_health: false,
            modify_energy: false,
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
        if let Some(skill_group_type) = skill.skill_group_type() {
            let next_level = self.next_skill_level(skill);
            let prerequisites_met = self.prerequisites_met(skill);
            if !matches!(self.skills.get(&skill), Some(level) if *level == skill.max_level()) {
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
                            if matches!(skill, Skill::General(GeneralSkill::HealthIncrease)) {
                                self.modify_health = true;
                            }
                            if matches!(skill, Skill::General(GeneralSkill::EnergyIncrease)) {
                                self.modify_energy = true;
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
        if self.has_skill(skill) {
            if let Some(skill_group_type) = skill.skill_group_type() {
                if let Some(mut skill_group) = self
                    .skill_groups
                    .iter_mut()
                    .find(|x| x.skill_group_type == skill_group_type)
                {
                    let level = self.skills.get(&skill).copied();
                    if let Some(level) = level {
                        skill_group.available_sp += skill.skill_cost(level);
                        if level.map_or(false, |l| l > 1) {
                            self.skills.insert(skill, level.map(|l| l - 1));
                        } else {
                            self.skills.remove(&skill);
                        }
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
            skill_group.available_sp = skill_group
                .available_sp
                .saturating_add(number_of_skill_points);
            skill_group.earned_sp = skill_group.earned_sp.saturating_add(number_of_skill_points);
        } else {
            warn!("Tried to add skill points to a skill group that player does not have");
        }
    }

    /// Adds a skill point while subtracting the necessary amount of experience
    pub fn earn_skill_point(&mut self, skill_group_type: SkillGroupType) {
        let sp_cost = self.skill_point_cost(skill_group_type);
        if let Some(mut skill_group) = self
            .skill_groups
            .iter_mut()
            .find(|x| x.skill_group_type == skill_group_type)
        {
            skill_group.exp = skill_group.exp.saturating_sub(sp_cost);
            skill_group.available_sp = skill_group.available_sp.saturating_add(1);
            skill_group.earned_sp = skill_group.earned_sp.saturating_add(1);
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
    pub fn prerequisites_met(&self, skill: Skill) -> bool {
        let next_level = self.next_skill_level(skill);
        skill.prerequisite_skills(next_level).iter().all(|(s, l)| {
            self.skill_level(*s).map_or(false, |l_b| l_b >= *l)
            /* self.has_skill(*s) && self.skills.get(s).map_or(false, |l_b|
             * l_b >= l) */
        })
    }

    /// Gets the available points for a particular skill group
    pub fn available_sp(&self, skill_group: SkillGroupType) -> u16 {
        self.skill_groups
            .iter()
            .find(|s_g| s_g.skill_group_type == skill_group)
            .map_or(0, |s_g| s_g.available_sp)
    }

    /// Gets the total earned points for a particular skill group
    pub fn earned_sp(&self, skill_group: SkillGroupType) -> u16 {
        self.skill_groups
            .iter()
            .find(|s_g| s_g.skill_group_type == skill_group)
            .map_or(0, |s_g| s_g.earned_sp)
    }

    /// Gets the available experience for a particular skill group
    pub fn experience(&self, skill_group: SkillGroupType) -> u16 {
        self.skill_groups
            .iter()
            .find(|s_g| s_g.skill_group_type == skill_group)
            .map_or(0, |s_g| s_g.exp)
    }

    /// Gets skill point cost to purchase skill of next level
    pub fn skill_cost(&self, skill: Skill) -> u16 {
        let next_level = self.next_skill_level(skill);
        skill.skill_cost(next_level)
    }

    /// Checks if player has sufficient skill points to purchase a skill
    pub fn sufficient_skill_points(&self, skill: Skill) -> bool {
        if let Some(skill_group_type) = skill.skill_group_type() {
            if let Some(skill_group) = self
                .skill_groups
                .iter()
                .find(|x| x.skill_group_type == skill_group_type)
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

    /// Checks if the player has available SP to spend
    pub fn has_available_sp(&self) -> bool {
        self.skill_groups.iter().any(|sg| {
            sg.available_sp > 0
                && (sg.earned_sp - sg.available_sp) < sg.skill_group_type.max_skill_points()
        })
    }

    /// Checks how much experience is needed for the next skill point in a tree
    pub fn skill_point_cost(&self, skill_group: SkillGroupType) -> u16 {
        if let Some(level) = self
            .skill_groups
            .iter()
            .find(|sg| sg.skill_group_type == skill_group)
            .map(|sg| sg.earned_sp)
        {
            skill_group.skill_point_cost(level)
        } else {
            skill_group.skill_point_cost(0)
        }
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

    /// Checks the next level of a skill
    fn next_skill_level(&self, skill: Skill) -> Option<u16> {
        if let Ok(level) = self.skill_level(skill) {
            level.map(|l| l + 1)
        } else {
            skill.max_level().map(|_| 1)
        }
    }
}

impl Skill {
    /// Returns a vec of prerequisite skills (it should only be necessary to
    /// note direct prerequisites)
    pub fn prerequisite_skills(self, level: Option<u16>) -> HashMap<Skill, Option<u16>> {
        let mut prerequisites = HashMap::new();
        if let Some(level) = level {
            if level > 1 {
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
    pub fn skill_cost(self, level: Option<u16>) -> u16 {
        // TODO: Better balance the costs later
        level.unwrap_or(1)
    }

    /// Returns the maximum level a skill can reach, returns None if the skill
    /// doesn't level
    pub fn max_level(self) -> Option<u16> { SKILL_MAX_LEVEL.get(&self).copied().flatten() }

    /// Returns the skill group type for a skill from the static skill group
    /// definitions.
    pub fn skill_group_type(self) -> Option<SkillGroupType> {
        SKILL_GROUP_DEFS
            .iter()
            .find_map(|(key, val)| val.contains(&self).then_some(*key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refund_skill() {
        let mut skillset = SkillSet::default();
        skillset.unlock_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
        skillset.add_skill_points(SkillGroupType::Weapon(ToolKind::Axe), 1);
        skillset.unlock_skill(Skill::Axe(AxeSkill::UnlockLeap));

        assert_eq!(skillset.skill_groups[1].available_sp, 0);
        assert_eq!(skillset.skills.len(), 1);
        assert_eq!(skillset.has_skill(Skill::Axe(AxeSkill::UnlockLeap)), true);

        skillset.refund_skill(Skill::Axe(AxeSkill::UnlockLeap));

        assert_eq!(skillset.skill_groups[1].available_sp, 1);
        assert_eq!(skillset.skills.get(&Skill::Axe(AxeSkill::UnlockLeap)), None);
    }

    #[test]
    fn test_unlock_skillgroup() {
        let mut skillset = SkillSet::default();
        skillset.unlock_skill_group(SkillGroupType::Weapon(ToolKind::Axe));

        assert_eq!(skillset.skill_groups.len(), 2);
        assert_eq!(
            skillset.skill_groups[1],
            SkillGroup::new(SkillGroupType::Weapon(ToolKind::Axe))
        );
    }

    #[test]
    fn test_unlock_skill() {
        let mut skillset = SkillSet::default();

        skillset.unlock_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
        skillset.add_skill_points(SkillGroupType::Weapon(ToolKind::Axe), 1);

        assert_eq!(skillset.skill_groups[1].available_sp, 1);
        assert_eq!(skillset.skills.len(), 0);

        // Try unlocking a skill with enough skill points
        skillset.unlock_skill(Skill::Axe(AxeSkill::UnlockLeap));

        assert_eq!(skillset.skill_groups[1].available_sp, 0);
        assert_eq!(skillset.skills.len(), 1);
        assert_eq!(skillset.has_skill(Skill::Axe(AxeSkill::UnlockLeap)), true);

        // Try unlocking a skill without enough skill points
        skillset.unlock_skill(Skill::Axe(AxeSkill::DsCombo));

        assert_eq!(skillset.skills.len(), 1);
        assert_eq!(skillset.skills.get(&Skill::Axe(AxeSkill::DsCombo)), None);
    }

    #[test]
    fn test_add_skill_points() {
        let mut skillset = SkillSet::default();
        skillset.unlock_skill_group(SkillGroupType::Weapon(ToolKind::Axe));
        skillset.add_skill_points(SkillGroupType::Weapon(ToolKind::Axe), 1);

        assert_eq!(skillset.skill_groups[1].available_sp, 1);
    }
}
