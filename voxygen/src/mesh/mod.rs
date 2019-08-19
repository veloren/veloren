pub mod segment;
pub mod terrain;
mod vol;

use crate::render::{self, Mesh};

pub trait Meshable<P: render::Pipeline, T: render::Pipeline> {
    type Pipeline: render::Pipeline;
    type TranslucentPipeline: render::Pipeline;
    type Supplement;

    // Generate meshes - one opaque, one translucent
    fn generate_mesh(
        &self,
        supp: Self::Supplement,
    ) -> (Mesh<Self::Pipeline>, Mesh<Self::TranslucentPipeline>);
}
