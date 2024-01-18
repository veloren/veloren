use crate::{
    assets::{self, Asset, AssetExt},
    comp::{
        item::tool::ToolKind,
        skills::{GeneralSkill, Skill},
    },
};
use core::borrow::{Borrow, BorrowMut};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specs::{Component, DerefFlaggedStorage};
use std::{collections::BTreeSet, hash::Hash};
use tracing::{trace, warn};

pub mod skills;

#[cfg(test)] mod test;

/// BTreeSet is used here to ensure that skills are ordered. This is important
/// to ensure that the hash created from it is consistent so that we don't
/// needlessly force a respec when loading skills from persistence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillTreeMap(HashMap<SkillGroupKind, BTreeSet<Skill>>);

impl Asset for SkillTreeMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

pub struct SkillGroupDef {
    pub skills: BTreeSet<Skill>,
    pub total_skill_point_cost: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillLevelMap(HashMap<Skill, u16>);

impl Asset for SkillLevelMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

/// Contains the prerequisite skills for each skill. It cannot currently detect
/// cyclic dependencies, so if you modify the prerequisite map ensure that there
/// are no cycles of prerequisites.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillPrerequisitesMap(HashMap<Skill, SkillPrerequisite>);

impl Asset for SkillPrerequisitesMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillCostMap(HashMap<Skill, u16>);

impl Asset for SkillCostMap {
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
                        let max_level = skill.max_level();
                        (1..=max_level)
                            .map(|level| skill.skill_cost(level))
                            .sum::<u16>()
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
        map.iter().flat_map(|(sgk, skills)| skills.iter().map(move |s| (*s, *sgk))).collect()
    };
    // Loads the maximum level that a skill can obtain
    pub static ref SKILL_MAX_LEVEL: HashMap<Skill, u16> = {
        SkillLevelMap::load_expect_cloned(
            "common.skill_trees.skill_max_levels",
        ).0
    };
    // Loads the prerequisite skills for a particular skill
    pub static ref SKILL_PREREQUISITES: HashMap<Skill, SkillPrerequisite> = {
        SkillPrerequisitesMap::load_expect_cloned(
            "common.skill_trees.skill_prerequisites",
        ).0
    };
    pub static ref SKILL_GROUP_HASHES: HashMap<SkillGroupKind, Vec<u8>> = {
        let map = SkillTreeMap::load_expect_cloned(
            "common.skill_trees.skills_skill-groups_manifest",
        ).0;
        let mut hashes = HashMap::new();
        for (skill_group_kind, skills) in map.iter() {
            let mut hasher = Sha256::new();
            let json_input: Vec<_> = skills.iter().map(|skill| (*skill, skill.max_level())).collect();
            let hash_input = serde_json::to_string(&json_input).unwrap_or_default();
            hasher.update(hash_input.as_bytes());
            let hash_result = hasher.finalize();
            hashes.insert(*skill_group_kind, hash_result.iter().copied().collect());
        }
        hashes
    };
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum SkillGroupKind {
    General,
    Weapon(ToolKind),
}

impl SkillGroupKind {
    /// Gets the cost in experience of earning a skill point
    /// Changing this is forward compatible with persistence and will
    /// automatically force a respec for skill group kinds that are affected.
    pub fn skill_point_cost(self, level: u16) -> u32 {
        use std::f32::consts::E;
        match self {
            Self::Weapon(ToolKind::Sword | ToolKind::Axe) => {
                let level = level as f32;
                ((400.0 * (level / (level + 20.0)).powi(2) + 5.0 * E.powf(0.025 * level))
                    .min(u32::MAX as f32) as u32)
                    .saturating_mul(25)
            },
            _ => {
                const EXP_INCREMENT: f32 = 10.0;
                const STARTING_EXP: f32 = 70.0;
                const EXP_CEILING: f32 = 1000.0;
                const SCALING_FACTOR: f32 = 0.125;
                (EXP_INCREMENT
                    * (EXP_CEILING
                        / EXP_INCREMENT
                        / (1.0
                            + E.powf(-SCALING_FACTOR * level as f32)
                                * (EXP_CEILING / STARTING_EXP - 1.0)))
                        .floor()) as u32
            },
        }
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
    // Invariant should be maintained that this is the same kind as the key that the skill group is
    // inserted into the skillset as.
    pub skill_group_kind: SkillGroupKind,
    // The invariant that (earned_exp >= available_exp) should not be violated
    pub available_exp: u32,
    pub earned_exp: u32,
    // The invariant that (earned_sp >= available_sp) should not be violated
    pub available_sp: u16,
    pub earned_sp: u16,
    // Used for persistence
    pub ordered_skills: Vec<Skill>,
}

impl SkillGroup {
    fn new(skill_group_kind: SkillGroupKind) -> SkillGroup {
        SkillGroup {
            skill_group_kind,
            available_exp: 0,
            earned_exp: 0,
            available_sp: 0,
            earned_sp: 0,
            ordered_skills: Vec::new(),
        }
    }

