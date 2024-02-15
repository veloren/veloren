use crate::{make_case_elim, make_proj_elim};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

make_proj_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct Body {
        pub species: Species,
        pub body_type: BodyType,
    }
);

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        let species = *ALL_SPECIES.choose(&mut rng).unwrap();
        Self::random_with(&mut rng, &species)
    }

    #[inline]
    pub fn random_with(rng: &mut impl rand::Rng, &species: &Species) -> Self {
        let body_type = *ALL_BODY_TYPES.choose(rng).unwrap();
        Self { species, body_type }
    }
}

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::BipedSmall(body) }
}

make_case_elim!(
    species,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Species {
        Gnome = 0,
        Sahagin = 1,
        Adlet = 2,
        Gnarling = 3,
        Mandragora = 4,
        Kappa = 5,
        Cactid = 6,
        Gnoll = 7,
        Haniwa = 8,
        Myrmidon = 9,
        Husk = 10,
        Boreal = 11,
        Bushly = 12,
        Irrwurz = 13,
        Clockwork = 14,
        Flamekeeper = 15,
        ShamanicSpirit = 16,
        Jiangshi = 17,
    }
);

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub gnome: SpeciesMeta,
    pub sahagin: SpeciesMeta,
    pub adlet: SpeciesMeta,
    pub gnarling: SpeciesMeta,
    pub mandragora: SpeciesMeta,
    pub kappa: SpeciesMeta,
    pub cactid: SpeciesMeta,
    pub gnoll: SpeciesMeta,
    pub haniwa: SpeciesMeta,
    pub myrmidon: SpeciesMeta,
    pub husk: SpeciesMeta,
    pub boreal: SpeciesMeta,
    pub bushly: SpeciesMeta,
    pub irrwurz: SpeciesMeta,
    pub clockwork: SpeciesMeta,
    pub flamekeeper: SpeciesMeta,
    pub shamanic_spirit: SpeciesMeta,
    pub jiangshi: SpeciesMeta,
}

impl<'a, SpeciesMeta> core::ops::Index<&'a Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    #[inline]
    fn index(&self, &index: &'a Species) -> &Self::Output {
        match index {
            Species::Gnome => &self.gnome,
            Species::Sahagin => &self.sahagin,
            Species::Adlet => &self.adlet,
            Species::Gnarling => &self.gnarling,
            Species::Mandragora => &self.mandragora,
            Species::Kappa => &self.kappa,
            Species::Cactid => &self.cactid,
            Species::Gnoll => &self.gnoll,
            Species::Haniwa => &self.haniwa,
            Species::Myrmidon => &self.myrmidon,
            Species::Husk => &self.husk,
            Species::Boreal => &self.boreal,
            Species::Bushly => &self.bushly,
            Species::Irrwurz => &self.irrwurz,
            Species::Clockwork => &self.clockwork,
            Species::Flamekeeper => &self.flamekeeper,
            Species::ShamanicSpirit => &self.shamanic_spirit,
            Species::Jiangshi => &self.jiangshi,
        }
    }
}

pub const ALL_SPECIES: [Species; 18] = [
    Species::Gnome,
    Species::Sahagin,
    Species::Adlet,
    Species::Gnarling,
    Species::Mandragora,
    Species::Kappa,
    Species::Cactid,
    Species::Gnoll,
    Species::Haniwa,
    Species::Myrmidon,
    Species::Husk,
    Species::Boreal,
    Species::Bushly,
    Species::Irrwurz,
    Species::Clockwork,
    Species::Flamekeeper,
    Species::ShamanicSpirit,
    Species::Jiangshi,
];

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

make_case_elim!(
    body_type,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum BodyType {
        Female = 0,
        Male = 1,
    }
);
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
