use crate::{
    comp,
    comp::{body::humanoid::Species, Body},
    sync::Uid,
};
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthChange {
    pub amount: i32,
    pub cause: HealthSource,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthSource {
    Attack { by: Uid }, // TODO: Implement weapon
    Projectile { owner: Option<Uid> },
    Suicide,
    World,
    Revive,
    Command,
    LevelUp,
    Item,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Health {
    current: u32,
    maximum: u32,
    pub last_change: (f64, HealthChange),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Exp {
    current: u32,
    maximum: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Level {
    amount: u32,
}

impl Health {
    pub fn current(&self) -> u32 { self.current }

    pub fn maximum(&self) -> u32 { self.maximum }

    pub fn set_to(&mut self, amount: u32, cause: HealthSource) {
        let amount = amount.min(self.maximum);
        self.last_change = (0.0, HealthChange {
            amount: amount as i32 - self.current as i32,
            cause,
        });
        self.current = amount;
    }

    pub fn change_by(&mut self, change: HealthChange) {
        self.current = ((self.current as i32 + change.amount).max(0) as u32).min(self.maximum);
        self.last_change = (0.0, change);
    }

    // This is private because max hp is based on the level
    fn set_maximum(&mut self, amount: u32) {
        self.maximum = amount;
        self.current = self.current.min(self.maximum);
    }
}
#[derive(Debug)]
pub enum StatChangeError {
    Underflow,
    Overflow,
}
use std::{error::Error, fmt};
use std::collections::HashMap;
use crate::comp::stats::Skill::{TestT1Skill2, TestT1Skill1};
use crate::comp::stats;

impl fmt::Display for StatChangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Underflow => "insufficient stat quantity",
            Self::Overflow => "stat quantity would overflow",
        })
    }
}
impl Error for StatChangeError {}

impl Exp {
    /// Used to determine how much exp is required to reach the next level. When
    /// a character levels up, the next level target is increased by this value
    const EXP_INCREASE_FACTOR: u32 = 25;

    pub fn current(&self) -> u32 { self.current }

    pub fn maximum(&self) -> u32 { self.maximum }

    pub fn set_current(&mut self, current: u32) { self.current = current; }

    pub fn change_by(&mut self, current: i64) {
        self.current = ((self.current as i64) + current) as u32;
    }

    pub fn change_maximum_by(&mut self, maximum: i64) {
        self.maximum = ((self.maximum as i64) + maximum) as u32;
    }

    pub fn update_maximum(&mut self, level: u32) {
        self.maximum = level
            .saturating_mul(Self::EXP_INCREASE_FACTOR)
            .saturating_add(Self::EXP_INCREASE_FACTOR);
    }
}

impl Level {
    pub fn set_level(&mut self, level: u32) { self.amount = level; }

    pub fn level(&self) -> u32 { self.amount }

    pub fn change_by(&mut self, level: u32) { self.amount += level; }
}

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
    Axes
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillGroup {
    pub skills: Vec<Skill>,
    pub exp: u32,
    pub available_sp: u8
}

impl Default for SkillGroup {
    fn default() -> Self {
        Self {
            skills: Vec::new(),
            exp: 0,
            available_sp: 0
        }
    }
}

