use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;
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
pub enum ParticleEmitterMode {
    Sprinkler,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] // Copy
pub struct ParticleEmitters(pub Vec<ParticleEmitter>);

impl Default for ParticleEmitters {
    fn default() -> Self { Self(vec![ParticleEmitter::default()]) }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticleEmitter {
    pub mode: ParticleEmitterMode,

    // spawn X particles per Y, that live for Z
    // pub model_ref: &str, // can we have some kind of stack based key like a u8?
    pub count: (u8, u8),
    pub frequency: Duration,

    // relative to Pos, Ori components?
    // can these be functions that returns a Vec3<f32>?
    pub initial_lifespan: Duration,
    pub initial_offset: (Vec3<f32>, Vec3<f32>), // fn() -> Vec3<f32>,
    pub initial_scale: (f32, f32),              // fn() -> Vec3<f32>,
    pub initial_orientation: (Vec3<f32>, Vec3<f32>), // fn() -> Vec3<f32>,
    pub initial_velocity: (Vec3<f32>, Vec3<f32>), // fn() -> Vec3<f32>,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            mode: ParticleEmitterMode::Sprinkler,
            // model_key: "voxygen.voxel.not_found",
            count: (2, 5),
            frequency: Duration::from_millis(100),
            initial_lifespan: Duration::from_secs(20),
            initial_offset: (vek::Vec3::broadcast(-0.1), vek::Vec3::broadcast(0.1)),
            initial_orientation: (vek::Vec3::broadcast(0.0), vek::Vec3::broadcast(1.0)),
            initial_scale: (0.1, 2.0),
            initial_velocity: (
                vek::Vec3::new(0.0, 0.0, 0.2),
                vek::Vec3::new(0.01, 0.01, 1.0),
            ),
        }
    }
}

impl Component for ParticleEmitter {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
