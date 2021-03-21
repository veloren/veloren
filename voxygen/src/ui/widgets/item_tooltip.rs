use super::image_frame::ImageFrame;
use crate::hud::{
    get_quality_col,
    img_ids::Imgs,
    item_imgs::{animate_by_pulse, ItemImgs},
    util,
};
use client::Client;
use common::comp::item::{
    armor::Protection, Item, ItemDesc, ItemKind, MaterialStatManifest, Quality,
};
use conrod_core::{
    builder_method, builder_methods, image, input::global::Global, position::Dimension, text,
    widget, widget_ids, Color, Colorable, FontSize, Positionable, Sizeable, Ui, UiCell, Widget,
    WidgetCommon, WidgetStyle,
};
use std::time::{Duration, Instant};

#[derive(Copy, Clone)]
struct Hover(widget::Id, [f64; 2]);
#[derive(Copy, Clone)]
enum HoverState {
    Hovering(Hover),
    Fading(Instant, Hover, Option<(Instant, widget::Id)>),
    Start(Instant, widget::Id),
    None,
}

// Spacing between the tooltip and mouse
const MOUSE_PAD_Y: f64 = 15.0;
const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0); // Default text color

pub struct ItemTooltipManager {
    tooltip_id: widget::Id,
    state: HoverState,
    // How long before a tooltip is displayed when hovering
    hover_dur: Duration,
    // How long it takes a tooltip to disappear
    fade_dur: Duration,
    // Current scaling of the ui
    logical_scale_factor: f64,
}
impl ItemTooltipManager {
    pub fn new(
        mut generator: widget::id::Generator,
        hover_dur: Duration,
        fade_dur: Duration,
        logical_scale_factor: f64,
    ) -> Self {
        Self {
            tooltip_id: generator.next(),
            state: HoverState::None,
            hover_dur,
            fade_dur,
            logical_scale_factor,
        }
    }

