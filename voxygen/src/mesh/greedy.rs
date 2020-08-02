use crate::render::{self, mesh::Quad, ColLightFmt, ColLightInfo, TerrainPipeline};
use vek::*;

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;

/// `max_size`:
///
/// `draw_delta`:
///
/// `greedy_size`:
///
/// `greedy_size_cross`:
///
/// `get_light`:
///
/// `get_color`:
///
///
/// `create_shadow`:
/// Create a shadow vertex (used for both shadow and display rendering)
/// given its position, normal, and meta information.  Note that the position
/// received here is relative to `draw_delta`--it still needs to be translated
/// to mesh coordinates.
///
/// `create_opaque`:
pub struct GreedyConfig<D, FL, FC, FO, FS, FP> {
    pub data: D,
    /// The minimum position to mesh, in the coordinate system used
    /// for queries against the volume.
    pub draw_delta: Vec3<i32>,
    /// For each dimension i, for faces drawn in planes *parallel* to i,
    /// represents the number of voxels considered along dimenson i in those
    /// planes, starting from `draw_delta`.
    pub greedy_size: Vec3<usize>,
    /// For each dimension i, represents the number of planes considered
    /// *orthogonal* to dimension i, starting from `draw_delta`.  This should
    /// usually be the same as greedy_size.
    ///
    /// An important exception is during chunk rendering (where vertical faces
    /// at chunk boundaries would otherwise be rendered twice, and also
    /// force us to use more than 5 bits to represent x and y
    /// positions--though there may be a clever way aruond the latter).
    /// Thus, for chunk rendering we set the number of *vertical* planes to
    /// one less than the chunk size along the x and y dimensions, but keep
    /// the number of *horizontal* planes large enough to cover the whole
    /// chunk.
    pub greedy_size_cross: Vec3<usize>,
    /// Given a position, return the lighting information for the voxel at that
    /// position.
    pub get_light: FL,
    /// Given a position, return the color information for the voxel at that
    /// position.
    pub get_color: FC,
    /// Given a position, return the opacity information for the voxel at that
    /// position. Currently, we don't support real translucent lighting, so the
    /// value should either be `false` (for opaque blocks) or `true`
    /// (otherwise).
    pub get_opacity: FO,
    /// Given a position and a normal, should we draw the face between the
    /// position and position - normal (i.e. the voxel "below" this vertex)?
    /// If so, provide its orientation, together with any other meta
    /// information required for the mesh that needs to split up faces.  For
    /// example, terrain faces currently record a bit indicating whether
    /// they are exposed to water or not, so we should not merge faces where
    /// one is submerged in water and the other is not, even if they
    /// otherwise have the same orientation, dimensions, and are
    /// next to each other.
    pub should_draw: FS,
    /// Create an opauqe quad (used for only display rendering) from its
    /// top-left atlas position, the rectangle's dimensions in (2D) atlas
    /// space, a world position, the u and v axes of the rectangle in (3D)
    /// world space, the normal facing out frmo the rectangle in world
    /// space, and meta information common to every voxel in this rectangle.
    pub push_quad: FP,
}

/// A suspended greedy mesh, with enough information to recover color data.
///
/// The reason this exists is that greedy meshing is split into two parts.
/// First, the meshing itself needs to be performed; secondly, we generate a
/// texture atlas.  We do things in this order to avoid having to copy all the
/// old vertices to the correct location.  However, when trying to use the same
/// texture atlas for more than one model, this approach runs into the
/// problem that enough model information needs to be remembered to be able to
/// generate the colors after the function returns, so we box up the actual
/// coloring part as a continuation.  When called with a final tile size and
/// vector, the continuation will consume the color data and write it to the
/// vector.
pub type SuspendedMesh<'a> = dyn for<'r> FnOnce(&'r mut ColLightInfo) + 'a;

/// Shared state for a greedy mesh, potentially passed along to multiple models.
///
/// For an explanation of why we want this, see `SuspendedMesh`.
pub struct GreedyMesh<'a> {
    atlas: guillotiere::SimpleAtlasAllocator,
    col_lights_size: Vec2<u16>,
    max_size: guillotiere::Size,
    suspended: Vec<Box<SuspendedMesh<'a>>>,
}

