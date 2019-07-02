use specs::{Component, NullStorage, VecStorage};
use vek::*;

// TODO: decide whether sound effect names should be strongly typed (which
// might make sound mods more difficult). If we end up using a procedural
// audio approach we will need to send more parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sound {
    pub name: String,
    pub volume: f32,
}

// AudioEmitter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioEmitter(pub Vec<Sound>);

impl Component for AudioEmitter {
    type Storage = VecStorage<Self>;
}

// AudioListener
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioListener(pub Vec<Sound>);

impl Component for AudioListener {
    type Storage = VecStorage<Self>;
}
