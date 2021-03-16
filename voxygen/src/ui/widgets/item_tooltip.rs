use super::image_frame::ImageFrame;
use crate::hud::{
    get_quality_col,
    img_ids::Imgs,
    item_imgs::{animate_by_pulse, ItemImgs},
    util,
};
use client::Client;
use common::comp::item::{Item, ItemKind, MaterialStatManifest, Quality};
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
        title_text: &str,
        desc_text: &str,
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
                .title(title_text)
                .desc(desc_text)
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
    title_text: &'a str,
    desc_text: &'a str,
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
            self.title_text,
            self.desc_text,
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
        title_text: &'a str,
        desc_text: &'a str,
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
        title_text: &'a str,
        desc_text: &'a str,
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
            title_text,
            desc_text,
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
    title_text: &'a str,
    desc_text: &'a str,
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
        stat1,
        stat2,
        stat3,
        stat4,
        stat5,
        diff1,
        diff2,
        diff3,
        diff4,
        diff5,
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
        title { title_text = &'a str }
        desc { desc_text = &'a str }
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
            title_text: "",
            desc_text: "",
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

        if let Some(ref item) = self.item {
            let item = item;

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

            //let _i18n = &self.localized_strings;

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

            // Icon BG
            widget::Image::new(quality_col_img)
                .wh(ICON_SIZE)
                .graphics_for(id)
                .parent(id)
                .top_left_with_margins_on(state.ids.image_frame, V_PAD, H_PAD)
                .set(state.ids.item_frame, ui);

            // Icon
            widget::Image::new(animate_by_pulse(
                &self.item_imgs.img_ids_or_not_found_img(item.into()),
                self.pulse,
            ))
            .color(Some(conrod_core::color::WHITE))
            .w_h(ICON_SIZE[0] * 0.8, ICON_SIZE[1] * 0.8)
            .middle_of(state.ids.item_frame)
            .set(state.ids.item_render, ui);

            // Title
            let title = widget::Text::new(&title)
                .w(text_w)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.title)
                .color(quality);

            if self.image.is_some() {
                title
                    .right_from(state.ids.image, H_PAD)
                    .align_top_of(state.ids.image)
            } else {
                title.right_from(state.ids.item_frame, 10.0)
            }
            .set(state.ids.title, ui);

            // Subtitle
            widget::Text::new(&subtitle)
                .w(text_w)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.desc)
                .color(conrod_core::color::GREY)
                .down_from(state.ids.title, H_PAD)
                .set(state.ids.subtitle, ui);

            // Stats
            match item.kind() {
                ItemKind::Tool(tool) => {
                    let stat1 = tool.base_power(self.msm, item.components()) * 10.0;
                    let stat2 = tool.base_speed(self.msm, item.components());
                    let stat3 = tool.base_poise_strength(self.msm, item.components()) * 10.0;
                    let stat4 = tool.base_crit_chance(self.msm, item.components()) * 100.0;
                    let stat5 = tool.base_crit_mult(self.msm, item.components());

                    widget::Text::new(&format!("Power : {:.1}", stat1))
                        .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .h(2.0)
                        .down_from(state.ids.item_frame, H_PAD)
                        .set(state.ids.stat1, ui);
                    widget::Text::new(&format!("Speed : {:.1}", stat2))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .h(2.0)
                        .set(state.ids.stat2, ui);
                    widget::Text::new(&format!("Poise : {:.1}", stat3))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .h(2.0)
                        .set(state.ids.stat3, ui);
                    widget::Text::new(&format!("Crit Chance : {:.1}%", stat4))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .h(2.0)
                        .set(state.ids.stat4, ui);
                    widget::Text::new(&format!("Crit Mult : x{:.1}", stat5))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .h(2.0)
                        .set(state.ids.stat5, ui);
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
                            let diff1 =
                                util::comparison(tool_stats.power, equipped_tool_stats.power);
                            let diff2 =
                                util::comparison(tool_stats.speed, equipped_tool_stats.speed);
                            let diff3 = util::comparison(
                                tool_stats.poise_strength,
                                equipped_tool_stats.poise_strength,
                            );
                            let diff4 = util::comparison(
                                tool_stats.crit_chance,
                                equipped_tool_stats.crit_chance,
                            );
                            let diff5 = util::comparison(
                                tool_stats.crit_mult,
                                equipped_tool_stats.crit_mult,
                            );

                            widget::Text::new(&format!("{} {:.1}", &diff1.0, &diff.power * 10.0))
                                .align_middle_y_of(state.ids.stat1)
                                .right_from(state.ids.stat1, 10.0)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(self.style.desc)
                                .color(diff1.1)
                                .h(2.0)
                                .set(state.ids.diff1, ui);
                            widget::Text::new(&format!("{} {:.1}", &diff2.0, &diff.speed))
                                .align_middle_y_of(state.ids.stat2)
                                .right_from(state.ids.stat2, 10.0)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(self.style.desc)
                                .color(diff2.1)
                                .h(2.0)
                                .set(state.ids.diff2, ui);
                            widget::Text::new(&format!(
                                "{} {:.1}",
                                &diff3.0,
                                &diff.poise_strength * 10.0
                            ))
                            .align_middle_y_of(state.ids.stat3)
                            .right_from(state.ids.stat3, 10.0)
                            .graphics_for(id)
                            .parent(id)
                            .with_style(self.style.desc)
                            .color(diff3.1)
                            .h(2.0)
                            .set(state.ids.diff3, ui);
                            widget::Text::new(&format!("{} {:.1}%", &diff4.0, &diff.crit_chance * 100.0))
                            .align_middle_y_of(state.ids.stat4)
                            .right_from(state.ids.stat4, 10.0)
                            .graphics_for(id)
                            .parent(id)
                            .with_style(self.style.desc)
                            .color(diff4.1)
                            .h(2.0)
                            .set(state.ids.diff4, ui);
                            widget::Text::new(&format!("{} {:.1}", &diff5.0, &diff.crit_mult))
                            .align_middle_y_of(state.ids.stat5)
                            .right_from(state.ids.stat5, 10.0)
                            .graphics_for(id)
                            .parent(id)
                            .with_style(self.style.desc)
                            .color(diff5.1)
                            .h(2.0)
                            .set(state.ids.diff5, ui);
                        }
                    }
                },
                ItemKind::Armor(armor) => {
                    let stat1 = armor.get_protection();
                    let stat2 = armor.get_poise_resilience();

                    widget::Text::new(&format!("Armour : {}", util::protec2string(stat1)))
                        .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .h(2.0)
                        .down_from(state.ids.item_frame, H_PAD)
                        .set(state.ids.stat1, ui);
                    widget::Text::new(&format!("Poise res : {}", util::protec2string(stat2)))
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .h(2.0)
                        .set(state.ids.stat2, ui);

                    if let Some(equipped_item) = equip_slot.cloned().next() {
                        if let ItemKind::Armor(equipped_armor) = equipped_item.kind() {
                            let diff = armor.stats - equipped_armor.stats;
                            let diff1 = util::comparison(
                                &armor.stats.protection,
                                &equipped_armor.stats.protection,
                            );
                            let diff2 = util::comparison(
                                &armor.stats.poise_resilience,
                                &equipped_armor.stats.poise_resilience,
                            );

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
                            .h(2.0)
                            .set(state.ids.diff1, ui);
                            widget::Text::new(&format!(
                                "{} {}",
                                &diff2.0,
                                util::protec2string(diff.poise_resilience)
                            ))
                            .align_middle_y_of(state.ids.stat2)
                            .right_from(state.ids.stat2, 10.0)
                            .graphics_for(id)
                            .parent(id)
                            .with_style(self.style.desc)
                            .color(diff2.1)
                            .h(2.0)
                            .set(state.ids.diff2, ui);
                        }
                    }
                },
                ItemKind::Consumable { effect, .. } => {
                    dbg!(&util::consumable_desc(effect));
                    widget::Text::new(&util::consumable_desc(effect))
                        .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .h(2.0)
                        .down_from(state.ids.item_frame, H_PAD)
                        .set(state.ids.stat1, ui);
                },
                _ => (),
            }

            widget::Text::new(&desc)
                .w(text_w)
                .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.desc)
                .color(conrod_core::color::GREY)
                .w(text_w)
                .set(state.ids.desc, ui);
        }
    }

    /// Default width is based on the description font size unless the text is
    /// small enough to fit on a single line
    fn default_x_dimension(&self, ui: &Ui) -> Dimension {
        let single_line_title_w = widget::Text::new(self.title_text)
            .with_style(self.style.title)
            .get_w(ui)
            .unwrap_or(0.0);
        let single_line_desc_w = widget::Text::new(self.desc_text)
            .with_style(self.style.desc)
            .get_w(ui)
            .unwrap_or(0.0);

        let text_w = single_line_title_w.max(single_line_desc_w);
        let inner_w = if self.image.is_some() {
            match self.image_dims {
                Some((w, _)) => w + text_w + H_PAD,
                None => text_w / (1.0 - IMAGE_W_FRAC) + H_PAD,
            }
        } else {
            text_w
        };

        let width =
            inner_w.min(self.style.desc.font_size(&ui.theme) as f64 * DEFAULT_CHAR_W) + 2.0 * H_PAD;
        Dimension::Absolute(width)
    }

    fn default_y_dimension(&self, ui: &Ui) -> Dimension {
        let (text_w, image_w) = self.text_image_width(self.get_w(ui).unwrap_or(0.0));

        let icone_h = widget::Image::new(self.imgs.inv_slot_grey)
            .wh(ICON_SIZE)
            .get_h(ui)
            .unwrap_or(0.0);

        let desc_h = if self.desc_text.is_empty() {
            0.0
        } else {
            widget::Text::new(self.desc_text)
                .with_style(self.style.desc)
                .w(text_w)
                .get_h(ui)
                .unwrap_or(0.0)
                + self.style.desc.font_size(&ui.theme) as f64 * TEXT_SPACE_FACTOR
        };
        // Image defaults to square shape
        let image_h = self.image_dims.map_or(image_w, |(_, h)| h);
        // Title height + desc height + padding/spacing
        let height = (icone_h + desc_h).max(image_h) + 2.0 * V_PAD;
        Dimension::Absolute(height)
    }
}

impl<'a> Colorable for ItemTooltip<'a> {
    builder_method!(color { style.color = Some(Color) });
}
