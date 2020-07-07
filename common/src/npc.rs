use crate::{
    assets,
    comp::{self, AllBodies, Body},
};
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::{str::FromStr, sync::Arc};

#[derive(Clone, Copy, PartialEq)]
pub enum NpcKind {
    Humanoid,
    Wolf,
    Pig,
    Duck,
    Ogre,
    Rat,
    StoneGolem,
    Reddragon,
    Crocodile,
}

pub const ALL_NPCS: [NpcKind; 9] = [
    NpcKind::Humanoid,
    NpcKind::Wolf,
    NpcKind::Pig,
    NpcKind::Duck,
    NpcKind::Ogre,
    NpcKind::Rat,
    NpcKind::StoneGolem,
    NpcKind::Reddragon,
    NpcKind::Crocodile,
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
    /// The keyword used to refer to this species (e.g. via the command
    /// console).  Should be unique per species and distinct from all body
    /// types (maybe in the future, it will just be unique per body type).
    pub keyword: String,
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
        let npc_names = &*NPC_NAMES;
        ALL_NPCS
            .iter()
            .copied()
            .find(|&npc| npc_names[npc].keyword == s)
            .ok_or(())
    }
}

pub fn get_npc_name(npc_type: NpcKind) -> &'static str {
    let BodyNames { keyword, names } = &NPC_NAMES[npc_type];

    // If no pretty name is found, fall back to the keyword.
    names.choose(&mut rand::thread_rng()).unwrap_or(keyword)
}

/// Randomly generates a body associated with this NPC kind.
pub fn kind_to_body(kind: NpcKind) -> Body {
    match kind {
        NpcKind::Humanoid => comp::humanoid::Body::random().into(),
        NpcKind::Pig => comp::quadruped_small::Body::random().into(),
        NpcKind::Wolf => comp::quadruped_medium::Body::random().into(),
        NpcKind::Duck => comp::bird_medium::Body::random().into(),
        NpcKind::Ogre => comp::biped_large::Body::random().into(),
        NpcKind::Rat => comp::critter::Body::random().into(),
        NpcKind::StoneGolem => comp::golem::Body::random().into(),
        NpcKind::Reddragon => comp::dragon::Body::random().into(),
        NpcKind::Crocodile => comp::quadruped_low::Body::random().into(),
    }
}

/// A combination of an NpcKind (representing an outer species to generate), and
/// a function that generates a fresh Body of a species that is part of that
/// NpcKind each time it's called.  The reason things are done this way is that
/// when parsing spawn strings, we'd like to be able to randomize features that
/// haven't already been specified; for instance, if no species is specified we
/// should randomize species, while if a species is specified we can still
/// randomize other attributes like gender or clothing.
///
/// TODO: Now that we return a closure, consider having the closure accept a
/// source of randomness explicitly, rather than always using ThreadRng.
pub struct NpcBody(pub NpcKind, pub Box<dyn FnMut() -> Body>);

impl FromStr for NpcBody {
    type Err = ();

    /// Get an NPC kind from a string.  If a body kind is matched without an
    /// associated species, generate the species randmly within it; if an
    /// explicit species is found, generate a random member of the species;
    /// otherwise, return Err(()).
    fn from_str(s: &str) -> Result<Self, ()> { Self::from_str_with(s, kind_to_body) }
}

impl NpcBody {
    /// If there is an exact name match for a body kind, call kind_to_body on
    /// it. Otherwise, if an explicit species is found, generate a random
    /// member of the species; otherwise, return Err(()).
    pub fn from_str_with(s: &str, kind_to_body: fn(NpcKind) -> Body) -> Result<Self, ()> {
        fn parse<
            'a,
            B: Into<Body> + 'static,
            // NOTE: Should be cheap in all cases, but if it weren't we should revamp the indexing
            // method to take references instead of owned values.
            Species: 'static,
            BodyMeta,
            SpeciesData: for<'b> core::ops::Index<&'b Species, Output = SpeciesNames>,
        >(
            s: &str,
            npc_kind: NpcKind,
            body_data: &'a comp::BodyData<BodyMeta, SpeciesData>,
            conv_func: for<'d> fn(&mut rand::rngs::ThreadRng, &'d Species) -> B,
        ) -> Option<NpcBody>
        where
            &'a SpeciesData: IntoIterator<Item = Species>,
        {
            let npc_names = &body_data.species;
            body_data
                .species
                .into_iter()
                .find(|species| npc_names[species].keyword == s)
                .map(|species| {
                    NpcBody(
                        npc_kind,
                        Box::new(move || conv_func(&mut rand::thread_rng(), &species).into()),
                    )
                })
        }
        let npc_names = &NPC_NAMES;
        // First, parse npc kind names.
        NpcKind::from_str(s)
            .map(|kind| NpcBody(kind, Box::new(move || kind_to_body(kind))))
            .ok()
            // Otherwise, npc kind names aren't sufficient; we parse species names instead.
            .or_else(|| {
                parse(
                    s,
                    NpcKind::Humanoid,
                    &npc_names.humanoid,
                    comp::humanoid::Body::random_with,
                )
            })
            .or_else(|| {
                parse(
                    s,
                    NpcKind::Pig,
                    &npc_names.quadruped_small,
                    comp::quadruped_small::Body::random_with,
                )
            })
            .or_else(|| {
                parse(
                    s,
                    NpcKind::Wolf,
                    &npc_names.quadruped_medium,
                    comp::quadruped_medium::Body::random_with,
                )
            })
            .or_else(|| {
                parse(
                    s,
                    NpcKind::Duck,
                    &npc_names.bird_medium,
                    comp::bird_medium::Body::random_with,
                )
            })
            .or_else(|| {
                parse(
                    s,
                    NpcKind::Ogre,
                    &npc_names.biped_large,
                    comp::biped_large::Body::random_with,
                )
            })
            .or_else(|| {
                parse(
                    s,
                    NpcKind::Rat,
                    &npc_names.critter,
                    comp::critter::Body::random_with,
                )
            })
            .or_else(|| {
                parse(
                    s,
                    NpcKind::StoneGolem,
                    &npc_names.golem,
                    comp::golem::Body::random_with,
                )
            })
            .or_else(|| {
                parse(
                    s,
                    NpcKind::Reddragon,
                    &npc_names.dragon,
                    comp::dragon::Body::random_with,
                )
            })
            .or_else(|| {
                parse(
                    s,
                    NpcKind::Crocodile,
                    &npc_names.quadruped_low,
                    comp::quadruped_low::Body::random_with,
                )
            })
            .ok_or(())
    }
}
