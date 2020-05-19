use crate::{
    mesh::{vol, Meshable},
    render::{self, mesh::Quad, FluidPipeline, Mesh, ShadowPipeline, TerrainPipeline},
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
type ShadowVertex = <ShadowPipeline as render::Pipeline>::Vertex;

trait Blendable {
    fn is_blended(&self) -> bool;
}

impl Blendable for BlockKind {
    fn is_blended(&self) -> bool {
        match self {
            _ => false,
        }
    }
}

fn calc_light<V: RectRasterableVol<Vox = Block> + ReadVol + Debug>(
    bounds: Aabb<i32>,
    vol: &VolGrid2d<V>,
) -> impl FnMut(Vec3<i32>) -> f32 + '_ {
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

impl<'a, V: RectRasterableVol<Vox = Block> + ReadVol + Debug>
    Meshable<'a, TerrainPipeline, FluidPipeline> for VolGrid2d<V>
{
    type Pipeline = TerrainPipeline;
    type ShadowPipeline = ShadowPipeline;
    type Supplement = Aabb<i32>;
    type TranslucentPipeline = FluidPipeline;

    fn generate_mesh(
        &'a self,
        range: Self::Supplement,
    ) -> (
        Mesh<Self::Pipeline>,
        Mesh<Self::TranslucentPipeline>,
        Mesh<Self::ShadowPipeline>,
    ) {
        let mut light = calc_light(range, self);

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

        // // We use multiple meshes and then combine them later such that we can group
        // similar z // levels together (better rendering performance)
        // let mut opaque_meshes = vec![Mesh::new(); ((z_end + 1 - z_start).clamped(1,
        // 60) as usize / 10).max(1)];
        let mut opaque_mesh = Mesh::new();
        let mut fluid_mesh = Mesh::new();

        for x in 1..range.size().w - 1 {
            for y in 1..range.size().w - 1 {
                let mut blocks = [[[None; 3]; 3]; 3];
                for i in 0..3 {
                    for j in 0..3 {
                        for k in 0..3 {
                            blocks[k][j][i] = Some(flat_get(
                                Vec3::new(x, y, z_start) + Vec3::new(i as i32, j as i32, k as i32)
                                    - 1,
                            ));
                        }
                    }
                }

                let mut lights = [[[None; 3]; 3]; 3];
                for i in 0..3 {
                    for j in 0..3 {
                        for k in 0..3 {
                            lights[k][j][i] = if blocks[k][j][i]
                                .map(|block| block.is_opaque())
                                .unwrap_or(false)
                            {
                                None
                            } else {
                                Some(light(
                                    Vec3::new(
                                        x + range.min.x,
                                        y + range.min.y,
                                        z_start + range.min.z,
                                    ) + Vec3::new(i as i32, j as i32, k as i32)
                                        - 1,
                                ))
                            };
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

                for z in z_start..z_end + 1 {
                    let pos = Vec3::new(x, y, z);
                    let offs = (pos - Vec3::new(1, 1, -range.min.z)).map(|e| e as f32);

                    lights[0] = lights[1];
                    lights[1] = lights[2];
                    blocks[0] = blocks[1];
                    blocks[1] = blocks[2];

                    for i in 0..3 {
                        for j in 0..3 {
                            let block = Some(flat_get(pos + Vec3::new(i as i32, j as i32, 2) - 1));
                            blocks[2][j][i] = block;
                        }
                    }
                    for i in 0..3 {
                        for j in 0..3 {
                            lights[2][j][i] = if blocks[2][j][i]
                                .map(|block| block.is_opaque())
                                .unwrap_or(false)
                            {
                                None
                            } else {
                                Some(light(
                                    pos + range.min + Vec3::new(i as i32, j as i32, 2) - 1,
                                ))
                            };
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

                    // let opaque_mesh_index = ((z - z_start) * opaque_meshes.len() as i32 / (z_end
                    // + 1 - z_start).max(1)) as usize; let selected_opaque_mesh
                    // = &mut opaque_meshes[opaque_mesh_index]; Create mesh
                    // polygons
                    if block.map_or(false, |vox| vox.is_opaque()) {
                        vol::push_vox_verts(
                            &mut opaque_mesh, //selected_opaque_mesh,
                            faces_to_make(&blocks, None, |vox| {
                                if vox.is_opaque() {
                                    None
                                } else {
                                    Some(vox.is_fluid())
                                }
                            }),
                            offs,
                            &colors,
                            |pos, norm, col, light, ao, &meta| {
                                //let light = (light.min(ao) * 255.0) as u32;
                                let light = (light * 255.0) as u32;
                                let ao = (ao * 255.0) as u32;
                                let norm = if norm.x != 0.0 {
                                    if norm.x < 0.0 { 0 } else { 1 }
                                } else if norm.y != 0.0 {
                                    if norm.y < 0.0 { 2 } else { 3 }
                                } else {
                                    if norm.z < 0.0 { 4 } else { 5 }
                                };
                                TerrainVertex::new(norm, light, ao, pos, col, meta)
                            },
                            &lights,
                        );
                    } else if block.map_or(false, |vox| vox.is_fluid()) {
                        vol::push_vox_verts(
                            &mut fluid_mesh,
                            // NOTE: want to skip blocks that aren't either next to air, or next to
                            // opaque blocks like ground.  Addnig the blocks next to ground lets us
                            // make sure we compute lighting effects both at the water surface, and
                            // just before hitting the ground.
                            faces_to_make(&blocks, Some(()), |vox| {
                                if vox.is_air() { Some(()) } else { None }
                            }),
                            offs,
                            &colors,
                            |pos, norm, col, light, _ao, _meta| {
                                /* let rel_pos = pos - offs;
                                let rel_vox_pos = if rel_pos == offs {
                                    rel_pos + norm + 1.0
                                } else {
                                    rel_pos + 1.0
                                }.map(|e| e as usize);
                                let vox_neighbor = blocks[rel_vox_pos.z][rel_vox_pos.y][rel_vox_pos.x];
                                if vox_neighbor.is_opaque() {
                                } else {
                                } */
                                FluidVertex::new(pos, norm, col, light, 0.3)
                            },
                            &lights,
                        );
                    }
                }
            }
        }

        // let opaque_mesh = opaque_meshes
        //     .into_iter()
        //     .rev()
        //     .fold(Mesh::new(), |mut opaque_mesh, m: Mesh<Self::Pipeline>| {
        //         m.verts().chunks_exact(3).rev().for_each(|vs| {
        //             opaque_mesh.push(vs[0]);
        //             opaque_mesh.push(vs[1]);
        //             opaque_mesh.push(vs[2]);
        //         });
        //         opaque_mesh
        //     });

        let mut shadow_mesh = Mesh::new();

        let x_size = (range.size().w - 2) as usize;
        let y_size = (range.size().h - 2) as usize;
        let z_size = (z_end - z_start + 1) as usize;
        let draw_delta = Vec3::new(1, 1, z_start);
        let mesh_delta = Vec3::new(0, 0, z_start + range.min.z);

        // x (u = y, v = z)
        greedy_mesh_cross_section(
            Vec3::new(y_size, z_size, x_size),
            |pos| {
                should_draw_greedy(
                    Vec3::new(pos.z, pos.x, pos.y),
                    draw_delta,
                    Vec3::unit_x(), /* , pos.z, 0, x_size */
                    |pos| flat_get(pos),
                )
            },
            |pos, dim, faces_forward| {
                shadow_mesh.push_quad(create_quad_greedy(
                    Vec3::new(pos.z, pos.x, pos.y),
                    mesh_delta,
                    dim,
                    Vec2::new(Vec3::unit_y(), Vec3::unit_z()),
                    Vec3::unit_x(),
                    faces_forward,
                ));
            },
        );

        // y (u = z, v = x)
        greedy_mesh_cross_section(
            Vec3::new(z_size, x_size, y_size),
            |pos| {
                should_draw_greedy(
                    Vec3::new(pos.y, pos.z, pos.x),
                    draw_delta,
                    Vec3::unit_y(), /* , pos.z, 0, y_size */
                    |pos| flat_get(pos),
                )
            },
            |pos, dim, faces_forward| {
                shadow_mesh.push_quad(create_quad_greedy(
                    Vec3::new(pos.y, pos.z, pos.x),
                    mesh_delta,
                    dim,
                    Vec2::new(Vec3::unit_z(), Vec3::unit_x()),
                    Vec3::unit_y(),
                    faces_forward,
                ));
            },
        );

        // z (u = x, v = y)
        greedy_mesh_cross_section(
            Vec3::new(x_size, y_size, z_size),
            |pos| {
                should_draw_greedy(
                    Vec3::new(pos.x, pos.y, pos.z),
                    draw_delta,
                    Vec3::unit_z(), /* , pos.z, 0, z_size */
                    |pos| flat_get(pos),
                )
            },
            |pos, dim, faces_forward| {
                shadow_mesh.push_quad(create_quad_greedy(
                    Vec3::new(pos.x, pos.y, pos.z),
                    mesh_delta,
                    dim,
                    Vec2::new(Vec3::unit_x(), Vec3::unit_y()),
                    Vec3::unit_z(),
                    faces_forward,
                ));
            },
        );

        (opaque_mesh, fluid_mesh, shadow_mesh)
    }
}

/// Use the 6 voxels/blocks surrounding the center
/// to detemine which faces should be drawn
/// Unlike the one in segments.rs this uses a provided array of blocks instead
/// of retrieving from a volume
/// blocks[z][y][x]
fn faces_to_make<M: Clone>(
    blocks: &[[[Option<Block>; 3]; 3]; 3],
    error_makes_face: Option<M>,
    should_add: impl Fn(Block) -> Option<M>,
) -> [Option<M>; 6] {
    // Faces to draw
    let make_face = |opt_v: Option<Block>| {
        opt_v
            .map(|v| should_add(v))
            .unwrap_or(error_makes_face.clone())
    };
    [
        make_face(blocks[1][1][0]),
        make_face(blocks[1][1][2]),
        make_face(blocks[1][0][1]),
        make_face(blocks[1][2][1]),
        make_face(blocks[0][1][1]),
        make_face(blocks[2][1][1]),
    ]
}

// Greedy meshing.
fn greedy_mesh_cross_section(
    /* mask: &mut [bool], */
    dims: Vec3<usize>,
    // Should we draw a face here (below this vertex)?  If so, is it front or back facing?
    draw_face: impl Fn(Vec3<usize>) -> Option<bool>,
    // Vertex, width and height, and whether it's front facing (face is implicit from the cross
    // section).
    mut push_quads: impl FnMut(Vec3<usize>, Vec2<usize>, bool),
) {
    // mask represents which faces are either set while the other is unset, or unset
    // while the other is set.
    let mut mask = vec![None; dims.y * dims.x];
    (0..dims.z + 1).for_each(|d| {
        // Compute mask
        mask.iter_mut().enumerate().for_each(|(posi, mask)| {
            let i = posi % dims.x;
            let j = posi / dims.x;
            *mask = draw_face(Vec3::new(i, j, d));
        });

        (0..dims.y).for_each(|j| {
            let mut i = 0;
            while i < dims.x {
                // Compute width (number of set x bits for this row and layer, starting at the
                // current minimum column).
                if let Some(ori) = mask[j * dims.x + i] {
                    let width = 1 + mask[j * dims.x + i + 1..j * dims.x + dims.x]
                        .iter()
                        .take_while(move |&&mask| mask == Some(ori))
                        .count();
                    let max_x = i + width;
                    // Compute height (number of rows having w set x bits for this layer, starting
                    // at the current minimum column and row).
                    let height = 1
                        + (j + 1..dims.y)
                            .take_while(|h| {
                                mask[h * dims.x + i..h * dims.x + max_x]
                                    .iter()
                                    .all(|&mask| mask == Some(ori))
                            })
                            .count();
                    let max_y = j + height;
                    // Add quad.
                    push_quads(Vec3::new(i, j, d /* + 1 */), Vec2::new(width, height), ori);
                    // Unset mask bits in drawn region, so we don't try to re-draw them.
                    (j..max_y).for_each(|l| {
                        mask[l * dims.x + i..l * dims.x + max_x]
                            .iter_mut()
                            .for_each(|mask| {
                                *mask = None;
                            });
                    });
                    // Update x value.
                    i = max_x;
                } else {
                    i += 1;
                }
            }
        });
    });
}

fn create_quad_greedy(
    origin: Vec3<usize>,
    mesh_delta: Vec3<i32>,
    dim: Vec2<usize>,
    uv: Vec2<Vec3<f32>>,
    norm: Vec3<f32>,
    faces_forward: bool,
) -> Quad<ShadowPipeline> {
    let origin = origin.map(|e| e as i32) + mesh_delta;
    // let origin = (uv.x * origin.x + uv.y * origin.y + norm * origin.z) +
    // Vec3::new(0, 0, z_start + range.min.z - 1);//Vec3::new(-1, -1, z_start +
    // range.min.z - 1);
    let origin = origin.map(|e| e as f32); // + orientation.z;
    // let origin = uv.x * origin.x + uv.y * origin.y + norm * origin.z +
    // Vec3::new(0.0, 0.0, (z_start + range.min.z - 1) as f32);
    /* if (origin.x < 0.0 || origin.y < 0.0) {
        return;
    } */
    // let ori = if faces_forward { Vec3::new(u, v, norm) } else { Vec3::new(uv.y,
    // uv.x, -norm) };
    let dim = uv.map2(dim.map(|e| e as f32), |e, f| e * f);
    let (dim, norm) = if faces_forward {
        (dim, norm)
    } else {
        (Vec2::new(dim.y, dim.x), -norm)
    };
    // let (uv, norm, origin) = if faces_forward { (uv, norm, origin) } else {
    // (Vec2::new(uv.y, uv.x), -norm, origin) }; let (uv, norm, origin) = if
    // faces_forward { (uv, norm, origin) } else { (Vec2::new(uv.y, uv.x), -norm,
    // origin/* - norm*/) }; let origin = Vec3::new(origin.x as f32., origin.y
    // as f32, (origin.z + z_start) as f32); let norm = norm.map(|e| e as f32);
    Quad::new(
        ShadowVertex::new(origin, norm),
        ShadowVertex::new(origin + dim.x, norm),
        ShadowVertex::new(origin + dim.x + dim.y, norm),
        ShadowVertex::new(origin + dim.y, norm),
    )
}

fn should_draw_greedy(
    pos: Vec3<usize>,
    draw_delta: Vec3<i32>,
    delta: Vec3<i32>,
    /* depth, min_depth, max_depth, */ flat_get: impl Fn(Vec3<i32>) -> Block,
) -> Option<bool> {
    let pos = pos.map(|e| e as i32) + draw_delta; // - delta;
    //
    /* if (depth as isize) <= min_depth {
        // let to = flat_get(pos).is_opaque();
        debug_assert!(depth <= max_depth);
        /* if depth >= max_depth - 1 {
            let from = flat_get(pos - delta).is_opaque();
        } else {
            None
        } */
        if flat_get(pos + delta).is_opaque() {
            Some(true)
        } else {
            None
        }
    } else */
    {
        let from = flat_get(pos - delta).is_opaque(); // map(|v| v.is_opaque()).unwrap_or(false);
        //
        /* if depth > max_depth {
            if from {
                // Backward-facing
                Some(false)
            } else {
                None
            }
        } else */
        {
            let to = flat_get(pos).is_opaque(); //map(|v| v.is_opaque()).unwrap_or(false);
            if from == to {
                None
            } else {
                // If going from transparent to opaque, forward facing; otherwise, backward
                // facing.
                Some(from)
            }
        }
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
