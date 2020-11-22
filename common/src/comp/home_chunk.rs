use specs::Component;
use specs_idvs::IdvStorage;
use vek::Vec2;

/// This component exists in order to fix a bug that caused entitites
/// such as campfires to duplicate because the chunk was double-loaded.
/// See https://gitlab.com/veloren/veloren/-/merge_requests/1543
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct HomeChunk(pub Vec2<i32>);

impl Component for HomeChunk {
    type Storage = IdvStorage<Self>;
}