    /// Returns the amount of experience in a skill group that has been spent to
    /// acquire skill points Relies on the invariant that (earned_exp >=
    /// available_exp) to ensure function does not underflow
    pub fn spent_exp(&self) -> u32 { self.earned_exp - self.available_exp }

    /// Adds a skill point while subtracting the necessary amount of experience
    fn earn_skill_point(&mut self) -> Result<(), SpRewardError> {
        let sp_cost = self.skill_group_kind.skill_point_cost(self.earned_sp);
        // If there is insufficient available exp, checked sub will fail as the result
        // would be less than 0
        let new_available_exp = self
            .available_exp
            .checked_sub(sp_cost)
            .ok_or(SpRewardError::InsufficientExp)?;
        let new_earned_sp = self
            .earned_sp
            .checked_add(1)
            .ok_or(SpRewardError::Overflow)?;
        let new_available_sp = self
            .available_sp
            .checked_add(1)
            .ok_or(SpRewardError::Overflow)?;
        self.available_exp = new_available_exp;
        self.earned_sp = new_earned_sp;
        self.available_sp = new_available_sp;
        Ok(())
    }

    /// Also attempts to earn a skill point after adding experience. If a skill
    /// point was earned, returns how many skill points the skill group now has
    /// earned in total.
    pub fn add_experience(&mut self, amount: u32) -> Option<u16> {
        self.earned_exp = self.earned_exp.saturating_add(amount);
        self.available_exp = self.available_exp.saturating_add(amount);

        let mut return_val = None;
        // Attempt to earn skill point
        while self.earn_skill_point().is_ok() {
            return_val = Some(self.earned_sp);
        }
        return_val
    }
}

/// Contains all of a player's skill groups and skills. Provides methods for
/// manipulating assigned skills and skill groups including unlocking skills,
/// refunding skills etc.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillSet {
    skill_groups: HashMap<SkillGroupKind, SkillGroup>,
    skills: HashMap<Skill, u16>,
    pub modify_health: bool,
    pub modify_energy: bool,
}

impl Component for SkillSet {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

impl Default for SkillSet {
    /// Instantiate a new skill set with the default skill groups with no
    /// unlocked skills in them - used when adding a skill set to a new
    /// player
    fn default() -> Self {
        // Create an empty skillset
        let mut skill_group = Self {
            skill_groups: HashMap::new(),
            skills: SkillSet::initial_skills(),
            modify_health: false,
            modify_energy: false,
        };

        // Insert default skill groups
        skill_group.unlock_skill_group(SkillGroupKind::General);
        skill_group.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Pick));

        skill_group
    }
}

impl SkillSet {
    pub fn initial_skills() -> HashMap<Skill, u16> {
        let mut skills = HashMap::new();
        skills.insert(Skill::UnlockGroup(SkillGroupKind::General), 1);
        skills.insert(
            Skill::UnlockGroup(SkillGroupKind::Weapon(ToolKind::Pick)),
            1,
        );
        skills
    }

    /// NOTE: This does *not* return an error on failure, since we can partially
    /// recover from some failures.  Instead, it returns the error in the
    /// second return value; make sure to handle it if present!
    pub fn load_from_database(
        skill_groups: HashMap<SkillGroupKind, SkillGroup>,
        mut all_skills: HashMap<SkillGroupKind, Result<Vec<Skill>, SkillsPersistenceError>>,
    ) -> (Self, Option<SkillsPersistenceError>) {
        let mut skillset = SkillSet {
            skill_groups,
            skills: SkillSet::initial_skills(),
            modify_health: true,
            modify_energy: true,
        };
        let mut persistence_load_error = None;

        // Loops while checking the all_skills hashmap. For as long as it can find an
        // entry where the skill group kind is unlocked, insert the skills corresponding
        // to that skill group kind. When no more skill group kinds can be found, break
        // the loop.
        while let Some(skill_group_kind) = all_skills
            .keys()
            .find(|kind| skillset.has_skill(Skill::UnlockGroup(**kind)))
            .copied()
        {
            // Remove valid skill group kind from the hash map so that loop eventually
            // terminates.
            if let Some(skills_result) = all_skills.remove(&skill_group_kind) {
                match skills_result {
                    Ok(skills) => {
                        let backup_skillset = skillset.clone();
                        // Iterate over all skills and make sure that unlocking them is successful.
                        // If any fail, fall back to skillset before
                        // unlocking any to allow a full respec
                        if !skills
                            .iter()
                            .all(|skill| skillset.unlock_skill(*skill).is_ok())
                        {
                            skillset = backup_skillset;
                            // If unlocking failed, set persistence_load_error
                            persistence_load_error =
                                Some(SkillsPersistenceError::SkillsUnlockFailed)
                        }
                    },
                    Err(persistence_error) => persistence_load_error = Some(persistence_error),
                }
            }
        }

        (skillset, persistence_load_error)
    }

