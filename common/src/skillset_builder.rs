#![warn(clippy::pedantic)]
//#![warn(clippy::nursery)]
use crate::comp::skillset::{skills::Skill, SkillGroupKind, SkillSet};

use crate::assets::{self, AssetExt};
use serde::{Deserialize, Serialize};
use tracing::warn;

/// `SkillSetBuilder` preset. Consider using loading from assets, when possible.
/// When you're adding new enum variant,
/// handle it in [`with_preset`](SkillSetBuilder::with_preset) method
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Preset {
    Rank1,
    Rank2,
    Rank3,
    Rank4,
    Rank5,
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
    Skill((Skill, u16)),
    Group(SkillGroupKind),
}

#[must_use]
fn skills_from_asset_expect(asset_specifier: &str) -> Vec<(Skill, u16)> {
    let nodes = SkillSetTree::load_expect(asset_specifier).read();

    skills_from_nodes(&nodes.0)
}

#[must_use]
fn skills_from_nodes(nodes: &[SkillNode]) -> Vec<(Skill, u16)> {
    let mut skills = Vec::new();
    for node in nodes {
        match node {
            SkillNode::Tree(asset) => {
                skills.append(&mut skills_from_asset_expect(asset));
            },
            SkillNode::Skill(req) => {
                skills.push(*req);
            },
            SkillNode::Group(group) => {
                skills.push((Skill::UnlockGroup(*group), 1));
            },
        }
    }

    skills
}

#[derive(Default)]
pub struct SkillSetBuilder(SkillSet);

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
    pub fn with_preset(self, preset: Preset) -> Self {
        match preset {
            Preset::Rank1 => self.with_asset_expect("common.skillset.preset.rank1.fullskill"),
            Preset::Rank2 => self.with_asset_expect("common.skillset.preset.rank2.fullskill"),
            Preset::Rank3 => self.with_asset_expect("common.skillset.preset.rank3.fullskill"),
            Preset::Rank4 => self.with_asset_expect("common.skillset.preset.rank4.fullskill"),
            Preset::Rank5 => self.with_asset_expect("common.skillset.preset.rank5.fullskill"),
        }
    }

    #[must_use]
    /// # Panics
    /// will panic only in tests
    /// 1) If added skill doesn't have any group
    /// 2) If added skill already applied
    /// 3) If added skill wasn't applied at the end
    pub fn with_skill(mut self, skill: Skill, level: u16) -> Self {
        let Some(group) = skill.skill_group_kind() else {
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
        for _ in 0..level {
            skill_set.add_skill_points(group, skill_set.skill_cost(skill));
            if let Err(err) = skill_set.unlock_skill(skill) {
                let err_msg = format!("Failed to add skill: {skill:?}. Error: {err:?}");
                common_base::dev_panic!(err_msg);
            }
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
fn skill_is_applied(skill_set: &SkillSet, skill: Skill, level: u16) -> bool {
    if let Ok(applied_level) = skill_set.skill_level(skill) {
        applied_level == level
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_skillset_assets() {
        let skillsets = assets::read_expect_dir::<SkillSetTree>("common.skillset", true);
        for skillset in skillsets {
            drop({
                let mut skillset_builder = SkillSetBuilder::default();
                let nodes = &*skillset.0;
                let tree = skills_from_nodes(nodes);
                for (skill, level) in tree {
                    skillset_builder = skillset_builder.with_skill(skill, level);
                }

                skillset_builder
            });
        }
    }
}
