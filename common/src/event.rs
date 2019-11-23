use crate::comp;
use comp::item::Tool;
use parking_lot::Mutex;
use serde::Deserialize;
use specs::Entity as EcsEntity;
use sphynx::Uid;
use std::{collections::VecDeque, ops::DerefMut};
use vek::*;

pub struct SfxEventItem {
    pub sfx: SfxEvent,
    pub pos: Option<Vec3<f32>>,
}

impl SfxEventItem {
    pub fn new(sfx: SfxEvent, pos: Option<Vec3<f32>>) -> Self {
        Self { sfx, pos }
    }

    pub fn at_player_position(sfx: SfxEvent) -> Self {
        Self { sfx, pos: None }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Deserialize, Hash, Eq)]
pub enum SfxEvent {
    Idle,
    PlaceBlock,
    RemoveBlock,
    OpenChest,
    ChatMessageReceived,
    OpenBag,
    LevelUp,
    Roll,
    Climb,
    Swim,
    Run,
    GliderOpen,
    Glide,
    GliderClose,
    Jump,
    Fall,
    InventoryAdd,
    InventoryDrop,
    LightLantern,
    ExtinguishLantern,
    Attack(Tool),
    AttackWolf,
}

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
        change: comp::HealthChange,
    },
    Destroy {
        entity: EcsEntity,
        cause: comp::HealthSource,
    },
    InventoryManip(EcsEntity, comp::InventoryManip),
    Respawn(EcsEntity),
    Shoot {
        entity: EcsEntity,
        dir: Vec3<f32>,
        body: comp::Body,
        light: Option<comp::LightEmitter>,
        projectile: comp::Projectile,
        gravity: Option<comp::Gravity>,
    },
    LandOnGround {
        entity: EcsEntity,
        vel: Vec3<f32>,
    },
    Mount(EcsEntity, EcsEntity),
    Unmount(EcsEntity),
    Possess(Uid, Uid),
    CreatePlayer {
        entity: EcsEntity,
        name: String,
        body: comp::Body,
        main: Option<String>,
    },
    CreateNpc {
        pos: comp::Pos,
        stats: comp::Stats,
        body: comp::Body,
        agent: comp::Agent,
        scale: comp::Scale,
    },
    ClientDisconnect(EcsEntity),
    ChunkRequest(EcsEntity, Vec2<i32>),
    ChatCmd(EcsEntity, String),
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
