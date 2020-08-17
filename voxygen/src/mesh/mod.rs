pub mod greedy;
pub mod segment;
pub mod terrain;

use crate::render::{self, Mesh};

pub type MeshGen<P, T, M> = (
    Mesh<<M as Meshable<P, T>>::Pipeline>,
    Mesh<<M as Meshable<P, T>>::TranslucentPipeline>,
    Mesh<<M as Meshable<P, T>>::ShadowPipeline>,
    <M as Meshable<P, T>>::Result,
);

/// FIXME: Remove this whole trait at some point.  This "abstraction" is never
/// abstracted over, and is organized completely differently from how we
/// actually mesh things nowadays.
pub trait Meshable<P: render::Pipeline, T> {
    type Pipeline: render::Pipeline;
    type TranslucentPipeline: render::Pipeline;
    type ShadowPipeline: render::Pipeline;
    type Supplement;
    type Result;

    // Generate meshes - one opaque, one translucent, one shadow
    fn generate_mesh(self, supp: Self::Supplement) -> MeshGen<P, T, Self>;
}
