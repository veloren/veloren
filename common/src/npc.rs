use rand::seq::SliceRandom;
use serde_json;
use std::fs::File;

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

pub fn get_npc_name(npc_type: NpcKind) -> String {
    let file = File::open("common/assets/npc_names.json").expect("file should open read only");
    let json: serde_json::Value =
        serde_json::from_reader(file).expect("file should be proper JSON");
    let npc_names = json
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
