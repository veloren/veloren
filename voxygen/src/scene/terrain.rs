// Standard
use std::collections::HashMap;

// Library
use vek::*;

// Crate
use crate::{
    Error,
    render::{
        Consts,
        Globals,
        Mesh,
        Model,
        Renderer,
        TerrainPipeline,
        TerrainLocals,
    },
};

struct TerrainChunk {
    // GPU data
    model: Model<TerrainPipeline>,
    locals: Consts<TerrainLocals>,
}

pub struct Terrain {
    chunks: HashMap<Vec3<i32>, TerrainChunk>,
}

impl Terrain {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn maintain_gpu_data(&mut self, renderer: &mut Renderer) {
        // TODO
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        /*
        renderer.render_terrain_chunk(
            &self.model,
            globals,
            &self.locals,
            &self.bone_consts,
        );
        */
    }
}
