use common::{
    comp::{Health, Player},
    uid::{IdMaps, Uid},
};
use specs::{
    storage::GenericReadStorage, Component, Entities, Entity, Read, ReadStorage, WriteStorage,
};
use std::sync::atomic::{AtomicPtr, Ordering};

pub struct EcsWorld<'a, 'b> {
    pub entities: &'b Entities<'a>,
    pub health: EcsComponentAccess<'a, 'b, Health>,
    pub uid: EcsComponentAccess<'a, 'b, Uid>,
    pub player: EcsComponentAccess<'a, 'b, Player>,
    pub id_maps: &'b Read<'a, IdMaps>,
}

pub enum EcsComponentAccess<'a, 'b, T: Component> {
    Read(&'b ReadStorage<'a, T>),
    ReadOwned(ReadStorage<'a, T>),
    Write(&'b WriteStorage<'a, T>),
    WriteOwned(WriteStorage<'a, T>),
}

impl<'a, 'b, T: Component> EcsComponentAccess<'a, 'b, T> {
    pub fn get(&self, entity: Entity) -> Option<&T> {
        match self {
            EcsComponentAccess::Read(e) => e.get(entity),
            EcsComponentAccess::Write(e) => e.get(entity),
            EcsComponentAccess::ReadOwned(e) => e.get(entity),
            EcsComponentAccess::WriteOwned(e) => e.get(entity),
        }
    }
}

impl<'a, 'b, T: Component> From<&'b ReadStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: &'b ReadStorage<'a, T>) -> Self { Self::Read(a) }
}

impl<'a, 'b, T: Component> From<ReadStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: ReadStorage<'a, T>) -> Self { Self::ReadOwned(a) }
}

impl<'a, 'b, T: Component> From<&'b WriteStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: &'b WriteStorage<'a, T>) -> Self { Self::Write(a) }
}

impl<'a, 'b, T: Component> From<WriteStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: WriteStorage<'a, T>) -> Self { Self::WriteOwned(a) }
}

/// This structure wraps the ECS pointer to ensure safety
pub struct EcsAccessManager {
    ecs_pointer: AtomicPtr<EcsWorld<'static, 'static>>,
}

impl Default for EcsAccessManager {
    fn default() -> Self {
        Self {
            ecs_pointer: AtomicPtr::new(std::ptr::null_mut()),
        }
    }
}

impl EcsAccessManager {
    // This function take a World reference and a function to execute ensuring the
    // pointer will never be corrupted during the execution of the function!
    pub fn execute_with<T>(&self, world: &EcsWorld, func: impl FnOnce() -> T) -> T {
        let _guard = scopeguard::guard((), |_| {
            // ensure the pointer is cleared in any case
            self.ecs_pointer
                .store(std::ptr::null_mut(), Ordering::Relaxed);
        });
        self.ecs_pointer
            .store(world as *const _ as *mut _, Ordering::Relaxed);
        func()
    }

    /// This unsafe function returns a reference to the Ecs World
    ///
    /// # Safety
    /// This function is safe to use if it matches the following requirements
    ///  - The reference and subreferences like Entities, Components ... aren't
    ///    leaked out the thread
    ///  - The reference and subreferences lifetime doesn't exceed the source
    ///    function lifetime
    ///  - Always safe when called from `retrieve_action` if you don't pass a
    ///    reference somewhere else
    ///  - All that ensure that the reference doesn't exceed the execute_with
    ///    function scope
    pub unsafe fn get(&self) -> Option<&EcsWorld> {
        // ptr::as_ref will automatically check for null
        self.ecs_pointer.load(Ordering::Relaxed).as_ref()
    }
}
