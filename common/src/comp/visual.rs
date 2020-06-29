use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightEmitter {
    pub col: Rgb<f32>,
    pub strength: f32,
    pub flicker: f32,
    pub animated: bool,
}

impl Default for LightEmitter {
    fn default() -> Self {
        Self {
            col: Rgb::one(),
            strength: 1.0,
            flicker: 0.0,
            animated: false,
        }
    }
}

impl Component for LightEmitter {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightAnimation {
    pub offset: Vec3<f32>,
    pub col: Rgb<f32>,
    pub strength: f32,
}

impl Default for LightAnimation {
    fn default() -> Self {
        Self {
            offset: Vec3::zero(),
            col: Rgb::zero(),
            strength: 0.0,
        }
    }
}

impl Component for LightAnimation {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}


#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticleEmitter {

    /// Mode 1: sprinkler (inital_velocity, lifespan)
    /// Mode 2: smoke (initial_position, boyancy_const, wind, lifespan)
    pub mode: u8, // enum?

    // pub vertices: Vec<i8>,
    // pub texture: RasterFooBar,

    // // mode 1 -- sprinkler.
    // pub initial_position: [i8; 3],
    // pub initial_velocity: [i8; 3],
    // pub lifespan: u32, // in ticks?
    
    // // mode 2 -- smoke
    // pub initial_position: [i8; 3],
    // pub boyancy_const: [i8; 3],
    // pub wind_sway: [i8; 3],
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            mode: 0,
        }
    }
}

impl Component for ParticleEmitter {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}