impl<'a> GreedyMesh<'a> {
    /// Construct a new greedy mesher.
    ///
    /// Takes as input the maximum allowable size of the texture atlas used to
    /// store the light/color data for this mesh.
    ///
    /// NOTE: It is an error to pass any size > u16::MAX.
    ///
    /// Even aside from the above limitation, this will not necessarily always
    /// be the same as the maximum atlas size supported by the hardware.
    /// For instance, since we want to reserve 4 bits for a bone index for
    /// figures in their shadow vertex, the atlas parameter for figures has
    /// to have at least 2 bits of the normal; thus, it can only take up at
    /// most 30 bits total, meaning we are restricted to "only" at most 2^15
    /// × 2^15 atlases even if the hardware supports larger ones.
    pub fn new(max_size: guillotiere::Size) -> Self {
        let min_max_dim = max_size.width.min(max_size.height);
        assert!(
            min_max_dim >= 4,
            "min_max_dim={:?} >= 4 ({:?}",
            min_max_dim,
            max_size
        );
        // TODO: Collect information to see if we can choose a good value here.
        let large_size_threshold = 256.min(min_max_dim / 2 + 1);
        let small_size_threshold = 33.min(large_size_threshold / 2 + 1);
        let size = guillotiere::Size::new(32, 32).min(max_size);
        let atlas =
            guillotiere::SimpleAtlasAllocator::with_options(size, &guillotiere::AllocatorOptions {
                snap_size: 1,
                small_size_threshold,
                large_size_threshold,
            });
        let col_lights_size = Vec2::new(1u16, 1u16);
        Self {
            atlas,
            col_lights_size,
            max_size,
            suspended: Vec::new(),
        }
    }

    /// Perform greedy meshing on a model, separately producing "pure" model
    /// data (the opaque mesh, ttogether with atlas positions connecting
    /// each rectangle with texture information), and raw light and color
    /// data ready to be used as a texture (accessible with `finalize`).
    /// Texture data built up within the same greedy mesh will be inserted
    /// into the same atlas, which can be used to group texture data for
    /// things like figures that are the result of meshing multiple models.
    ///
    /// Returns an estimate of the bounds of the current meshed model.
    ///
    /// For more information on the config parameter, see [GreedyConfig].
    pub fn push<M: PartialEq, D: 'a, FL, FC, FO, FS, FP>(
        &mut self,
        config: GreedyConfig<D, FL, FC, FO, FS, FP>,
    ) -> Aabb<u16>
    where
        FL: for<'r> FnMut(&'r mut D, Vec3<i32>) -> f32 + 'a,
        FC: for<'r> FnMut(&'r mut D, Vec3<i32>) -> Rgb<u8> + 'a,
        FO: for<'r> FnMut(&'r mut D, Vec3<i32>) -> bool + 'a,
        FS: for<'r> FnMut(&'r mut D, Vec3<i32>, Vec3<i32>, Vec2<Vec3<i32>>) -> Option<(bool, M)>,
        FP: FnMut(Vec2<u16>, Vec2<Vec2<u16>>, Vec3<f32>, Vec2<Vec3<f32>>, Vec3<f32>, &M),
    {
        let (bounds, /* opaque, *//*shadow, */ cont) = greedy_mesh(
            &mut self.atlas,
            &mut self.col_lights_size,
            self.max_size,
            config,
        );
        self.suspended.push(cont);
        bounds
    }

    /// Finalize the mesh, producing texture color data for the whole model.
    ///
    /// By delaying finalization until the contents of the whole texture atlas
    /// are known, we can perform just a single allocation to construct a
    /// precisely fitting atlas.  This will also let us (in the future)
    /// suspend meshing partway through in order to meet frame budget, and
    /// potentially use a single staged upload to the GPU.
    ///
    /// Returns the ColLightsInfo corresponding to the consstructed atlas.
    pub fn finalize(self) -> ColLightInfo {
        let cur_size = self.col_lights_size;
        let col_lights = vec![/*Default::default()*/TerrainVertex::make_col_light(254, Rgb::broadcast(254)); usize::from(cur_size.x) * usize::from(cur_size.y)];
        let mut col_lights_info = (col_lights, cur_size);
        self.suspended.into_iter().for_each(|cont| {
            cont(&mut col_lights_info);
        });
        col_lights_info
    }

    pub fn max_size(&self) -> guillotiere::Size { self.max_size }
}

