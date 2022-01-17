use super::*;
use crate::comp::{skillset::SkillPrerequisitesMap, Skill};
use hashbrown::HashMap;

#[cfg(test)]
use petgraph::{algo::is_cyclic_undirected, graph::UnGraph};

#[test]
fn check_cyclic_skill_deps() {
    let skill_prereqs =
        SkillPrerequisitesMap::load_expect_cloned("common.skill_trees.skill_prerequisites").0;
    let mut graph = UnGraph::new_undirected();
    let mut nodes = HashMap::<Skill, _>::new();
    let mut add_node = |graph: &mut UnGraph<Skill, _>, node: Skill| {
        *nodes
            .entry(node.clone())
            .or_insert_with(|| graph.add_node(node.clone()))
    };

    for (skill, prereqs) in skill_prereqs.iter() {
        let skill_node = add_node(&mut graph, *skill);
        for (prereq, _) in prereqs.iter() {
            let prereq_node = add_node(&mut graph, *prereq);
            graph.add_edge(prereq_node, skill_node, ());
        }
    }

    assert!(!is_cyclic_undirected(&graph));
}
