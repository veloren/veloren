use super::{
    img_ids::{Imgs, ImgsRot},
    MapMarkers, QUALITY_COMMON, QUALITY_DEBUG, QUALITY_EPIC, QUALITY_HIGH, QUALITY_LOW,
    QUALITY_MODERATE, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    hud::{Graphic, Ui},
    session::settings_change::{Interface as InterfaceChange, Interface::*},
    ui::{fonts::Fonts, img_ids, KeyedJobs},
    GlobalState,
};
use client::{self, Client};
use common::{
    comp,
    comp::group::Role,
    grid::Grid,
    slowjob::SlowJobPool,
    terrain::{
        Block, BlockKind, CoordinateConversions, TerrainChunk, TerrainChunkSize, TerrainGrid,
    },
    vol::{ReadVol, RectVolSize},
};
use common_net::msg::world_msg::SiteKind;
use conrod_core::{
    color, position,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
use hashbrown::HashMap;
use image::{DynamicImage, RgbaImage};
use specs::{saveload::MarkerAllocator, WorldExt};
use std::sync::Arc;
use vek::*;

struct MinimapColumn {
    /// Coordinate of lowest z-slice
    zlo: i32,
    /// Z-slices of colors and filled-ness
    layers: Vec<Grid<(Rgba<u8>, bool)>>,
    /// Color and filledness above the highest layer
    above: (Rgba<u8>, bool),
    /// Color and filledness below the lowest layer
    below: (Rgba<u8>, bool),
}

pub struct VoxelMinimap {
    chunk_minimaps: HashMap<Vec2<i32>, MinimapColumn>,
    composited: RgbaImage,
    image_id: img_ids::Rotations,
    last_pos: Vec3<i32>,
    last_ceiling: i32,
    keyed_jobs: KeyedJobs<Vec2<i32>, MinimapColumn>,
}

const VOXEL_MINIMAP_SIDELENGTH: u32 = 256;

impl VoxelMinimap {
    pub fn new(ui: &mut Ui) -> Self {
        let composited = RgbaImage::from_pixel(
            VOXEL_MINIMAP_SIDELENGTH,
            VOXEL_MINIMAP_SIDELENGTH,
            image::Rgba([0, 0, 0, 64]),
        );
        Self {
            chunk_minimaps: HashMap::new(),
            image_id: ui.add_graphic_with_rotations(Graphic::Image(
                Arc::new(DynamicImage::ImageRgba8(composited.clone())),
                Some(Rgba::from([0.0, 0.0, 0.0, 0.0])),
            )),
            composited,
            last_pos: Vec3::zero(),
            last_ceiling: 0,
            keyed_jobs: KeyedJobs::new("IMAGE_PROCESSING"),
        }
    }

    fn block_color(block: &Block) -> Option<Rgba<u8>> {
        block
            .get_color()
            .map(|rgb| Rgba::new(rgb.r, rgb.g, rgb.b, 255))
            .or_else(|| {
                matches!(block.kind(), BlockKind::Water).then(|| Rgba::new(119, 149, 197, 255))
            })
    }

    /// Each layer is a slice of the terrain near that z-level
    fn composite_layer_slice(chunk: &TerrainChunk, layers: &mut Vec<Grid<(Rgba<u8>, bool)>>) {
        for z in chunk.get_min_z()..=chunk.get_max_z() {
            let grid = Grid::populate_from(Vec2::new(32, 32), |v| {
                let mut rgba = Rgba::<f32>::zero();
                let (weights, zoff) = (&[1, 2, 4, 1, 1, 1][..], -2);
                for (dz, weight) in weights.iter().enumerate() {
                    let color = chunk
                        .get(Vec3::new(v.x, v.y, dz as i32 + z + zoff))
                        .ok()
                        .and_then(Self::block_color)
                        .unwrap_or_else(Rgba::zero);
                    rgba += color.as_() * *weight as f32;
                }
                let rgba: Rgba<u8> = (rgba / weights.iter().map(|x| *x as f32).sum::<f32>()).as_();
                (rgba, true)
            });
            layers.push(grid);
        }
    }

    /// Each layer is the overhead as if its z-level were the ceiling
    fn composite_layer_overhead(chunk: &TerrainChunk, layers: &mut Vec<Grid<(Rgba<u8>, bool)>>) {
        for z in chunk.get_min_z()..=chunk.get_max_z() {
            let grid = Grid::populate_from(Vec2::new(32, 32), |v| {
                let mut rgba = None;

                let mut seen_solids: u32 = 0;
                let mut seen_air: u32 = 0;
                for dz in chunk.get_min_z()..=z {
                    if let Some(color) = chunk
                        .get(Vec3::new(v.x, v.y, z - dz + chunk.get_min_z()))
                        .ok()
                        .and_then(Self::block_color)
                    {
                        if seen_air > 0 {
                            rgba = Some(color);
                            break;
                        }
                        seen_solids += 1;
                    } else {
                        seen_air += 1;
                    }
                    // Don't penetrate too far into ground, only penetrate through shallow
                    // ceilings
                    if seen_solids > 12 {
                        break;
                    }
                }
                let block = chunk.get(Vec3::new(v.x, v.y, z)).ok();
                // Treat Leaves and Wood as translucent for the purposes of ceiling checks,
                // since otherwise trees would cause ceiling removal to trigger
                // when running under a branch.
                let is_filled = block.map_or(true, |b| {
                    b.is_filled() && !matches!(b.kind(), BlockKind::Leaves | BlockKind::Wood)
                });
                let rgba = rgba.unwrap_or_else(|| Rgba::new(0, 0, 0, 255));
                (rgba, is_filled)
            });
            layers.push(grid);
        }
    }

    fn add_chunks_near(
        &mut self,
        pool: &SlowJobPool,
        terrain: &TerrainGrid,
        cpos: Vec2<i32>,
    ) -> bool {
        let mut new_chunks = false;

        for (key, chunk) in terrain.iter() {
            let delta: Vec2<u32> = (key - cpos).map(i32::abs).as_();
            if delta.x < VOXEL_MINIMAP_SIDELENGTH / TerrainChunkSize::RECT_SIZE.x
                && delta.y < VOXEL_MINIMAP_SIDELENGTH / TerrainChunkSize::RECT_SIZE.y
                && !self.chunk_minimaps.contains_key(&key)
            {
                if let Some((_, column)) = self.keyed_jobs.spawn(Some(pool), key, || {
                    let arc_chunk = Arc::clone(chunk);
                    move |_| {
                        let mut layers = Vec::new();
                        const MODE_OVERHEAD: bool = true;
                        if MODE_OVERHEAD {
                            Self::composite_layer_overhead(&arc_chunk, &mut layers);
                        } else {
                            Self::composite_layer_slice(&arc_chunk, &mut layers);
                        }
                        let above = arc_chunk
                            .get(Vec3::new(0, 0, arc_chunk.get_max_z() + 1))
                            .ok()
                            .copied()
                            .unwrap_or_else(Block::empty);
                        let below = arc_chunk
                            .get(Vec3::new(0, 0, arc_chunk.get_min_z() - 1))
                            .ok()
                            .copied()
                            .unwrap_or_else(Block::empty);
                        MinimapColumn {
                            zlo: arc_chunk.get_min_z(),
                            layers,
                            above: (
                                Self::block_color(&above).unwrap_or_else(Rgba::zero),
                                above.is_filled(),
                            ),
                            below: (
                                Self::block_color(&below).unwrap_or_else(Rgba::zero),
                                below.is_filled(),
                            ),
                        }
                    }
                }) {
                    self.chunk_minimaps.insert(key, column);
                    new_chunks = true;
                }
            }
        }
        new_chunks
    }

    fn remove_chunks_far(&mut self, terrain: &TerrainGrid, cpos: Vec2<i32>) {
        self.chunk_minimaps.retain(|key, _| {
            let delta: Vec2<u32> = (key - cpos).map(i32::abs).as_();
            delta.x < 1 + VOXEL_MINIMAP_SIDELENGTH / TerrainChunkSize::RECT_SIZE.x
                && delta.y < 1 + VOXEL_MINIMAP_SIDELENGTH / TerrainChunkSize::RECT_SIZE.y
                && terrain.get_key(*key).is_some()
        });
    }

    pub fn maintain(&mut self, client: &Client, ui: &mut Ui) {
        let player = client.entity();
        let pos = if let Some(pos) = client.state().ecs().read_storage::<comp::Pos>().get(player) {
            pos.0
        } else {
            return;
        };
        let vpos = pos.xy() - VOXEL_MINIMAP_SIDELENGTH as f32 / 2.0;
        let cpos: Vec2<i32> = vpos
            .map2(TerrainChunkSize::RECT_SIZE, |i, j| (i as u32).div_euclid(j))
            .as_();

        let pool = client.state().ecs().read_resource::<SlowJobPool>();
        let terrain = client.state().terrain();
        let new_chunks = self.add_chunks_near(&pool, &terrain, cpos);
        self.remove_chunks_far(&terrain, cpos);

        // ceiling_offset is the distance from the player to a block heuristically
        // detected as the ceiling height (a non-tree solid block above them, or
        // the sky if no such block exists). This is used for determining which
        // z-slice of the minimap to show, such that house roofs and caves and
        // dungeons are all handled uniformly.
        let ceiling_offset = {
            let voff = Vec2::new(
                VOXEL_MINIMAP_SIDELENGTH as f32,
                VOXEL_MINIMAP_SIDELENGTH as f32,
            ) / 2.0;
            let coff: Vec2<i32> = voff
                .map2(TerrainChunkSize::RECT_SIZE, |i, j| (i as u32).div_euclid(j))
                .as_();
            let cmod: Vec2<i32> = vpos
                .map2(TerrainChunkSize::RECT_SIZE, |i, j| (i as u32).rem_euclid(j))
                .as_();
            let column = self.chunk_minimaps.get(&(cpos + coff));
            // TODO: evaluate clippy, toolchain upgrade 2021-12-19
            #[allow(clippy::unnecessary_lazy_evaluations)]
            column
                .map(
                    |MinimapColumn {
                         zlo, layers, above, ..
                     }| {
                        (0..layers.len() as i32)
                            .find(|dz| {
                                layers
                                    .get((pos.z as i32 - zlo + dz) as usize)
                                    .and_then(|grid| grid.get(cmod))
                                    .map_or(false, |(_, b)| *b)
                            })
                            .unwrap_or_else(||
                                // if the `find` returned None, there's no solid blocks above the
                                // player within the chunk
                                if above.1 {
                                    // if the `above` block is solid, the chunk has an infinite
                                    // solid ceiling, and so we render from 1 block above the
                                    // player (which is where the player's head is if they're 2
                                    // blocks tall)
                                    1
                                } else {
                                    // if the ceiling is a non-solid sky, use the largest value
                                    // (subsequent arithmetic on ceiling_offset must be saturating)
                                    i32::MAX
                                }
                            )
                    },
                )
                .unwrap_or(0)
        };
        if self.last_pos.xy() != cpos
            || self.last_pos.z != pos.z as i32
            || self.last_ceiling != ceiling_offset
            || new_chunks
        {
            self.last_pos = cpos.with_z(pos.z as i32);
            self.last_ceiling = ceiling_offset;
            for y in 0..VOXEL_MINIMAP_SIDELENGTH {
                for x in 0..VOXEL_MINIMAP_SIDELENGTH {
                    let voff = Vec2::new(x as f32, y as f32);
                    let coff: Vec2<i32> = voff
                        .map2(TerrainChunkSize::RECT_SIZE, |i, j| (i as u32).div_euclid(j))
                        .as_();
                    let cmod: Vec2<i32> = voff
                        .map2(TerrainChunkSize::RECT_SIZE, |i, j| (i as u32).rem_euclid(j))
                        .as_();
                    let column = self.chunk_minimaps.get(&(cpos + coff));
                    let color: Rgba<u8> = column
                        .and_then(|column| {
                            let MinimapColumn {
                                zlo,
                                layers,
                                above,
                                below,
                            } = column;
                            if (pos.z as i32).saturating_add(ceiling_offset) < *zlo {
                                // If the ceiling is below the bottom of a chunk, color it black,
                                // so that the middles of caves/dungeons don't show the forests
                                // around them.
                                Some(Rgba::new(0, 0, 0, 255))
                            } else {
                                // Otherwise, take the pixel from the precomputed z-level view at
                                // the ceiling's height (using the top slice of the chunk if the
                                // ceiling is above the chunk, (e.g. so that forests with
                                // differently-tall trees are handled properly)
                                // TODO: evaluate clippy, toolchain upgrade 2021-12-19
                                #[allow(clippy::unnecessary_lazy_evaluations)]
                                layers
                                    .get(
                                        (((pos.z as i32 - zlo).saturating_add(ceiling_offset))
                                            as usize)
                                            .min(layers.len().saturating_sub(1)),
                                    )
                                    .and_then(|grid| grid.get(cmod).map(|c| c.0.as_()))
                                    .or_else(|| {
                                        Some(if pos.z as i32 > *zlo {
                                            above.0
                                        } else {
                                            below.0
                                        })
                                    })
                            }
                        })
                        .unwrap_or_else(Rgba::zero);
                    self.composited.put_pixel(
                        x,
                        VOXEL_MINIMAP_SIDELENGTH - y - 1,
                        image::Rgba([color.r, color.g, color.b, color.a]),
                    );
                }
            }

            ui.replace_graphic(
                self.image_id.none,
                Graphic::Image(
                    Arc::new(DynamicImage::ImageRgba8(self.composited.clone())),
                    Some(Rgba::from([0.0, 0.0, 0.0, 0.0])),
                ),
            );
        }
    }
}

widget_ids! {
    struct Ids {
        mmap_frame,
        mmap_frame_2,
        mmap_frame_bg,
        mmap_location,
        mmap_button,
        mmap_plus,
        mmap_minus,
        mmap_north_button,
        map_layers[],
        indicator,
        mmap_north,
        mmap_east,
        mmap_south,
        mmap_west,
        mmap_site_icons_bgs[],
        mmap_site_icons[],
        member_indicators[],
        location_marker,
        location_marker_group[],
        voxel_minimap,
    }
}

#[derive(WidgetCommon)]
pub struct MiniMap<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    rot_imgs: &'a ImgsRot,
    world_map: &'a (Vec<img_ids::Rotations>, Vec2<u32>),
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    ori: Vec3<f32>,
    global_state: &'a GlobalState,
    location_markers: &'a MapMarkers,
    voxel_minimap: &'a VoxelMinimap,
}

