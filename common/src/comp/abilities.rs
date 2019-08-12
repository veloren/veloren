use {
    specs::{Component, FlaggedStorage, NullStorage, VecStorage},
    specs_idvs::IDVStorage,
    std::ops::{Deref, DerefMut},
    vek::*,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ability<D: 'static + Send + Sync + Default> {
    started: bool,
    pub time: f32,
    data: D,
    already_synced: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MoveDir(pub Vec2<f32>);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Wield {
    pub applied: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Attack {
    pub applied: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Roll;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Build;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Jump;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Glide;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Respawn;

impl<D: 'static + Send + Sync + Default> Ability<D> {
    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn restart(&mut self) {
        self.started = true;
        self.time = 0.0;
        self.data = Default::default();
    }

    pub fn try_start(&mut self) -> bool {
        if !self.started {
            self.restart();
            true
        } else {
            false
        }
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn stop(&mut self) {
        self.started = false;
        self.data = Default::default();
    }

    /// Returns whether a sync is needed and assumes the ability is synced now.
    pub fn sync(&mut self) -> bool {
        let ret = !self.already_synced;
        self.already_synced = true;
        ret
    }
}
impl<D: 'static + Send + Sync + Default> Component for Ability<D> {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl<D: 'static + Send + Sync + Default> Deref for Ability<D> {
    type Target = D;

    fn deref(&self) -> &D {
        &self.data
    }
}

impl<D: 'static + Send + Sync + Default> DerefMut for Ability<D> {
    fn deref_mut(&mut self) -> &mut D {
        self.already_synced = false;
        &mut self.data
    }
}
