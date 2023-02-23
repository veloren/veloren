use super::*;
use crate::comp::{skillset::SkillPrerequisitesMap, Skill};
use hashbrown::HashMap;

// Unneeded cfg(test) here keeps rust-analyzer happy
#[cfg(test)]
use petgraph::{algo::is_cyclic_directed, graph::DiGraph};

#[test]
fn check_cyclic_skill_deps() {
    let skill_prereqs =
        SkillPrerequisitesMap::load_expect_cloned("common.skill_trees.skill_prerequisites").0;
    let mut graph = DiGraph::new();
    let mut nodes = HashMap::<Skill, _>::new();
    let mut add_node = |graph: &mut DiGraph<Skill, _>, node: Skill| {
        *nodes.entry(node).or_insert_with(|| graph.add_node(node))
    };

    for (skill, prereqs) in skill_prereqs.iter() {
        let skill_node = add_node(&mut graph, *skill);
        let prereqs = match prereqs {
            SkillPrerequisite::Any(skills) => skills,
            SkillPrerequisite::All(skills) => skills,
        };
        for (prereq, _) in prereqs.iter() {
            let prereq_node = add_node(&mut graph, *prereq);
            graph.add_edge(prereq_node, skill_node, ());
        }
    }

    assert!(!is_cyclic_directed(&graph));
}
