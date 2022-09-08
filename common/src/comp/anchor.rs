use specs::{Component, Entity};
use vek::Vec2;

/// This component exists in order to fix a bug that caused entities
/// such as campfires to duplicate because the chunk was double-loaded.
/// See https://gitlab.com/veloren/veloren/-/merge_requests/1543
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// An entity with an Entity Anchor will be destroyed when its anchor Entity
    /// no longer exists
    Entity(Entity),
    /// An entity with Chunk Anchor will be destroyed when both the chunk it's
    /// currently positioned within and its anchor chunk are unloaded
    Chunk(Vec2<i32>),
}

impl Component for Anchor {
    type Storage = specs::VecStorage<Self>;
}
