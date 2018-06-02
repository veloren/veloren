use gfx;
use enum_map::EnumMap;

use region::{TerrainChunk, Voxel, BlockMaterial};
use renderer::ColorFormat;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "vert_pos",
        norm: [f32; 3] = "vert_norm",
        col: [f32; 3] = "vert_col",
    }

    constant Constants {
        trans: [[f32; 4]; 4] = "uni_trans",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        constants: gfx::ConstantBuffer<Constants> = "constants",
        out: gfx::RenderTarget<ColorFormat> = "target",
    }
}

pub struct Mesh {
    vertices: Vec<Vertex>,
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh {
            vertices: Vec::new(),
        }
    }

    pub fn from(chunk: &TerrainChunk) -> Mesh {
        let color_map = enum_map! {
            BlockMaterial::Air => vec4!(0.0, 0.0, 0.0, 0.0),
            BlockMaterial::Grass => vec4!(0.0, 1.0, 0.0, 1.0),
            BlockMaterial::Stone => vec4!(0.5, 0.5, 0.5, 1.0),
        };

        // TODO: Make this mesh the chunk
        unimplemented!();
    }

    pub fn vert_count(&self) -> u32 {
        self.vertices.len() as u32
    }

    pub fn vertices<'a>(&'a self) -> &'a Vec<Vertex> {
        &self.vertices
    }

    pub fn add(&mut self, verts: &[Vertex]) {
        self.vertices.extend_from_slice(verts);
    }
}
