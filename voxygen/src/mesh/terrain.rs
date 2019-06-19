use crate::{
    mesh::{vol, Meshable},
    render::{self, Mesh, TerrainPipeline},
};
use common::{
    terrain::Block,
    vol::{BaseVol, ReadVol, VolSize},
    volumes::vol_map_2d::VolMap2d,
};
use std::fmt::Debug;
use vek::*;

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;

/*
impl<M> Meshable for Dyna<Block, M> {
    type Pipeline = TerrainPipeline;
    type Supplement = ();

    fn generate_mesh(&self, _: Self::Supplement) -> Mesh<Self::Pipeline> {
        let mut mesh = Mesh::new();

        for pos in self
            .iter_positions()
            .filter(|pos| pos.map(|e| e >= 1).reduce_and())
            .filter(|pos| {
                pos.map2(self.get_size(), |e, sz| e < sz as i32 - 1)
                    .reduce_and()
            })
        {
            let offs = pos.map(|e| e as f32 - 1.0);

            if let Some(col) = self.get(pos).ok().and_then(|vox| vox.get_color()) {
                let col = col.map(|e| e as f32 / 255.0);

                vol::push_vox_verts(&mut mesh, self, pos, offs, col, TerrainVertex::new, true);
            }
        }

        mesh
    }
}
*/

impl<V: BaseVol<Vox = Block> + ReadVol + Debug, S: VolSize + Clone> Meshable for VolMap2d<V, S> {
    type Pipeline = TerrainPipeline;
    type Supplement = Aabb<i32>;

    fn generate_mesh(&self, range: Self::Supplement) -> Mesh<Self::Pipeline> {
        let mut mesh = Mesh::new();

        for x in range.min.x + 1..range.max.x - 1 {
            for y in range.min.y + 1..range.max.y - 1 {
                let mut neighbour_light = [[[1.0f32; 3]; 3]; 3];

                for z in (range.min.z..range.max.z).rev() {
                    let pos = Vec3::new(x, y, z);

                    // Create mesh polygons
                    if let Some(col) = self.get(pos).ok().and_then(|vox| vox.get_color()) {
                        let avg_light = neighbour_light
                            .iter()
                            .map(|row| row.iter())
                            .flatten()
                            .map(|col| col.iter())
                            .flatten()
                            .fold(0.0, |a, x| a + x)
                            / 27.0;
                        let light = avg_light;

                        let col = col.map(|e| e as f32 / 255.0);

                        let offs = (pos - range.min * Vec3::new(1, 1, 0)).map(|e| e as f32)
                            - Vec3::new(1.0, 1.0, 0.0);

                        vol::push_vox_verts(
                            &mut mesh,
                            self,
                            pos,
                            offs,
                            col,
                            |pos, norm, col, ao, light| {
                                TerrainVertex::new(
                                    pos,
                                    norm,
                                    Lerp::lerp(Rgb::zero(), col, ao),
                                    light,
                                )
                            },
                            false,
                            &neighbour_light,
                        );
                    }

                    // Shift lighting
                    neighbour_light[2] = neighbour_light[1];
                    neighbour_light[1] = neighbour_light[0];

                    // Accumulate shade under opaque blocks
                    for i in 0..3 {
                        for j in 0..3 {
                            neighbour_light[0][i][j] = if let Some(opacity) = self
                                .get(pos + Vec3::new(i as i32 - 1, j as i32 - 1, -1))
                                .ok()
                                .and_then(|vox| vox.get_opacity())
                            {
                                (neighbour_light[0][i][j] * (1.0 - opacity * 0.2))
                                    .max(1.0 - opacity * 1.0)
                            } else {
                                (neighbour_light[0][i][j] * 1.035).min(1.0)
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
        mesh
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
