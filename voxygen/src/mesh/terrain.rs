use crate::{
    mesh::{vol, Meshable},
    render::{self, FluidPipeline, Mesh, TerrainPipeline},
};
use common::{
    terrain::Block,
    vol::{ReadVol, RectRasterableVol, Vox},
    volumes::vol_grid_2d::VolGrid2d,
};
use hashbrown::{HashMap, HashSet};
use std::fmt::Debug;
use vek::*;

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;
type FluidVertex = <FluidPipeline as render::Pipeline>::Vertex;

const DIRS: [Vec2<i32>; 4] = [
    Vec2 { x: 1, y: 0 },
    Vec2 { x: 0, y: 1 },
    Vec2 { x: -1, y: 0 },
    Vec2 { x: 0, y: -1 },
];

const DIRS_3D: [Vec3<i32>; 6] = [
    Vec3 { x: 1, y: 0, z: 0 },
    Vec3 { x: 0, y: 1, z: 0 },
    Vec3 { x: 0, y: 0, z: 1 },
    Vec3 { x: -1, y: 0, z: 0 },
    Vec3 { x: 0, y: -1, z: 0 },
    Vec3 { x: 0, y: 0, z: -1 },
];

fn calc_light<V: RectRasterableVol<Vox = Block> + ReadVol + Debug>(
    bounds: Aabb<i32>,
    vol: &VolGrid2d<V>,
) -> impl Fn(Vec3<i32>) -> f32 {
    let sunlight = 24;

    let outer = Aabb {
        min: bounds.min - sunlight,
        max: bounds.max + sunlight,
    };

    let mut voids = HashMap::new();
    let mut rays = vec![outer.size().d; outer.size().product() as usize];
    for x in 0..outer.size().w {
        for y in 0..outer.size().h {
            let mut outside = true;
            for z in (0..outer.size().d).rev() {
                let block = vol
                    .get(outer.min + Vec3::new(x, y, z))
                    .ok()
                    .copied()
                    .unwrap_or(Block::empty());

                if !block.is_air() && outside {
                    rays[(outer.size().w * y + x) as usize] = z;
                    outside = false;
                }

                if (block.is_air() || block.is_fluid()) && !outside {
                    voids.insert(Vec3::new(x, y, z), None);
                }
            }
        }
    }

    let mut opens = HashSet::new();
    'voids: for (pos, l) in &mut voids {
        for dir in &DIRS {
            let col = Vec2::<i32>::from(*pos) + dir;
            if pos.z
                > *rays
                    .get(((outer.size().w * col.y) + col.x) as usize)
                    .unwrap_or(&0)
            {
                *l = Some(sunlight - 1);
                opens.insert(*pos);
                continue 'voids;
            }
        }

        if pos.z
            >= *rays
                .get(((outer.size().w * pos.y) + pos.x) as usize)
                .unwrap_or(&0)
        {
            *l = Some(sunlight - 1);
            opens.insert(*pos);
        }
    }

    while opens.len() > 0 {
        let mut new_opens = HashSet::new();
        for open in &opens {
            let parent_l = voids[open].unwrap_or(0);
            for dir in &DIRS_3D {
                let other = *open + *dir;
                if !opens.contains(&other) {
                    if let Some(l) = voids.get_mut(&other) {
                        if l.unwrap_or(0) < parent_l - 1 {
                            new_opens.insert(other);
                        }
                        *l = Some(parent_l - 1);
                    }
                }
            }
        }
        opens = new_opens;
    }

    move |wpos| {
        let pos = wpos - outer.min;
        rays.get(((outer.size().w * pos.y) + pos.x) as usize)
            .and_then(|ray| if pos.z > *ray { Some(1.0) } else { None })
            .or_else(|| {
                if let Some(Some(l)) = voids.get(&pos) {
                    Some(*l as f32 / sunlight as f32)
                } else {
                    None
                }
            })
            .unwrap_or(0.0)
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

        let light = calc_light(range, self);

        for x in range.min.x + 1..range.max.x - 1 {
            for y in range.min.y + 1..range.max.y - 1 {
                let mut lights = [[[0.0; 3]; 3]; 3];
                for i in 0..3 {
                    for j in 0..3 {
                        for k in 0..3 {
                            lights[k][j][i] = light(
                                Vec3::new(x, y, range.min.z)
                                    + Vec3::new(i as i32, j as i32, k as i32)
                                    - 1,
                            );
                        }
                    }
                }

                let get_color = |pos| {
                    self.get(pos)
                        .ok()
                        .filter(|vox| vox.is_opaque())
                        .and_then(|vox| vox.get_color())
                        .map(|col| Rgba::from_opaque(col))
                        .unwrap_or(Rgba::zero())
                };

                let mut colors = [[[Rgba::zero(); 3]; 3]; 3];
                for i in 0..3 {
                    for j in 0..3 {
                        for k in 0..3 {
                            colors[k][j][i] = get_color(
                                Vec3::new(x, y, range.min.z)
                                    + Vec3::new(i as i32, j as i32, k as i32)
                                    - 1,
                            );
                        }
                    }
                }

                for z in range.min.z..range.max.z {
                    let pos = Vec3::new(x, y, z);
                    let offs = (pos - (range.min + 1) * Vec3::new(1, 1, 0)).map(|e| e as f32);

                    lights[0] = lights[1];
                    lights[1] = lights[2];
                    colors[0] = colors[1];
                    colors[1] = colors[2];

                    for i in 0..3 {
                        for j in 0..3 {
                            lights[2][j][i] = light(pos + Vec3::new(i as i32, j as i32, 2) - 1);
                        }
                    }
                    for i in 0..3 {
                        for j in 0..3 {
                            colors[2][j][i] = get_color(pos + Vec3::new(i as i32, j as i32, 2) - 1);
                        }
                    }

                    let block = self.get(pos).ok();

                    // Create mesh polygons
                    if block.map(|vox| vox.is_opaque()).unwrap_or(false) {
                        vol::push_vox_verts(
                            &mut opaque_mesh,
                            self,
                            pos,
                            offs,
                            &colors,
                            |pos, norm, col, ao, light| {
                                TerrainVertex::new(pos, norm, col, light.min(ao))
                            },
                            false,
                            &lights,
                            |vox| !vox.is_opaque(),
                            |vox| vox.is_opaque(),
                        );
                    } else if block.map(|vox| vox.is_fluid()).unwrap_or(false) {
                        vol::push_vox_verts(
                            &mut fluid_mesh,
                            self,
                            pos,
                            offs,
                            &colors,
                            |pos, norm, col, ao, light| {
                                FluidVertex::new(pos, norm, col, light.min(ao), 0.3)
                            },
                            false,
                            &lights,
                            |vox| vox.is_air(),
                            |vox| vox.is_opaque(),
                        );
                    }
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
