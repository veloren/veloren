use hashbrown::HashMap;
use petgraph::{
    dot::{Config, Dot},
    Graph,
};
use std::{fs::File, io::Write};
use structopt::StructOpt;
use veloren_common::comp::{
    item::tool::ToolKind,
    skillset::{
        skills::Skill, SkillGroupKind, SkillPrerequisite, SKILL_GROUP_DEFS, SKILL_PREREQUISITES,
    },
};

#[derive(StructOpt)]
struct Cli {
    /// Available arguments: "sword"
    skill_group: String,
}

fn main() {
    let args = Cli::from_args();
    let skill_group = match args.skill_group.as_str() {
        "sword" => SkillGroupKind::Weapon(ToolKind::Sword),
        _ => {
            println!("Invalid argument, available arguments:\n\"sword\"");
            return;
        },
    };

    let skills = SKILL_GROUP_DEFS
        .get(&skill_group)
        .map_or(Vec::new(), |def| def.skills.iter().collect::<Vec<_>>());
    let mut graph = Graph::new();
    let mut nodes = HashMap::new();
    let mut add_node = |graph: &mut Graph<_, _>, node: Skill| {
        *nodes.entry(node).or_insert_with(|| graph.add_node(node))
    };
    for skill in skills {
        let prerequisites = SKILL_PREREQUISITES.get(skill).map_or(Vec::new(), |p| {
            let p = match p {
                SkillPrerequisite::Any(skills) => skills,
                SkillPrerequisite::All(skills) => skills,
            };
            p.iter().collect::<Vec<_>>()
        });

        let out_node = add_node(&mut graph, *skill);
        for prerequisite in prerequisites.iter().map(|(s, _)| s) {
            let in_node = add_node(&mut graph, **prerequisite);
            graph.add_edge(in_node, out_node, ());
        }
    }
    // you can render the dot file as a png with `dot -Tpng recipe_graph.dot >
    // recipe_graph.png` or interactively view it with `xdot recipe_graph.dot`
    let mut f = File::create("skill_graph.dot").unwrap();
    write!(f, "{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel])).unwrap();
}
