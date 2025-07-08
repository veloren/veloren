use common_base::{enum_iter, struct_iter};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

struct_iter! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct Body {
        pub species: Species,
        pub body_type: BodyType,
    }
}

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

enum_iter! {
    ~const_array(ALL)
    #[derive(
        Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
        IronDwarf = 14,
        Flamekeeper = 15,
        ShamanicSpirit = 16,
        Jiangshi = 17,
        TreasureEgg = 18,
        GnarlingChieftain = 19,
        BloodmoonHeiress = 20,
        Bloodservant = 21,
        Harlequin = 22,
        GoblinThug = 23,
        GoblinChucker = 24,
        GoblinRuffian = 25,
        GreenLegoom = 26,
        OchreLegoom = 27,
        PurpleLegoom = 28,
        RedLegoom = 29,
        Ashen = 30,
    }
}

/// Data representing per-species generic data.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub iron_dwarf: SpeciesMeta,
    pub flamekeeper: SpeciesMeta,
    pub shamanic_spirit: SpeciesMeta,
    pub jiangshi: SpeciesMeta,
    pub treasure_egg: SpeciesMeta,
    pub gnarling_chieftain: SpeciesMeta,
    pub bloodmoon_heiress: SpeciesMeta,
    pub bloodservant: SpeciesMeta,
    pub harlequin: SpeciesMeta,
    pub goblin_thug: SpeciesMeta,
    pub goblin_chucker: SpeciesMeta,
    pub goblin_ruffian: SpeciesMeta,
    pub green_legoom: SpeciesMeta,
    pub ochre_legoom: SpeciesMeta,
    pub purple_legoom: SpeciesMeta,
    pub red_legoom: SpeciesMeta,
    pub ashen: SpeciesMeta,
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
            Species::IronDwarf => &self.iron_dwarf,
            Species::Flamekeeper => &self.flamekeeper,
            Species::ShamanicSpirit => &self.shamanic_spirit,
            Species::Jiangshi => &self.jiangshi,
            Species::TreasureEgg => &self.treasure_egg,
            Species::GnarlingChieftain => &self.gnarling_chieftain,
            Species::BloodmoonHeiress => &self.bloodmoon_heiress,
            Species::Bloodservant => &self.bloodservant,
            Species::Harlequin => &self.harlequin,
            Species::GoblinThug => &self.goblin_thug,
            Species::GoblinChucker => &self.goblin_chucker,
            Species::GoblinRuffian => &self.goblin_ruffian,
            Species::GreenLegoom => &self.green_legoom,
            Species::OchreLegoom => &self.ochre_legoom,
            Species::PurpleLegoom => &self.purple_legoom,
            Species::RedLegoom => &self.red_legoom,
            Species::Ashen => &self.ashen,
        }
    }
}

pub const ALL_SPECIES: [Species; Species::NUM_KINDS] = Species::ALL;

impl<'a, SpeciesMeta: 'a> IntoIterator for &'a AllSpecies<SpeciesMeta> {
    type IntoIter = std::iter::Copied<std::slice::Iter<'static, Self::Item>>;
    type Item = Species;

    fn into_iter(self) -> Self::IntoIter { ALL_SPECIES.iter().copied() }
}

enum_iter! {
    ~const_array(ALL)
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EnumString, Display)]
    #[repr(u32)]
    pub enum BodyType {
        Female = 0,
        Male = 1,
    }
}
pub const ALL_BODY_TYPES: [BodyType; BodyType::NUM_KINDS] = BodyType::ALL;
