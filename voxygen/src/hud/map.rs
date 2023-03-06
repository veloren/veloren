use super::{
    img_ids::{Imgs, ImgsRot},
    MapMarkers, QUALITY_COMMON, QUALITY_EPIC, QUALITY_HIGH, QUALITY_LOW, QUALITY_MODERATE, TEXT_BG,
    TEXT_BLUE_COLOR, TEXT_COLOR, TEXT_GRAY_COLOR, TEXT_VELORITE, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    game_input::GameInput,
    session::settings_change::{Interface as InterfaceChange, Interface::*},
    ui::{fonts::Fonts, img_ids, ImageFrame, Tooltip, TooltipManager, Tooltipable},
    window::KeyMouse,
    GlobalState,
};
use client::{self, Client, SiteInfoRich};
use common::{
    comp,
    comp::group::Role,
    terrain::{CoordinateConversions, TerrainChunkSize},
    trade::Good,
    vol::RectVolSize,
};
use common_net::msg::world_msg::{PoiKind, SiteId, SiteKind};
use conrod_core::{
    color,
    input::MouseButton as ConrodMouseButton,
    position,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, UiCell, Widget, WidgetCommon,
};
use i18n::Localization;
use specs::{saveload::MarkerAllocator, WorldExt};
use std::borrow::Cow;
use vek::*;
use winit::event::MouseButton;

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
        map_layers[],
        map_title,
        qlog_title,
        zoom_slider,
        mmap_site_icons[],
        mmap_poi_icons[],
        mmap_poi_title_bgs[],
        mmap_poi_titles[],
        peaks_txt,
        peaks_txt_bg,
        site_difs[],
        member_indicators[],
        member_height_indicators[],
        location_marker,
        location_marker_group[],
        map_settings_align,
        show_towns_img,
        show_towns_box,
        show_towns_text,
        show_sea_chapels_img,
        show_sea_chapels_box,
        show_sea_chapels_text,
        show_castles_img,
        show_castles_box,
        show_castles_text,
        show_bridges_img,
        show_bridges_box,
        show_bridges_text,
        show_dungeons_img,
        show_dungeons_box,
        show_dungeons_text,
        show_caves_img,
        show_caves_box,
        show_caves_text,
        show_trees_img,
        show_trees_box,
        show_trees_text,
        show_peaks_img,
        show_peaks_box,
        show_peaks_text,
        show_biomes_img,
        show_biomes_box,
        show_biomes_text,
        show_voxel_map_img,
        show_voxel_map_box,
        show_voxel_map_text,
        show_difficulty_img,
        show_difficulty_box,
        show_difficulty_text,
        recenter_button,
        drag_txt,
        drag_ico,
        zoom_txt,
        zoom_ico,
        waypoint_binding_txt,
        waypoint_txt,
        map_mode_btn,
        map_mode_overlay,
        minimap_mode_btn,
        minimap_mode_overlay,

    }
}

const SHOW_ECONOMY: bool = false; // turn this display off (for 0.9) until we have an improved look

#[derive(WidgetCommon)]
pub struct Map<'a> {
    client: &'a Client,
    world_map: &'a (Vec<img_ids::Rotations>, Vec2<u32>),
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    _pulse: f32,
    localized_strings: &'a Localization,
    global_state: &'a GlobalState,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    location_markers: &'a MapMarkers,
    map_drag: Vec2<f64>,
}
impl<'a> Map<'a> {
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        rot_imgs: &'a ImgsRot,
        world_map: &'a (Vec<img_ids::Rotations>, Vec2<u32>),
        fonts: &'a Fonts,
        pulse: f32,
        localized_strings: &'a Localization,
        global_state: &'a GlobalState,
        tooltip_manager: &'a mut TooltipManager,
        location_markers: &'a MapMarkers,
        map_drag: Vec2<f64>,
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
            location_markers,
            map_drag,
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    SettingsChange(InterfaceChange),
    Close,
    RequestSiteInfo(SiteId),
    SetLocationMarker(Vec2<i32>),
    MapDrag(Vec2<f64>),
    RemoveMarker,
}

