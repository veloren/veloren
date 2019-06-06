use crate::assets;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use serde_json;
use std::sync::Arc;

pub enum NpcKind {
    Humanoid,
    Wolf,
    Pig,
}

impl NpcKind {
    fn as_str(&self) -> &'static str {
        match *self {
            NpcKind::Humanoid => "humanoid",
            NpcKind::Wolf => "wolf",
            NpcKind::Pig => "pig",
        }
    }
}

lazy_static! {
    static ref NPC_NAMES_JSON: Arc<serde_json::Value> =
        assets::load_expect("common/npc_names.json");
}

pub fn get_npc_name(npc_type: NpcKind) -> String {
    let npc_names = NPC_NAMES_JSON
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