    pub fn maintain(&mut self, input: &Global, logical_scale_factor: f64) {
        self.logical_scale_factor = logical_scale_factor;

        let current = &input.current;

        if let Some(um_id) = current.widget_under_mouse {
            match self.state {
                HoverState::Hovering(hover) if um_id == hover.0 => (),
                HoverState::Hovering(hover) => {
                    self.state =
                        HoverState::Fading(Instant::now(), hover, Some((Instant::now(), um_id)))
                },
                HoverState::Fading(_, _, Some((_, id))) if um_id == id => {},
                HoverState::Fading(start, hover, _) => {
                    self.state = HoverState::Fading(start, hover, Some((Instant::now(), um_id)))
                },
                HoverState::Start(_, id) if um_id == id => (),
                HoverState::Start(_, _) | HoverState::None => {
                    self.state = HoverState::Start(Instant::now(), um_id)
                },
            }
        } else {
            match self.state {
                HoverState::Hovering(hover) => {
                    self.state = HoverState::Fading(Instant::now(), hover, None)
                },
                HoverState::Fading(start, hover, Some((_, _))) => {
                    self.state = HoverState::Fading(start, hover, None)
                },
                HoverState::Start(_, _) => self.state = HoverState::None,
                HoverState::Fading(_, _, None) | HoverState::None => (),
            }
        }

        // Handle fade timing
        if let HoverState::Fading(start, _, maybe_hover) = self.state {
            if start.elapsed() > self.fade_dur {
                self.state = match maybe_hover {
                    Some((start, hover)) => HoverState::Start(start, hover),
                    None => HoverState::None,
                };
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    fn set_tooltip(
        &mut self,
        tooltip: &ItemTooltip,
        item: Option<Item>,
        title_col: Color,
        img_id: Option<image::Id>,
        image_dims: Option<(f64, f64)>,
        src_id: widget::Id,
        ui: &mut UiCell,
    ) {
        let tooltip_id = self.tooltip_id;
        let mp_h = MOUSE_PAD_Y / self.logical_scale_factor;

        let tooltip = |transparency, mouse_pos: [f64; 2], ui: &mut UiCell| {
            // Fill in text and the potential image beforehand to get an accurate size for
            // spacing
            let tooltip = tooltip
                .clone()
                .item(item)
                .title_col(title_col)
                .image(img_id)
                .image_dims(image_dims);

            let [t_w, t_h] = tooltip.get_wh(ui).unwrap_or([0.0, 0.0]);
            let [m_x, m_y] = [mouse_pos[0], mouse_pos[1]];
            let (w_w, w_h) = (ui.win_w, ui.win_h);

            // Determine position based on size and mouse position
            // Flow to the top left of the mouse when there is space
            let x = if (m_x + w_w / 2.0) > t_w {
                m_x - t_w / 2.0
            } else {
                m_x + t_w / 2.0
            };
            let y = if w_h - (m_y + w_h / 2.0) > t_h + mp_h {
                m_y + mp_h + t_h / 2.0
            } else {
                m_y - mp_h - t_h / 2.0
            };
            tooltip
                .floating(true)
                .transparency(transparency)
                .x_y(x, y)
                .set(tooltip_id, ui);
        };

        match self.state {
            HoverState::Hovering(Hover(id, xy)) if id == src_id => tooltip(1.0, xy, ui),
            HoverState::Fading(start, Hover(id, xy), _) if id == src_id => tooltip(
                (0.1f32 - start.elapsed().as_millis() as f32 / self.hover_dur.as_millis() as f32)
                    .max(0.0),
                xy,
                ui,
            ),
            HoverState::Start(start, id) if id == src_id && start.elapsed() > self.hover_dur => {
                let xy = ui.global_input().current.mouse.xy;
                self.state = HoverState::Hovering(Hover(id, xy));
                tooltip(1.0, xy, ui);
            },
            _ => (),
        }
    }
}

pub struct ItemTooltipped<'a, W> {
    inner: W,
    tooltip_manager: &'a mut ItemTooltipManager,
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    pulse: f32,
    item: Option<Item>,
    msm: &'a MaterialStatManifest,
    img_id: Option<image::Id>,
    image_dims: Option<(f64, f64)>,
    tooltip: &'a ItemTooltip<'a>,
    title_col: Color,
}
impl<'a, W: Widget> ItemTooltipped<'a, W> {
    pub fn tooltip_image(mut self, img_id: image::Id) -> Self {
        self.img_id = Some(img_id);
        self
    }

    pub fn tooltip_image_dims(mut self, dims: (f64, f64)) -> Self {
        self.image_dims = Some(dims);
        self
    }

    pub fn set(self, id: widget::Id, ui: &mut UiCell) -> W::Event {
        let event = self.inner.set(id, ui);
        self.tooltip_manager.set_tooltip(
            self.tooltip,
            self.item,
            self.title_col,
            self.img_id,
            self.image_dims,
            id,
            ui,
        );
        event
    }
}

pub trait ItemTooltipable {
    // If `Tooltip` is expensive to construct accept a closure here instead.
    fn with_item_tooltip<'a>(
        self,
        tooltip_manager: &'a mut ItemTooltipManager,
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        pulse: f32,
        item: Option<Item>,
        msm: &'a MaterialStatManifest,
        tooltip: &'a ItemTooltip<'a>,
        title_col: Color,
    ) -> ItemTooltipped<'a, Self>
    where
        Self: std::marker::Sized;
}
impl<W: Widget> ItemTooltipable for W {
    fn with_item_tooltip<'a>(
        self,
        tooltip_manager: &'a mut ItemTooltipManager,
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        pulse: f32,
        item: Option<Item>,
        msm: &'a MaterialStatManifest,
        tooltip: &'a ItemTooltip<'a>,
        title_col: Color,
    ) -> ItemTooltipped<'a, W> {
        ItemTooltipped {
            inner: self,
            tooltip_manager,
            client,
            imgs,
            item_imgs,
            pulse,
            item,
            msm,
            img_id: None,
            image_dims: None,
            tooltip,
            title_col,
        }
    }
}

