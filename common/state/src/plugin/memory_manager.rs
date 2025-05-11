use atomic_refcell::AtomicRefCell;
use common::{
    comp::{Health, Player},
    uid::{IdMaps, Uid},
};
use core::ptr::NonNull;
use specs::{
    Component, Entities, Entity, Read, ReadStorage, WriteStorage, storage::GenericReadStorage,
};
use tracing::error;

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

impl<T: Component> EcsComponentAccess<'_, '_, T> {
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

impl<'a, T: Component> From<ReadStorage<'a, T>> for EcsComponentAccess<'a, '_, T> {
    fn from(a: ReadStorage<'a, T>) -> Self { Self::ReadOwned(a) }
}

impl<'a, 'b, T: Component> From<&'b WriteStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: &'b WriteStorage<'a, T>) -> Self { Self::Write(a) }
}

impl<'a, T: Component> From<WriteStorage<'a, T>> for EcsComponentAccess<'a, '_, T> {
    fn from(a: WriteStorage<'a, T>) -> Self { Self::WriteOwned(a) }
}

/// This structure wraps the ECS pointer to ensure safety
pub struct EcsAccessManager {
    ecs_pointer: AtomicRefCell<Option<NonNull<EcsWorld<'static, 'static>>>>,
}

// SAFETY: Synchronization is handled via `AtomicRefCell`. These impls are
// bounded on the Send/Sync properties of the EcsWorld reference.
unsafe impl Send for EcsAccessManager where for<'a, 'b, 'c> &'a EcsWorld<'b, 'c>: Send {}
unsafe impl Sync for EcsAccessManager where for<'a, 'b, 'c> &'a EcsWorld<'b, 'c>: Sync {}

impl Default for EcsAccessManager {
    fn default() -> Self {
        Self {
            ecs_pointer: AtomicRefCell::new(None),
        }
    }
}

impl EcsAccessManager {
    // This function take a World reference and a function to execute ensuring the
    // pointer will never be corrupted during the execution of the function!
    pub fn execute_with<T>(&self, world: &EcsWorld, func: impl FnOnce() -> T) -> T {
        let _guard = scopeguard::guard((), |_| {
            // ensure the pointer is cleared in any case
            if let Ok(mut ptr) = self.ecs_pointer.try_borrow_mut() {
                *ptr = None;
            } else {
                error!("EcsWorld reference still in use when `func` finished, aborting");
                std::process::abort();
            }
        });
        *self.ecs_pointer.borrow_mut() =
            Some(NonNull::from(world).cast::<EcsWorld<'static, 'static>>());
        func()
    }

    /// Calls the provided closure with a reference to the ecs world.
    ///
    /// # Aborts
    ///
    /// Aborts if `execute_with` returns while calling this. For example, this
    /// can happen if this is called from a separate thread and there isn't
    /// anything preventing `execute_with` from returning.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: for<'a, 'b, 'c> FnOnce(Option<&'a EcsWorld<'b, 'c>>) -> R,
    {
        let ptr = self.ecs_pointer.borrow();
        let ecs_world = ptr.map(|ptr| {
            // SAFETY: If this is Some, we are inside an `execute_with` call and this is
            // a valid reference at least until `execute_with` returns.
            //
            // We hold a shared borrow guard while the reference is in use here and abort
            // if `execute_with` finishes while this is still held.
            //
            // The called closure can't escape the reference because it must be callable for
            // any set of lifetimes. Variance of the lifetime parameters in EcsWorld are
            // not an issue for the same reason:
            // https://discord.com/channels/273534239310479360/592856094527848449/1111018259815342202
            unsafe { ptr.as_ref() }
        });
        f(ecs_world)
    }
}
