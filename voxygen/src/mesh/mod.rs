pub mod segment;
pub mod terrain;

// Library
use vek::*;

// Crate
use crate::render::{
    self,
    Mesh,
};

pub trait Meshable {
    type Pipeline: render::Pipeline;
    type Supplement;

    fn generate_mesh(&self, supp: Self::Supplement) -> Mesh<Self::Pipeline>;
}
