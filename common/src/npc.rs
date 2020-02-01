use crate::{assets, comp::AllBodies};
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use std::{str::FromStr, sync::Arc};

#[derive(Clone, Copy, PartialEq)]
pub enum NpcKind {
    Humanoid,
    Wolf,
    Pig,
    Duck,
    Giant,
    Rat,
}

pub const ALL_NPCS: [NpcKind; 6] = [
    NpcKind::Humanoid,
    NpcKind::Wolf,
    NpcKind::Pig,
    NpcKind::Duck,
    NpcKind::Giant,
    NpcKind::Rat,
];

/// Body-specific NPC name metadata.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct BodyNames {
    /// The keyword used to refer to this body type (e.g. via the command
    /// console).  Should be unique per body type.
    pub keyword: String,
    /// A list of canonical names for NPCs with this body types (currently used
    /// when spawning this kind of NPC from the console).  Going forward,
    /// these names will likely be split up by species.
    pub names: Vec<String>,
}

/// Species-specific NPC name metadata.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct SpeciesNames {
    /// The generic name for NPCs of this species.
    pub generic: String,
}

/// Type holding configuration data for NPC names.
pub type NpcNames = AllBodies<BodyNames, SpeciesNames>;

lazy_static! {
    pub static ref NPC_NAMES: Arc<NpcNames> = assets::load_expect("common.npc_names");
}

impl FromStr for NpcKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let npc_names_json = &*NPC_NAMES;
        ALL_NPCS
            .iter()
            .copied()
            .find(|&npc| npc_names_json[npc].keyword == s)
            .ok_or(())
    }
}

pub fn get_npc_name(npc_type: NpcKind) -> &'static str {
    let BodyNames { keyword, names } = &NPC_NAMES[npc_type];

    // If no pretty name is found, fall back to the keyword.
    names.choose(&mut rand::thread_rng()).unwrap_or(keyword)
}
