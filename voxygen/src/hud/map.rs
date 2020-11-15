use super::{
    img_ids::{Imgs, ImgsRot},
    Show, TEXT_COLOR, UI_GOLD, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    i18n::Localization,
    ui::{fonts::Fonts, img_ids, ImageFrame, ImageSlider, Tooltip, TooltipManager, Tooltipable},
    GlobalState,
};
use client::{self, Client};
use common::{comp, msg::world_msg::SiteKind, terrain::TerrainChunkSize, vol::RectVolSize};
use conrod_core::{
    color, position,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
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
        grid,
        map_title,
        qlog_title,
        zoom_slider,
        mmap_site_icons[],
    }
}

#[derive(WidgetCommon)]
pub struct Map<'a> {
    _show: &'a Show,
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
        show: &'a Show,
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
            _show: show,
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
        let mut events = Vec::new();
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
            .w_h(1052.0, 886.0)
            .mid_top_with_margin_on(ui.window, 5.0)
            .color(Some(UI_MAIN))
            .set(state.ids.bg, ui);

        Image::new(self.imgs.map_frame)
            .w_h(1052.0, 886.0)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.frame, ui);

        // Map Content Alignment
        Rectangle::fill_with([814.0, 834.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.frame, 46.0, 2.0)
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
        Text::new(self.localized_strings.get("hud.map.map_title"))
            .mid_top_with_margin_on(state.ids.frame, 3.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(29))
            .color(TEXT_COLOR)
            .set(state.ids.map_title, ui);

        // Questlog Title
        Text::new(self.localized_strings.get("hud.map.qlog_title"))
            .mid_top_with_margin_on(state.ids.qlog_align, 6.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(21))
            .color(TEXT_COLOR)
            .set(state.ids.qlog_title, ui);

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
        let worldsize = worldsize.map2(TerrainChunkSize::RECT_SIZE, |e, f| e as f64 * f as f64);

        // Coordinates
        let player_pos = self
            .client
            .state()
            .ecs()
            .read_storage::<comp::Pos>()
            .get(self.client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        let max_zoom = (worldsize / TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
            .reduce_partial_max()/*.min(f64::MAX)*/;

        let map_size = Vec2::new(760.0, 760.0);

        let w_src = max_zoom / zoom;
        let h_src = max_zoom / zoom;
        let rect_src = position::Rect::from_xy_dim(
            [
                player_pos.x as f64 / TerrainChunkSize::RECT_SIZE.x as f64,
                (worldsize.y - player_pos.y as f64) / TerrainChunkSize::RECT_SIZE.y as f64,
            ],
            [w_src, h_src],
        );
        Image::new(world_map.none)
            .mid_top_with_margin_on(state.ids.map_align, 10.0)
            .w_h(map_size.x, map_size.y)
            .parent(state.ids.bg)
            .source_rectangle(rect_src)
            .set(state.ids.grid, ui);

        if let Some(new_val) = ImageSlider::discrete(
            self.global_state.settings.gameplay.map_zoom as i32,
            1,
            30,
            self.imgs.slider_indicator_small,
            self.imgs.slider,
        )
        .w_h(600.0, 22.0 * 2.0)
        .mid_bottom_with_margin_on(state.ids.grid, -55.0)
        .track_breadth(12.0 * 2.0)
        .slider_length(22.0 * 2.0)
        .pad_track((12.0, 12.0))
        .set(state.ids.zoom_slider, ui)
        {
            events.push(Event::MapZoom(new_val as f64));
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
        for (i, site) in self.client.sites().iter().enumerate() {
            let rwpos = site.wpos.map(|e| e as f32) - player_pos;
            let rcpos =
                rwpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e / sz as f32) * zoom as f32 * 3.0
                    / 4.0;
            let rpos =
                Vec2::unit_x().rotated_z(0.0) * rcpos.x + Vec2::unit_y().rotated_z(0.0) * rcpos.y;

            if rpos
                .map2(map_size, |e, sz| e.abs() > sz as f32 / 2.0)
                .reduce_or()
            {
                continue;
            }
            let title = match &site.kind {
                SiteKind::Town => "Town",
                SiteKind::Dungeon => "Dungeon",
                SiteKind::Castle => "Castle",
            };
            let desc = "";
            Button::image(match &site.kind {
                SiteKind::Town => self.imgs.mmap_site_town,
                SiteKind::Dungeon => self.imgs.mmap_site_dungeon,
                SiteKind::Castle => self.imgs.mmap_site_castle,
            })
            .x_y_position_relative_to(
                state.ids.grid,
                position::Relative::Scalar(rpos.x as f64),
                position::Relative::Scalar(rpos.y as f64),
            )
            .w_h(20.0 * 1.2, 20.0 * 1.2)
            .hover_image(match &site.kind {
                SiteKind::Town => self.imgs.mmap_site_town_hover,
                SiteKind::Dungeon => self.imgs.mmap_site_dungeon_hover,
                SiteKind::Castle => self.imgs.mmap_site_castle_hover,
            })            
            .image_color(UI_HIGHLIGHT_0)
            .parent(ui.window)
            .with_tooltip(self.tooltip_manager, title, desc, &site_tooltip, TEXT_COLOR)
            .set(state.ids.mmap_site_icons[i], ui);
        }

        // Cursor pos relative to playerpos and widget size
        // Cursor stops moving on an axis as soon as it's position exceeds the maximum
        // // size of the widget

        /*let rel = Vec2::from(player_pos).map2(worldsize, |e: f32, sz: f64| {
            (e as f64 / sz).clamped(0.0, 1.0)
        });*/
        //let xy = rel * 760.0;
        let scale = 0.6;
        let arrow_sz = Vec2::new(32.0, 37.0) * scale;
        Image::new(self.rot_imgs.indicator_mmap_small.target_north)
            .middle_of(state.ids.grid)
            .w_h(arrow_sz.x, arrow_sz.y)
            .color(Some(UI_HIGHLIGHT_0))
            .floating(true)
            .parent(ui.window)
            .set(state.ids.indicator, ui);

        events
    }
}
