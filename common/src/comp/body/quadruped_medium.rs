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
    Wolf = 0,
    Saber = 1,
    Viper = 2,
    Tuskram = 3,
    Alligator = 4,
    Monitor = 5,
    Lion = 6,
    Tarasque = 7,
}

/// Data representing per-species generic data.
///
/// NOTE: Deliberately don't (yet?) implement serialize.
#[derive(Clone, Debug, Deserialize)]
pub struct AllSpecies<SpeciesMeta> {
    pub wolf: SpeciesMeta,
    pub saber: SpeciesMeta,
    pub viper: SpeciesMeta,
    pub tuskram: SpeciesMeta,
    pub alligator: SpeciesMeta,
    pub monitor: SpeciesMeta,
    pub lion: SpeciesMeta,
    pub tarasque: SpeciesMeta,
}

impl<SpeciesMeta> core::ops::Index<Species> for AllSpecies<SpeciesMeta> {
    type Output = SpeciesMeta;

    fn index(&self, index: Species) -> &Self::Output {
        match index {
            Species::Wolf => &self.wolf,
            Species::Saber => &self.saber,
            Species::Viper => &self.viper,
            Species::Tuskram => &self.tuskram,
            Species::Alligator => &self.alligator,
            Species::Monitor => &self.monitor,
            Species::Lion => &self.lion,
            Species::Tarasque => &self.tarasque,
        }
    }
}

pub const ALL_SPECIES: [Species; 8] = [
    Species::Wolf,
    Species::Saber,
    Species::Viper,
    Species::Tuskram,
    Species::Alligator,
    Species::Monitor,
    Species::Lion,
    Species::Tarasque,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum BodyType {
    Female = 0,
    Male = 1,
}
pub const ALL_BODY_TYPES: [BodyType; 2] = [BodyType::Female, BodyType::Male];
