use rand::seq::SliceRandom;
use serde_json;
use std::fs::File;
use std::io::Error;

pub enum NpcKind {
    Wolf,
    Pig,
}

impl NpcKind {
    fn as_str(&self) -> &'static str {
        match *self {
            NpcKind::Wolf => "wolf",
            NpcKind::Pig => "pig",
        }
    }
}

const npc_names_dir: &str = "common/assets/npc_names.json";

pub fn get_npc_name(npc_type: NpcKind) -> String {
    let npc_names_file =
        File::open(npc_names_dir).expect(&format!("opening {} in read-only mode", npc_names_dir));
    let npc_names_json: serde_json::Value = serde_json::from_reader(npc_names_file)
        .expect(&format!("reading json contents from {}", npc_names_dir));
    let npc_names = npc_names_json
        .get(npc_type.as_str())
        .expect("accessing json using NPC type provided as key")
        .as_array()
        .expect("parsing accessed json value into an array");
    let npc_name = npc_names
        .choose(&mut rand::thread_rng())
        .expect("getting a random NPC name")
        .as_str()
        .expect("parsing NPC name json value into a &str");
    String::from(npc_name)
}
