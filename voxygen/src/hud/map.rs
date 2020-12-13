use super::{
    img_ids::{Imgs, ImgsRot},
    QUALITY_COMMON, QUALITY_DEBUG, QUALITY_EPIC, QUALITY_HIGH, QUALITY_LOW, QUALITY_MODERATE,
    TEXT_COLOR, TEXT_GRAY_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    i18n::Localization,
    ui::{fonts::Fonts, img_ids, ImageFrame, Tooltip, TooltipManager, Tooltipable},
    GlobalState,
};
use client::{self, Client};
use common::{comp, terrain::TerrainChunkSize, vol::RectVolSize};
use common_net::msg::world_msg::SiteKind;
use conrod_core::{
    color, position,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use specs::WorldExt;
use vek::*;

widget_ids! {
    struct Ids {
        frame,
        bg,
        icon,
        close,
        title,
        map_align,
        qlog_align,
        location_name,
        indicator,
        indicator_overlay,
        grid,
        map_title,
        qlog_title,
        zoom_slider,
        mmap_site_icons[],
        site_difs[],
        map_settings_align,
        show_towns_img,
        show_towns_box,
        show_towns_text,
        show_castles_img,
        show_castles_box,
        show_castles_text,
        show_dungeons_img,
        show_dungeons_box,
        show_dungeons_text,
        show_difficulty_img,
        show_difficulty_box,
        show_difficulty_text,
        recenter_button,
        drag_txt,
        drag_ico,
        zoom_txt,
        zoom_ico,
        show_caves_img,
        show_caves_box,
        show_caves_text,
    }
}

#[derive(WidgetCommon)]
pub struct Map<'a> {
    client: &'a Client,
    world_map: &'a (img_ids::Rotations, Vec2<u32>),
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    _pulse: f32,
    localized_strings: &'a Localization,
    global_state: &'a GlobalState,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
}
impl<'a> Map<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        rot_imgs: &'a ImgsRot,
        world_map: &'a (img_ids::Rotations, Vec2<u32>),
        fonts: &'a Fonts,
        pulse: f32,
        localized_strings: &'a Localization,
        global_state: &'a GlobalState,
        tooltip_manager: &'a mut TooltipManager,
    ) -> Self {
        Self {
            imgs,
            rot_imgs,
            world_map,
            client,
            fonts,
            common: widget::CommonBuilder::default(),
            _pulse: pulse,
            localized_strings,
            global_state,
            tooltip_manager,
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    MapZoom(f64),
    MapDrag(Vec2<f64>),
    ShowDifficulties(bool),
    ShowTowns(bool),
    ShowCastles(bool),
    ShowDungeons(bool),
    ShowCaves(bool),
    Close,
}

impl<'a> Widget for Map<'a> {
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

    #[allow(clippy::useless_format)] // TODO: Pending review in #587
    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let zoom = self.global_state.settings.gameplay.map_zoom * 0.8;
        let show_difficulty = self.global_state.settings.gameplay.map_show_difficulty;
        let show_towns = self.global_state.settings.gameplay.map_show_towns;
        let show_dungeons = self.global_state.settings.gameplay.map_show_dungeons;
        let show_castles = self.global_state.settings.gameplay.map_show_castles;
        let show_caves = self.global_state.settings.gameplay.map_show_caves;
        let mut events = Vec::new();
        let i18n = &self.localized_strings;
        // Tooltips
        let site_tooltip = Tooltip::new({
            // Edge images [t, b, r, l]
            // Corner images [tr, tl, br, bl]
            let edge = &self.rot_imgs.tt_side;
            let corner = &self.rot_imgs.tt_corner;
            ImageFrame::new(
                [edge.cw180, edge.none, edge.cw270, edge.cw90],
                [corner.none, corner.cw270, corner.cw90, corner.cw180],
                Color::Rgba(0.08, 0.07, 0.04, 1.0),
                5.0,
            )
        })
        .title_font_size(self.fonts.cyri.scale(15))
        .parent(ui.window)
        .desc_font_size(self.fonts.cyri.scale(12))
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);
        // Frame
        Image::new(self.imgs.map_bg)
            .w_h(1202.0, 886.0)
            .mid_top_with_margin_on(ui.window, 5.0)
            .color(Some(UI_MAIN))
            .set(state.ids.bg, ui);

        Image::new(self.imgs.map_frame)
            .w_h(1202.0, 886.0)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.frame, ui);

        // Map Content Alignment
        Rectangle::fill_with([814.0, 834.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.frame, 46.0, 240.0)
            .set(state.ids.map_align, ui);

        // Questlog Content Alignment
        Rectangle::fill_with([232.0, 814.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.frame, 44.0, 2.0)
            .set(state.ids.qlog_align, ui);

        // Icon
        Image::new(self.imgs.map_icon)
            .w_h(30.0, 30.0)
            .top_left_with_margins_on(state.ids.frame, 6.0, 8.0)
            .set(state.ids.icon, ui);

        // Map Title
        Text::new(i18n.get("hud.map.map_title"))
            .mid_top_with_margin_on(state.ids.frame, 3.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(29))
            .color(TEXT_COLOR)
            .set(state.ids.map_title, ui);

        // Questlog Title
        Text::new(i18n.get("hud.map.qlog_title"))
            .mid_top_with_margin_on(state.ids.qlog_align, 6.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(21))
            .color(TEXT_COLOR)
            .set(state.ids.qlog_title, ui);

        // Location Name
        /*match self.client.current_chunk() {
            Some(chunk) => Text::new(chunk.meta().name())
                .mid_top_with_margin_on(state.ids.bg, 55.0)
                .font_size(self.fonts.alkhemi.scale(60))
                .color(TEXT_COLOR)
                .font_id(self.fonts.alkhemi.conrod_id)
                .parent(state.ids.frame)
                .set(state.ids.location_name, ui),
            None => Text::new(" ")
                .mid_top_with_margin_on(state.ids.bg, 3.0)
                .font_size(self.fonts.alkhemi.scale(40))
                .font_id(self.fonts.alkhemi.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.location_name, ui),
        }*/
        Image::new(self.imgs.map_frame_art)
            .mid_top_with_margin_on(state.ids.map_align, 5.0)
            .w_h(765.0, 765.0)
            .parent(state.ids.bg)
            .set(state.ids.grid, ui);
        // Map Image
        let (world_map, worldsize) = self.world_map;

        // Coordinates
        let player_pos = self
            .client
            .state()
            .ecs()
            .read_storage::<comp::Pos>()
            .get(self.client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        let max_zoom = worldsize
            .reduce_partial_max() as f64/*.min(f64::MAX)*/;

        let map_size = Vec2::new(760.0, 760.0);

        let w_src = max_zoom / zoom;
        let h_src = max_zoom / zoom;
        // Handle dragging
        let drag = self.global_state.settings.gameplay.map_drag;
        let dragged: Vec2<f64> = ui
            .widget_input(state.ids.grid)
            .drags()
            .left()
            .map(|drag| Vec2::<f64>::from(drag.delta_xy))
            .sum();
        // Drag represents offset of view from the player_pos in chunk coords
        let drag_new = drag + dragged / map_size / zoom * max_zoom;
        events.push(Event::MapDrag(drag_new));

        let rect_src = position::Rect::from_xy_dim(
            [
                (player_pos.x as f64 / TerrainChunkSize::RECT_SIZE.x as f64) - drag.x,
                (worldsize.y as f64 - (player_pos.y as f64 / TerrainChunkSize::RECT_SIZE.y as f64))
                    + drag.y,
            ],
            [w_src, h_src],
        );
        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.ids.frame, 0.0, 0.0)
            .set(state.ids.close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }
        Image::new(world_map.none)
            .mid_top_with_margin_on(state.ids.map_align, 10.0)
            .w_h(map_size.x, map_size.y)
            .parent(state.ids.bg)
            .source_rectangle(rect_src)
            .set(state.ids.grid, ui);
        // Handle zooming with the mousewheel
        let scrolled: f64 = ui
            .widget_input(state.ids.grid)
            .scrolls()
            .map(|scroll| scroll.y)
            .sum();
        let new_zoom_lvl = (self.global_state.settings.gameplay.map_zoom
            * (scrolled * 0.05 * -1.0).exp2())
        .clamped(1.25, max_zoom / 64.0);
        events.push(Event::MapZoom(new_zoom_lvl as f64));
        // Icon settings
        // Alignment
        Rectangle::fill_with([150.0, 200.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.frame, 55.0, 10.0)
            .set(state.ids.map_settings_align, ui);
        // Checkboxes
        // Show difficulties
        Image::new(self.imgs.map_dif_6)
            .top_left_with_margins_on(state.ids.map_settings_align, 5.0, 5.0)
            .w_h(20.0, 20.0)
            .set(state.ids.show_difficulty_img, ui);
        if Button::image(if show_difficulty {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox
        })
        .w_h(18.0, 18.0)
        .hover_image(if show_difficulty {
            self.imgs.checkbox_checked_mo
        } else {
            self.imgs.checkbox_mo
        })
        .press_image(if show_difficulty {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox_press
        })
        .right_from(state.ids.show_difficulty_img, 10.0)
        .set(state.ids.show_difficulty_box, ui)
        .was_clicked()
        {
            events.push(Event::ShowDifficulties(!show_difficulty));
        }
        Text::new(i18n.get("hud.map.difficulty"))
            .right_from(state.ids.show_difficulty_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_difficulty_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_difficulty_text, ui);
        // Towns
        Image::new(self.imgs.mmap_site_town)
            .down_from(state.ids.show_difficulty_img, 10.0)
            .w_h(20.0, 20.0)
            .set(state.ids.show_towns_img, ui);
        if Button::image(if show_towns {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox
        })
        .w_h(18.0, 18.0)
        .hover_image(if show_towns {
            self.imgs.checkbox_checked_mo
        } else {
            self.imgs.checkbox_mo
        })
        .press_image(if show_towns {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox_press
        })
        .right_from(state.ids.show_towns_img, 10.0)
        .set(state.ids.show_towns_box, ui)
        .was_clicked()
        {
            events.push(Event::ShowTowns(!show_towns));
        }
        Text::new(i18n.get("hud.map.towns"))
            .right_from(state.ids.show_towns_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_towns_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_towns_text, ui);
        // Castles
        Image::new(self.imgs.mmap_site_castle)
            .down_from(state.ids.show_towns_img, 10.0)
            .w_h(20.0, 20.0)
            .set(state.ids.show_castles_img, ui);
        if Button::image(if show_castles {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox
        })
        .w_h(18.0, 18.0)
        .hover_image(if show_castles {
            self.imgs.checkbox_checked_mo
        } else {
            self.imgs.checkbox_mo
        })
        .press_image(if show_castles {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox_press
        })
        .right_from(state.ids.show_castles_img, 10.0)
        .set(state.ids.show_castles_box, ui)
        .was_clicked()
        {
            events.push(Event::ShowCastles(!show_castles));
        }
        Text::new(i18n.get("hud.map.castles"))
            .right_from(state.ids.show_castles_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_castles_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_castles_text, ui);
        // Dungeons
        Image::new(self.imgs.mmap_site_dungeon)
            .down_from(state.ids.show_castles_img, 10.0)
            .w_h(20.0, 20.0)
            .set(state.ids.show_dungeons_img, ui);
        if Button::image(if show_dungeons {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox
        })
        .w_h(18.0, 18.0)
        .hover_image(if show_dungeons {
            self.imgs.checkbox_checked_mo
        } else {
            self.imgs.checkbox_mo
        })
        .press_image(if show_dungeons {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox_press
        })
        .right_from(state.ids.show_dungeons_img, 10.0)
        .set(state.ids.show_dungeons_box, ui)
        .was_clicked()
        {
            events.push(Event::ShowDungeons(!show_dungeons));
        }
        Text::new(i18n.get("hud.map.dungeons"))
            .right_from(state.ids.show_dungeons_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_dungeons_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_dungeons_text, ui);
        // Caves
        Image::new(self.imgs.mmap_site_cave)
            .down_from(state.ids.show_dungeons_img, 10.0)
            .w_h(20.0, 20.0)
            .set(state.ids.show_caves_img, ui);
        if Button::image(if show_caves {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox
        })
        .w_h(18.0, 18.0)
        .hover_image(if show_caves {
            self.imgs.checkbox_checked_mo
        } else {
            self.imgs.checkbox_mo
        })
        .press_image(if show_caves {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox_press
        })
        .right_from(state.ids.show_caves_img, 10.0)
        .set(state.ids.show_caves_box, ui)
        .was_clicked()
        {
            events.push(Event::ShowCaves(!show_caves));
        }
        Text::new(i18n.get("hud.map.caves"))
            .right_from(state.ids.show_caves_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_caves_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_caves_text, ui);
        // Map icons
        if state.ids.mmap_site_icons.len() < self.client.sites().len() {
            state.update(|state| {
                state
                    .ids
                    .mmap_site_icons
                    .resize(self.client.sites().len(), &mut ui.widget_id_generator())
            });
        }
        if state.ids.site_difs.len() < self.client.sites().len() {
            state.update(|state| {
                state
                    .ids
                    .site_difs
                    .resize(self.client.sites().len(), &mut ui.widget_id_generator())
            });
        }
        for (i, site) in self.client.sites().iter().enumerate() {
            // Site pos in world coordinates relative to the player
            let rwpos = site.wpos.map(|e| e as f32) - player_pos;
            // Convert to chunk coordinates
            let rcpos = rwpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e / sz as f32)
                // Add map dragging
                + drag.map(|e| e as f32);
            // Convert to fractional coordinates relative to the worldsize
            let rfpos = rcpos / max_zoom as f32;
            // Convert to relative pixel coordinates from the center of the map
            // Accounting for zooming
            let rpos = rfpos.map2(map_size, |e, sz| e * sz as f32 * zoom as f32);

            if rpos
                .map2(map_size, |e, sz| e.abs() > sz as f32 / 2.0)
                .reduce_or()
            {
                continue;
            }
            let title = site.name.as_deref().unwrap_or_else(|| match &site.kind {
                SiteKind::Town => i18n.get("hud.map.town"),
                SiteKind::Dungeon { .. } => i18n.get("hud.map.dungeon"),
                SiteKind::Castle => i18n.get("hud.map.castle"),
                SiteKind::Cave => i18n.get("hud.map.cave"),
            });
            let (difficulty, desc) = match &site.kind {
                SiteKind::Town => (0, i18n.get("hud.map.town").to_string()),
                SiteKind::Dungeon { difficulty } => (
                    *difficulty,
                    i18n.get("hud.map.difficulty_dungeon")
                        .replace("{difficulty}", difficulty.to_string().as_str()),
                ),
                SiteKind::Castle => (0, i18n.get("hud.map.castle").to_string()),
                SiteKind::Cave => (0, i18n.get("hud.map.cave").to_string()),
            };
            let site_btn = Button::image(match &site.kind {
                SiteKind::Town => self.imgs.mmap_site_town,
                SiteKind::Dungeon { .. } => self.imgs.mmap_site_dungeon,
                SiteKind::Castle => self.imgs.mmap_site_castle,
                SiteKind::Cave => self.imgs.mmap_site_cave,
            })
            .x_y_position_relative_to(
                state.ids.grid,
                position::Relative::Scalar(rpos.x as f64),
                position::Relative::Scalar(rpos.y as f64),
            )
            .w_h(20.0 * 1.2, 20.0 * 1.2)
            .hover_image(match &site.kind {
                SiteKind::Town => self.imgs.mmap_site_town_hover,
                SiteKind::Dungeon { .. } => self.imgs.mmap_site_dungeon_hover,
                SiteKind::Castle => self.imgs.mmap_site_castle_hover,
                SiteKind::Cave => self.imgs.mmap_site_cave_hover,
            })
            .image_color(UI_HIGHLIGHT_0)
            .with_tooltip(
                self.tooltip_manager,
                title,
                &desc,
                &site_tooltip,
                match &site.kind {
                    SiteKind::Town => TEXT_COLOR,
                    SiteKind::Castle => TEXT_COLOR,
                    SiteKind::Dungeon { .. } => match difficulty {
                        0 => QUALITY_LOW,
                        1 => QUALITY_COMMON,
                        2 => QUALITY_MODERATE,
                        3 => QUALITY_HIGH,
                        4 => QUALITY_EPIC,
                        5 => QUALITY_DEBUG,
                        _ => TEXT_COLOR,
                    },
                    SiteKind::Cave => TEXT_COLOR,
                },
            );
            // Only display sites that are toggled on
            match &site.kind {
                SiteKind::Town => {
                    if show_towns {
                        site_btn.set(state.ids.mmap_site_icons[i], ui);
                    }
                },
                SiteKind::Dungeon { .. } => {
                    if show_dungeons {
                        site_btn.set(state.ids.mmap_site_icons[i], ui);
                    }
                },
                SiteKind::Castle => {
                    if show_castles {
                        site_btn.set(state.ids.mmap_site_icons[i], ui);
                    }
                },
                SiteKind::Cave => {
                    if show_caves {
                        site_btn.set(state.ids.mmap_site_icons[i], ui);
                    }
                },
            }

            // Difficulty from 0-6
            // 0 = towns and places without a difficulty level
            if show_difficulty {
                let size = 1.8; // Size factor for difficulty indicators
                let dif_img = Image::new(match difficulty {
                    1 => self.imgs.map_dif_1,
                    2 => self.imgs.map_dif_2,
                    3 => self.imgs.map_dif_3,
                    4 => self.imgs.map_dif_4,
                    5 => self.imgs.map_dif_6,
                    _ => self.imgs.nothing,
                })
                .mid_top_with_margin_on(state.ids.mmap_site_icons[i], match difficulty {
                    5 => -12.0 * size,
                    _ => -4.0 * size,
                })
                .w(match difficulty {
                    5 => 12.0 * size,
                    _ => 4.0 * size * difficulty as f64,
                })
                .h(match difficulty {
                    5 => 12.0 * size,
                    _ => 4.0 * size,
                })
                .color(Some(match difficulty {
                    0 => QUALITY_LOW,
                    1 => QUALITY_COMMON,
                    2 => QUALITY_MODERATE,
                    3 => QUALITY_HIGH,
                    4 => QUALITY_EPIC,
                    5 => QUALITY_DEBUG,
                    _ => TEXT_COLOR,
                }));
                match &site.kind {
                    SiteKind::Town => {
                        if show_towns {
                            dif_img.set(state.ids.site_difs[i], ui)
                        }
                    },
                    SiteKind::Dungeon { .. } => {
                        if show_dungeons {
                            dif_img.set(state.ids.site_difs[i], ui)
                        }
                    },
                    SiteKind::Castle => {
                        if show_castles {
                            dif_img.set(state.ids.site_difs[i], ui)
                        }
                    },
                    SiteKind::Cave => {
                        if show_caves {
                            dif_img.set(state.ids.site_difs[i], ui)
                        }
                    },
                }
            }
        }

        // Cursor pos relative to playerpos and widget size
        // Cursor stops moving on an axis as soon as it's position exceeds the maximum
        // // size of the widget

        /*let rel = Vec2::from(player_pos).map2(worldsize, |e: f32, sz: f64| {
            (e as f64 / sz).clamped(0.0, 1.0)
        });*/
        //let xy = rel * 760.0;

        // Offset from map center due to dragging
        let rcpos = drag.map(|e| e as f32);
        // Convert to fractional coordinates relative to the worldsize
        let rfpos = rcpos / max_zoom as f32;
        // Convert to relative pixel coordinates from the center of the map
        // Accounting for zooming
        let rpos = rfpos.map2(map_size, |e, sz| e * sz as f32 * zoom as f32);
        // Don't show if outside or near the edge of the map
        let arrow_sz = {
            let scale = 0.5;
            Vec2::new(36.0, 37.0) * scale
        };
        // Hide if icon could go off of the edge of the map
        let arrow_mag = arrow_sz.map(|e| e as f32 / 2.0).magnitude();
        if !rpos
            .map2(map_size, |e, sz| e.abs() + arrow_mag > sz as f32 / 2.0)
            .reduce_or()
        {
            Image::new(self.rot_imgs.indicator_mmap_small.target_north)
                .x_y_position_relative_to(
                    state.ids.grid,
                    position::Relative::Scalar(rpos.x as f64),
                    position::Relative::Scalar(rpos.y as f64),
                )
                .w_h(arrow_sz.x, arrow_sz.y)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.indicator, ui);
        }

        // Info about controls
        let icon_size = Vec2::new(25.6, 28.8);
        let recenter: bool;
        if drag.x != 0.0 || drag.y != 0.0 {
            recenter = true
        } else {
            recenter = false
        };
        if Button::image(self.imgs.button)
            .w_h(92.0, icon_size.y)
            .mid_bottom_with_margin_on(state.ids.grid, -36.0)
            .hover_image(if recenter {
                self.imgs.button_hover
            } else {
                self.imgs.button
            })
            .press_image(if recenter {
                self.imgs.button_press
            } else {
                self.imgs.button
            })
            .label(i18n.get("hud.map.recenter"))
            .label_y(conrod_core::position::Relative::Scalar(1.0))
            .label_color(if recenter {
                TEXT_COLOR
            } else {
                TEXT_GRAY_COLOR
            })
            .image_color(if recenter {
                TEXT_COLOR
            } else {
                TEXT_GRAY_COLOR
            })
            .label_font_size(self.fonts.cyri.scale(12))
            .label_font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.recenter_button, ui)
            .was_clicked()
        {
            events.push(Event::MapDrag(Vec2::zero()));
        };

        Image::new(self.imgs.m_move_ico)
            .bottom_left_with_margins_on(state.ids.grid, -36.0, 0.0)
            .w_h(icon_size.x, icon_size.y)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.drag_ico, ui);
        Text::new(i18n.get("hud.map.drag"))
            .right_from(state.ids.drag_ico, 5.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.grid)
            .color(TEXT_COLOR)
            .set(state.ids.drag_txt, ui);
        Image::new(self.imgs.m_scroll_ico)
            .right_from(state.ids.drag_txt, 5.0)
            .w_h(icon_size.x, icon_size.y)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.zoom_ico, ui);
        Text::new(i18n.get("hud.map.zoom"))
            .right_from(state.ids.zoom_ico, 5.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.grid)
            .color(TEXT_COLOR)
            .set(state.ids.zoom_txt, ui);

        events
    }
}
