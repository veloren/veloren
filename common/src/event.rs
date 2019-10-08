use crate::comp;
use parking_lot::Mutex;
use specs::Entity as EcsEntity;
use sphynx::Uid;
use std::{collections::VecDeque, ops::DerefMut};
use vek::*;

pub enum LocalEvent {
    Jump(EcsEntity),
    WallLeap {
        entity: EcsEntity,
        wall_dir: Vec3<f32>,
    },
    Boost {
        entity: EcsEntity,
        vel: Vec3<f32>,
    },
}

pub enum ServerEvent {
    Explosion {
        pos: Vec3<f32>,
        radius: f32,
    },
    Damage {
        uid: Uid,
        dmg: u32,
        cause: comp::HealthSource,
    },
    Destroy {
        entity: EcsEntity,
        cause: comp::HealthSource,
    },
    Respawn(EcsEntity),
    Shoot {
        entity: EcsEntity,
        dir: Vec3<f32>,
        projectile: comp::Projectile,
    },
    LandOnGround {
        entity: EcsEntity,
        vel: Vec3<f32>,
    },
    Mount(EcsEntity, EcsEntity),
    Unmount(EcsEntity),
}

pub struct EventBus<E> {
    queue: Mutex<VecDeque<E>>,
}

impl<E> Default for EventBus<E> {
    fn default() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }
}

impl<E> EventBus<E> {
    pub fn emitter(&self) -> Emitter<E> {
        Emitter {
            bus: self,
            events: VecDeque::new(),
        }
    }

    pub fn emit(&self, event: E) {
        self.queue.lock().push_front(event);
    }

    pub fn recv_all(&self) -> impl ExactSizeIterator<Item = E> {
        std::mem::replace(self.queue.lock().deref_mut(), VecDeque::new()).into_iter()
    }
}

pub struct Emitter<'a, E> {
    bus: &'a EventBus<E>,
    events: VecDeque<E>,
}

impl<'a, E> Emitter<'a, E> {
    pub fn emit(&mut self, event: E) {
        self.events.push_front(event);
    }
}

impl<'a, E> Drop for Emitter<'a, E> {
    fn drop(&mut self) {
        self.bus.queue.lock().append(&mut self.events);
    }
}
