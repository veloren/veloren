pub mod segment;

// Library
use vek::*;

// Crate
use crate::render::{
    self,
    Mesh,
};

pub trait Meshable {
    type Pipeline: render::Pipeline;

    fn generate_mesh(&self) -> Mesh<Self::Pipeline> {
        self.generate_mesh_with_offset(Vec3::zero())
    }

    fn generate_mesh_with_offset(&self, offs: Vec3<f32>) -> Mesh<Self::Pipeline>;
}