/// Vertical spacing between elements of the tooltip
const V_PAD: f64 = 10.0;
/// Horizontal spacing between elements of the tooltip
const H_PAD: f64 = 10.0;
/// Vertical spacing between stats
const V_PAD_STATS: f64 = 6.0;
/// Default portion of inner width that goes to an image
const IMAGE_W_FRAC: f64 = 0.3;
/// Default width multiplied by the description font size
const DEFAULT_CHAR_W: f64 = 20.0;
/// Text vertical spacing factor to account for overhanging text
const TEXT_SPACE_FACTOR: f64 = 0.35;
// Item icon size
const ICON_SIZE: [f64; 2] = [64.0, 64.0];

/// A widget for displaying tooltips
#[derive(Clone, WidgetCommon)]
pub struct ItemTooltip<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    item: Option<Item>,
    msm: &'a MaterialStatManifest,
    title_col: Color,
    image: Option<image::Id>,
    image_dims: Option<(f64, f64)>,
    style: Style,
    transparency: f32,
    image_frame: ImageFrame,
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    pulse: f32,
}

#[derive(Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {
    #[conrod(default = "Color::Rgba(1.0, 1.0, 1.0, 1.0)")]
    pub color: Option<Color>,
    title: widget::text::Style,
    desc: widget::text::Style,
    // add background imgs here
}

