use crate::{
    mesh::{vol, Meshable},
    render::{self, FluidPipeline, Mesh, TerrainPipeline},
};
use common::{
    terrain::{Block, BlockKind},
    vol::{BaseVol, ReadVol, VolSize},
    volumes::vol_map_2d::VolMap2d,
};
use std::fmt::Debug;
use vek::*;

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;
type FluidVertex = <FluidPipeline as render::Pipeline>::Vertex;

fn block_shadow_density(kind: BlockKind) -> Option<f32> {
    match kind {
        BlockKind::Air => None,
        BlockKind::Normal => Some(0.85),
        BlockKind::Dense => Some(3.0),
        BlockKind::Water => Some(0.8),
    }
}

impl<V: BaseVol<Vox = Block> + ReadVol + Debug, S: VolSize + Clone> Meshable for VolMap2d<V, S> {
    type Pipeline = TerrainPipeline;
    type TranslucentPipeline = FluidPipeline;
    type Supplement = Aabb<i32>;

    fn generate_mesh(
        &self,
        range: Self::Supplement,
    ) -> (Mesh<Self::Pipeline>, Mesh<Self::TranslucentPipeline>) {
        let mut opaque_mesh = Mesh::new();
        let mut fluid_mesh = Mesh::new();

        for x in range.min.x + 1..range.max.x - 1 {
            for y in range.min.y + 1..range.max.y - 1 {
                let mut neighbour_light = [[[1.0f32; 3]; 3]; 3];

                for z in (range.min.z..range.max.z).rev() {
                    let pos = Vec3::new(x, y, z);
                    let offs = (pos - (range.min + 1) * Vec3::new(1, 1, 0)).map(|e| e as f32);

                    let block = self.get(pos).ok();

                    // Create mesh polygons
                    if let Some(col) = block
                        .filter(|vox| vox.is_opaque())
                        .and_then(|vox| vox.get_color())
                    {
                        let col = col.map(|e| e as f32 / 255.0);

                        vol::push_vox_verts(
                            &mut opaque_mesh,
                            self,
                            pos,
                            offs,
                            col,
                            |pos, norm, col, ao, light| {
                                TerrainVertex::new(pos, norm, col, light * ao)
                            },
                            false,
                            &neighbour_light,
                            |vox| !vox.is_opaque(),
                            |vox| vox.is_opaque(),
                        );
                    } else if let Some(col) = block
                        .filter(|vox| vox.is_fluid())
                        .and_then(|vox| vox.get_color())
                    {
                        let col = col.map(|e| e as f32 / 255.0);

                        vol::push_vox_verts(
                            &mut fluid_mesh,
                            self,
                            pos,
                            offs,
                            col,
                            |pos, norm, col, ao, light| {
                                FluidVertex::new(pos, norm, col, light * ao, 0.3)
                            },
                            false,
                            &neighbour_light,
                            |vox| vox.is_air(),
                            |vox| vox.is_opaque(),
                        );
                    }

                    // Shift lighting
                    neighbour_light[2] = neighbour_light[1];
                    neighbour_light[1] = neighbour_light[0];

                    // Accumulate shade under opaque blocks
                    for i in 0..3 {
                        for j in 0..3 {
                            neighbour_light[0][i][j] = if let Some(density) = self
                                .get(pos + Vec3::new(i as i32 - 1, j as i32 - 1, -1))
                                .ok()
                                .and_then(|vox| block_shadow_density(vox.kind()))
                            {
                                (neighbour_light[0][i][j] * (1.0 - density * 0.1))
                                    .max(1.0 - density)
                            } else {
                                (neighbour_light[0][i][j] * 1.025).min(1.0)
                            };
                        }
                    }

                    // Spread light
                    neighbour_light[0] = [[neighbour_light[0]
                        .iter()
                        .map(|col| col.iter())
                        .flatten()
                        .copied()
                        .fold(0.0, |a, x| a + x)
                        / 9.0; 3]; 3];
                }
            }
        }

        (opaque_mesh, fluid_mesh)
    }
}

/*
impl<V: BaseVol<Vox = Block> + ReadVol + Debug, S: VolSize + Clone> Meshable for VolMap3d<V, S> {
    type Pipeline = TerrainPipeline;
    type Supplement = Aabb<i32>;

    fn generate_mesh(&self, range: Self::Supplement) -> Mesh<Self::Pipeline> {
        let mut mesh = Mesh::new();

        let mut last_chunk_pos = self.pos_key(range.min);
        let mut last_chunk = self.get_key(last_chunk_pos);

        let size = range.max - range.min;
        for x in 1..size.x - 1 {
            for y in 1..size.y - 1 {
                for z in 1..size.z - 1 {
                    let pos = Vec3::new(x, y, z);

                    let new_chunk_pos = self.pos_key(range.min + pos);
                    if last_chunk_pos != new_chunk_pos {
                        last_chunk = self.get_key(new_chunk_pos);
                        last_chunk_pos = new_chunk_pos;
                    }
                    let offs = pos.map(|e| e as f32 - 1.0);
                    if let Some(chunk) = last_chunk {
                        let chunk_pos = Self::chunk_offs(range.min + pos);
                        if let Some(col) = chunk.get(chunk_pos).ok().and_then(|vox| vox.get_color())
                        {
                            let col = col.map(|e| e as f32 / 255.0);

                            vol::push_vox_verts(
                                &mut mesh,
                                self,
                                range.min + pos,
                                offs,
                                col,
                                TerrainVertex::new,
                                false,
                            );
                        }
                    } else {
                        if let Some(col) = self
                            .get(range.min + pos)
                            .ok()
                            .and_then(|vox| vox.get_color())
                        {
                            let col = col.map(|e| e as f32 / 255.0);

                            vol::push_vox_verts(
                                &mut mesh,
                                self,
                                range.min + pos,
                                offs,
                                col,
                                TerrainVertex::new,
                                false,
                            );
                        }
                    }
                }
            }
        }
        mesh
    }
}
*/