    /// Check if a particular skill group is accessible for an entity, *if* it
    /// exists.
    fn skill_group_accessible_if_exists(&self, skill_group_kind: SkillGroupKind) -> bool {
        self.has_skill(Skill::UnlockGroup(skill_group_kind))
    }

    /// Checks if a particular skill group is accessible for an entity
    pub fn skill_group_accessible(&self, skill_group_kind: SkillGroupKind) -> bool {
        self.skill_groups.contains_key(&skill_group_kind)
            && self.skill_group_accessible_if_exists(skill_group_kind)
    }

    ///  Unlocks a skill group for a player. It starts with 0 exp and 0 skill
    ///  points.
    fn unlock_skill_group(&mut self, skill_group_kind: SkillGroupKind) {
        if !self.skill_groups.contains_key(&skill_group_kind) {
            self.skill_groups
                .insert(skill_group_kind, SkillGroup::new(skill_group_kind));
        }
    }

    /// Returns an iterator over skill groups
    pub fn skill_groups(&self) -> impl Iterator<Item = &SkillGroup> { self.skill_groups.values() }

    /// Returns a reference to a particular skill group in a skillset
    fn skill_group(&self, skill_group: SkillGroupKind) -> Option<&SkillGroup> {
        self.skill_groups.get(&skill_group)
    }

    /// Returns a mutable reference to a particular skill group in a skillset
    /// Requires that skillset contains skill that unlocks the skill group
    fn skill_group_mut(&mut self, skill_group: SkillGroupKind) -> Option<&mut SkillGroup> {
        // In order to mutate skill group, we check that the prerequisite skill has been
        // acquired, as this is one of the requirements for us to consider the skill
        // group accessible.
        let skill_group_accessible = self.skill_group_accessible(skill_group);
        if skill_group_accessible {
            self.skill_groups.get_mut(&skill_group)
        } else {
            None
        }
    }

    /// Adds experience to the skill group within an entity's skill set, will
    /// attempt to earn a skill point while doing so. If a skill point was
    /// earned, returns the number of earned skill points in the skill group.
    pub fn add_experience(&mut self, skill_group_kind: SkillGroupKind, amount: u32) -> Option<u16> {
        if let Some(skill_group) = self.skill_group_mut(skill_group_kind) {
            skill_group.add_experience(amount)
        } else {
            warn!("Tried to add experience to a skill group that player does not have");
            None
        }
    }

