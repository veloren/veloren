use crate::{
    mesh::{vol, Meshable},
    render::{self, FluidPipeline, Mesh, TerrainPipeline},
};
use common::{
    terrain::{Block, BlockKind},
    vol::{ReadVol, RectRasterableVol},
    volumes::vol_grid_2d::VolGrid2d,
};
use std::fmt::Debug;
use vek::*;

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;
type FluidVertex = <FluidPipeline as render::Pipeline>::Vertex;

fn block_shadow_density(kind: BlockKind) -> (f32, f32) {
    // (density, cap)
    match kind {
        BlockKind::Normal => (0.085, 0.3),
        BlockKind::Dense => (0.3, 0.0),
        BlockKind::Water => (0.15, 0.0),
        kind if kind.is_air() => (0.0, 0.0),
        _ => (1.0, 0.0),
    }
}

impl<V: RectRasterableVol<Vox = Block> + ReadVol + Debug> Meshable<TerrainPipeline, FluidPipeline>
    for VolGrid2d<V>
{
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
                            let (density, cap) = self
                                .get(pos + Vec3::new(i as i32 - 1, j as i32 - 1, -1))
                                .ok()
                                .map(|vox| block_shadow_density(vox.kind()))
                                .unwrap_or((0.0, 0.0));

                            neighbour_light[0][i][j] = (neighbour_light[0][i][j] * (1.0 - density))
                                .max(cap.min(neighbour_light[1][i][j]));
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
impl<V: BaseVol<Vox = Block> + ReadVol + Debug> Meshable for VolGrid3d<V> {
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
