use crate::assets;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use serde_json;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
pub enum NpcKind {
    Humanoid,
    Wolf,
    Pig,
    Duck,
    Giant,
    Rat,
}

impl NpcKind {
    fn as_str(self) -> &'static str {
        match self {
            NpcKind::Humanoid => "humanoid",
            NpcKind::Wolf => "wolf",
            NpcKind::Pig => "pig",
            NpcKind::Duck => "duck",
            NpcKind::Giant => "giant",
            NpcKind::Rat => "rat",
        }
    }
}

impl FromStr for NpcKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "humanoid" => Ok(NpcKind::Humanoid),
            "wolf" => Ok(NpcKind::Wolf),
            "pig" => Ok(NpcKind::Pig),
            "duck" => Ok(NpcKind::Duck),
            "giant" => Ok(NpcKind::Giant),
            "rat" => Ok(NpcKind::Rat),

            _ => Err(()),
        }
    }
}

lazy_static! {
    static ref NPC_NAMES_JSON: Arc<serde_json::Value> = assets::load_expect("common.npc_names");
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
