use specs::Entity as EcsEntity;
use std::{collections::VecDeque, ops::DerefMut, sync::Mutex};
use vek::*;

pub enum Event {
    LandOnGround { entity: EcsEntity, vel: Vec3<f32> },
    Explosion { pos: Vec3<f32>, radius: f32 },
}

#[derive(Default)]
pub struct EventBus {
    queue: Mutex<VecDeque<Event>>,
}

impl EventBus {
    pub fn emitter(&self) -> Emitter {
        Emitter {
            bus: self,
            events: VecDeque::new(),
        }
    }

    pub fn emit(&self, event: Event) {
        self.queue.lock().unwrap().push_front(event);
    }

    pub fn recv_all(&self) -> impl ExactSizeIterator<Item = Event> {
        std::mem::replace(self.queue.lock().unwrap().deref_mut(), VecDeque::new()).into_iter()
    }
}

pub struct Emitter<'a> {
    bus: &'a EventBus,
    events: VecDeque<Event>,
}

impl<'a> Emitter<'a> {
    pub fn emit(&mut self, event: Event) {
        self.events.push_front(event);
    }
}

impl<'a> Drop for Emitter<'a> {
    fn drop(&mut self) {
        self.bus.queue.lock().unwrap().append(&mut self.events);
    }
}
