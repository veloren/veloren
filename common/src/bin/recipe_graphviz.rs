use hashbrown::HashMap;
use petgraph::{
    dot::{Config, Dot},
    Graph,
};
use std::{fs::File, io::Write};
use veloren_common::{
    assets::AssetExt,
    comp::item::ItemDesc,
    recipe::{RecipeBook, RecipeInput},
};

fn main() {
    let recipes = RecipeBook::load_expect_cloned("common.recipe_book");
    let mut graph = Graph::new();
    let mut nodes = HashMap::new();
    let mut add_node = |graph: &mut Graph<_, _>, node: &str| {
        *nodes
            .entry(node.to_owned())
            .or_insert_with(|| graph.add_node(node.to_owned()))
    };
    for (_, recipe) in recipes.iter() {
        let output = String::from(
            recipe
                .output
                .0
                .item_definition_id()
                .itemdef_id()
                .expect("Recipe book can only create simple items (probably)"),
        );
        let inputs = recipe
            .inputs
            .iter()
            .map(|(i, _, _)| i)
            .filter_map(|input| {
                if let RecipeInput::Item(item) = input {
                    item.item_definition_id().itemdef_id().map(String::from)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let out_node = add_node(&mut graph, &output);
        for input in inputs.iter() {
            let in_node = add_node(&mut graph, input);
            graph.add_edge(in_node, out_node, ());
        }
    }
    // you can render the dot file as a png with `dot -Tpng recipe_graph.dot >
    // recipe_graph.png` or interactively view it with `xdot recipe_graph.dot`
    let mut f = File::create("recipe_graph.dot").unwrap();
    writeln!(f, "digraph {{").unwrap();
    writeln!(f, "rankdir = \"LR\"").unwrap();
    writeln!(
        f,
        "{:#?}",
        Dot::with_attr_getters(
            &graph,
            &[Config::EdgeNoLabel, Config::GraphContentOnly],
            &|_, _| "".to_owned(),
            &|_, _| { "constraint=false".to_owned() }
        )
    )
    .unwrap();
    writeln!(f, "}}").unwrap();
}
