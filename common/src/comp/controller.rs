use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use sphynx::Uid;
use vek::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ControlEvent {
    Mount(Uid),
    Unmount,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    pub primary: bool,
    pub secondary: bool,
    pub move_dir: Vec2<f32>,
    pub look_dir: Vec3<f32>,
    pub sit: bool,
    pub jump: bool,
    pub roll: bool,
    pub glide: bool,
    pub climb: bool,
    pub climb_down: bool,
    pub wall_leap: bool,
    pub respawn: bool,
    pub events: Vec<ControlEvent>,
}

impl Controller {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn push_event(&mut self, event: ControlEvent) {
        self.events.push(event);
    }
}

impl Component for Controller {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MountState {
    Unmounted,
    MountedBy(Uid),
}

impl Component for MountState {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mounting(pub Uid);

impl Component for Mounting {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