fn get_site_economy(site_rich: &SiteInfoRich) -> String {
    if SHOW_ECONOMY {
        let site = &site_rich.site;
        if let Some(economy) = &site_rich.economy {
            use common::trade::Good::{Armor, Coin, Food, Ingredients, Potions, Tools};
            let mut result = format!("\n\nPopulation {:?}", economy.population);
            result += "\nStock";
            for i in [Food, Potions, Ingredients, Coin, Tools, Armor].iter() {
                result += &format!("\n  {:?}={:.3}", *i, *economy.stock.get(i).unwrap_or(&0.0));
            }
            result += "\nPrice";
            for i in [Food, Potions, Ingredients, Coin, Tools, Armor].iter() {
                result += &format!("\n  {:?}={:.3}", *i, *economy.values.get(i).unwrap_or(&0.0));
            }

            let mut trade_sorted: Vec<(&Good, &f32)> = economy.last_exports.iter().collect();
            trade_sorted.sort_unstable_by(|a, b| a.1.partial_cmp(b.1).unwrap());
            if trade_sorted.first().is_some() {
                result += &format!("\nTrade {:.1} ", *(trade_sorted.first().unwrap().1));
                for i in trade_sorted.iter().filter(|x| *x.1 != 0.0) {
                    result += &format!("{:?} ", i.0);
                }
                result += &format!("{:.3}", *(trade_sorted.last().unwrap().1));
            }
            result
        } else {
            format!("\nloading economy for\n{:?}", site.id)
        }
    } else {
        "".into()
    }
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

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Map::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let zoom = self.global_state.settings.interface.map_zoom;
        let show_difficulty = self.global_state.settings.interface.map_show_difficulty;
        let show_towns = self.global_state.settings.interface.map_show_towns;
        let show_dungeons = self.global_state.settings.interface.map_show_dungeons;
        let show_castles = self.global_state.settings.interface.map_show_castles;
        let show_bridges = self.global_state.settings.interface.map_show_bridges;
        let show_caves = self.global_state.settings.interface.map_show_caves;
        let show_trees = self.global_state.settings.interface.map_show_trees;
        let show_peaks = self.global_state.settings.interface.map_show_peaks;
        let show_biomes = self.global_state.settings.interface.map_show_biomes;
        let show_voxel_map = self.global_state.settings.interface.map_show_voxel_map;
        let show_topo_map = self.global_state.settings.interface.map_show_topo_map;
        let location_marker_binding = self
            .global_state
            .settings
            .controls
            .keybindings
            .get(&GameInput::MapSetMarker)
            .cloned()
            .flatten()
            .unwrap_or(KeyMouse::Mouse(MouseButton::Middle));
        let key_layout = &self.global_state.window.key_layout;
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
        Text::new(&i18n.get_msg("hud-map-map_title"))
            .mid_top_with_margin_on(state.ids.frame, 3.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(29))
            .color(TEXT_COLOR)
            .set(state.ids.map_title, ui);

        // Questlog Title
        Text::new(&i18n.get_msg("hud-map-qlog_title"))
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

        // Map Size
        let worldsize = self.world_map.1;

        // Coordinates
        let player_pos = self
            .client
            .state()
            .ecs()
            .read_storage::<comp::Pos>()
            .get(self.client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        let map_size = Vec2::new(760.0, 760.0);

        let player_pos_chunks =
            player_pos.xy().map(|x| x as f64) / TerrainChunkSize::RECT_SIZE.map(|x| x as f64);
        let min_drag = player_pos_chunks - worldsize.map(|x| x as f64);
        let max_drag = player_pos_chunks;
        let drag = self.map_drag.clamped(min_drag, max_drag);

        impl From<KeyMouse> for ConrodMouseButton {
            fn from(key: KeyMouse) -> Self {
                match key {
                    KeyMouse::Mouse(MouseButton::Left) => ConrodMouseButton::Left,
                    KeyMouse::Mouse(MouseButton::Right) => ConrodMouseButton::Right,
                    KeyMouse::Mouse(MouseButton::Middle) => ConrodMouseButton::Middle,
                    KeyMouse::Mouse(MouseButton::Other(0)) => ConrodMouseButton::X1,
                    KeyMouse::Mouse(MouseButton::Other(1)) => ConrodMouseButton::X2,
                    KeyMouse::Mouse(MouseButton::Other(2)) => ConrodMouseButton::Button6,
                    KeyMouse::Mouse(MouseButton::Other(3)) => ConrodMouseButton::Button7,
                    KeyMouse::Mouse(MouseButton::Other(4)) => ConrodMouseButton::Button8,
                    _ => conrod_core::input::MouseButton::Unknown,
                }
            }
        }
        enum MarkerChange {
            Pos(Vec2<f32>),
            ClickPos,
            Remove,
        }

        let handle_widget_mouse_events =
            |widget, marker: MarkerChange, ui: &mut UiCell, events: &mut Vec<Event>, map_widget| {
                // Handle Location Marking
                if let Some(click) = ui
                    .widget_input(widget)
                    .clicks()
                    .button(ConrodMouseButton::from(location_marker_binding))
                    .next()
                {
                    match marker {
                        MarkerChange::Pos(ref wpos) => {
                            events.push(Event::SetLocationMarker(wpos.as_()))
                        },
                        MarkerChange::ClickPos => {
                            let tmp: Vec2<f64> = Vec2::<f64>::from(click.xy) / zoom - drag;
                            let wpos = tmp.as_::<f32>().cpos_to_wpos() + player_pos;
                            events.push(Event::SetLocationMarker(wpos.as_()));
                        },
                        MarkerChange::Remove => events.push(Event::RemoveMarker),
                    }
                }

                // Handle zooming with the mouse wheel
                let scrolled: f64 = ui
                    .widget_input(widget)
                    .scrolls()
                    .map(|scroll| scroll.y)
                    .sum();
                if scrolled != 0.0 {
                    let min_zoom = map_size.x / worldsize.reduce_partial_max() as f64 / 2.0;
                    let new_zoom_lvl: f64 = (f64::log2(zoom) - scrolled * 0.03)
                        .exp2()
                        .clamp(min_zoom, 16.0);
                    events.push(Event::SettingsChange(MapZoom(new_zoom_lvl)));
                    let cursor_mouse_pos = ui
                        .widget_input(map_widget)
                        .mouse()
                        .map(|mouse| mouse.rel_xy());
                    if let Some(cursor_pos) = cursor_mouse_pos {
                        let mouse_pos = Vec2::from_slice(&cursor_pos);
                        let drag_new = drag + mouse_pos * (1.0 / new_zoom_lvl - 1.0 / zoom);
                        if drag_new != drag {
                            events.push(Event::MapDrag(drag_new));
                        }
                    }
                }

                // Handle dragging
                let dragged: Vec2<f64> = ui
                    .widget_input(widget)
                    .drags()
                    .left()
                    .map(|drag| Vec2::<f64>::from(drag.delta_xy))
                    .sum();
                // Drag represents offset of view from the player_pos in chunk coords
                let drag_new = drag + dragged / zoom;
                if drag_new != drag {
                    events.push(Event::MapDrag(drag_new));
                }
            };

        handle_widget_mouse_events(
            state.ids.map_layers[0],
            MarkerChange::ClickPos,
            ui,
            &mut events,
            state.ids.map_layers[0],
        );

        let rect_src = position::Rect::from_xy_dim(
            [
                (player_pos.x as f64 / TerrainChunkSize::RECT_SIZE.x as f64) - drag.x,
                (worldsize.y as f64 - (player_pos.y as f64 / TerrainChunkSize::RECT_SIZE.y as f64))
                    + drag.y,
            ],
            [map_size.x / zoom, map_size.y / zoom],
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

        // Map Layer Images
        for (index, layer) in self.world_map.0.iter().enumerate() {
            if index == 0 {
                Button::image(layer.none)
                    .mid_top_with_margin_on(state.ids.map_align, 10.0)
                    .w_h(map_size.x, map_size.y)
                    .parent(state.ids.bg)
                    .source_rectangle(rect_src)
                    .set(state.ids.map_layers[index], ui);
            } else if show_topo_map {
                Button::image(layer.none)
                    .mid_top_with_margin_on(state.ids.map_align, 10.0)
                    .w_h(map_size.x, map_size.y)
                    .parent(state.ids.bg)
                    .source_rectangle(rect_src)
                    .graphics_for(state.ids.map_layers[0])
                    .set(state.ids.map_layers[index], ui);
            }
        }

        // Icon settings
        // Alignment
        Rectangle::fill_with([150.0, 200.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.frame, 55.0, 10.0)
            .set(state.ids.map_settings_align, ui);
        // Checkboxes
        // Show difficulties
        Image::new(self.imgs.map_dif_icon)
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
            events.push(Event::SettingsChange(MapShowDifficulty(!show_difficulty)));
        }
        Text::new(&i18n.get_msg("hud-map-difficulty"))
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
            events.push(Event::SettingsChange(MapShowTowns(!show_towns)));
        }
        Text::new(&i18n.get_msg("hud-map-towns"))
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
            events.push(Event::SettingsChange(MapShowCastles(!show_castles)));
        }
        Text::new(&i18n.get_msg("hud-map-castles"))
            .right_from(state.ids.show_castles_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_castles_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_castles_text, ui);
        // Bridges
        Image::new(self.imgs.mmap_site_bridge)
            .down_from(state.ids.show_castles_img, 10.0)
            .w_h(20.0, 20.0)
            .set(state.ids.show_bridges_img, ui);
        if Button::image(if show_bridges {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox
        })
        .w_h(18.0, 18.0)
        .hover_image(if show_bridges {
            self.imgs.checkbox_checked_mo
        } else {
            self.imgs.checkbox_mo
        })
        .press_image(if show_bridges {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox_press
        })
        .right_from(state.ids.show_bridges_img, 10.0)
        .set(state.ids.show_bridges_box, ui)
        .was_clicked()
        {
            events.push(Event::SettingsChange(MapShowBridges(!show_bridges)));
        }
        Text::new(&i18n.get_msg("hud-map-bridge"))
            .right_from(state.ids.show_bridges_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_bridges_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_bridges_text, ui);
        // Dungeons
        Image::new(self.imgs.mmap_site_dungeon)
            .down_from(state.ids.show_bridges_img, 10.0)
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
            events.push(Event::SettingsChange(MapShowDungeons(!show_dungeons)));
        }
        Text::new(&i18n.get_msg("hud-map-dungeons"))
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
            events.push(Event::SettingsChange(MapShowCaves(!show_caves)));
        }
        Text::new(&i18n.get_msg("hud-map-caves"))
            .right_from(state.ids.show_caves_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_caves_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_caves_text, ui);
        // Trees
        Image::new(self.imgs.mmap_site_tree)
            .down_from(state.ids.show_caves_img, 10.0)
            .w_h(20.0, 20.0)
            .set(state.ids.show_trees_img, ui);
        if Button::image(if show_trees {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox
        })
        .w_h(18.0, 18.0)
        .hover_image(if show_trees {
            self.imgs.checkbox_checked_mo
        } else {
            self.imgs.checkbox_mo
        })
        .press_image(if show_trees {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox_press
        })
        .right_from(state.ids.show_trees_img, 10.0)
        .set(state.ids.show_trees_box, ui)
        .was_clicked()
        {
            events.push(Event::SettingsChange(MapShowTrees(!show_trees)));
        }
        Text::new(&i18n.get_msg("hud-map-trees"))
            .right_from(state.ids.show_trees_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_trees_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_trees_text, ui);
        // Biomes
        Image::new(self.imgs.mmap_poi_biome)
            .down_from(state.ids.show_trees_img, 10.0)
            .w_h(20.0, 20.0)
            .set(state.ids.show_biomes_img, ui);
        if Button::image(if show_biomes {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox
        })
        .w_h(18.0, 18.0)
        .hover_image(if show_biomes {
            self.imgs.checkbox_checked_mo
        } else {
            self.imgs.checkbox_mo
        })
        .press_image(if show_biomes {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox_press
        })
        .right_from(state.ids.show_biomes_img, 10.0)
        .set(state.ids.show_biomes_box, ui)
        .was_clicked()
        {
            events.push(Event::SettingsChange(MapShowBiomes(!show_biomes)));
        }
        Text::new(&i18n.get_msg("hud-map-biomes"))
            .right_from(state.ids.show_biomes_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_biomes_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_biomes_text, ui);
        // Peaks
        Image::new(self.imgs.mmap_poi_peak)
            .down_from(state.ids.show_biomes_img, 10.0)
            .w_h(20.0, 20.0)
            .set(state.ids.show_peaks_img, ui);
        if Button::image(if show_peaks {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox
        })
        .w_h(18.0, 18.0)
        .hover_image(if show_peaks {
            self.imgs.checkbox_checked_mo
        } else {
            self.imgs.checkbox_mo
        })
        .press_image(if show_peaks {
            self.imgs.checkbox_checked
        } else {
            self.imgs.checkbox_press
        })
        .right_from(state.ids.show_peaks_img, 10.0)
        .set(state.ids.show_peaks_box, ui)
        .was_clicked()
        {
            events.push(Event::SettingsChange(MapShowPeaks(!show_peaks)));
        }
        Text::new(&i18n.get_msg("hud-map-peaks"))
            .right_from(state.ids.show_peaks_box, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.show_peaks_box)
            .color(TEXT_COLOR)
            .set(state.ids.show_peaks_text, ui);
        // Voxel map (TODO: enable this once Pfau approves the final UI, and once
        // there's a non-placeholder graphic for the checkbox)
        const EXPOSE_VOXEL_MAP_TOGGLE_IN_UI: bool = false;
        if EXPOSE_VOXEL_MAP_TOGGLE_IN_UI {
            Image::new(self.imgs.mmap_poi_peak)
                .down_from(state.ids.show_peaks_img, 10.0)
                .w_h(20.0, 20.0)
                .set(state.ids.show_voxel_map_img, ui);
            if Button::image(if show_voxel_map {
                self.imgs.checkbox_checked
            } else {
                self.imgs.checkbox
            })
            .w_h(18.0, 18.0)
            .hover_image(if show_voxel_map {
                self.imgs.checkbox_checked_mo
            } else {
                self.imgs.checkbox_mo
            })
            .press_image(if show_voxel_map {
                self.imgs.checkbox_checked
            } else {
                self.imgs.checkbox_press
            })
            .right_from(state.ids.show_voxel_map_img, 10.0)
            .set(state.ids.show_voxel_map_box, ui)
            .was_clicked()
            {
                events.push(Event::SettingsChange(MapShowVoxelMap(!show_voxel_map)));
            }
            Text::new(&i18n.get_msg("hud-map-voxel_map"))
                .right_from(state.ids.show_voxel_map_box, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .graphics_for(state.ids.show_voxel_map_box)
                .color(TEXT_COLOR)
                .set(state.ids.show_voxel_map_text, ui);
        }
        // Map icons
        if state.ids.mmap_poi_icons.len() < self.client.pois().len() {
            state.update(|state| {
                state
                    .ids
                    .mmap_poi_icons
                    .resize(self.client.pois().len(), &mut ui.widget_id_generator())
            });
            state.update(|state| {
                state
                    .ids
                    .mmap_poi_titles
                    .resize(self.client.pois().len(), &mut ui.widget_id_generator())
            });
            state.update(|state| {
                state
                    .ids
                    .mmap_poi_title_bgs
                    .resize(self.client.pois().len(), &mut ui.widget_id_generator())
            });
        }
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

        let wpos_to_rpos_fade =
            |wpos: Vec2<f32>, bounding_rect_size: Vec2<f32>, fade_start: f32| {
                // Site pos in world coordinates relative to the player
                let rwpos = wpos - player_pos;
                // Convert to chunk coordinates
                let rcpos = rwpos.wpos_to_cpos()
                // Add map dragging
                + drag.map(|e| e as f32);
                // Convert to relative pixel coordinates from the center of the map
                // Accounting for zooming
                let rpos = rcpos.map(|e| e * zoom as f32);

                let dist_to_closest_map_edge =
                    (rpos.map2(map_size, |e, sz| sz as f32 / 2.0 - e.abs()) - bounding_rect_size)
                        .reduce_partial_min();
                match dist_to_closest_map_edge {
                    x if x <= 0.0 => None,
                    x if x < fade_start => Some((
                        rpos,
                        // Easing function
                        1.0 - 2.0_f32.powf(-10.0 * x / fade_start),
                    )),
                    _ => Some((rpos, 1.0)),
                }
            };

        for (i, site_rich) in self.client.sites().values().enumerate() {
            let site = &site_rich.site;

            let rside = zoom as f32 * 8.0 * 1.2;

            let (rpos, fade) = match wpos_to_rpos_fade(
                site.wpos.map(|e| e as f32),
                Vec2::from(rside / 2.0),
                rside / 2.0,
            ) {
                Some(rpos) => rpos,
                None => continue,
            };

            let title =
                site.name
                    .as_deref()
                    .map(Cow::Borrowed)
                    .unwrap_or_else(|| match &site.kind {
                        SiteKind::Town => i18n.get_msg("hud-map-town"),
                        SiteKind::Dungeon { .. } => i18n.get_msg("hud-map-dungeon"),
                        SiteKind::Castle => i18n.get_msg("hud-map-castle"),
                        SiteKind::Cave => i18n.get_msg("hud-map-cave"),
                        SiteKind::Tree => i18n.get_msg("hud-map-tree"),
                        SiteKind::Gnarling => i18n.get_msg("hud-map-gnarling"),
                        SiteKind::ChapelSite => i18n.get_msg("hud-map-chapel_Site"),
                        SiteKind::Bridge => i18n.get_msg("hud-map-bridge"),
                    });
            let (difficulty, desc) = match &site.kind {
                SiteKind::Town => (None, i18n.get_msg("hud-map-town")),
                SiteKind::Dungeon { difficulty } => {
                    if *difficulty < 5 {
                        (
                            Some(*difficulty),
                            i18n.get_msg_ctx("hud-map-difficulty_dungeon", &i18n::fluent_args! {
                                "difficulty" => difficulty + 1
                            }),
                        )
                    } else {
                        (
                            Some(*difficulty),
                            i18n.get_msg_ctx("hud-map-difficulty_dungeon", &i18n::fluent_args! {
                                "difficulty" => difficulty
                            }),
                        )
                    }
                },
                SiteKind::Castle => (None, i18n.get_msg("hud-map-castle")),
                SiteKind::Cave => (None, i18n.get_msg("hud-map-cave")),
                SiteKind::Tree => (None, i18n.get_msg("hud-map-tree")),
                SiteKind::Gnarling => (Some(0), i18n.get_msg("hud-map-gnarling")),
                SiteKind::ChapelSite => (Some(3), i18n.get_msg("hud-map-chapel_site")),
                SiteKind::Bridge => (None, i18n.get_msg("hud-map-bridge")),
            };
            let desc = desc.into_owned() + &get_site_economy(site_rich);
            let site_btn = Button::image(match &site.kind {
                SiteKind::Town => self.imgs.mmap_site_town,
                SiteKind::ChapelSite => self.imgs.mmap_site_sea_chapel,
                SiteKind::Castle => self.imgs.mmap_site_castle,
                SiteKind::Cave => self.imgs.mmap_site_cave,
                SiteKind::Tree => self.imgs.mmap_site_tree,
                SiteKind::Gnarling => self.imgs.mmap_site_gnarling,
                SiteKind::Dungeon { difficulty } => match difficulty {
                    4 => self.imgs.mmap_site_minotaur,
                    5 => self.imgs.mmap_site_mindflayer,
                    _ => self.imgs.mmap_site_dungeon,
                },
                SiteKind::Bridge => self.imgs.mmap_site_bridge,
            })
            .x_y_position_relative_to(
                state.ids.map_layers[0],
                position::Relative::Scalar(rpos.x as f64),
                position::Relative::Scalar(rpos.y as f64),
            )
            .w_h(rside as f64, rside as f64)
            .hover_image(match &site.kind {
                SiteKind::Town => self.imgs.mmap_site_town_hover,
                SiteKind::ChapelSite => self.imgs.mmap_site_sea_chapel_hover,
                SiteKind::Castle => self.imgs.mmap_site_castle_hover,
                SiteKind::Cave => self.imgs.mmap_site_cave_hover,
                SiteKind::Tree => self.imgs.mmap_site_tree_hover,
                SiteKind::Gnarling => self.imgs.mmap_site_gnarling_hover,
                SiteKind::Dungeon { difficulty } => match difficulty {
                    4 => self.imgs.mmap_site_minotaur_hover,
                    5 => self.imgs.mmap_site_mindflayer_hover,
                    _ => self.imgs.mmap_site_dungeon_hover,
                },
                SiteKind::Bridge => self.imgs.mmap_site_bridge_hover,
            })
            .image_color(UI_HIGHLIGHT_0.alpha(fade))
            .with_tooltip(
                self.tooltip_manager,
                &title,
                &desc,
                &site_tooltip,
                match &site.kind {
                    SiteKind::Town => TEXT_COLOR,
                    SiteKind::Castle => TEXT_COLOR,
                    SiteKind::Dungeon { .. } | SiteKind::Gnarling | SiteKind::ChapelSite => {
                        match difficulty {
                            Some(0) => QUALITY_LOW,
                            Some(1) => QUALITY_COMMON,
                            Some(2) => QUALITY_MODERATE,
                            Some(3) => QUALITY_HIGH,
                            Some(4 | 5) => QUALITY_EPIC,
                            _ => TEXT_COLOR,
                        }
                    },
                    SiteKind::Cave => TEXT_COLOR,
                    SiteKind::Tree => TEXT_COLOR,
                    SiteKind::Bridge => TEXT_COLOR,
                },
            );

            handle_widget_mouse_events(
                state.ids.mmap_site_icons[i],
                MarkerChange::Pos(site.wpos.map(|e| e as f32)),
                ui,
                &mut events,
                state.ids.map_layers[0],
            );

            // Only display sites that are toggled on
            let show_site = match &site.kind {
                SiteKind::Town => show_towns,
                SiteKind::Dungeon { .. } | SiteKind::Gnarling | SiteKind::ChapelSite => {
                    show_dungeons
                },
                SiteKind::Castle => show_castles,
                SiteKind::Cave => show_caves,
                SiteKind::Tree => show_trees,
                SiteKind::Bridge => show_bridges,
            };
            if show_site {
                let tooltip_visible = site_btn.set_ext(state.ids.mmap_site_icons[i], ui).1;

                if SHOW_ECONOMY && tooltip_visible && site_rich.economy.is_none() {
                    events.push(Event::RequestSiteInfo(site.id));
                }
            }

            // Difficulty from 0-6
            // 0 = towns and places without a difficulty level
            if show_difficulty {
                let rsize = zoom * 2.4; // Size factor for difficulty indicators
                let dif_img = Image::new(match difficulty {
                    Some(0) => self.imgs.map_dif_1,
                    Some(1) => self.imgs.map_dif_2,
                    Some(2) => self.imgs.map_dif_3,
                    Some(3) => self.imgs.map_dif_4,
                    Some(4 | 5) => self.imgs.map_dif_5,
                    Some(_) => self.imgs.map_dif_unknown,
                    None => self.imgs.nothing,
                })
                .mid_top_with_margin_on(state.ids.mmap_site_icons[i], match difficulty {
                    Some(0 | 1) => -1.0 * rsize,
                    Some(_) => -2.0 * rsize,
                    _ => -1.0 * rsize,
                })
                .w(match difficulty {
                    Some(0) => 1.0 * rsize,
                    Some(1 | 2) => 2.0 * rsize,
                    Some(_) => 3.0 * rsize,
                    _ => 1.0 * rsize,
                })
                .h(match difficulty {
                    Some(0 | 1) => 1.0 * rsize,
                    Some(_) => 2.0 * rsize,
                    _ => 1.0 * rsize,
                })
                .color(Some(match difficulty {
                    Some(0) => QUALITY_LOW,
                    Some(1) => QUALITY_COMMON,
                    Some(2) => QUALITY_MODERATE,
                    Some(3) => QUALITY_HIGH,
                    Some(4 | 5) => QUALITY_EPIC, // Change this whenever difficulty is fixed
                    _ => TEXT_COLOR,
                }));
                match &site.kind {
                    SiteKind::Town => {
                        if show_towns {
                            dif_img.set(state.ids.site_difs[i], ui)
                        }
                    },
                    SiteKind::Dungeon { .. } | SiteKind::Gnarling | SiteKind::ChapelSite => {
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
                    SiteKind::Tree => {
                        if show_trees {
                            dif_img.set(state.ids.site_difs[i], ui)
                        }
                    },
                    SiteKind::Bridge => {
                        if show_bridges {
                            dif_img.set(state.ids.site_difs[i], ui)
                        }
                    },
                }

                handle_widget_mouse_events(
                    state.ids.site_difs[i],
                    MarkerChange::Pos(site.wpos.map(|e| e as f32)),
                    ui,
                    &mut events,
                    state.ids.map_layers[0],
                );
            }
        }
        for (i, poi) in self.client.pois().iter().enumerate() {
            // TODO: computation of text size to pass to wpos_to_rpos_fade, so it can
            // determine when it's going past the edge of the map screen
            let (rpos, fade) = match wpos_to_rpos_fade(
                poi.wpos.map(|e| e as f32),
                Vec2::from(zoom as f32 * 3.0),
                zoom as f32 * 5.0,
            ) {
                Some(rpos) => rpos,
                None => continue,
            };
            let title = &poi.name;
            match poi.kind {
                PoiKind::Peak(alt) => {
                    let height = format!("{} m", alt);
                    if show_peaks && zoom > 2.0 {
                        Text::new(title)
                            .x_y_position_relative_to(
                                state.ids.map_layers[0],
                                position::Relative::Scalar(rpos.x as f64),
                                position::Relative::Scalar(rpos.y as f64 + zoom * 4.0),
                            )
                            .font_size(self.fonts.cyri.scale((zoom * 3.0) as u32))
                            .font_id(self.fonts.cyri.conrod_id)
                            .graphics_for(state.ids.map_layers[0])
                            .color(TEXT_BG.alpha(fade))
                            .set(state.ids.mmap_poi_title_bgs[i], ui);
                        Text::new(title)
                                .bottom_left_with_margins_on(state.ids.mmap_poi_title_bgs[i], 1.0, 1.0)
                                .font_size(self.fonts.cyri.scale((zoom * 3.0) as u32))
                                .font_id(self.fonts.cyri.conrod_id)
                                //.graphics_for(state.ids.map_layers[0])
                                .color(TEXT_COLOR.alpha(fade))
                                .set(state.ids.mmap_poi_titles[i], ui);

                        handle_widget_mouse_events(
                            state.ids.mmap_poi_titles[i],
                            MarkerChange::Pos(poi.wpos.map(|e| e as f32)),
                            ui,
                            &mut events,
                            state.ids.map_layers[0],
                        );

                        // Show peak altitude
                        if ui
                            .widget_input(state.ids.mmap_poi_titles[i])
                            .mouse()
                            .map_or(false, |m| m.is_over())
                        {
                            Text::new(&height)
                                .mid_bottom_with_margin_on(
                                    state.ids.mmap_poi_title_bgs[i],
                                    zoom * 3.5,
                                )
                                .font_size(self.fonts.cyri.scale((zoom * 3.0) as u32))
                                .font_id(self.fonts.cyri.conrod_id)
                                .graphics_for(state.ids.map_layers[0])
                                .color(TEXT_BG.alpha(fade))
                                .set(state.ids.peaks_txt_bg, ui);
                            Text::new(&height)
                                .bottom_left_with_margins_on(state.ids.peaks_txt_bg, 1.0, 1.0)
                                .font_size(self.fonts.cyri.scale((zoom * 3.0) as u32))
                                .font_id(self.fonts.cyri.conrod_id)
                                .graphics_for(state.ids.map_layers[0])
                                .color(TEXT_COLOR.alpha(fade))
                                .set(state.ids.peaks_txt, ui);
                        }
                    }
                },
                PoiKind::Lake(size) => {
                    if show_biomes && zoom > 2.0 && zoom.powi(2) * size as f64 > 30.0 {
                        let font_scale_factor = if size > 20 {
                            size as f64 / 25.0
                        } else if size > 10 {
                            size as f64 / 10.0
                        } else if size > 5 {
                            size as f64 / 6.0
                        } else {
                            size as f64 / 2.5
                        };
                        Text::new(title)
                            .x_y_position_relative_to(
                                state.ids.map_layers[0],
                                position::Relative::Scalar(rpos.x as f64),
                                position::Relative::Scalar(rpos.y as f64),
                            )
                            .font_size(
                                self.fonts.cyri.scale(
                                    (2.0 + font_scale_factor * zoom).clamp(10.0, 18.0) as u32,
                                ),
                            )
                            .font_id(self.fonts.cyri.conrod_id)
                            .graphics_for(state.ids.map_layers[0])
                            .color(TEXT_BLUE_COLOR.alpha(fade))
                            .set(state.ids.mmap_poi_icons[i], ui);
                    }
                },
            }
        }
        // Group member indicators
        let client_state = self.client.state();
        let stats = client_state.ecs().read_storage::<comp::Stats>();
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
            let stats = entity.and_then(|entity| stats.get(entity));
            let name = if let Some(stats) = stats {
                stats.name.to_string()
            } else {
                "".to_string()
            };

            if let Some(member_pos) = member_pos {
                let factor = 1.2;
                let side_length = 20.0 * factor;

                let (rpos, fade) = match wpos_to_rpos_fade(
                    member_pos.0.xy(),
                    Vec2::from(side_length / 2.0),
                    side_length / 2.0,
                ) {
                    Some(x) => x,
                    None => continue,
                };

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
                .w_h(side_length as f64, side_length as f64)
                .image_color(Color::Rgba(1.0, 1.0, 1.0, fade))
                .floating(true)
                .with_tooltip(self.tooltip_manager, &name, "", &site_tooltip, TEXT_COLOR)
                .set(state.ids.member_indicators[i], ui);

                handle_widget_mouse_events(
                    state.ids.member_indicators[i],
                    MarkerChange::Pos(member_pos.0.xy()),
                    ui,
                    &mut events,
                    state.ids.map_layers[0],
                );
            }
        }

        let factor = 1.4;
        let side_length = 20.0 * factor;
        // Groups location markers
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
            if let Some((rpos, fade)) =
                wpos_to_rpos_fade(lm, Vec2::from(side_length / 2.0), side_length / 2.0)
            {
                let name = self
                    .client
                    .player_list()
                    .get(&uid)
                    .map(|info| info.player_alias.as_str())
                    .or_else(|| {
                        uid_allocator
                            .retrieve_entity_internal(uid.into())
                            .and_then(|entity| stats.get(entity))
                            .map(|stats| stats.name.as_str())
                    })
                    .unwrap_or("");

                let image_id = match self.client.group_info().map(|info| info.1) {
                    Some(leader) if leader == uid => self.imgs.location_marker_group_leader,
                    _ => self.imgs.location_marker_group,
                };

                Button::image(image_id)
                    .x_y_position_relative_to(
                        state.ids.map_layers[0],
                        position::Relative::Scalar(rpos.x as f64),
                        position::Relative::Scalar(rpos.y as f64 + 10.0 * factor as f64),
                    )
                    .w_h(side_length as f64, side_length as f64)
                    .image_color(Color::Rgba(1.0, 1.0, 1.0, fade))
                    .floating(true)
                    .with_tooltip(
                        self.tooltip_manager,
                        &i18n.get_msg("hud-map-marked_location"),
                        &format!(
                            "X: {}, Y: {}\n\n{}",
                            lm.x as i32,
                            lm.y as i32,
                            i18n.get_msg_ctx("hud-map-placed_by", &i18n::fluent_args! {
                                "name" => name
                            }),
                        ),
                        &site_tooltip,
                        TEXT_VELORITE,
                    )
                    .set(state.ids.location_marker_group[i], ui);
                handle_widget_mouse_events(
                    state.ids.location_marker_group[i],
                    MarkerChange::Pos(lm),
                    ui,
                    &mut events,
                    state.ids.map_layers[0],
                );
            }
        }
        // Location marker
        if let Some((lm, (rpos, fade))) = self.location_markers.owned.and_then(|lm| {
            let lm = lm.as_();
            Some(lm).zip(wpos_to_rpos_fade(
                lm,
                Vec2::from(side_length / 2.0),
                side_length / 2.0,
            ))
        }) {
            if Button::image(self.imgs.location_marker)
                .x_y_position_relative_to(
                    state.ids.map_layers[0],
                    position::Relative::Scalar(rpos.x as f64),
                    position::Relative::Scalar(rpos.y as f64 + 10.0 * factor as f64),
                )
                .w_h(side_length as f64, side_length as f64)
                .image_color(Color::Rgba(1.0, 1.0, 1.0, fade))
                .floating(true)
                .with_tooltip(
                    self.tooltip_manager,
                    &i18n.get_msg("hud-map-marked_location"),
                    &format!(
                        "X: {}, Y: {}\n\n{}",
                        lm.x as i32,
                        lm.y as i32,
                        i18n.get_msg("hud-map-marked_location_remove")
                    ),
                    &site_tooltip,
                    TEXT_VELORITE,
                )
                .set(state.ids.location_marker, ui)
                .was_clicked()
            {
                events.push(Event::RemoveMarker);
            }

            handle_widget_mouse_events(
                state.ids.location_marker,
                MarkerChange::Remove,
                ui,
                &mut events,
                state.ids.map_layers[0],
            );
        }

        // Cursor pos relative to playerpos and widget size
        // Cursor stops moving on an axis as soon as it's position exceeds the maximum
        // // size of the widget

        // Don't show if outside or near the edge of the map
        let arrow_sz = {
            let scale = 0.5;
            Vec2::new(36.0, 37.0) * scale
        };
        // Hide if icon could go off of the edge of the map
        if let Some((rpos, fade)) =
            wpos_to_rpos_fade(player_pos.xy(), arrow_sz, arrow_sz.reduce_partial_min())
        {
            Image::new(self.rot_imgs.indicator_mmap_small.target_north)
                .x_y_position_relative_to(
                    state.ids.map_layers[0],
                    position::Relative::Scalar(rpos.x as f64),
                    position::Relative::Scalar(rpos.y as f64),
                )
                .w_h(arrow_sz.x as f64, arrow_sz.y as f64)
                .color(Some(UI_HIGHLIGHT_0.alpha(fade)))
                .set(state.ids.indicator, ui);

            handle_widget_mouse_events(
                state.ids.indicator,
                MarkerChange::Pos(player_pos.xy()),
                ui,
                &mut events,
                state.ids.map_layers[0],
            );
        }

        // Info about controls
        let icon_size = Vec2::new(25.6, 28.8);
        let recenter: bool = drag.x != 0.0 || drag.y != 0.0;
        if Button::image(self.imgs.button)
            .w_h(92.0, icon_size.y)
            .mid_bottom_with_margin_on(state.ids.map_layers[0], -36.0)
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
            .label(&i18n.get_msg("hud-map-recenter"))
            .label_y(position::Relative::Scalar(1.0))
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
            .bottom_left_with_margins_on(state.ids.map_layers[0], -36.0, 0.0)
            .w_h(icon_size.x, icon_size.y)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.drag_ico, ui);
        Text::new(&i18n.get_msg("hud-map-drag"))
            .right_from(state.ids.drag_ico, 5.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.map_layers[0])
            .color(TEXT_COLOR)
            .set(state.ids.drag_txt, ui);
        Image::new(self.imgs.m_scroll_ico)
            .right_from(state.ids.drag_txt, 5.0)
            .w_h(icon_size.x, icon_size.y)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.zoom_ico, ui);
        Text::new(&i18n.get_msg("hud-map-zoom"))
            .right_from(state.ids.zoom_ico, 5.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.map_layers[0])
            .color(TEXT_COLOR)
            .set(state.ids.zoom_txt, ui);

        Text::new(&location_marker_binding.display_shortest(key_layout))
            .right_from(state.ids.zoom_txt, 15.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.map_layers[0])
            .color(TEXT_COLOR)
            .set(state.ids.waypoint_binding_txt, ui);

        Text::new(&i18n.get_msg("hud-map-mid_click"))
            .right_from(state.ids.waypoint_binding_txt, 5.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.map_layers[0])
            .color(TEXT_COLOR)
            .set(state.ids.waypoint_txt, ui);

        // Show topographic map
        if Button::image(self.imgs.button)
            .w_h(92.0, icon_size.y)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .bottom_right_with_margins_on(state.ids.map_layers[0], -36.0, 0.0)
            .with_tooltip(
                self.tooltip_manager,
                &i18n.get_msg("hud-map-change_map_mode"),
                "",
                &site_tooltip,
                TEXT_COLOR,
            )
            .set(state.ids.map_mode_btn, ui)
            .was_clicked()
        {
            events.push(Event::SettingsChange(MapShowTopoMap(!show_topo_map)));
        };
        Button::image(self.imgs.map_mode_overlay)
            .w_h(92.0, icon_size.y)
            .graphics_for(state.ids.map_mode_btn)
            .middle_of(state.ids.map_mode_btn)
            .set(state.ids.map_mode_overlay, ui);

        // Render voxel view on minimap
        if Button::image(self.imgs.button)
            .w_h(92.0, icon_size.y)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .left_from(state.ids.map_mode_btn, 5.0)
            .with_tooltip(
                self.tooltip_manager,
                &i18n.get_msg("hud-map-toggle_minimap_voxel"),
                &i18n.get_msg("hud-map-zoom_minimap_explanation"),
                &site_tooltip,
                TEXT_COLOR,
            )
            .set(state.ids.minimap_mode_btn, ui)
            .was_clicked()
        {
            events.push(Event::SettingsChange(MapShowVoxelMap(!show_voxel_map)));
        };
        Button::image(self.imgs.minimap_mode_overlay)
            .w_h(92.0, icon_size.y)
            .graphics_for(state.ids.minimap_mode_btn)
            .middle_of(state.ids.minimap_mode_btn)
            .set(state.ids.minimap_mode_overlay, ui);

        events
    }
}
