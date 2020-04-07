use super::{
    img_ids::{Imgs, ImgsRot},
    Show, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    i18n::VoxygenLocalization,
    ui::{fonts::ConrodVoxygenFonts, img_ids},
};
use client::{self, Client};
use common::{comp, terrain::TerrainChunkSize, vol::RectVolSize};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
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
    }
}

#[derive(WidgetCommon)]
pub struct Map<'a> {
    _show: &'a Show,
    client: &'a Client,
    world_map: &'a (img_ids::Rotations, Vec2<u32>),
    imgs: &'a Imgs,
    rot_imgs: &'a ImgsRot,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    _pulse: f32,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
}
impl<'a> Map<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        rot_imgs: &'a ImgsRot,
        world_map: &'a (img_ids::Rotations, Vec2<u32>),
        fonts: &'a ConrodVoxygenFonts,
        pulse: f32,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
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
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    Close,
}

impl<'a> Widget for Map<'a> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

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
        Text::new(&self.localized_strings.get("hud.map.map_title"))
            .mid_top_with_margin_on(state.ids.frame, 3.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(29))
            .color(TEXT_COLOR)
            .set(state.ids.map_title, ui);

        // Questlog Title
        Text::new(&format!(
            "{}",
            &self.localized_strings.get("hud.map.qlog_title")
        ))
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
            return Some(Event::Close);
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

        Image::new(world_map.none)
            .mid_top_with_margin_on(state.ids.map_align, 10.0)
            .w_h(760.0, 760.0)
            .parent(state.ids.bg)
            .set(state.ids.grid, ui);
        // Coordinates
        let player_pos = self
            .client
            .state()
            .ecs()
            .read_storage::<comp::Pos>()
            .get(self.client.entity())
            .map_or(Vec3::zero(), |pos| pos.0);

        let x = player_pos.x as f64 / worldsize.x * 760.0;
        let y = player_pos.y as f64 / worldsize.y * 760.0;
        let x_rel = player_pos.x as f64 / worldsize.x;
        let y_rel = player_pos.y as f64 / worldsize.y;
        let indic_scale = 0.6;        
        Image::new(self.rot_imgs.indicator_mmap_small.target_north)
            .bottom_left_with_margins_on(
                state.ids.grid,
                if y_rel > 0.0 &&  y_rel < 1.0 { y - 37.0 * indic_scale / 2.0 } else { 760.0 - 37.0 * indic_scale / 2.0 },
                if x_rel > 0.0 &&  x_rel < 1.0 { x - 32.0 * indic_scale / 2.0 } else { 760.0 - 32.0 * indic_scale / 2.0 },
            )
            .w_h(32.0 * indic_scale, 37.0 * indic_scale)
            .color(Some(UI_HIGHLIGHT_0))
            .floating(true)
            .parent(ui.window)
            .set(state.ids.indicator, ui);

        None
    }
}
