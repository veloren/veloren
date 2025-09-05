use common_assets::{AssetCombined, AssetHandle, Ron};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use vek::Vec3;

use crate::{comp::Mass, npc::NpcBody};

#[derive(Debug, Deserialize)]
struct PluginSpecies {
    id: String,
    mass: f32,
    dimensions: [f32; 3],
    base_health: u16,
    flee_health: f32,
    parasite_drag: f32,
    base_accel: f32,
    base_ori_rate: f32,
    swim_thrust: Option<f32>,
}

lazy_static! {
    static ref PLUGIN_SPECIES: AssetHandle<Ron<Vec<PluginSpecies>>> =
        Ron::load_expect_combined_static("common.plugin_bodies");
}

mod spec_parser {
    use serde::{Deserialize, Deserializer, Serializer};

    use super::{Body, Species};

    pub fn serialize<S>(species: &Species, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&super::PLUGIN_SPECIES.read().0[*species].id)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Species, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)
            .and_then(|species| {
                Body::from_name(&species).map_err(|_err| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(&species),
                        &"known species",
                    )
                })
            })
            .map(|body| body.species)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Body {
    #[serde(with = "spec_parser")]
    pub species: Species,
}

struct NotFound;

impl Body {
    // TODO: use SpeciesIter here?
    // At the moment, I'm mimicking what Self::random() does
    pub fn iter() -> impl Iterator<Item = Self> { std::iter::once(Body { species: 0 }) }

    pub fn mass(&self) -> Mass { Mass(PLUGIN_SPECIES.read().0[self.species].mass) }

    pub fn dimensions(&self) -> Vec3<f32> {
        Vec3::from_slice(&PLUGIN_SPECIES.read().0[self.species].dimensions)
    }

    pub fn base_health(&self) -> u16 { PLUGIN_SPECIES.read().0[self.species].base_health }

    pub fn flee_health(&self) -> f32 { PLUGIN_SPECIES.read().0[self.species].flee_health }

    pub fn parasite_drag(&self) -> f32 { PLUGIN_SPECIES.read().0[self.species].parasite_drag }

    pub fn base_accel(&self) -> f32 { PLUGIN_SPECIES.read().0[self.species].base_accel }

    pub fn base_ori_rate(&self) -> f32 { PLUGIN_SPECIES.read().0[self.species].base_ori_rate }

    pub fn swim_thrust(&self) -> Option<f32> { PLUGIN_SPECIES.read().0[self.species].swim_thrust }

    pub fn random() -> Self { Body { species: 0 } }

    pub fn id(&self) -> String { PLUGIN_SPECIES.read().0[self.species].id.clone() }

    #[inline]
    pub fn random_with(_rng: &mut impl rand::Rng, &species: &Species) -> Self { Self { species } }

    fn from_name(species: &str) -> Result<Body, NotFound> {
        let guard = PLUGIN_SPECIES.read();
        let elem = guard
            .0
            .iter()
            .enumerate()
            .find(|(_n, spec)| spec.id == species);
        if let Some((n, _)) = elem {
            Ok(Body { species: n })
        } else {
            Err(NotFound)
        }
    }
}

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::Plugin(body) }
}

pub type Species = usize;

/// Data representing per-species generic data.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub plugin: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, _index: &'a Species) -> &Self::Output { &self.plugin }
}

pub struct SpeciesIter {
    current: usize,
    max: usize,
}

impl Iterator for SpeciesIter {
    type Item = Species;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.max {
            None
        } else {
            let result = self.current;
            self.current += 1;
            Some(result)
        }
    }
}

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = SpeciesIter;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter {
        SpeciesIter {
            current: 0,
            max: PLUGIN_SPECIES.read().0.len(),
        }
    }
}

pub fn parse_name(s: &str) -> Option<NpcBody> {
    tracing::info!("parse_name {s}");
    let guard = PLUGIN_SPECIES.read();
    let elem = guard
        .0
        .iter()
        .enumerate()
        .find(|(_n, species)| species.id == s);
    elem.map(|(n, _species)| {
        NpcBody(
            crate::npc::NpcKind::Plugin,
            Box::new(move || crate::comp::body::Body::Plugin(Body { species: n })),
        )
    })
}

pub fn test() {
    println!("{:?}", &*PLUGIN_SPECIES);
}
