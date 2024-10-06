use crate::{combat::Attack, comp::item::Reagent, effect::Effect};
use rand::{Rng, thread_rng};
use serde::{Deserialize, Serialize};
use vek::Rgb;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Explosion {
    pub effects: Vec<RadiusEffect>,
    pub radius: f32,
    pub reagent: Option<Reagent>,
    pub min_falloff: f32,
    pub friendly_fire: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RadiusEffect {
    TerrainDestruction(f32, Rgb<f32>),
    Entity(Effect),
    Attack(Attack),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ColorPreset {
    Black,
    InkBomb,
    IceBomb,
}

impl ColorPreset {
    pub fn to_rgb(&self) -> Rgb<f32> {
        match self {
            Self::Black => Rgb::black(),
            Self::InkBomb => Rgb::new(4.0, 7.0, 32.0),
            Self::IceBomb => {
                let variation = thread_rng().gen::<f32>();
                Rgb::new(
                    83.0 - (20.0 * variation),
                    212.0 - (52.0 * variation),
                    255.0 - (62.0 * variation),
                )
            },
        }
    }
}
