pub mod segment;
pub mod terrain;
mod vol;

use crate::render::{self, Mesh};

pub trait Meshable<'a, P: render::Pipeline, T: render::Pipeline> {
    type Pipeline: render::Pipeline;
    type TranslucentPipeline: render::Pipeline;
    type ShadowPipeline: render::Pipeline;
    type Supplement;

    // Generate meshes - one opaque, one translucent, one shadow
    fn generate_mesh(
        &'a self,
        supp: Self::Supplement,
    ) -> (
        Mesh<Self::Pipeline>,
        Mesh<Self::TranslucentPipeline>,
        Mesh<Self::ShadowPipeline>,
    );
}