widget_ids! {
    struct Ids {
        title,
        subtitle,
        desc,
        main_stat,
        main_stat_text,
        stats[],
        diff_main_stat,
        diffs[],
        item_frame,
        item_render,
        image_frame,
        image,
        background,
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> ItemTooltip<'a> {
    builder_methods! {
        pub desc_text_color { style.desc.color = Some(Color) }
        pub title_font_size { style.title.font_size = Some(FontSize) }
        pub desc_font_size { style.desc.font_size = Some(FontSize) }
        pub title_justify { style.title.justify = Some(text::Justify) }
        pub desc_justify { style.desc.justify = Some(text::Justify) }
        image { image = Option<image::Id> }
        item { item = Option<Item> }
        msm { msm = &'a MaterialStatManifest }
        image_dims { image_dims = Option<(f64, f64)> }
        transparency { transparency = f32 }
        title_col { title_col = Color}
    }

    pub fn new(
        image_frame: ImageFrame,
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        pulse: f32,
        msm: &'a MaterialStatManifest,
    ) -> Self {
        ItemTooltip {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            item: None,
            msm,
            transparency: 1.0,
            image_frame,
            image: None,
            image_dims: None,
            title_col: TEXT_COLOR,
            client: &client,
            imgs: &imgs,
            item_imgs: &item_imgs,
            pulse,
        }
    }

    /// Align the text to the left of its bounding **Rect**'s *x* axis range.
    //pub fn left_justify(self) -> Self {
    //    self.justify(text::Justify::Left)
    //}

    /// Align the text to the middle of its bounding **Rect**'s *x* axis range.
    //pub fn center_justify(self) -> Self {
    //    self.justify(text::Justify::Center)
    //}

    /// Align the text to the right of its bounding **Rect**'s *x* axis range.
    //pub fn right_justify(self) -> Self {
    //    self.justify(text::Justify::Right)
    //}

    fn text_image_width(&self, total_width: f64) -> (f64, f64) {
        let inner_width = (total_width - H_PAD * 2.0).max(0.0);
        // Image defaults to 30% of the width
        let image_w = if self.image.is_some() {
            match self.image_dims {
                Some((w, _)) => w,
                None => (inner_width - H_PAD).max(0.0) * IMAGE_W_FRAC,
            }
        } else {
            0.0
        };
        // Text gets the remaining width
        let text_w = (inner_width
            - if self.image.is_some() {
                image_w + H_PAD
            } else {
                0.0
            })
        .max(0.0);

        (text_w, image_w)
    }

    /// Specify the font used for displaying the text.
    pub fn font_id(mut self, font_id: text::font::Id) -> Self {
        self.style.title.font_id = Some(Some(font_id));
        self.style.desc.font_id = Some(Some(font_id));
        self
    }
}

impl<'a> Widget for ItemTooltip<'a> {
    type Event = ();
    type State = State;
    type Style = Style;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style { self.style.clone() }

    fn update(self, args: widget::UpdateArgs<Self>) {
        let widget::UpdateArgs {
            id,
            state,
            rect,
            style,
            ui,
            ..
        } = args;

        fn stats_count(item: &Item) -> usize {
            let mut count = match item.kind() {
                ItemKind::Armor(_) => 1,
                ItemKind::Tool(_) => 5,
                ItemKind::Consumable { .. } => 1,
                _ => 0,
            };
            if item.num_slots() != 0 {
                count += 1
            }
            count as usize
        }

        if let Some(ref item) = self.item {
            let inventories = self.client.inventories();
            let inventory = match inventories.get(self.client.entity()) {
                Some(l) => l,
                None => return,
            };

            let quality = get_quality_col(item);

            let equip_slot = inventory.equipped_items_of_kind(item.kind().clone());

            let (title, desc) = (item.name().to_string(), item.description().to_string());

            let subtitle = util::kind_text(item.kind());

            let text_color = conrod_core::color::WHITE;

            // Widths
            let (text_w, image_w) = self.text_image_width(rect.w());

            // Apply transparency
            let color = style.color(ui.theme()).alpha(self.transparency);

            // Color quality
            let quality_col_img = match &item.quality() {
                Quality::Low => self.imgs.inv_slot_grey,
                Quality::Common => self.imgs.inv_slot,
                Quality::Moderate => self.imgs.inv_slot_green,
                Quality::High => self.imgs.inv_slot_blue,
                Quality::Epic => self.imgs.inv_slot_purple,
                Quality::Legendary => self.imgs.inv_slot_gold,
                Quality::Artifact => self.imgs.inv_slot_orange,
                _ => self.imgs.inv_slot_red,
            };

            // Spacing for overhanging text
            let title_space = self.style.title.font_size(&ui.theme) as f64 * TEXT_SPACE_FACTOR;

            state.update(|s| {
                s.ids
                    .stats
                    .resize(stats_count(item), &mut ui.widget_id_generator())
            });

            state.update(|s| {
                s.ids
                    .diffs
                    .resize(stats_count(item), &mut ui.widget_id_generator())
            });

            // Background image frame
            self.image_frame
                .wh(rect.dim())
                .xy(rect.xy())
                .graphics_for(id)
                .parent(id)
                .color(quality)
                .set(state.ids.image_frame, ui);

            // Image
            if let Some(img_id) = self.image {
                widget::Image::new(img_id)
                    .w_h(image_w, self.image_dims.map_or(image_w, |(_, h)| h))
                    .graphics_for(id)
                    .parent(id)
                    .color(Some(quality))
                    .top_left_with_margins_on(state.ids.image_frame, V_PAD, H_PAD)
                    .set(state.ids.image, ui);
            }

            // Title
            widget::Text::new(&title)
                .w(text_w)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.title)
                .top_left_with_margins_on(state.ids.image_frame, V_PAD, H_PAD)
                .center_justify()
                .color(quality)
                .set(state.ids.title, ui);

            // Item frame
            widget::Image::new(quality_col_img)
                .wh(ICON_SIZE)
                .graphics_for(id)
                .parent(id)
                .top_left_with_margins_on(state.ids.image_frame, V_PAD, H_PAD)
                .down_from(state.ids.title, V_PAD)
                .set(state.ids.item_frame, ui);

            // Item render
            widget::Image::new(animate_by_pulse(
                &self.item_imgs.img_ids_or_not_found_img(item.into()),
                self.pulse,
            ))
            .color(Some(conrod_core::color::WHITE))
            .w_h(ICON_SIZE[0] * 0.8, ICON_SIZE[1] * 0.8)
            .middle_of(state.ids.item_frame)
            .set(state.ids.item_render, ui);

            // Subtitle
            widget::Text::new(&subtitle)
                .w(text_w)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.desc)
                .color(conrod_core::color::GREY)
                .right_from(state.ids.item_frame, H_PAD)
                .set(state.ids.subtitle, ui);

            // Stats
            match item.kind() {
                ItemKind::Tool(tool) => {
                    let power = tool.base_power(self.msm, item.components()) * 10.0;
                    let speed = tool.base_speed(self.msm, item.components());
                    let poise_str = tool.base_poise_strength(self.msm, item.components()) * 10.0;
                    let crit_chance = tool.base_crit_chance(self.msm, item.components()) * 100.0;
                    let crit_mult = tool.base_crit_mult(self.msm, item.components());
                    let dps = power * speed;

                    // DPS
                    widget::Text::new(&format!("{:.1}", dps))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .font_size(35)
                        .align_middle_y_of(state.ids.item_frame)
                        .right_from(state.ids.item_frame, V_PAD)
                        .set(state.ids.main_stat, ui);

                    widget::Text::new(&"DPS".to_string())
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .align_bottom_of(state.ids.main_stat)
                        .right_from(state.ids.main_stat, V_PAD)
                        .set(state.ids.main_stat_text, ui);

                    // Power
                    widget::Text::new(&format!("- Power : {:.1}", power))
                        .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.item_frame, V_PAD_STATS )
                        .set(state.ids.stats[0], ui);

                    // Speed
                    widget::Text::new(&format!("- Speed : {:.1}", speed))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[0], V_PAD_STATS )
                        .set(state.ids.stats[1], ui);

                    // Poise
                    widget::Text::new(&format!("- Poise : {:.1}", poise_str))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[1], V_PAD_STATS )
                        .set(state.ids.stats[2], ui);

                    // Crit chance
                    widget::Text::new(&format!("- Crit Chance : {:.1}%", crit_chance))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[2], V_PAD_STATS )
                        .set(state.ids.stats[3], ui);

                    // Crit mult
                    widget::Text::new(&format!("- Crit Mult : x{:.1}", crit_mult))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[3], V_PAD_STATS )
                        .set(state.ids.stats[4], ui);
                    if let Some(equipped_item) = equip_slot.cloned().next() {
                        if let ItemKind::Tool(equipped_tool) = equipped_item.kind() {
                            let tool_stats = tool
                                .stats
                                .resolve_stats(self.msm, item.components())
                                .clamp_speed();
                            let equipped_tool_stats = equipped_tool
                                .stats
                                .resolve_stats(self.msm, equipped_item.components())
                                .clamp_speed();
                            let diff = tool_stats - equipped_tool_stats;
                            let power_diff =
                                util::comparison(tool_stats.power, equipped_tool_stats.power);
                            let speed_diff =
                                util::comparison(tool_stats.speed, equipped_tool_stats.speed);
                            let poise_strength_diff = util::comparison(
                                tool_stats.poise_strength,
                                equipped_tool_stats.poise_strength,
                            );
                            let crit_chance_diff = util::comparison(
                                tool_stats.crit_chance,
                                equipped_tool_stats.crit_chance,
                            );
                            let crit_mult_diff = util::comparison(
                                tool_stats.crit_mult,
                                equipped_tool_stats.crit_mult,
                            );
                            let equipped_dps =
                                equipped_tool_stats.power * equipped_tool_stats.speed;
                            let diff_main_stat = util::comparison(dps, equipped_dps);

                            if equipped_dps - dps != 0.0 {
                                widget::Text::new(&diff_main_stat.0)
                                    .right_from(state.ids.main_stat_text, 5.0)
                                    .graphics_for(id)
                                    .parent(id)
                                    .with_style(self.style.desc)
                                    .color(diff_main_stat.1)
                                    .set(state.ids.diff_main_stat, ui);
                            }

                            if diff.power != 0.0 {
                                widget::Text::new(&format!(
                                    "{} {:.1}",
                                    &power_diff.0,
                                    &diff.power * 10.0
                                ))
                                .align_middle_y_of(state.ids.stats[0])
                                .right_from(state.ids.stats[0], 10.0)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(self.style.desc)
                                .color(power_diff.1)
                                .set(state.ids.diffs[0], ui);
                            }
                            if diff.speed != 0.0 {
                                widget::Text::new(&format!("{} {:.1}", &speed_diff.0, &diff.speed))
                                    .align_middle_y_of(state.ids.stats[1])
                                    .right_from(state.ids.stats[1], 10.0)
                                    .graphics_for(id)
                                    .parent(id)
                                    .with_style(self.style.desc)
                                    .color(speed_diff.1)
                                    .set(state.ids.diffs[1], ui);
                            }
                            if diff.poise_strength != 0.0 {
                                widget::Text::new(&format!(
                                    "{} {:.1}",
                                    &poise_strength_diff.0,
                                    &diff.poise_strength * 10.0
                                ))
                                .align_middle_y_of(state.ids.stats[2])
                                .right_from(state.ids.stats[2], 10.0)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(self.style.desc)
                                .color(poise_strength_diff.1)
                                .set(state.ids.diffs[2], ui);
                            }
                            if diff.crit_chance != 0.0 {
                                widget::Text::new(&format!(
                                    "{} {:.1}%",
                                    &crit_chance_diff.0,
                                    &diff.crit_chance * 100.0
                                ))
                                .align_middle_y_of(state.ids.stats[3])
                                .right_from(state.ids.stats[3], 10.0)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(self.style.desc)
                                .color(crit_chance_diff.1)
                                .set(state.ids.diffs[3], ui);
                            }
                            if diff.crit_mult != 0.0 {
                                widget::Text::new(&format!(
                                    "{} {:.1}",
                                    &crit_mult_diff.0, &diff.crit_mult
                                ))
                                .align_middle_y_of(state.ids.stats[4])
                                .right_from(state.ids.stats[4], 10.0)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(self.style.desc)
                                .color(crit_mult_diff.1)
                                .set(state.ids.diffs[4], ui);
                            }
                        }
                    }
                },
                ItemKind::Armor(armor) => {
                    let protection = armor.get_protection();
                    let poise_res = armor.get_poise_resilience();

                    /*// Armour
                    widget::Text::new(&format!("- Armour : {}", util::protec2string(protection)))
                        .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.item_frame, V_PAD)
                        .set(state.ids.stat1, ui);*/

                    // Armour
                    widget::Text::new(&util::protec2string(protection))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .font_size(35)
                        .align_middle_y_of(state.ids.item_frame)
                        .right_from(state.ids.item_frame, V_PAD)
                        .set(state.ids.main_stat, ui);

                    widget::Text::new(&"Armor".to_string())
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .align_bottom_of(state.ids.main_stat)
                        .right_from(state.ids.main_stat, V_PAD)
                        .set(state.ids.main_stat_text, ui);

                    // Poise res
                    widget::Text::new(&format!("- Poise res : {}", util::protec2string(poise_res)))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                        .set(state.ids.stats[0], ui);

                    // Slots
                    if item.num_slots() > 0 {
                        widget::Text::new(&format!("- Slots : {}", item.num_slots()))
                            .graphics_for(id)
                            .parent(id)
                            .with_style(self.style.desc)
                            .color(text_color)
                            .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                            .down_from(state.ids.stats[0], V_PAD_STATS )
                            .set(state.ids.stats[1], ui);
                    }

                    if let Some(equipped_item) = equip_slot.cloned().next() {
                        if let ItemKind::Armor(equipped_armor) = equipped_item.kind() {
                            let diff = armor.stats - equipped_armor.stats;
                            let protection_diff = util::comparison(
                                &armor.stats.protection,
                                &equipped_armor.stats.protection,
                            );
                            let poise_res_diff = util::comparison(
                                &armor.stats.poise_resilience,
                                &equipped_armor.stats.poise_resilience,
                            );

                            if diff.protection != Protection::Normal(0.0) {
                                widget::Text::new(&protection_diff.0)
                                    .right_from(state.ids.main_stat_text, 5.0)
                                    .graphics_for(id)
                                    .parent(id)
                                    .with_style(self.style.desc)
                                    .color(protection_diff.1)
                                    .set(state.ids.diff_main_stat, ui);
                            }

                            /*if diff.protection != Protection::Normal(0.0) {
                                widget::Text::new(&format!(
                                    "{} {}",
                                    &diff1.0,
                                    util::protec2string(diff.protection)
                                ))
                                .align_middle_y_of(state.ids.stat1)
                                .right_from(state.ids.stat1, 10.0)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(self.style.desc)
                                .color(diff1.1)
                                .set(state.ids.diff1, ui);
                            }*/

                            if diff.poise_resilience != Protection::Normal(0.0) {
                                widget::Text::new(&format!(
                                    "{} {}",
                                    &poise_res_diff.0,
                                    util::protec2string(diff.poise_resilience)
                                ))
                                .align_middle_y_of(state.ids.stats[0])
                                .right_from(state.ids.stats[0], 10.0)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(self.style.desc)
                                .color(poise_res_diff.1)
                                .set(state.ids.diffs[0], ui);
                            }
                        }
                    }
                },
                ItemKind::Consumable { effect, .. } => {
                    widget::Text::new(&util::consumable_desc(effect))
                        .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.item_frame, V_PAD)
                        .set(state.ids.stats[0], ui);
                },
                _ => (),
            }

            if !desc.is_empty() {
                widget::Text::new(&desc)
                    .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                    .graphics_for(id)
                    .parent(id)
                    .with_style(self.style.desc)
                    .color(conrod_core::color::GREY)
                    .down_from(
                        if stats_count(item) > 0 {
                            state.ids.stats[state.ids.stats.len() - 1]
                        } else {
                            (state.ids.item_frame)
                        },
                        V_PAD,
                    )
                    .w(text_w)
                    .set(state.ids.desc, ui);
            }
        }
    }

    /// Default width is based on the description font size unless the text is
    /// small enough to fit on a single line
    fn default_x_dimension(&self, ui: &Ui) -> Dimension { Dimension::Absolute(280.0) }

    fn default_y_dimension(&self, ui: &Ui) -> Dimension {
        fn stats_count(item: &Item) -> usize {
            let mut count = match item.kind() {
                ItemKind::Armor(_) => 2,
                ItemKind::Tool(_) => 5,
                ItemKind::Consumable { .. } => 1,
                _ => 0,
            };
            if item.num_slots() != 0 {
                count += 1
            }
            count as usize
        }

        if let Some(ref item) = self.item {
            let (title, desc) = (item.name().to_string(), item.description().to_string());

            let (text_w, image_w) = self.text_image_width(280.0);
            // Title
            let title_h = widget::Text::new(&title)
                .w(text_w)
                .with_style(self.style.title)
                .get_h(ui)
                .unwrap_or(0.0);

            // Item frame
            let frame_h = ICON_SIZE[1];

            // Stat
            let stat_h = widget::Text::new(&"placeholder".to_string())
                .with_style(self.style.desc)
                .get_h(ui)
                .unwrap_or(0.0);

            // Description

            let desc_h: f64 = if !desc.is_empty() {
                widget::Text::new(&desc)
                    .with_style(self.style.desc)
                    .color(conrod_core::color::GREY)
                    .w(text_w)
                    .get_h(ui)
                    .unwrap_or(0.0)
            } else {
                0.0
            };

            dbg!(title_h);
            dbg!(frame_h);
            dbg!(stat_h * stats_count(item) as f64);
            dbg!(desc_h);
            let height = title_h
                + frame_h
                + stat_h * stats_count(item) as f64
                + desc_h
                + V_PAD * 5.0 + V_PAD_STATS * stats_count(item) as f64;
            Dimension::Absolute(height)
        } else {
            Dimension::Absolute(10.0)
        }
    }
}

impl<'a> Colorable for ItemTooltip<'a> {
    builder_method!(color { style.color = Some(Color) });
}