fn greedy_mesh<'a, M: PartialEq, D: 'a, FL, FC, FO, FS, FP>(
    atlas: &mut guillotiere::SimpleAtlasAllocator,
    col_lights_size: &mut Vec2<u16>,
    max_size: guillotiere::Size,
    GreedyConfig {
        mut data,
        draw_delta,
        greedy_size,
        greedy_size_cross,
        get_light,
        get_color,
        get_opacity,
        mut should_draw,
        mut push_quad,
    }: GreedyConfig<D, FL, FC, FO, FS, FP>,
) -> (
    Aabb<u16>,
    // Mesh<O>,
    // Mesh<S>,
    Box<SuspendedMesh<'a>>,
)
where
    FL: for<'r> FnMut(&'r mut D, Vec3<i32>) -> f32 + 'a,
    FC: for<'r> FnMut(&'r mut D, Vec3<i32>) -> Rgb<u8> + 'a,
    FO: for<'r> FnMut(&'r mut D, Vec3<i32>) -> bool + 'a,
    FS: for<'r> FnMut(&'r mut D, Vec3<i32>, Vec3<i32>, Vec2<Vec3<i32>>) -> Option<(bool, M)>,
    FP: FnMut(Vec2<u16>, Vec2<Vec2<u16>>, Vec3<f32>, Vec2<Vec3<f32>>, Vec3<f32>, &M),
{
    // let mut opaque_mesh = Mesh::new();
    // let mut shadow_mesh = Mesh::new();

    // TODO: Collect information to see if we can choose a good value here.
    let mut todo_rects = Vec::with_capacity(1024);

    /* let mut bounds = Aabb {
        min: Vec3::zero(),
        max: Vec3::zero(),
    }; */

    /* let compute_bounds = |pos: Vec3<usize>, dim: Vec2<usize>, uv: Vec2<Vec3<u16>>/*, norm: Vec3<u16>, faces_forward: bool*/| {
        Aabb {
            min: pos,
            max: pos + uv.x.map(usize::from) * dim.x + uv.y.map(usize::from) * dim.y,
        }
    }; */

    // x (u = y, v = z)
    greedy_mesh_cross_section(
        Vec3::new(greedy_size.y, greedy_size.z, greedy_size_cross.x),
        |pos| {
            should_draw(
                &mut data,
                draw_delta + Vec3::new(pos.z, pos.x, pos.y),
                Vec3::unit_x(), /* , pos.z, 0, x_size */
                Vec2::new(Vec3::unit_y(), Vec3::unit_z()),
            )
        },
        |pos, dim, &(faces_forward, ref meta)| {
            let pos = Vec3::new(pos.z, pos.x, pos.y);
            let uv = Vec2::new(Vec3::unit_y(), Vec3::unit_z());
            let norm = Vec3::unit_x();
            // bounds.expand_to_contain(compute_bounds(pos, dim, uv));
            let atlas_pos = if let Some(atlas_pos) = add_to_atlas(
                atlas,
                &mut todo_rects,
                pos,
                uv,
                dim,
                norm,
                faces_forward,
                max_size,
                col_lights_size,
            ) {
                // assert!(atlas_pos.max.x - atlas_pos.min.x == dim.x as i32);
                // assert!(atlas_pos.max.y - atlas_pos.min.y == dim.y as i32);
                atlas_pos
            } else {
                return;
            };
            create_quad_greedy(
                // &mut shadow_mesh,
                // &mut opaque_mesh,
                /* Vec3::new(pos.z, pos.x, pos.y) */
                pos,
                dim,
                uv,
                norm,
                faces_forward,
                // Rgba::from_opaque(flat_get(pos).color),
                // lightm
                // ao,
                meta,
                atlas_pos,
                // |pos| flat_get(pos),
                // |pos, norm, meta| create_shadow(pos, norm, meta),
                // |atlas_pos, pos, norm, meta| create_opaque(atlas_pos, pos, norm, meta),
                |atlas_pos, dim, pos, draw_dim, norm, meta| {
                    push_quad(atlas_pos, dim, pos, draw_dim, norm, meta)
                },
            );
        },
    );

    // y (u = z, v = x)
    greedy_mesh_cross_section(
        Vec3::new(greedy_size.z, greedy_size.x, greedy_size_cross.y),
        |pos| {
            should_draw(
                &mut data,
                draw_delta + Vec3::new(pos.y, pos.z, pos.x),
                Vec3::unit_y(),
                Vec2::new(Vec3::unit_z(), Vec3::unit_x()),
            )
        },
        |pos, dim, &(faces_forward, ref meta)| {
            let pos = Vec3::new(pos.y, pos.z, pos.x);
            let uv = Vec2::new(Vec3::unit_z(), Vec3::unit_x());
            let norm = Vec3::unit_y();
            // bounds.expand_to_contain(compute_bounds(pos, dim, uv));
            let atlas_pos = if let Some(atlas_pos) = add_to_atlas(
                atlas,
                &mut todo_rects,
                pos,
                uv,
                dim,
                norm,
                faces_forward,
                max_size,
                col_lights_size,
            ) {
                atlas_pos
            } else {
                return;
            };
            create_quad_greedy(
                // &mut shadow_mesh,
                // &mut opaque_mesh,
                pos,
                dim,
                uv,
                norm,
                faces_forward,
                // Rgba::from_opaque(flat_get(pos).color),
                meta,
                atlas_pos,
                // |pos, norm, meta| create_shadow(pos, norm, meta),
                // |atlas_pos, pos, norm, meta| create_opaque(atlas_pos, pos, norm, meta),
                |atlas_pos, dim, pos, draw_dim, norm, meta| {
                    push_quad(atlas_pos, dim, pos, draw_dim, norm, meta)
                },
            );
        },
    );

    // z (u = x, v = y)
    greedy_mesh_cross_section(
        Vec3::new(greedy_size.x, greedy_size.y, greedy_size_cross.z),
        |pos| {
            /* if pos.z == 0 {
                let pos = pos.map(|e| e as i32) + draw_delta; // - delta;
                let to = flat_get(pos).is_opaque(); //map(|v| v.is_opaque()).unwrap_or(false);
                if to { Some(false) } else { None }
            } else */
            {
                should_draw(
                    &mut data,
                    draw_delta + Vec3::new(pos.x, pos.y, pos.z),
                    Vec3::unit_z(),
                    Vec2::new(Vec3::unit_x(), Vec3::unit_y()),
                )
            }
        },
        |pos, dim, &(faces_forward, ref meta)| {
            let pos = Vec3::new(pos.x, pos.y, pos.z);
            let uv = Vec2::new(Vec3::unit_x(), Vec3::unit_y());
            let norm = Vec3::unit_z();
            // bounds.expand_to_contain(compute_bounds(pos, dim, uv));
            let atlas_pos = if let Some(atlas_pos) = add_to_atlas(
                atlas,
                &mut todo_rects,
                pos,
                uv,
                dim,
                norm,
                faces_forward,
                max_size,
                col_lights_size,
            ) {
                atlas_pos
            } else {
                return;
            };
            create_quad_greedy(
                // &mut shadow_mesh,
                // &mut opaque_mesh,
                pos,
                dim,
                uv,
                norm,
                faces_forward,
                // Rgba::from_opaque(flat_get(pos).color),
                meta,
                atlas_pos,
                // |pos, norm, meta| create_shadow(pos, norm, meta),
                // |atlas_pos, pos, norm, meta| create_opaque(atlas_pos, pos, norm, meta),
                |atlas_pos, dim, pos, draw_dim, norm, meta| {
                    push_quad(atlas_pos, dim, pos, draw_dim, norm, meta)
                },
            );
        },
    );

    // NOTE: Safe because bound dimensions actually fit in a u16.
    // let bounds = bounds.map(|e| e as u16);
    // NOTE: Safe because draw_delta fits in i16.
    let bounds = Aabb {
        min: Vec3::zero(),
        // NOTE: Safe because greedy_size fit in u16.
        max: greedy_size.map(|e| e as u16),
    };
    (
        bounds,
        /* opaque_mesh, *//*shadow_mesh, */
        Box::new(move |col_lights_info| {
            let mut data = data;
            draw_col_lights(
                col_lights_info,
                &mut data,
                todo_rects,
                draw_delta,
                get_light,
                get_color,
                get_opacity,
                TerrainVertex::make_col_light,
            );
        }),
    )
}

