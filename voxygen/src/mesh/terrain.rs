use crate::{
    mesh::{
        greedy::{self, GreedyConfig, GreedyMesh},
        Meshable,
    },
    render::{self, ColLightInfo, FluidPipeline, Mesh, ShadowPipeline, TerrainPipeline},
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

#[derive(Clone, Copy, PartialEq)]
enum FaceKind {
    /// Opaque face that is facing something non-opaque; either
    /// water (Opaque(true)) or something else (Opaque(false)).
    Opaque(bool),
    /// Fluid face that is facing something non-opaque, non-tangible,
    /// and non-fluid (most likely air).
    Fluid,
}

trait Blendable {
    fn is_blended(&self) -> bool;
}

impl Blendable for BlockKind {
    #[allow(clippy::match_single_binding)] // TODO: Pending review in #587
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
    Meshable<TerrainPipeline, FluidPipeline> for &'a VolGrid2d<V>
{
    type Pipeline = TerrainPipeline;
    type Result = (Aabb<f32>, ColLightInfo);
    type ShadowPipeline = ShadowPipeline;
    type Supplement = (Aabb<i32>, Vec2<u16>);
    type TranslucentPipeline = FluidPipeline;

    #[allow(clippy::collapsible_if)]
    #[allow(clippy::many_single_char_names)]
    #[allow(clippy::needless_range_loop)] // TODO: Pending review in #587
    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    #[allow(clippy::panic_params)] // TODO: Pending review in #587

    fn generate_mesh(
        self,
        (range, max_texture_size): Self::Supplement,
    ) -> (
        Mesh<Self::Pipeline>,
        Mesh<Self::TranslucentPipeline>,
        Mesh<Self::ShadowPipeline>,
        Self::Result,
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
                    None => panic!("x {} y {} z {} d {} h {}", x, y, z, d, h),
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
        // let mut opaque_mesh = Mesh::new();
        // let mut fluid_mesh = Mesh::new();

        /* for x in 1..range.size().w - 1 {
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
                        .map(Rgba::from_opaque)
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
                    /* if block.map_or(false, |vox| vox.is_opaque()) {
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
                    } else */if block.map_or(false, |vox| vox.is_fluid()) {
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
        }*/

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
        let max_size =
            guillotiere::Size::new(i32::from(max_texture_size.x), i32::from(max_texture_size.y));
        let greedy_size = Vec3::new(
            (range.size().w - 2) as usize,
            (range.size().h - 2) as usize,
            (z_end - z_start + 1) as usize,
        );
        let greedy_size_cross = Vec3::new(greedy_size.x - 1, greedy_size.y - 1, greedy_size.z);
        let draw_delta = Vec3::new(1, 1, z_start);

        let get_light = |_: &mut (), pos: Vec3<i32>| light(pos + range.min);
        let get_color =
            |_: &mut (), pos: Vec3<i32>| flat_get(pos).get_color().unwrap_or(Rgb::zero());
        let get_opacity = |_: &mut (), pos: Vec3<i32>| !flat_get(pos).is_opaque();
        let flat_get = |pos| flat_get(pos);
        let should_draw = |_: &mut (), pos: Vec3<i32>, delta: Vec3<i32>, _uv| {
            should_draw_greedy(pos, delta, flat_get)
        };
        // NOTE: Conversion to f32 is fine since this i32 is actually in bounds for u16.
        // let create_shadow = |pos, norm, meta| ShadowVertex::new(pos + Vec3::new(0.0,
        // 0.0, (z_start + range.min.z) as f32), norm, meta);
        let mesh_delta = Vec3::new(0.0, 0.0, (z_start + range.min.z) as f32);
        let create_opaque = |atlas_pos, pos, norm, meta| {
            TerrainVertex::new(atlas_pos, pos + mesh_delta, norm, meta)
        };
        let create_transparent = |_atlas_pos, pos, norm| FluidVertex::new(pos + mesh_delta, norm);

        let mut greedy = GreedyMesh::new(max_size);
        let mut opaque_mesh = Mesh::new();
        let mut fluid_mesh = Mesh::new();
        let bounds = greedy.push(
            GreedyConfig {
                data: (),
                draw_delta,
                greedy_size,
                greedy_size_cross,
                get_light,
                get_color,
                get_opacity,
                should_draw,
                push_quad: |atlas_origin, dim, origin, draw_dim, norm, meta: &FaceKind| match meta {
                    FaceKind::Opaque(meta) => {
                        opaque_mesh.push_quad(greedy::create_quad(
                            atlas_origin,
                            dim,
                            origin,
                            draw_dim,
                            norm,
                            meta,
                            |atlas_pos, pos, norm, &meta| create_opaque(atlas_pos, pos, norm, meta),
                        ));
                    },
                    FaceKind::Fluid => {
                        fluid_mesh.push_quad(greedy::create_quad(
                            atlas_origin,
                            dim,
                            origin,
                            draw_dim,
                            norm,
                            &(),
                            |atlas_pos, pos, norm, &_meta| create_transparent(atlas_pos, pos, norm),
                        ));
                    },
                },
            },
        );

        let bounds = bounds.map(f32::from);
        let bounds = Aabb {
            min: bounds.min + mesh_delta,
            max: bounds.max + mesh_delta,
        };
        let (col_lights, col_lights_size) = greedy.finalize();
        // println!("z_bounds{:?}, bounds: {:?}", (mesh_delta.z, mesh_delta.z +

        (
            opaque_mesh,
            fluid_mesh,
            Mesh::new(),
            (bounds, (col_lights, col_lights_size)),
        )
    }
}

fn should_draw_greedy(
    pos: Vec3<i32>,
    delta: Vec3<i32>,
    flat_get: impl Fn(Vec3<i32>) -> Block,
) -> Option<(bool, FaceKind)> {
    // TODO: Verify conversion.
    // let pos = pos.map(|e| e as i32) + draw_delta; // - delta;
    let from = flat_get(pos - delta); // map(|v| v.is_opaque()).unwrap_or(false);
    let to = flat_get(pos); //map(|v| v.is_opaque()).unwrap_or(false);
    let from_opaque = from.is_opaque();
    if from_opaque == to.is_opaque() {
        // Check the interface of fluid and non-tangible non-fluids (e.g. air).
        let from_fluid = from.is_fluid();
        if from_fluid == to.is_fluid() || from.is_tangible() || to.is_tangible() {
            None
        } else {
            // While fluid is not culled, we still try to keep a consistent orientation as
            // we do for land; if going from fluid to non-fluid,
            // forwards-facing; otherwise, backwards-facing.
            Some((from_fluid, FaceKind::Fluid))
        }
    } else {
        // If going from transparent to opaque, backward facing; otherwise, forward
        // facing.  Also, if either from or to is fluid, set the meta accordingly.
        Some((
            from_opaque,
            FaceKind::Opaque(if from_opaque {
                to.is_fluid()
            } else {
                from.is_fluid()
            }),
        ))
    }
}
