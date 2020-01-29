use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Body {
    pub species: Species,
    pub body_type: BodyType,
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        let species = *(&ALL_SPECIES).choose(&mut rng).unwrap();
        let body_type = *(&ALL_BODY_TYPES).choose(&mut rng).unwrap();
        Self { species, body_type }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum Species {
    Duck = 0,
    Chicken = 1,
    Goose = 2,
    Peacock = 3,
}

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub duck: SpeciesMeta,
    pub chicken: SpeciesMeta,
    pub goose: SpeciesMeta,
    pub peacock: SpeciesMeta,
}

impl<SpeciesMeta> core::ops::Index<Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    fn index(&self, index: Species) -> &Self::Output {
        match index {
            Species::Duck => &self.duck,
            Species::Chicken => &self.chicken,
            Species::Goose => &self.goose,
            Species::Peacock => &self.peacock,
        }
    }
}

pub const ALL_SPECIES: [Species; 4] = [
    Species::Duck,
    Species::Chicken,
    Species::Goose,
    Species::Peacock,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum BodyType {
    Female = 0,
    Male = 1,
}
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
