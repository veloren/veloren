use region::Voxel;
use render_volume::{RenderVoxel, RenderVolume};
use coord::vec3::Vec3;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "vert_pos",
        norm: [f32; 3] = "vert_norm",
        col: [f32; 4] = "vert_col",
    }
}

#[derive(Copy, Clone)]
pub struct Poly {
    verts: [Vertex; 3],
}

impl Poly {
    pub fn new(v0: Vertex, v1: Vertex, v2: Vertex) -> Poly {
        Poly {
            verts: [v0, v1, v2],
        }
    }
}

#[derive(Copy, Clone)]
pub struct Quad {
    verts: [Vertex; 4],
}

impl Quad {
    pub fn new(v0: Vertex, v1: Vertex, v2: Vertex, v3: Vertex) -> Quad {
        Quad {
            verts: [v0, v1, v2, v3],
        }
    }

    pub fn flat_with_color(p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3], norm: [f32; 3], col: [f32; 4]) -> Quad {
        Quad {
            verts: [
                Vertex { pos: p0, norm, col },
                Vertex { pos: p1, norm, col },
                Vertex { pos: p2, norm, col },
                Vertex { pos: p3, norm, col },
            ],
        }
    }

    pub fn with_offset(&self, off: [f32; 3]) -> Quad {
        let mut nquad = *self;
        nquad.verts[0].pos = [nquad.verts[0].pos[0] + off[0], nquad.verts[0].pos[1] + off[1], nquad.verts[0].pos[2] + off[2]];
        nquad.verts[1].pos = [nquad.verts[1].pos[0] + off[0], nquad.verts[1].pos[1] + off[1], nquad.verts[1].pos[2] + off[2]];
        nquad.verts[2].pos = [nquad.verts[2].pos[0] + off[0], nquad.verts[2].pos[1] + off[1], nquad.verts[2].pos[2] + off[2]];
        nquad.verts[3].pos = [nquad.verts[3].pos[0] + off[0], nquad.verts[3].pos[1] + off[1], nquad.verts[3].pos[2] + off[2]];
        nquad
    }
}

pub struct Mesh {
    verts: Vec<Vertex>,
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh {
            verts: Vec::new(),
        }
    }

    pub fn from<V: RenderVolume>(vol: &V) -> Mesh
        where V::VoxelType : RenderVoxel
    {
        let mut mesh = Mesh::new();

        for x in 0..vol.size().x {
            for y in 0..vol.size().y {
                for z in 0..vol.size().z {
                    let vox = vol.at(Vec3::from((x, y, z))).expect("Attempted to mesh voxel outside volume");

                    if vox.is_opaque() {
                        // +x
                        if !vol.at(Vec3::from((x + 1, y, z))).unwrap_or(V::VoxelType::empty()).is_opaque() {
                            let col = vox.get_color();
                            mesh.add_quads(&[Quad::flat_with_color(
                                [1.0, 0.0, 0.0],
                                [1.0, 1.0, 0.0],
                                [1.0, 1.0, 1.0],
                                [1.0, 0.0, 1.0],
                                [1.0, 0.0, 0.0], // Normal
                                [col.x, col.y, col.z, col.w], // Color
                            ).with_offset([x as f32, y as f32, z as f32])]);
                        }
                        // -x
                        if !vol.at(Vec3::from((x - 1, y, z))).unwrap_or(V::VoxelType::empty()).is_opaque() {
                            let col = vox.get_color();
                            mesh.add_quads(&[Quad::flat_with_color(
                                [0.0, 1.0, 0.0],
                                [0.0, 0.0, 0.0],
                                [0.0, 0.0, 1.0],
                                [0.0, 1.0, 1.0],
                                [-1.0, 0.0, 0.0], // Normal
                                [col.x, col.y, col.z, col.w], // Color
                            ).with_offset([x as f32, y as f32, z as f32])]);
                        }
                        // +y
                        if !vol.at(Vec3::from((x, y + 1, z))).unwrap_or(V::VoxelType::empty()).is_opaque() {
                            let col = vox.get_color();
                            mesh.add_quads(&[Quad::flat_with_color(
                                [1.0, 1.0, 0.0],
                                [0.0, 1.0, 0.0],
                                [0.0, 1.0, 1.0],
                                [1.0, 1.0, 1.0],
                                [0.0, 1.0, 0.0], // Normal
                                [col.x, col.y, col.z, col.w], // Color
                            ).with_offset([x as f32, y as f32, z as f32])]);
                        }
                        // -y
                        if !vol.at(Vec3::from((x, y - 1, z))).unwrap_or(V::VoxelType::empty()).is_opaque() {
                            let col = vox.get_color();
                            mesh.add_quads(&[Quad::flat_with_color(
                                [0.0, 0.0, 0.0],
                                [1.0, 0.0, 0.0],
                                [1.0, 0.0, 1.0],
                                [0.0, 0.0, 1.0],
                                [0.0, -1.0, 0.0], // Normal
                                [col.x, col.y, col.z, col.w], // Color
                            ).with_offset([x as f32, y as f32, z as f32])]);
                        }
                        // +z
                        if !vol.at(Vec3::from((x, y, z + 1))).unwrap_or(V::VoxelType::empty()).is_opaque() {
                            let col = vox.get_color();
                            mesh.add_quads(&[Quad::flat_with_color(
                                [0.0, 0.0, 1.0],
                                [1.0, 0.0, 1.0],
                                [1.0, 1.0, 1.0],
                                [0.0, 1.0, 1.0],
                                [0.0, 0.0, 1.0], // Normal
                                [col.x, col.y, col.z, col.w], // Color
                            ).with_offset([x as f32, y as f32, z as f32])]);
                        }
                        // -z
                        if !vol.at(Vec3::from((x, y, z - 1))).unwrap_or(V::VoxelType::empty()).is_opaque() {
                            let col = vox.get_color();
                            mesh.add_quads(&[Quad::flat_with_color(
                                [1.0, 0.0, 0.0],
                                [0.0, 0.0, 0.0],
                                [0.0, 1.0, 0.0],
                                [1.0, 1.0, 0.0],
                                [0.0, 0.0, -1.0], // Normal
                                [col.x, col.y, col.z, col.w], // Color
                            ).with_offset([x as f32, y as f32, z as f32])]);
                        }
                    }
                }
            }
        }

        mesh
    }

    pub fn vert_count(&self) -> u32 {
        self.verts.len() as u32
    }

    pub fn vertices<'a>(&'a self) -> &'a Vec<Vertex> {
        &self.verts
    }

    pub fn add(&mut self, verts: &[Vertex]) {
        self.verts.extend_from_slice(verts);
    }

    pub fn add_polys(&mut self, polys: &[Poly]) {
        for p in polys {
            self.verts.extend_from_slice(&p.verts);
        }
    }

    pub fn add_quads(&mut self, quads: &[Quad]) {
        for q in quads {
            self.add(&[q.verts[0], q.verts[1], q.verts[2], q.verts[2], q.verts[3], q.verts[0]]);
        }
    }
}
