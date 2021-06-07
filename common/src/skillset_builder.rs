#![warn(clippy::pedantic)]
//#![warn(clippy::nursery)]
use crate::comp::skills::{Skill, SkillGroupKind, SkillSet};

use crate::assets::{self, AssetExt};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum Preset {
    Empty,
}

#[derive(Debug, Deserialize, Clone)]
struct SkillSetTree(Vec<SkillNode>);
impl assets::Asset for SkillSetTree {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Debug, Deserialize, Clone)]
enum SkillNode {
    Tree(String),
    Skill((Skill, Option<u16>)),
    Group(SkillGroupKind),
}

#[must_use]
fn skills_from_asset_expect(asset_specifier: &str) -> Vec<(Skill, Option<u16>)> {
    let nodes = SkillSetTree::load_expect(asset_specifier).read().0.clone();

    skills_from_nodes(nodes)
}

#[must_use]
fn skills_from_nodes(nodes: Vec<SkillNode>) -> Vec<(Skill, Option<u16>)> {
    let mut skills = Vec::new();
    for node in nodes {
        match node {
            SkillNode::Tree(asset) => {
                skills.append(&mut skills_from_asset_expect(&asset));
            },
            SkillNode::Skill(req) => {
                skills.push(req);
            },
            SkillNode::Group(group) => {
                skills.push((Skill::UnlockGroup(group), None));
            },
        }
    }

    skills
}

pub struct SkillSetBuilder(SkillSet);

impl Default for SkillSetBuilder {
    fn default() -> Self { Self(SkillSet::default()) }
}

impl SkillSetBuilder {
    /// Creates `SkillSetBuilder` from `asset_specifier`
    #[must_use]
    pub fn from_asset_expect(asset_specifier: &str) -> Self {
        let builder = Self::default();

        builder.with_asset_expect(asset_specifier)
    }

    /// Applies `asset_specifier` with needed skill tree
    #[must_use]
    pub fn with_asset_expect(mut self, asset_specifier: &str) -> Self {
        let tree = skills_from_asset_expect(asset_specifier);
        for (skill, level) in tree {
            self = self.with_skill(skill, level);
        }

        self
    }

    /// Creates `SkillSetBuilder` for given preset
    #[must_use]
    pub fn from_preset(preset: Preset) -> Self {
        let builder = Self::default();

        builder.with_preset(preset)
    }

    /// Applies preset
    #[must_use]
    pub const fn with_preset(self, preset: Preset) -> Self {
        match preset {
            Preset::Empty => {},
        }
        self
    }

    #[must_use]
    /// # Panics
    /// will panic only in tests
    /// 1) If added skill doesn't have any group
    /// 2) If added skill already applied
    /// 3) If added skill wasn't applied at the end
    pub fn with_skill(mut self, skill: Skill, level: Option<u16>) -> Self {
        let group = if let Some(skill_group) = skill.skill_group_kind() {
            skill_group
        } else {
            let err = format!(
                "Tried to add skill: {:?} which does not have an associated skill group.",
                skill
            );
            common_base::dev_panic!(err, or return self);
        };

        let SkillSetBuilder(ref mut skill_set) = self;
        if skill_is_applied(skill_set, skill, level) {
            let err = format!(
                "Tried to add skill: {:?} with level {:?} which is already applied",
                skill, level,
            );
            common_base::dev_panic!(err, or return self);
        }
        for _ in 0..level.unwrap_or(1) {
            skill_set.add_skill_points(group, skill_set.skill_cost(skill));
            skill_set.unlock_skill(skill);
        }
        if !skill_is_applied(skill_set, skill, level) {
            let err = format!(
                "Failed to add skill: {:?}. Verify that it has the appropriate skill group \
                 available and meets all prerequisite skills.",
                skill
            );
            common_base::dev_panic!(err);
        }
        self
    }

    #[must_use]
    pub fn build(self) -> SkillSet { self.0 }
}

#[must_use]
fn skill_is_applied(skill_set: &SkillSet, skill: Skill, level: Option<u16>) -> bool {
    if let Ok(applied_level) = skill_set.skill_level(skill) {
        applied_level == level
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assets::Error;

    #[test]
    fn test_all_skillset_assets() {
        #[derive(Clone)]
        struct SkillSetList(Vec<SkillSetTree>);

        impl assets::Compound for SkillSetList {
            fn load<S: assets::source::Source>(
                cache: &assets::AssetCache<S>,
                specifier: &str,
            ) -> Result<Self, Error> {
                let list = cache
                    .load::<assets::Directory>(specifier)?
                    .read()
                    .iter()
                    .map(|spec| SkillSetTree::load_cloned(spec))
                    .collect::<Result<_, Error>>()?;

                Ok(Self(list))
            }
        }

        let skillsets = SkillSetList::load_expect_cloned("common.skillset.*").0;
        for skillset in skillsets {
            std::mem::drop({
                let mut skillset_builder = SkillSetBuilder::default();
                let nodes = skillset.0;
                let tree = skills_from_nodes(nodes);
                for (skill, level) in tree {
                    skillset_builder = skillset_builder.with_skill(skill, level);
                }

                skillset_builder
            });
        }
    }
}