// TODO: Better way to store this static data that doesn't create a new HashMap each time
pub fn skill_group_definitions() -> HashMap<SkillGroupType, Vec<Skill>> {
    let mut skill_group_definitions = HashMap::new();
    skill_group_definitions.insert(SkillGroupType::T1, vec![
        Skill::TestT1Skill1,
        Skill::TestT1Skill2,
        Skill::TestT1Skill3,
        Skill::TestT1Skill4,
        Skill::TestT1Skill5]);

    skill_group_definitions.insert(SkillGroupType::Swords, vec![
        Skill::TestSwordSkill1,
        Skill::TestSwordSkill2,
        Skill::TestSwordSkill3]);

    skill_group_definitions.insert(SkillGroupType::Axes, vec![
        Skill::TestAxeSkill1,
        Skill::TestAxeSkill2,
        Skill::TestAxeSkill3]);

    skill_group_definitions
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillSet
{
    pub skill_groups: HashMap<SkillGroupType, SkillGroup>
}

impl SkillSet {
    /// Instantiate a new skill set with the default skill groups with no unlocked skills in them -
    /// used when adding a skill set to a new player
    fn new() -> Self {
        let mut skill_groups = HashMap::new();
        skill_groups.insert(SkillGroupType::T1, SkillGroup::default());
        skill_groups.insert(SkillGroupType::Swords, SkillGroup::default());
        skill_groups.insert(SkillGroupType::Axes, SkillGroup::default());
        Self {
            skill_groups
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub name: String,
    pub health: Health,
    pub level: Level,
    pub exp: Exp,
    pub skill_set: SkillSet,
    pub endurance: u32,
    pub fitness: u32,
    pub willpower: u32,
    pub is_dead: bool,
}

impl Stats {
    pub fn should_die(&self) -> bool { self.health.current == 0 }

    pub fn revive(&mut self) {
        self.health
            .set_to(self.health.maximum(), HealthSource::Revive);
        self.is_dead = false;
    }

    // TODO: Delete this once stat points will be a thing
    pub fn update_max_hp(&mut self) { self.health.set_maximum(52 + 3 * self.level.amount); }

    pub fn refund_skill(&mut self, skill: Skill) {
        // TODO: check player has skill, remove skill and increase SP in skill group by 1
    }

    pub fn unlock_skill(&mut self, skill: Skill) {
        // Find the skill group type for the skill from the static skill definitions
        let skill_group_type = skill_group_definitions()
            .iter()
            .find_map(|(key, val)| if val.contains(&skill)  {Some(key) } else { None }  );

        // Find the skill group for the skill on the player, check that the skill is not
        // already unlocked and that they have available SP in that group, and then allocate the
        // skill and reduce the player's SP in that skill group by 1.
        if let Some(skill_group_type) = skill_group_type {
            if let Some(skill_group) = self.skill_set.skill_groups.get_mut(skill_group_type) {
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

impl Stats {
    pub fn new(name: String, body: Body) -> Self {
        let species = if let comp::Body::Humanoid(hbody) = body {
            Some(hbody.species)
        } else {
            None
        };

        // TODO: define base stats somewhere else (maybe method on Body?)
        let (endurance, fitness, willpower) = match species {
            Some(Species::Danari) => (0, 2, 3), // Small, flexible, intelligent, physically weak
            Some(Species::Dwarf) => (2, 2, 1),  // phyiscally strong, intelligent, slow reflexes
            Some(Species::Elf) => (1, 2, 2),    // Intelligent, quick, physically weak
            Some(Species::Human) => (2, 1, 2),  // Perfectly balanced
            Some(Species::Orc) => (3, 2, 0),    /* Physically strong, non intelligent, medium */
            // reflexes
            Some(Species::Undead) => (1, 3, 1), /* Very good reflexes, equally intelligent and */
            // strong
            None => (0, 0, 0),
        };

        let mut stats = Self {
            name,
            health: Health {
                current: 0,
                maximum: 0,
                last_change: (0.0, HealthChange {
                    amount: 0,
                    cause: HealthSource::Revive,
                }),
            },
            level: Level { amount: 1 },
            exp: Exp {
                current: 0,
                maximum: 50,
            },
            skill_set: SkillSet::default(),
            endurance,
            fitness,
            willpower,
            is_dead: false,
        };

        stats.update_max_hp();
        stats
            .health
            .set_to(stats.health.maximum(), HealthSource::Revive);

        stats
    }

    pub fn with_max_health(mut self, amount: u32) -> Self {
        self.health.maximum = amount;
        self.health.current = amount;
        self
    }
}

impl Component for Stats {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Dying {
    pub cause: HealthSource,
}

impl Component for Dying {
    type Storage = IDVStorage<Self>;
}
