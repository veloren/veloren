// impls of `InterpolatableComponent` on things defined in `common`, since `common_net` is
// downstream of `common`
use common::comp::{Pos, Vel};
use super::InterpolatableComponent;
use specs::{Component, Entity, World};
use specs_idvs::IdvStorage;
use vek::Vec3;

#[derive(Default)]
pub struct PosBuffer(pub [Vec3<f32>; 4]);

impl Component for PosBuffer {
    type Storage = IdvStorage<Self>;
}

impl InterpolatableComponent for Pos {
    type InterpData = PosBuffer;

    fn interpolate(self, data: &mut Self::InterpData, entity: Entity, world: &World) -> Self {
        for i in 0..data.0.len()-1 {
            data.0[i] = data.0[i+1];
        }
        data.0[data.0.len()-1] = self.0;
        self
    }
}
