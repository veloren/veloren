pub mod greedy;
pub mod segment;
pub mod terrain;

use crate::render::{self, Mesh};

pub trait Meshable<P: render::Pipeline, T> {
    type Pipeline: render::Pipeline;
    type TranslucentPipeline: render::Pipeline;
    type ShadowPipeline: render::Pipeline;
    type Supplement;
    type Result;

    // Generate meshes - one opaque, one translucent, one shadow
    fn generate_mesh(
        self,
        supp: Self::Supplement,
    ) -> (
        Mesh<Self::Pipeline>,
        Mesh<Self::TranslucentPipeline>,
        Mesh<Self::ShadowPipeline>,
        Self::Result,
    );
}