// Greedy meshing a single cross-section.
fn greedy_mesh_cross_section<M: PartialEq>(
    /* mask: &mut [bool], */
    dims: Vec3<usize>,
    // Should we draw a face here (below this vertex)?  If so, provide its meta information.
    mut draw_face: impl FnMut(Vec3<i32>) -> Option<M>,
    // Vertex, width and height, and meta information about the block.
    mut push_quads: impl FnMut(Vec3<usize>, Vec2<usize>, &M),
) {
    // mask represents which faces are either set while the other is unset, or unset
    // while the other is set.
    let mut mask = (0..dims.y * dims.x).map(|_| None).collect::<Vec<_>>();
    (0..dims.z + 1).for_each(|d| {
        // Compute mask
        mask.iter_mut().enumerate().for_each(|(posi, mask)| {
            let i = posi % dims.x;
            let j = posi / dims.x;
            // NOTE: Safe because dims.z actually fits in a u16.
            *mask = draw_face(Vec3::new(i as i32, j as i32, d as i32));
        });

        (0..dims.y).for_each(|j| {
            let mut i = 0;
            while i < dims.x {
                // Compute width (number of set x bits for this row and layer, starting at the
                // current minimum column).
                if let Some(ori) = &mask[j * dims.x + i] {
                    let width = 1 + mask[j * dims.x + i + 1..j * dims.x + dims.x]
                        .iter()
                        .take_while(move |&mask| mask.as_ref() == Some(ori))
                        .count();
                    let max_x = i + width;
                    // Compute height (number of rows having w set x bits for this layer, starting
                    // at the current minimum column and row).
                    let height = 1
                        + (j + 1..dims.y)
                            .take_while(|h| {
                                mask[h * dims.x + i..h * dims.x + max_x]
                                    .iter()
                                    .all(|mask| mask.as_ref() == Some(ori))
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

fn add_to_atlas(
    atlas: &mut guillotiere::SimpleAtlasAllocator,
    todo_rects: &mut Vec<(
        Vec3<i32>,
        Vec2<Vec3<u16>>,
        guillotiere::Rectangle,
        Vec3<i32>,
    )>,
    pos: Vec3<usize>,
    uv: Vec2<Vec3<u16>>,
    dim: Vec2<usize>,
    norm: Vec3<i16>,
    faces_forward: bool,
    max_size: guillotiere::Size,
    cur_size: &mut Vec2<u16>,
) -> Option<guillotiere::Rectangle> {
    // TODO: Check this conversion.
    let atlas_rect;
    loop {
        // NOTE: Conversion to i32 is safe because he x, y, and z dimensions for any
        // chunk index must fit in at least an i16 (lower for x and y, probably
        // lower for z).
        let res = atlas.allocate(guillotiere::Size::new(dim.x as i32 + 1, dim.y as i32 + 1));
        if let Some(atlas_rect_) = res {
            atlas_rect = atlas_rect_;
            break;
        }
        // Allocation failure.
        let current_size = atlas.size();
        if current_size == max_size {
            // NOTE: Currently, if we fail to allocate a terrain chunk in the atlas and we
            // have already reached the maximum texture size, we choose to just skip the
            // geometry and log a warning, rather than panicking or trying to use a fallback
            // technique (e.g. a texture array).
            //
            // FIXME: Either make more robust, or explicitly document that limits on texture
            // size need to be respected for terrain data (the OpenGL minimum requirement is
            // 1024 × 1024, but in practice almost all computers support 4096 × 4096 or
            // higher; see
            // https://feedback.wildfiregames.com/report/opengl/feature/GL_MAX_TEXTURE_SIZE).
            panic!(
                "Could not add texture to atlas using simple allocator (pos={:?}, dim={:?});we \
                 could not fit the whole model into a single texture on this machine
                        (max texture size={:?}, so we are discarding this rectangle.",
                pos, dim, max_size
            );
            // return None;
        }
        // Otherwise, we haven't reached max size yet, so double the size (or reach the
        // max texture size) and try again.
        let new_size = guillotiere::Size::new(
            max_size.width.min(current_size.width.saturating_mul(2)),
            max_size.height.min(current_size.height.saturating_mul(2)),
        );
        atlas.grow(new_size);
        // atlas.grow((current_size * 2).min(max_size));
    }
    // NOTE: Conversion is correct because our initial max size for the atlas was
    // a u16 and we never grew the atlas, meaning all valid coordinates within the
    // atlas also fit into a u16.
    *cur_size = Vec2::new(
        cur_size.x.max(atlas_rect.max.x as u16),
        cur_size.y.max(atlas_rect.max.y as u16),
    );

    /* let (dim, uv, norm) = if faces_forward {
        // NOTE: Conversion to u16 safe by function precondition.
        (dim.map(|e| e as u16), uv, norm)
    } else {
        // NOTE: Conversion to u16 safe by function precondition.
        (Vec2::new(dim.y as u16, dim.x as u16), Vec2::new(uv.y, uv.x), -norm)
    }; */

    // NOTE: pos can be converted safely from usize to i32 because all legal block
    // coordinates in this chunk must fit in an i32 (actually we have the much
    // stronger property that this holds across the whole map).
    let norm = norm.map(i32::from);
    todo_rects.push((
        pos.map(|e| e as i32) + if faces_forward { -norm } else { Vec3::zero() },
        uv,
        atlas_rect,
        if faces_forward { norm } else { -norm },
    ));
    Some(atlas_rect)
}

/// We deferred actually recording the colors within the rectangles in order to
/// generate a texture of minimal size; we now proceed to create and populate
/// it.
///
/// TODO: Consider using the heavier interface (not the simple one) which seems
/// to provide builtin support for what we're doing here.
fn draw_col_lights<D>(
    (col_lights, cur_size): &mut ColLightInfo,
    data: &mut D,
    todo_rects: Vec<(
        Vec3<i32>,
        Vec2<Vec3<u16>>,
        guillotiere::Rectangle,
        Vec3<i32>,
    )>,
    draw_delta: Vec3<i32>,
    mut get_light: impl FnMut(&mut D, Vec3<i32>) -> f32,
    mut get_color: impl FnMut(&mut D, Vec3<i32>) -> Rgb<u8>,
    mut get_opacity: impl FnMut(&mut D, Vec3<i32>) -> bool,
    mut make_col_light: impl FnMut(u8, Rgb<u8>) -> <<ColLightFmt as gfx::format::Formatted>::Surface as gfx::format::SurfaceTyped>::DataType,
) {
    /* for i in 0..todo_rects.len() {
        for j in 0..todo_rects.len() {
            if i == j {
                continue;
            }

            assert!(!todo_rects[i].2.intersects(&todo_rects[j].2));
        }
    } */
    todo_rects
        .into_iter()
        // .rev()
        .for_each(|(pos, uv, rect, delta)| {
            // NOTE: Conversions are safe because width, height, and offset must be
            // non-negative, and because every allocated coordinate in the atlas must be in
            // bounds for the original size, max_texture_size, which fit into a u16.
            let width = (rect.max.x - rect.min.x) as u16;//rect.width() as u16;
            let height = (rect.max.y - rect.min.y) as u16;//rect.height() as u16;
            /* if width > 32 || height > 32 {
                println!("Rect: {:?}", rect);
            } */
            let left = rect.min.x as u16;
            let top = rect.min.y as u16;
            let uv = uv.map(|e| e.map(i32::from));
            let pos = pos + draw_delta;//Vec3::new(0, 0, z_start - 1);// + mesh_delta;// + draw_delta;
            (0..height).for_each(|v| {
                let start = usize::from(cur_size.x) * usize::from(top + v) + usize::from(left);
                (0..width).zip(&mut col_lights[start..start + usize::from(width)]).for_each(|(u, col_light)| {
                    let pos = pos + uv.x * i32::from(u) + uv.y * i32::from(v);
                    // TODO: Consider optimizing to take advantage of the fact that this whole
                    // face should be facing nothing but air (this is not currently true, but
                    // could be if we used the right AO strategy).
                    // Each indirect light needs to come in through the direct light.
                    // Thus, we assign each light a score based on opacity (currently just 0 or
                    // 1, but it could support transluscent lights in the future).
                    // Thus, indirect_u_opacity and indirect_v_opacity are multiplied by
                    // direct_opacity, and indirect_uv_opacity is multiplied by
                    // the maximum of both of u and v's indirect opacities (since there are
                    // two choices for how to get to the direct surface).
                    let pos = pos +
                        if u + 1 == width { -uv.x } else { Vec3::zero() } +
                        if v + 1 == height { -uv.y } else { Vec3::zero() };
                    let uv = Vec2::new(
                        if u + 1 == width { -uv.x } else { uv.x },
                        if v + 1 == height { -uv.y } else  { uv.y },
                    );

                    let light_pos = pos + /*range.min + */delta;
                    // let block = flat_get(pos);

                    // Currently, we assume that direct_opacity is 1 (if it's 0, you can't see
                    // the face anyway, since it's blocked by the block directly in front of it).
                    // TODO: If we add non-0/1 opacities, fix this.
                    // top-left block
                    // let direct_opacity = !flat_get(pos + delta).is_opaque();
                    // bottom-left block
                    let direct_u_opacity = get_opacity(data, light_pos - uv.x);
                    // top-right block
                    let direct_v_opacity = get_opacity(data, light_pos - uv.y);
                    // top-left block
                    // NOTE: Currently, since we only have 0 / 1 opacities, we don't worry
                    // about whether the uv block itself is opaque, because if it is its light
                    // value will be 0 anyway.  But if we add translucent objects, we'll need
                    // to care about uv's opacity as well.
                    // let direct_uv_opacity = !flat_get(pos + delta - uv.x - uv.y).is_opaque();
                    // let indirect_opacity = direct_uv_opacity && (direct_u_opacity || direct_v_opacity) && direct_opacity;

                    // NOTE: Since we only support 0/1 opacities currently, we asssume
                    // direct_opacity is  1, and the light value will be zero anyway for objects
                    // with opacity 0, we only "multiply" by indirect_uv_opacity for now (since
                    // it's the only one that could be 0 even if its light value is not).
                    // However, "spiritually" these light values are all being multiplied by
                    // their opacities.
                    let darkness = (
                        // Light from the bottom-right-front block to this vertex always
                        // appears on this face, since it's the block this face is facing (so
                        // it can't be blocked by anything).
                        if /*direct_u_opacity || direct_v_opacity*/true/* || !flat_get(pos - uv.x - uv.y).is_opaque()*//* || !block.is_opaque()*/ { get_light(data, light_pos) } else { 0.0 } +
                        if /*direct_opacity || direct_uv_opacity*/true/* || !flat_get(pos - uv.y).is_opaque()*/ { get_light(data, light_pos - uv.x) } else { 0.0 } +
                        if /*direct_opacity || direct_uv_opacity*/true/* || !flat_get(pos - uv.x).is_opaque()*/ { get_light(data, light_pos - uv.y) } else { 0.0 } +
                        if direct_u_opacity || direct_v_opacity/* || !block.is_opaque()*/ { get_light(data, light_pos - uv.x - uv.y) } else { 0.0 }
                    ) / 4.0;
                    let col = get_color(data, pos);//.map(Rgba::from_opaque).unwrap_or(Rgba::zero());
                    let light = (darkness * 255.0) as u8;
                    *col_light = make_col_light(light, col);
                });
            });
        });
}

/// Precondition: when this function is called, atlas_pos should reflect an
/// actual valid position in a texture atlas (meaning it should fit into a u16).
fn create_quad_greedy</* S: render::Pipeline, *//*O: render::Pipeline, */ M>(
    // shadow_mesh: &mut Mesh<S>,
    // opaque_mesh: &mut Mesh<O>,
    origin: Vec3<usize>,
    dim: Vec2<usize>,
    uv: Vec2<Vec3<u16>>,
    norm: Vec3<i16>,
    faces_forward: bool,
    meta: &M,
    atlas_pos: guillotiere::Rectangle,
    // origin, norm, meta
    // create_shadow: impl Fn(Vec3<f32>, Vec3<f32>, &M) -> S::Vertex,
    // create_opaque: impl Fn(Vec2<u16>, Vec3<f32>, Vec3<f32>, &M) -> O::Vertex,
    mut push_quad: impl FnMut(Vec2<u16>, Vec2<Vec2<u16>>, Vec3<f32>, Vec2<Vec3<f32>>, Vec3<f32>, &M),
) /* -> Quad<ShadowPipeline> */
{
    let origin = origin.map(|e| e as f32);
    /* // NOTE: Conversion to u16 safe by function precondition.
    let dim = uv.map2(dim.map(|e| e as u16), |e, f| e * f); */
    // NOTE: Conversion to f32 safe by function precondition (u16 can losslessly
    // cast to f32, and dim fits in a u16).
    let draw_dim = uv.map2(dim.map(|e| e as f32), |e, f| e.map(f32::from) * f);
    let dim = Vec2::new(Vec2::new(dim.x as u16, 0), Vec2::new(0, dim.y as u16));
    let (draw_dim, dim, /* uv, */ norm) = if faces_forward {
        /* // NOTE: Conversion to u16 safe by function precondition.
        (dim.map(|e| e as u16), uv, norm) */
        (draw_dim, dim, norm)
    } else {
        /* // NOTE: Conversion to u16 safe by function precondition.
        (Vec2::new(dim.y as u16, dim.x as u16), Vec2::new(uv.y, uv.x), -norm) */
        (
            Vec2::new(draw_dim.y, draw_dim.x),
            Vec2::new(dim.y, dim.x),
            -norm,
        )
    };
    let norm = norm.map(f32::from);
    // let draw_dim = draw_dim.map(|e|  e.map(f32::from));
    // NOTE: Conversion to u16 safe by function precondition.
    let atlas_pos = Vec2::new(atlas_pos.min.x as u16, atlas_pos.min.y as u16);
    /* shadow_mesh.push_quad(Quad::new(
        create_shadow(origin, norm, &meta/*, atlas_pos*/),
        create_shadow(origin + draw_dim.x, norm, &meta/*, atlas_pos + dim.x*/),
        create_shadow(origin + draw_dim.x + draw_dim.y, norm, &meta/*, atlas_pos + dim.x + dim.y*/),
        create_shadow(origin + draw_dim.y, norm, &meta/*, atlas_pos + dim.y*/),
    )); */
    /* opaque_mesh.push_quad(Quad::new(
        create_opaque(atlas_pos, origin, norm, &meta),
        create_opaque(atlas_pos + dim.x, origin + draw_dim.x, norm, &meta),
        create_opaque(atlas_pos + dim.x + dim.y, origin + draw_dim.x + draw_dim.y, norm, &meta),
        create_opaque(atlas_pos + dim.y, origin + draw_dim.y, norm, &meta),
    )); */
    push_quad(atlas_pos, dim, origin, draw_dim, norm, meta);
}

pub fn create_quad<O: render::Pipeline, M>(
    atlas_pos: Vec2<u16>,
    dim: Vec2<Vec2<u16>>,
    origin: Vec3<f32>,
    draw_dim: Vec2<Vec3<f32>>,
    norm: Vec3<f32>,
    meta: &M,
    create_vertex: impl Fn(Vec2<u16>, Vec3<f32>, Vec3<f32>, &M) -> O::Vertex,
) -> Quad<O> {
    Quad::new(
        create_vertex(atlas_pos, origin, norm, meta),
        create_vertex(atlas_pos + dim.x, origin + draw_dim.x, norm, meta),
        create_vertex(
            atlas_pos + dim.x + dim.y,
            origin + draw_dim.x + draw_dim.y,
            norm,
            meta,
        ),
        create_vertex(atlas_pos + dim.y, origin + draw_dim.y, norm, meta),
        /* create_vertex(atlas_pos, origin, norm, meta),
        create_vertex(atlas_pos + dim.y, origin + draw_dim.y, norm, meta),
        create_vertex(atlas_pos + dim.x + dim.y, origin + draw_dim.x + draw_dim.y, norm, meta),
        create_vertex(atlas_pos + dim.x, origin + draw_dim.x, norm, meta), */
    )
}
