use crate::{
    mesh::{vol, Meshable},
    render::{self, FluidPipeline, Mesh, TerrainPipeline},
};
use common::{
    terrain::{Block, BlockKind},
    vol::{ReadVol, RectRasterableVol, Vox},
    volumes::vol_grid_2d::{CachedVolGrid2d, VolGrid2d},
};
use std::{collections::VecDeque, fmt::Debug};
use vek::*;

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;
type FluidVertex = <FluidPipeline as render::Pipeline>::Vertex;

trait Blendable {
    fn is_blended(&self) -> bool;
}

impl Blendable for BlockKind {
    fn is_blended(&self) -> bool {
        match self {
            BlockKind::Leaves => false,
            _ => true,
        }
    }
}

fn calc_light<V: RectRasterableVol<Vox = Block> + ReadVol + Debug>(
    bounds: Aabb<i32>,
    vol: &VolGrid2d<V>,
) -> impl Fn(Vec3<i32>) -> f32 {
    const UNKNOWN: u8 = 255;
    const OPAQUE: u8 = 254;
    const SUNLIGHT: u8 = 24;

    let outer = Aabb {
        min: bounds.min - Vec3::new(SUNLIGHT as i32 - 1, SUNLIGHT as i32 - 1, 1),
        max: bounds.max + Vec3::new(SUNLIGHT as i32 - 1, SUNLIGHT as i32 - 1, 1),
    };

    let mut vol_cached = vol.cached();

    let mut light_map = vec![UNKNOWN; outer.size().product() as usize];
    let lm_idx = {
        let (w, h, _) = outer.clone().size().into_tuple();
        move |x, y, z| (z * h * w + x * h + y) as usize
    };
    // Light propagation queue
    let mut prop_que = VecDeque::new();
    // Start sun rays
    for x in 0..outer.size().w {
        for y in 0..outer.size().h {
            let z = outer.size().d - 1;
            let is_air = vol_cached
                .get(outer.min + Vec3::new(x, y, z))
                .ok()
                .map_or(false, |b| b.is_air());

            light_map[lm_idx(x, y, z)] = if is_air {
                if vol_cached
                    .get(outer.min + Vec3::new(x, y, z - 1))
                    .ok()
                    .map_or(false, |b| b.is_air())
                {
                    light_map[lm_idx(x, y, z - 1)] = SUNLIGHT;
                    prop_que.push_back((x as u8, y as u8, z as u16));
                }
                SUNLIGHT
            } else {
                OPAQUE
            };
        }
    }

    // Determines light propagation
    let propagate = |src: u8,
                     dest: &mut u8,
                     pos: Vec3<i32>,
                     prop_que: &mut VecDeque<_>,
                     vol: &mut CachedVolGrid2d<V>| {
        if *dest != OPAQUE {
            if *dest == UNKNOWN {
                if vol
                    .get(outer.min + pos)
                    .ok()
                    .map_or(false, |b| b.is_air() || b.is_fluid())
                {
                    *dest = src - 1;
                    // Can't propagate further
                    if *dest > 1 {
                        prop_que.push_back((pos.x as u8, pos.y as u8, pos.z as u16));
                    }
                } else {
                    *dest = OPAQUE;
                }
            } else if *dest < src - 1 {
                *dest = src - 1;
                // Can't propagate further
                if *dest > 1 {
                    prop_que.push_back((pos.x as u8, pos.y as u8, pos.z as u16));
                }
            }
        }
    };

    // Propage light
    while let Some(pos) = prop_que.pop_front() {
        let pos = Vec3::new(pos.0 as i32, pos.1 as i32, pos.2 as i32);
        let light = light_map[lm_idx(pos.x, pos.y, pos.z)];

        // If ray propagate downwards at full strength
        if light == SUNLIGHT {
            // Down is special cased and we know up is a ray
            // Special cased ray propagation
            let pos = Vec3::new(pos.x, pos.y, pos.z - 1);
            let (is_air, is_fluid) = vol_cached
                .get(outer.min + pos)
                .ok()
                .map_or((false, false), |b| (b.is_air(), b.is_fluid()));
            light_map[lm_idx(pos.x, pos.y, pos.z)] = if is_air {
                prop_que.push_back((pos.x as u8, pos.y as u8, pos.z as u16));
                SUNLIGHT
            } else if is_fluid {
                prop_que.push_back((pos.x as u8, pos.y as u8, pos.z as u16));
                SUNLIGHT - 1
            } else {
                OPAQUE
            }
        } else {
            // Up
            // Bounds checking
            if pos.z + 1 < outer.size().d {
                propagate(
                    light,
                    light_map.get_mut(lm_idx(pos.x, pos.y, pos.z + 1)).unwrap(),
                    Vec3::new(pos.x, pos.y, pos.z + 1),
                    &mut prop_que,
                    &mut vol_cached,
                )
            }
            // Down
            if pos.z > 0 {
                propagate(
                    light,
                    light_map.get_mut(lm_idx(pos.x, pos.y, pos.z - 1)).unwrap(),
                    Vec3::new(pos.x, pos.y, pos.z - 1),
                    &mut prop_que,
                    &mut vol_cached,
                )
            }
        }
        // The XY directions
        if pos.y + 1 < outer.size().h {
            propagate(
                light,
                light_map.get_mut(lm_idx(pos.x, pos.y + 1, pos.z)).unwrap(),
                Vec3::new(pos.x, pos.y + 1, pos.z),
                &mut prop_que,
                &mut vol_cached,
            )
        }
        if pos.y > 0 {
            propagate(
                light,
                light_map.get_mut(lm_idx(pos.x, pos.y - 1, pos.z)).unwrap(),
                Vec3::new(pos.x, pos.y - 1, pos.z),
                &mut prop_que,
                &mut vol_cached,
            )
        }
        if pos.x + 1 < outer.size().w {
            propagate(
                light,
                light_map.get_mut(lm_idx(pos.x + 1, pos.y, pos.z)).unwrap(),
                Vec3::new(pos.x + 1, pos.y, pos.z),
                &mut prop_que,
                &mut vol_cached,
            )
        }
        if pos.x > 0 {
            propagate(
                light,
                light_map.get_mut(lm_idx(pos.x - 1, pos.y, pos.z)).unwrap(),
                Vec3::new(pos.x - 1, pos.y, pos.z),
                &mut prop_que,
                &mut vol_cached,
            )
        }
    }

    move |wpos| {
        let pos = wpos - outer.min;
        light_map
            .get(lm_idx(pos.x, pos.y, pos.z))
            .filter(|l| **l != OPAQUE && **l != UNKNOWN)
            .map(|l| *l as f32 / SUNLIGHT as f32)
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

        let mut lowest_opaque = range.size().d;
        let mut highest_opaque = 0;
        let mut lowest_fluid = range.size().d;
        let mut highest_fluid = 0;
        let mut lowest_air = range.size().d;
        let mut highest_air = 0;
        let flat_get = {
            let (w, h, d) = range.size().into_tuple();
            // z can range from -1..range.size().d + 1
            let d = d + 2;
            let flat = {
                let mut volume = self.cached();
                let mut flat = vec![Block::empty(); (w * h * d) as usize];
                let mut i = 0;
                for x in 0..range.size().w {
                    for y in 0..range.size().h {
                        for z in -1..range.size().d + 1 {
                            let block = volume
                                .get(range.min + Vec3::new(x, y, z))
                                .map(|b| *b)
                                .unwrap_or(Block::empty());
                            if block.is_opaque() {
                                lowest_opaque = lowest_opaque.min(z);
                                highest_opaque = highest_opaque.max(z);
                            } else if block.is_fluid() {
                                lowest_fluid = lowest_fluid.min(z);
                                highest_fluid = highest_fluid.max(z);
                            } else {
                                // Assume air
                                lowest_air = lowest_air.min(z);
                                highest_air = highest_air.max(z);
                            };
                            flat[i] = block;
                            i += 1;
                        }
                    }
                }
                flat
            };

            move |Vec3 { x, y, z }| {
                // z can range from -1..range.size().d + 1
                let z = z + 1;
                match flat.get((x * h * d + y * d + z) as usize).copied() {
                    Some(b) => b,
                    None => panic!("x {} y {} z {} d {} h {}"),
                }
            }
        };

        // TODO: figure out why this has to be -2 instead of -1
        // Constrain iterated area
        let z_start = if (lowest_air > lowest_opaque && lowest_air <= lowest_fluid)
            || (lowest_air > lowest_fluid && lowest_air <= lowest_opaque)
        {
            lowest_air - 2
        } else if lowest_fluid > lowest_opaque && lowest_fluid <= lowest_air {
            lowest_fluid - 2
        } else if lowest_fluid > lowest_air && lowest_fluid <= lowest_opaque {
            lowest_fluid - 1
        } else {
            lowest_opaque - 1
        }
        .max(0);
        let z_end = if (highest_air < highest_opaque && highest_air >= highest_fluid)
            || (highest_air < highest_fluid && highest_air >= highest_opaque)
        {
            highest_air + 1
        } else if highest_fluid < highest_opaque && highest_fluid >= highest_air {
            highest_fluid + 1
        } else if highest_fluid < highest_air && highest_fluid >= highest_opaque {
            highest_fluid
        } else {
            highest_opaque
        }
        .min(range.size().d - 1);
        for x in 1..range.size().w - 1 {
            for y in 1..range.size().w - 1 {
                let mut lights = [[[0.0; 3]; 3]; 3];
                for i in 0..3 {
                    for j in 0..3 {
                        for k in 0..3 {
                            lights[k][j][i] = light(
                                Vec3::new(x + range.min.x, y + range.min.y, z_start + range.min.z)
                                    + Vec3::new(i as i32, j as i32, k as i32)
                                    - 1,
                            );
                        }
                    }
                }

                let get_color = |maybe_block: Option<&Block>, neighbour: bool| {
                    maybe_block
                        .filter(|vox| vox.is_opaque() && (!neighbour || vox.is_blended()))
                        .and_then(|vox| vox.get_color())
                        .map(|col| Rgba::from_opaque(col))
                        .unwrap_or(Rgba::zero())
                };

                let mut blocks = [[[None; 3]; 3]; 3];
                for i in 0..3 {
                    for j in 0..3 {
                        for k in 0..3 {
                            let block = Some(flat_get(
                                Vec3::new(x, y, z_start) + Vec3::new(i as i32, j as i32, k as i32)
                                    - 1,
                            ));
                            blocks[k][j][i] = block;
                        }
                    }
                }

                for z in z_start..z_end + 1 {
                    let pos = Vec3::new(x, y, z);
                    let offs = (pos - Vec3::new(1, 1, -range.min.z)).map(|e| e as f32);

                    lights[0] = lights[1];
                    lights[1] = lights[2];
                    blocks[0] = blocks[1];
                    blocks[1] = blocks[2];

                    for i in 0..3 {
                        for j in 0..3 {
                            lights[2][j][i] =
                                light(pos + range.min + Vec3::new(i as i32, j as i32, 2) - 1);
                        }
                    }
                    for i in 0..3 {
                        for j in 0..3 {
                            let block = Some(flat_get(pos + Vec3::new(i as i32, j as i32, 2) - 1));
                            blocks[2][j][i] = block;
                        }
                    }

                    let block = blocks[1][1][1];
                    let colors = if block.map_or(false, |vox| vox.is_blended()) {
                        let mut colors = [[[Rgba::zero(); 3]; 3]; 3];
                        for i in 0..3 {
                            for j in 0..3 {
                                for k in 0..3 {
                                    colors[i][j][k] = get_color(
                                        blocks[i][j][k].as_ref(),
                                        i != 1 || j != 1 || k != 1,
                                    )
                                }
                            }
                        }
                        colors
                    } else {
                        [[[get_color(blocks[1][1][1].as_ref(), false); 3]; 3]; 3]
                    };

                    // Create mesh polygons
                    if block.map_or(false, |vox| vox.is_opaque()) {
                        vol::push_vox_verts(
                            &mut opaque_mesh,
                            faces_to_make(&blocks, false, |vox| !vox.is_opaque()),
                            offs,
                            &colors,
                            |pos, norm, col, ao, light| {
                                let light = (light.min(ao) * 255.0) as u32;
                                let norm = if norm.x != 0.0 {
                                    if norm.x < 0.0 {
                                        0
                                    } else {
                                        1
                                    }
                                } else if norm.y != 0.0 {
                                    if norm.y < 0.0 {
                                        2
                                    } else {
                                        3
                                    }
                                } else {
                                    if norm.z < 0.0 {
                                        4
                                    } else {
                                        5
                                    }
                                };
                                TerrainVertex::new(norm, light, pos, col)
                            },
                            &lights,
                        );
                    } else if block.map_or(false, |vox| vox.is_fluid()) {
                        vol::push_vox_verts(
                            &mut fluid_mesh,
                            faces_to_make(&blocks, false, |vox| vox.is_air()),
                            offs,
                            &colors,
                            |pos, norm, col, _ao, light| {
                                FluidVertex::new(pos, norm, col, light, 0.3)
                            },
                            &lights,
                        );
                    }
                }
            }
        }

        (opaque_mesh, fluid_mesh)
    }
}

/// Use the 6 voxels/blocks surrounding the center
/// to detemine which faces should be drawn
/// Unlike the one in segments.rs this uses a provided array of blocks instead
/// of retrieving from a volume
/// blocks[z][y][x]
fn faces_to_make(
    blocks: &[[[Option<Block>; 3]; 3]; 3],
    error_makes_face: bool,
    should_add: impl Fn(Block) -> bool,
) -> [bool; 6] {
    // Faces to draw
    let make_face = |opt_v: Option<Block>| opt_v.map(|v| should_add(v)).unwrap_or(error_makes_face);
    [
        make_face(blocks[1][1][0]),
        make_face(blocks[1][1][2]),
        make_face(blocks[1][0][1]),
        make_face(blocks[1][2][1]),
        make_face(blocks[0][1][1]),
        make_face(blocks[2][1][1]),
    ]
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
