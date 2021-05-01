use super::{
    img_ids::{Imgs, ImgsRot},
    Show, QUALITY_COMMON, QUALITY_DEBUG, QUALITY_EPIC, QUALITY_HIGH, QUALITY_LOW, QUALITY_MODERATE,
    TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    session::settings_change::{Interface as InterfaceChange, Interface::*},
    ui::{fonts::Fonts, img_ids},
    GlobalState,
};
use client::{self, Client};
use common::{comp, comp::group::Role, terrain::TerrainChunkSize, vol::RectVolSize};
use common_net::msg::world_msg::SiteKind;
use conrod_core::{
    color, position,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

use specs::{saveload::MarkerAllocator, WorldExt};
use vek::*;

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
    }
}

#[derive(WidgetCommon)]
pub struct MiniMap<'a> {
    show: &'a Show,
    client: &'a Client,
    imgs: &'a Imgs,
    rot_imgs: &'a ImgsRot,
    world_map: &'a (Vec<img_ids::Rotations>, Vec2<u32>),
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    ori: Vec3<f32>,
    global_state: &'a GlobalState,
    location_marker: Option<Vec2<f32>>,
}

impl<'a> MiniMap<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        rot_imgs: &'a ImgsRot,
        world_map: &'a (Vec<img_ids::Rotations>, Vec2<u32>),
        fonts: &'a Fonts,
        ori: Vec3<f32>,
        global_state: &'a GlobalState,
        location_marker: Option<Vec2<f32>>,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            rot_imgs,
            world_map,
            fonts,
            common: widget::CommonBuilder::default(),
            ori,
            global_state,
            location_marker,
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

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let mut events = Vec::new();

        let widget::UpdateArgs { state, ui, .. } = args;
        let mut zoom = self.global_state.settings.interface.minimap_zoom;
        const SCALE: f64 = 1.5; // TODO Make this a setting
        let show_minimap = self.global_state.settings.interface.minimap_show;
        let is_facing_north = self.global_state.settings.interface.minimap_face_north;
        let show_topo_map = self.global_state.settings.interface.map_show_topo_map;
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

            events.push(Event::SettingsChange(MinimapZoom(zoom)));

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
                let rcpos = rwpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e / sz as f32);
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

                Image::new(match &site.kind {
                    SiteKind::Town => self.imgs.mmap_site_town_bg,
                    SiteKind::Dungeon { .. } => self.imgs.mmap_site_dungeon_bg,
                    SiteKind::Castle => self.imgs.mmap_site_castle_bg,
                    SiteKind::Cave => self.imgs.mmap_site_cave_bg,
                    SiteKind::Tree => self.imgs.mmap_site_tree,
                })
                .x_y_position_relative_to(
                    state.ids.map_layers[0],
                    position::Relative::Scalar(rpos.x as f64),
                    position::Relative::Scalar(rpos.y as f64),
                )
                .w_h(20.0, 20.0)
                .color(Some(match &site.kind {
                    SiteKind::Town => Color::Rgba(1.0, 1.0, 1.0, 0.0),
                    SiteKind::Castle => Color::Rgba(1.0, 1.0, 1.0, 0.0),
                    SiteKind::Dungeon { difficulty } => match difficulty {
                        0 => QUALITY_LOW,
                        1 => QUALITY_COMMON,
                        2 => QUALITY_MODERATE,
                        3 => QUALITY_HIGH,
                        4 => QUALITY_EPIC,
                        5 => QUALITY_DEBUG,
                        _ => Color::Rgba(1.0, 1.0, 1.0, 0.0),
                    },
                    SiteKind::Cave => Color::Rgba(1.0, 1.0, 1.0, 0.0),
                    SiteKind::Tree => Color::Rgba(1.0, 1.0, 1.0, 0.0),
                }))
                .set(state.ids.mmap_site_icons_bgs[i], ui);
                Image::new(match &site.kind {
                    SiteKind::Town => self.imgs.mmap_site_town,
                    SiteKind::Dungeon { .. } => self.imgs.mmap_site_dungeon,
                    SiteKind::Castle => self.imgs.mmap_site_castle,
                    SiteKind::Cave => self.imgs.mmap_site_cave,
                    SiteKind::Tree => self.imgs.mmap_site_tree,
                })
                .middle_of(state.ids.mmap_site_icons_bgs[i])
                .w_h(20.0, 20.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.mmap_site_icons[i], ui);
            }

            // Group member indicators
            let client_state = self.client.state();
            let member_pos = client_state.ecs().read_storage::<common::comp::Pos>();
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
                    let rpos = match wpos_to_rpos(member_pos.0.xy().map(|e| e as f32), false) {
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

            // Location marker
            if self.show.map_marker {
                if let Some(rpos) = self.location_marker.and_then(|lm| wpos_to_rpos(lm, true)) {
                    let factor = 1.2;

                    Button::image(self.imgs.location_marker)
                    .x_y_position_relative_to(
                        state.ids.map_layers[0],
                        position::Relative::Scalar(rpos.x as f64),
                        position::Relative::Scalar(rpos.y as f64 + 8.0 * factor),
                    )
                    .w_h(16.0 * factor, 16.0 * factor)
                    //.image_color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                    .floating(true)
                    .set(state.ids.location_marker, ui);
                }
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
                .floating(true)
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
                let name_len = chunk.meta().name().chars().count();
                Text::new(chunk.meta().name())
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