    /// Gets the available experience for a particular skill group
    pub fn available_experience(&self, skill_group: SkillGroupKind) -> u32 {
        self.skill_group(skill_group)
            .map_or(0, |s_g| s_g.available_exp)
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
        for _ in 0..number_of_skill_points {
            let exp_needed = self.skill_point_cost(skill_group_kind);
            self.add_experience(skill_group_kind, exp_needed);
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

    /// Checks that the skill set contains all prerequisite skills of the
    /// required level for a particular skill
    pub fn prerequisites_met(&self, skill: Skill) -> bool {
        match skill.prerequisite_skills() {
            Some(SkillPrerequisite::All(skills)) => skills
                .iter()
                .all(|(s, l)| self.skill_level(*s).map_or(false, |l_b| l_b >= *l)),
            Some(SkillPrerequisite::Any(skills)) => skills
                .iter()
                .any(|(s, l)| self.skill_level(*s).map_or(false, |l_b| l_b >= *l)),
            None => true,
        }
    }

    /// Gets skill point cost to purchase skill of next level
    pub fn skill_cost(&self, skill: Skill) -> u16 {
        let next_level = self.next_skill_level(skill);
        skill.skill_cost(next_level)
    }

    /// Checks if player has sufficient skill points to purchase a skill
    pub fn sufficient_skill_points(&self, skill: Skill) -> bool {
        if let Some(skill_group_kind) = skill.skill_group_kind() {
            if let Some(skill_group) = self.skill_group(skill_group_kind) {
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
    fn next_skill_level(&self, skill: Skill) -> u16 {
        if let Ok(level) = self.skill_level(skill) {
            // If already has skill, then level + 1
            level + 1
        } else {
            // Otherwise the next level is the first level
            1
        }
    }

    /// Unlocks a skill for a player, assuming they have the relevant skill
    /// group unlocked and available SP in that skill group.
    ///
    /// NOTE: Please don't use pathological or clever implementations of to_mut
    /// here.
    pub fn unlock_skill_cow<'a, B, C: 'a>(
        this_: &'a mut B,
        skill: Skill,
        to_mut: impl FnOnce(&'a mut B) -> &'a mut C,
    ) -> Result<(), SkillUnlockError>
    where
        B: Borrow<SkillSet>,
        C: BorrowMut<SkillSet>,
    {
        if let Some(skill_group_kind) = skill.skill_group_kind() {
            let this = (*this_).borrow();
            let next_level = this.next_skill_level(skill);
            let prerequisites_met = this.prerequisites_met(skill);
            // Check that skill is not yet at max level
            if !matches!(this.skills.get(&skill), Some(level) if *level == skill.max_level()) {
                if let Some(skill_group) = this.skill_groups.get(&skill_group_kind)
                    && this.skill_group_accessible_if_exists(skill_group_kind)
                {
                    if prerequisites_met {
                        if let Some(new_available_sp) = skill_group
                            .available_sp
                            .checked_sub(skill.skill_cost(next_level))
                        {
                            // Perform all mutation inside this branch, to avoid triggering a copy
                            // on write or flagged storage in cases where this matters.
                            let this_ = to_mut(this_);
                            let this = this_.borrow_mut();
                            // NOTE: Verified to exist previously when we accessed
                            // this.skill_groups (assuming a non-pathological implementation of
                            // ToOwned).
                            let skill_group = this.skill_groups.get_mut(&skill_group_kind).expect(
                                "Verified to exist when we previously accessed this.skill_groups",
                            );
                            skill_group.available_sp = new_available_sp;
                            skill_group.ordered_skills.push(skill);
                            match skill {
                                Skill::UnlockGroup(group) => {
                                    this.unlock_skill_group(group);
                                },
                                Skill::General(GeneralSkill::HealthIncrease) => {
                                    this.modify_health = true;
                                },
                                Skill::General(GeneralSkill::EnergyIncrease) => {
                                    this.modify_energy = true;
                                },
                                _ => {},
                            }
                            this.skills.insert(skill, next_level);
                            Ok(())
                        } else {
                            trace!("Tried to unlock skill for skill group with insufficient SP");
                            Err(SkillUnlockError::InsufficientSP)
                        }
                    } else {
                        trace!("Tried to unlock skill without meeting prerequisite skills");
                        Err(SkillUnlockError::MissingPrerequisites)
                    }
                } else {
                    trace!("Tried to unlock skill for a skill group that player does not have");
                    Err(SkillUnlockError::UnavailableSkillGroup)
                }
            } else {
                trace!("Tried to unlock skill the player already has");
                Err(SkillUnlockError::SkillAlreadyUnlocked)
            }
        } else {
            warn!(
                ?skill,
                "Tried to unlock skill that does not exist in any skill group!"
            );
            Err(SkillUnlockError::NoParentSkillTree)
        }
    }

    /// Convenience function for the case where you have mutable access to the
    /// skill.
    pub fn unlock_skill(&mut self, skill: Skill) -> Result<(), SkillUnlockError> {
        Self::unlock_skill_cow(self, skill, |x| x)
    }

    /// Checks if the player has available SP to spend
    pub fn has_available_sp(&self) -> bool {
        self.skill_groups.iter().any(|(kind, sg)| {
            sg.available_sp > 0
            // Subtraction in bounds because of the invariant that available_sp <= earned_sp
                && (sg.earned_sp - sg.available_sp) < kind.total_skill_point_cost()
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
    pub fn skill_level(&self, skill: Skill) -> Result<u16, SkillError> {
        if let Some(level) = self.skills.get(&skill).copied() {
            Ok(level)
        } else {
            Err(SkillError::MissingSkill)
        }
    }
}

#[derive(Debug)]
pub enum SkillError {
    MissingSkill,
}

#[derive(Debug)]
pub enum SkillUnlockError {
    InsufficientSP,
    MissingPrerequisites,
    UnavailableSkillGroup,
    SkillAlreadyUnlocked,
    NoParentSkillTree,
}

#[derive(Debug)]
pub enum SpRewardError {
    InsufficientExp,
    UnavailableSkillGroup,
    Overflow,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
pub enum SkillsPersistenceError {
    HashMismatch,
    DeserializationFailure,
    SpentExpMismatch,
    SkillsUnlockFailed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SkillPrerequisite {
    All(HashMap<Skill, u16>),
    Any(HashMap<Skill, u16>),
}
