use vek::*;

pub struct NpcInfo {
    pub pos: Vec3<f32>,
    pub boss: bool,
}

#[derive(Default)]
pub struct ChunkSupplement {
    pub npcs: Vec<NpcInfo>,
}