impl<'a> MiniMap<'a> {
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        rot_imgs: &'a ImgsRot,
        world_map: &'a (Vec<img_ids::Rotations>, Vec2<u32>),
        fonts: &'a Fonts,
        ori: Vec3<f32>,
        global_state: &'a GlobalState,
        location_markers: &'a MapMarkers,
        voxel_minimap: &'a VoxelMinimap,
    ) -> Self {
        Self {
            client,
            imgs,
            rot_imgs,
            world_map,
            fonts,
            common: widget::CommonBuilder::default(),
            ori,
            global_state,
            location_markers,
            voxel_minimap,
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    SettingsChange(InterfaceChange),
}

impl<'a> Widget for MiniMap<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Minimap::update");
        let mut events = Vec::new();

        let widget::UpdateArgs { state, ui, .. } = args;
        let mut zoom = self.global_state.settings.interface.minimap_zoom;
        const SCALE: f64 = 1.5; // TODO Make this a setting
        let show_minimap = self.global_state.settings.interface.minimap_show;
        let is_facing_north = self.global_state.settings.interface.minimap_face_north;
        let show_topo_map = self.global_state.settings.interface.map_show_topo_map;
        let show_voxel_map = self.global_state.settings.interface.map_show_voxel_map;
        let orientation = if is_facing_north {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            self.ori
        };

        if show_minimap {
            Image::new(self.imgs.mmap_frame)
                .w_h(174.0 * SCALE, 190.0 * SCALE)
                .top_right_with_margins_on(ui.window, 5.0, 5.0)
                .color(Some(UI_MAIN))
                .set(state.ids.mmap_frame, ui);
            Image::new(self.imgs.mmap_frame_2)
                .w_h(174.0 * SCALE, 190.0 * SCALE)
                .middle_of(state.ids.mmap_frame)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.mmap_frame_2, ui);
            Rectangle::fill_with([170.0 * SCALE, 170.0 * SCALE], color::TRANSPARENT)
                .mid_top_with_margin_on(state.ids.mmap_frame_2, 18.0 * SCALE)
                .set(state.ids.mmap_frame_bg, ui);

            // Map size in chunk coords
            let worldsize = self.world_map.1;
            // Map Layers
            // It is assumed that there is at least one layer
            if state.ids.map_layers.len() < self.world_map.0.len() {
                state.update(|state| {
                    state
                        .ids
                        .map_layers
                        .resize(self.world_map.0.len(), &mut ui.widget_id_generator())
                });
            }

            // Zoom Buttons

            // Pressing + multiplies, and - divides, zoom by ZOOM_FACTOR.
            const ZOOM_FACTOR: f64 = 2.0;

            // TODO: Either prevent zooming all the way in, *or* see if we can interpolate
            // somehow if you zoom in too far.  Or both.
            let min_zoom = 1.0;
            let max_zoom = worldsize
                .reduce_partial_max() as f64/*.min(f64::MAX)*/;

            // NOTE: Not sure if a button can be clicked while disabled, but we still double
            // check for both kinds of zoom to make sure that not only was the
            // button clicked, it is also okay to perform the zoom action.
            // Note that since `Button::image` has side effects, we must perform
            // the `can_zoom_in` and `can_zoom_out` checks after the `&&` to avoid
            // undesired early termination.
            let can_zoom_in = zoom < max_zoom;
            let can_zoom_out = zoom > min_zoom;

            if Button::image(self.imgs.mmap_minus)
                .w_h(16.0 * SCALE, 18.0 * SCALE)
                .hover_image(self.imgs.mmap_minus_hover)
                .press_image(self.imgs.mmap_minus_press)
                .top_left_with_margins_on(state.ids.mmap_frame, 0.0, 0.0)
                .image_color(UI_HIGHLIGHT_0)
                .enabled(can_zoom_out)
                .set(state.ids.mmap_minus, ui)
                .was_clicked()
                && can_zoom_out
            {
                // Set the image dimensions here, rather than recomputing each time.
                zoom = min_zoom.max(zoom / ZOOM_FACTOR);
                // set_image_dims(zoom);
                events.push(Event::SettingsChange(MinimapZoom(zoom)));
            }
            if Button::image(self.imgs.mmap_plus)
                .w_h(18.0 * SCALE, 18.0 * SCALE)
                .hover_image(self.imgs.mmap_plus_hover)
                .press_image(self.imgs.mmap_plus_press)
                .right_from(state.ids.mmap_minus, 0.0)
                .image_color(UI_HIGHLIGHT_0)
                .enabled(can_zoom_in)
                .set(state.ids.mmap_plus, ui)
                .was_clicked()
                && can_zoom_in
            {
                zoom = min_zoom.max(zoom * ZOOM_FACTOR);
                // set_image_dims(zoom);
                events.push(Event::SettingsChange(MinimapZoom(zoom)));
            }

            // Always northfacing button
            if Button::image(if is_facing_north {
                self.imgs.mmap_north_press
            } else {
                self.imgs.mmap_north
            })
            .w_h(18.0 * SCALE, 18.0 * SCALE)
            .hover_image(if is_facing_north {
                self.imgs.mmap_north_press_hover
            } else {
                self.imgs.mmap_north_hover
            })
            .press_image(if is_facing_north {
                self.imgs.mmap_north_press_hover
            } else {
                self.imgs.mmap_north_press
            })
            .left_from(state.ids.mmap_button, 0.0)
            .image_color(UI_HIGHLIGHT_0)
            .set(state.ids.mmap_north_button, ui)
            .was_clicked()
            {
                events.push(Event::SettingsChange(MinimapFaceNorth(!is_facing_north)));
            }

            // Coordinates
            let player_pos = self
                .client
                .state()
                .ecs()
                .read_storage::<comp::Pos>()
                .get(self.client.entity())
                .map_or(Vec3::zero(), |pos| pos.0);

            // Get map image source rectangle dimensions.
            let w_src = max_zoom / zoom;
            let h_src = max_zoom / zoom;

            // Set map image to be centered around player coordinates.
            let rect_src = position::Rect::from_xy_dim(
                [
                    player_pos.x as f64 / TerrainChunkSize::RECT_SIZE.x as f64,
                    worldsize.y as f64
                        - (player_pos.y as f64 / TerrainChunkSize::RECT_SIZE.y as f64),
                ],
                [w_src, h_src],
            );

            let map_size = Vec2::new(170.0 * SCALE, 170.0 * SCALE);

            // Map Image
            // Map Layer Images
            for (index, layer) in self.world_map.0.iter().enumerate() {
                let world_map_rotation = if is_facing_north {
                    layer.none
                } else {
                    layer.source_north
                };
                if index == 0 {
                    Image::new(world_map_rotation)
                        .middle_of(state.ids.mmap_frame_bg)
                        .w_h(map_size.x, map_size.y)
                        .parent(state.ids.mmap_frame_bg)
                        .source_rectangle(rect_src)
                        .set(state.ids.map_layers[index], ui);
                } else if show_topo_map {
                    Image::new(world_map_rotation)
                        .middle_of(state.ids.mmap_frame_bg)
                        .w_h(map_size.x, map_size.y)
                        .parent(state.ids.mmap_frame_bg)
                        .source_rectangle(rect_src)
                        .graphics_for(state.ids.map_layers[0])
                        .set(state.ids.map_layers[index], ui);
                }
            }
            if show_voxel_map {
                let voxelmap_rotation = if is_facing_north {
                    self.voxel_minimap.image_id.none
                } else {
                    self.voxel_minimap.image_id.source_north
                };
                let cmod: Vec2<f64> = player_pos
                    .xy()
                    .map2(TerrainChunkSize::RECT_SIZE, |i, j| (i as u32).rem_euclid(j))
                    .as_();
                let rect_src = position::Rect::from_xy_dim(
                    [
                        cmod.x + VOXEL_MINIMAP_SIDELENGTH as f64 / 2.0,
                        -cmod.y + VOXEL_MINIMAP_SIDELENGTH as f64 / 2.0,
                    ],
                    [
                        TerrainChunkSize::RECT_SIZE.x as f64 * max_zoom / zoom,
                        TerrainChunkSize::RECT_SIZE.y as f64 * max_zoom / zoom,
                    ],
                );
                Image::new(voxelmap_rotation)
                    .middle_of(state.ids.mmap_frame_bg)
                    .w_h(map_size.x, map_size.y)
                    .parent(state.ids.mmap_frame_bg)
                    .source_rectangle(rect_src)
                    .graphics_for(state.ids.map_layers[0])
                    .set(state.ids.voxel_minimap, ui);
            }

            // Map icons
            if state.ids.mmap_site_icons.len() < self.client.sites().len() {
                state.update(|state| {
                    state
                        .ids
                        .mmap_site_icons
                        .resize(self.client.sites().len(), &mut ui.widget_id_generator())
                });
            }
            if state.ids.mmap_site_icons_bgs.len() < self.client.sites().len() {
                state.update(|state| {
                    state
                        .ids
                        .mmap_site_icons_bgs
                        .resize(self.client.sites().len(), &mut ui.widget_id_generator())
                });
            }

            let wpos_to_rpos = |wpos: Vec2<f32>, limit: bool| {
                // Site pos in world coordinates relative to the player
                let rwpos = wpos - player_pos;
                // Convert to chunk coordinates
                let rcpos = rwpos.wpos_to_cpos();
                // Convert to fractional coordinates relative to the worldsize
                let rfpos = rcpos / max_zoom as f32;
                // Convert to unrotated pixel coordinates from the player location on the map
                // (the center)
                // Accounting for zooming
                let rpixpos = rfpos.map2(map_size, |e, sz| e * sz as f32 * zoom as f32);
                let rpos = Vec2::unit_x().rotated_z(orientation.x) * rpixpos.x
                    + Vec2::unit_y().rotated_z(orientation.x) * rpixpos.y;

                if rpos
                    .map2(map_size, |e, sz| e.abs() > sz as f32 / 2.0)
                    .reduce_or()
                {
                    limit.then(|| {
                        let clamped = rpos / rpos.map(|e| e.abs()).reduce_partial_max();
                        clamped * map_size.map(|e| e as f32) / 2.0
                    })
                } else {
                    Some(rpos)
                }
            };

            for (i, site_rich) in self.client.sites().values().enumerate() {
                let site = &site_rich.site;

                let rpos = match wpos_to_rpos(site.wpos.map(|e| e as f32), false) {
                    Some(rpos) => rpos,
                    None => continue,
                };
                let difficulty = match &site.kind {
                    SiteKind::Town => None,
                    SiteKind::ChapelSite => Some(0),
                    SiteKind::Dungeon { difficulty } => Some(*difficulty),
                    SiteKind::Castle => None,
                    SiteKind::Cave => None,
                    SiteKind::Tree => None,
                    SiteKind::Gnarling => Some(0),
                    SiteKind::Bridge => None,
                };

                Image::new(match &site.kind {
                    SiteKind::Town => self.imgs.mmap_site_town_bg,
                    SiteKind::ChapelSite => self.imgs.mmap_site_sea_chapel_bg,
                    SiteKind::Dungeon { .. } => self.imgs.mmap_site_dungeon_bg,
                    SiteKind::Castle => self.imgs.mmap_site_castle_bg,
                    SiteKind::Cave => self.imgs.mmap_site_cave_bg,
                    SiteKind::Tree => self.imgs.mmap_site_tree,
                    SiteKind::Gnarling => self.imgs.mmap_site_gnarling_bg,
                    SiteKind::Bridge => self.imgs.mmap_site_bridge_bg,
                })
                .x_y_position_relative_to(
                    state.ids.map_layers[0],
                    position::Relative::Scalar(rpos.x as f64),
                    position::Relative::Scalar(rpos.y as f64),
                )
                .w_h(20.0, 20.0)
                .color(Some(match difficulty {
                    Some(0) => QUALITY_LOW,
                    Some(1) => QUALITY_COMMON,
                    Some(2) => QUALITY_MODERATE,
                    Some(3) => QUALITY_HIGH,
                    Some(4) => QUALITY_EPIC,
                    Some(5) => QUALITY_DEBUG,
                    _ => Color::Rgba(1.0, 1.0, 1.0, 0.0),
                }))
                .set(state.ids.mmap_site_icons_bgs[i], ui);
                Image::new(match &site.kind {
                    SiteKind::Town => self.imgs.mmap_site_town,
                    SiteKind::ChapelSite => self.imgs.mmap_site_sea_chapel,
                    SiteKind::Dungeon { .. } => self.imgs.mmap_site_dungeon,
                    SiteKind::Castle => self.imgs.mmap_site_castle,
                    SiteKind::Cave => self.imgs.mmap_site_cave,
                    SiteKind::Tree => self.imgs.mmap_site_tree,
                    SiteKind::Gnarling => self.imgs.mmap_site_gnarling,
                    SiteKind::Bridge => self.imgs.mmap_site_bridge,
                })
                .middle_of(state.ids.mmap_site_icons_bgs[i])
                .w_h(20.0, 20.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.mmap_site_icons[i], ui);
            }

            // Group member indicators
            let client_state = self.client.state();
            let member_pos = client_state.ecs().read_storage::<comp::Pos>();
            let group_members = self
                .client
                .group_members()
                .iter()
                .filter_map(|(u, r)| match r {
                    Role::Member => Some(u),
                    Role::Pet => None,
                })
                .collect::<Vec<_>>();
            let group_size = group_members.len();
            //let in_group = !group_members.is_empty();
            let uid_allocator = client_state
                .ecs()
                .read_resource::<common_net::sync::UidAllocator>();
            if state.ids.member_indicators.len() < group_size {
                state.update(|s| {
                    s.ids
                        .member_indicators
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };
            for (i, &uid) in group_members.iter().copied().enumerate() {
                let entity = uid_allocator.retrieve_entity_internal(uid.into());
                let member_pos = entity.and_then(|entity| member_pos.get(entity));

                if let Some(member_pos) = member_pos {
                    let rpos = match wpos_to_rpos(member_pos.0.xy(), false) {
                        Some(rpos) => rpos,
                        None => continue,
                    };

                    let factor = 1.2;
                    let z_comparison = (member_pos.0.z - player_pos.z) as i32;
                    Button::image(match z_comparison {
                        10..=i32::MAX => self.imgs.indicator_group_up,
                        i32::MIN..=-10 => self.imgs.indicator_group_down,
                        _ => self.imgs.indicator_group,
                    })
                    .x_y_position_relative_to(
                        state.ids.map_layers[0],
                        position::Relative::Scalar(rpos.x as f64),
                        position::Relative::Scalar(rpos.y as f64),
                    )
                    .w_h(16.0 * factor, 16.0 * factor)
                    .image_color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                    .set(state.ids.member_indicators[i], ui);
                }
            }

            // Group location markers
            if state.ids.location_marker_group.len() < self.location_markers.group.len() {
                state.update(|s| {
                    s.ids.location_marker_group.resize(
                        self.location_markers.group.len(),
                        &mut ui.widget_id_generator(),
                    )
                })
            };
            for (i, (&uid, &rpos)) in self.location_markers.group.iter().enumerate() {
                let lm = rpos.as_();
                if let Some(rpos) = wpos_to_rpos(lm, true) {
                    let (image_id, factor) = match self.client.group_info().map(|info| info.1) {
                        Some(leader) if leader == uid => {
                            (self.imgs.location_marker_group_leader, 1.2)
                        },
                        _ => (self.imgs.location_marker_group, 1.0),
                    };

                    Image::new(image_id)
                        .x_y_position_relative_to(
                            state.ids.map_layers[0],
                            position::Relative::Scalar(rpos.x as f64),
                            position::Relative::Scalar(rpos.y as f64 + 8.0 * factor),
                        )
                        .w_h(16.0 * factor, 16.0 * factor)
                        .parent(ui.window)
                        .set(state.ids.location_marker_group[i], ui)
                }
            }

            // Location marker
            if let Some(rpos) = self
                .location_markers
                .owned
                .and_then(|lm| wpos_to_rpos(lm.as_(), true))
            {
                let factor = 1.2;

                Image::new(self.imgs.location_marker)
                    .x_y_position_relative_to(
                        state.ids.map_layers[0],
                        position::Relative::Scalar(rpos.x as f64),
                        position::Relative::Scalar(rpos.y as f64 + 8.0 * factor),
                    )
                    .w_h(16.0 * factor, 16.0 * factor)
                    .parent(ui.window)
                    .set(state.ids.location_marker, ui)
            }
            // Indicator
            let ind_scale = 0.4;
            let ind_rotation = if is_facing_north {
                self.rot_imgs.indicator_mmap_small.target_north
            } else {
                self.rot_imgs.indicator_mmap_small.none
            };
            Image::new(ind_rotation)
                .middle_of(state.ids.map_layers[0])
                .w_h(32.0 * ind_scale, 37.0 * ind_scale)
                .color(Some(UI_HIGHLIGHT_0))
                .parent(ui.window)
                .set(state.ids.indicator, ui);

            // Compass directions
            let dirs = [
                (Vec2::new(0.0, 1.0), state.ids.mmap_north, "N", true),
                (Vec2::new(1.0, 0.0), state.ids.mmap_east, "E", false),
                (Vec2::new(0.0, -1.0), state.ids.mmap_south, "S", false),
                (Vec2::new(-1.0, 0.0), state.ids.mmap_west, "W", false),
            ];
            for (dir, id, name, bold) in dirs.iter() {
                let cardinal_dir = Vec2::unit_x().rotated_z(orientation.x as f64) * dir.x
                    + Vec2::unit_y().rotated_z(orientation.x as f64) * dir.y;
                let clamped = cardinal_dir / cardinal_dir.map(|e| e.abs()).reduce_partial_max();
                let pos = clamped * (map_size / 2.0 - 10.0);
                Text::new(name)
                    .x_y_position_relative_to(
                        state.ids.map_layers[0],
                        position::Relative::Scalar(pos.x),
                        position::Relative::Scalar(pos.y),
                    )
                    .font_size(self.fonts.cyri.scale(18))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(if *bold {
                        Color::Rgba(0.75, 0.0, 0.0, 1.0)
                    } else {
                        TEXT_COLOR
                    })
                    .parent(ui.window)
                    .set(*id, ui);
            }
        } else {
            Image::new(self.imgs.mmap_frame_closed)
                .w_h(174.0 * SCALE, 18.0 * SCALE)
                .color(Some(UI_MAIN))
                .top_right_with_margins_on(ui.window, 0.0, 5.0)
                .set(state.ids.mmap_frame, ui);
        }

        if Button::image(if show_minimap {
            self.imgs.mmap_open
        } else {
            self.imgs.mmap_closed
        })
        .w_h(18.0 * SCALE, 18.0 * SCALE)
        .hover_image(if show_minimap {
            self.imgs.mmap_open_hover
        } else {
            self.imgs.mmap_closed_hover
        })
        .press_image(if show_minimap {
            self.imgs.mmap_open_press
        } else {
            self.imgs.mmap_closed_press
        })
        .top_right_with_margins_on(state.ids.mmap_frame, 0.0, 0.0)
        .image_color(UI_HIGHLIGHT_0)
        .set(state.ids.mmap_button, ui)
        .was_clicked()
        {
            events.push(Event::SettingsChange(MinimapShow(!show_minimap)));
        }

        // TODO: Subregion name display

        // Title

        match self.client.current_chunk() {
            Some(chunk) => {
                // Count characters in the name to avoid clipping with the name display
                if let Some(name) = chunk.meta().name() {
                    let name_len = name.chars().count();
                    Text::new(name)
                        .mid_top_with_margin_on(state.ids.mmap_frame, match name_len {
                            15..=30 => 4.0,
                            _ => 2.0,
                        })
                        .font_size(self.fonts.cyri.scale(match name_len {
                            0..=15 => 18,
                            16..=30 => 14,
                            _ => 14,
                        }))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(state.ids.mmap_location, ui)
                }
            },
            None => Text::new(" ")
                .mid_top_with_margin_on(state.ids.mmap_frame, 0.0)
                .font_size(self.fonts.cyri.scale(18))
                .color(TEXT_COLOR)
                .set(state.ids.mmap_location, ui),
        }

        events
    }
}
