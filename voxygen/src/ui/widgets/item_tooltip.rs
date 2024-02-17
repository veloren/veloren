use super::image_frame::ImageFrame;
use crate::hud::{
    get_quality_col,
    img_ids::Imgs,
    item_imgs::{animate_by_pulse, ItemImgs},
    util, HudInfo,
};
use client::Client;
use common::{
    comp::{
        item::{
            armor::Protection, item_key::ItemKey, modular::ModularComponent, Item, ItemDesc,
            ItemI18n, ItemKind, ItemTag, MaterialStatManifest, Quality,
        },
        Energy,
    },
    trade::SitePrices,
};
use conrod_core::{
    builder_method, builder_methods, image, input::global::Global, position::Dimension, text,
    widget, widget_ids, Color, Colorable, FontSize, Positionable, Scalar, Sizeable, Ui, UiCell,
    Widget, WidgetCommon, WidgetStyle,
};
use i18n::Localization;
use lazy_static::lazy_static;
use std::{
    borrow::Borrow,
    time::{Duration, Instant},
};

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

pub struct ItemTooltipManager {
    state: HoverState,
    // How long before a tooltip is displayed when hovering
    hover_dur: Duration,
    // How long it takes a tooltip to disappear
    fade_dur: Duration,
    // Current scaling of the ui
    logical_scale_factor: f64,
    // Ids for tooltip
    tooltip_ids: widget::id::List,
}

impl ItemTooltipManager {
    pub fn new(hover_dur: Duration, fade_dur: Duration, logical_scale_factor: f64) -> Self {
        Self {
            state: HoverState::None,
            hover_dur,
            fade_dur,
            logical_scale_factor,
            tooltip_ids: widget::id::List::new(),
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

    fn set_tooltip<'a, I>(
        &mut self,
        tooltip: &'a ItemTooltip,
        items: impl Iterator<Item = I>,
        prices: &'a Option<SitePrices>,
        img_id: Option<image::Id>,
        image_dims: Option<(f64, f64)>,
        src_id: widget::Id,
        ui: &mut UiCell,
    ) where
        I: Borrow<dyn ItemDesc>,
    {
        let mp_h = MOUSE_PAD_Y / self.logical_scale_factor;

        let mut id_walker = self.tooltip_ids.walk();

        let tooltip = |transparency, mouse_pos: [f64; 2], ui: &mut UiCell| {
            let mut prev_id = None;
            for item in items {
                let tooltip_id =
                    id_walker.next(&mut self.tooltip_ids, &mut ui.widget_id_generator());
                // Fill in text and the potential image beforehand to get an accurate size for
                // spacing
                let tooltip = tooltip
                    .clone()
                    .item(item.borrow())
                    .prices(prices)
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

                if let Some(prev_id) = prev_id {
                    tooltip
                        .floating(true)
                        .transparency(transparency)
                        .up_from(prev_id, 5.0)
                        .set(tooltip_id, ui);
                } else {
                    tooltip
                        .floating(true)
                        .transparency(transparency)
                        .x_y(x, y)
                        .set(tooltip_id, ui);
                }

                prev_id = Some(tooltip_id);
            }
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

pub struct ItemTooltipped<'a, W, I> {
    inner: W,
    tooltip_manager: &'a mut ItemTooltipManager,

    items: I,
    prices: &'a Option<SitePrices>,
    img_id: Option<image::Id>,
    image_dims: Option<(f64, f64)>,
    tooltip: &'a ItemTooltip<'a>,
}

impl<'a, W: Widget, I: Iterator> ItemTooltipped<'a, W, I> {
    pub fn tooltip_image(mut self, img_id: image::Id) -> Self {
        self.img_id = Some(img_id);
        self
    }

    pub fn tooltip_image_dims(mut self, dims: (f64, f64)) -> Self {
        self.image_dims = Some(dims);
        self
    }

    pub fn set(self, id: widget::Id, ui: &mut UiCell) -> W::Event
    where
        <I as Iterator>::Item: Borrow<dyn ItemDesc>,
    {
        let event = self.inner.set(id, ui);
        self.tooltip_manager.set_tooltip(
            self.tooltip,
            self.items,
            self.prices,
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
    fn with_item_tooltip<'a, I>(
        self,
        tooltip_manager: &'a mut ItemTooltipManager,

        items: I,

        prices: &'a Option<SitePrices>,

        tooltip: &'a ItemTooltip<'a>,
    ) -> ItemTooltipped<'a, Self, I>
    where
        Self: Sized;
}
impl<W: Widget> ItemTooltipable for W {
    fn with_item_tooltip<'a, I>(
        self,
        tooltip_manager: &'a mut ItemTooltipManager,
        items: I,
        prices: &'a Option<SitePrices>,
        tooltip: &'a ItemTooltip<'a>,
    ) -> ItemTooltipped<'a, W, I> {
        ItemTooltipped {
            inner: self,
            tooltip_manager,
            items,
            prices,
            img_id: None,
            image_dims: None,
            tooltip,
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
/// Item icon size
const ICON_SIZE: [f64; 2] = [64.0, 64.0];
/// Total item tooltip width
const WIDTH: f64 = 320.0;

/// A widget for displaying tooltips
#[derive(Clone, WidgetCommon)]
pub struct ItemTooltip<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    item: &'a dyn ItemDesc,
    msm: &'a MaterialStatManifest,
    prices: &'a Option<SitePrices>,
    image: Option<image::Id>,
    image_dims: Option<(f64, f64)>,
    style: Style,
    transparency: f32,
    image_frame: ImageFrame,
    client: &'a Client,
    info: &'a HudInfo,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    pulse: f32,
    localized_strings: &'a Localization,
    item_i18n: &'a ItemI18n,
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
        quantity,
        subtitle,
        desc,
        prices_buy,
        prices_sell,
        tooltip_hints,
        stats[],
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

lazy_static! {
    static ref EMPTY_ITEM: Item = Item::new_from_asset_expect("common.items.weapons.empty.empty");
}

impl<'a> ItemTooltip<'a> {
    builder_methods! {
        pub desc_text_color { style.desc.color = Some(Color) }
        pub title_font_size { style.title.font_size = Some(FontSize) }
        pub desc_font_size { style.desc.font_size = Some(FontSize) }
        pub title_justify { style.title.justify = Some(text::Justify) }
        pub desc_justify { style.desc.justify = Some(text::Justify) }
        pub title_line_spacing { style.title.line_spacing = Some(Scalar) }
        pub desc_line_spacing { style.desc.line_spacing = Some(Scalar) }
        image { image = Option<image::Id> }
        item { item = &'a dyn ItemDesc }
        prices { prices = &'a Option<SitePrices> }
        msm { msm = &'a MaterialStatManifest }
        image_dims { image_dims = Option<(f64, f64)> }
        transparency { transparency = f32 }
    }

    pub fn new(
        image_frame: ImageFrame,
        client: &'a Client,
        info: &'a HudInfo,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        pulse: f32,
        msm: &'a MaterialStatManifest,
        localized_strings: &'a Localization,
        item_i18n: &'a ItemI18n,
    ) -> Self {
        ItemTooltip {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            item: &*EMPTY_ITEM,
            msm,
            prices: &None,
            transparency: 1.0,
            image_frame,
            image: None,
            image_dims: None,
            client,
            info,
            imgs,
            item_imgs,
            pulse,
            localized_strings,
            item_i18n,
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
    #[must_use]
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
            ui,
            ..
        } = args;

        let i18n = &self.localized_strings;
        let item_i18n = &self.item_i18n;

        let inventories = self.client.inventories();
        let inventory = match inventories.get(self.info.viewpoint_entity) {
            Some(l) => l,
            None => return,
        };

        let item = self.item;

        let quality = get_quality_col(item);

        let item_kind = &*item.kind();

        let equipped_item = inventory.equipped_items_replaceable_by(item_kind).next();

        let (title, desc) = util::item_text(item, i18n, item_i18n);

        let item_kind = util::kind_text(item_kind, i18n).to_string();

        let material = item.tags().into_iter().find_map(|t| match t {
            ItemTag::MaterialKind(material) => Some(material),
            _ => None,
        });

        let subtitle = if let Some(material) = material {
            format!(
                "{} ({})",
                item_kind,
                util::material_kind_text(&material, i18n)
            )
        } else {
            item_kind
        };

        let style = self.style.desc;

        let text_color = conrod_core::color::WHITE;

        // Widths
        let (text_w, image_w) = self.text_image_width(rect.w());

        // Color quality
        let quality_col_img = match &item.quality() {
            Quality::Low => self.imgs.inv_slot_grey,
            Quality::Common => self.imgs.inv_slot_common,
            Quality::Moderate => self.imgs.inv_slot_green,
            Quality::High => self.imgs.inv_slot_blue,
            Quality::Epic => self.imgs.inv_slot_purple,
            Quality::Legendary => self.imgs.inv_slot_gold,
            Quality::Artifact => self.imgs.inv_slot_orange,
            _ => self.imgs.inv_slot_red,
        };

        let stats_count = util::stats_count(item, self.msm);

        // Update widget array size
        state.update(|s| {
            s.ids
                .stats
                .resize(stats_count, &mut ui.widget_id_generator())
        });

        state.update(|s| {
            s.ids
                .diffs
                .resize(stats_count, &mut ui.widget_id_generator())
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

        // Item frame
        widget::Image::new(quality_col_img)
            .wh(ICON_SIZE)
            .graphics_for(id)
            .parent(id)
            .top_left_with_margins_on(state.ids.image_frame, V_PAD, H_PAD)
            .set(state.ids.item_frame, ui);

        // Item render
        widget::Image::new(animate_by_pulse(
            &self
                .item_imgs
                .img_ids_or_not_found_img(ItemKey::from(&item)),
            self.pulse,
        ))
        .color(Some(conrod_core::color::WHITE))
        .w_h(ICON_SIZE[0] * 0.8, ICON_SIZE[1] * 0.8)
        .middle_of(state.ids.item_frame)
        .set(state.ids.item_render, ui);

        let title_w = (text_w - H_PAD * 3.0 - ICON_SIZE[0]).max(0.0);

        // Title
        widget::Text::new(&title)
            .w(title_w)
            .graphics_for(id)
            .parent(id)
            .with_style(self.style.title)
            .top_left_with_margins_on(state.ids.image_frame, V_PAD, H_PAD)
            .right_from(state.ids.item_frame, H_PAD)
            .color(quality)
            .set(state.ids.title, ui);

        // Amount
        let (subtitle_relative_id, spacing) = if self.item.amount().get() > 1 {
            widget::Text::new(&format!("Amount: {}", self.item.amount().get()))
                .w(title_w)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.desc)
                .color(conrod_core::color::GREY)
                .down_from(state.ids.title, 2.0)
                .set(state.ids.quantity, ui);

            (state.ids.quantity, 2.0)
        } else {
            (state.ids.title, V_PAD)
        };

        // Subtitle
        widget::Text::new(&subtitle)
            .w(title_w)
            .graphics_for(id)
            .parent(id)
            .with_style(self.style.desc)
            .color(conrod_core::color::GREY)
            .down_from(subtitle_relative_id, spacing)
            .set(state.ids.subtitle, ui);

        // Stats
        match &*item.kind() {
            ItemKind::Tool(tool) => {
                let stats = tool.stats(item.stats_durability_multiplier());

                // Power
                widget::Text::new(&format!(
                    "{} : {:.1}",
                    i18n.get_msg("common-stats-power"),
                    stats.power * 10.0
                ))
                .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.desc)
                .color(text_color)
                .down_from(state.ids.item_frame, V_PAD)
                .set(state.ids.stats[0], ui);

                let mut stat_text = |text: String, i: usize| {
                    widget::Text::new(&text)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[i - 1], V_PAD_STATS)
                        .set(state.ids.stats[i], ui);
                };

                // Speed
                stat_text(
                    format!(
                        "{} : {:+.0}%",
                        i18n.get_msg("common-stats-speed"),
                        (stats.speed - 1.0) * 100.0
                    ),
                    1,
                );

                // Effect Power
                stat_text(
                    format!(
                        "{} : {:+.0}%",
                        i18n.get_msg("common-stats-effect-power"),
                        (stats.effect_power - 1.0) * 100.0
                    ),
                    2,
                );

                // Range
                stat_text(
                    format!(
                        "{} : {:+.0}%",
                        i18n.get_msg("common-stats-range"),
                        (stats.range - 1.0) * 100.0
                    ),
                    3,
                );

                // Energy Efficiency
                stat_text(
                    format!(
                        "{} : {:+.0}%",
                        i18n.get_msg("common-stats-energy_efficiency"),
                        (stats.energy_efficiency - 1.0) * 100.0
                    ),
                    4,
                );

                // Buff Strength
                stat_text(
                    format!(
                        "{} : {:+.0}%",
                        i18n.get_msg("common-stats-buff_strength"),
                        (stats.buff_strength - 1.0) * 100.0
                    ),
                    5,
                );

                if item.has_durability() {
                    let durability = Item::MAX_DURABILITY - item.durability_lost().unwrap_or(0);
                    stat_text(
                        format!(
                            "{} : {}/{}",
                            i18n.get_msg("common-stats-durability"),
                            durability,
                            Item::MAX_DURABILITY
                        ),
                        6,
                    )
                }

                if let Some(equipped_item) = equipped_item {
                    if let ItemKind::Tool(equipped_tool) = &*equipped_item.kind() {
                        let tool_stats = tool.stats(item.stats_durability_multiplier());
                        let equipped_tool_stats =
                            equipped_tool.stats(equipped_item.stats_durability_multiplier());
                        let diff = tool_stats - equipped_tool_stats;
                        let power_diff =
                            util::comparison(tool_stats.power, equipped_tool_stats.power);
                        let speed_diff =
                            util::comparison(tool_stats.speed, equipped_tool_stats.speed);
                        let effect_power_diff = util::comparison(
                            tool_stats.effect_power,
                            equipped_tool_stats.effect_power,
                        );
                        let range_diff =
                            util::comparison(tool_stats.range, equipped_tool_stats.range);
                        let energy_efficiency_diff = util::comparison(
                            tool_stats.energy_efficiency,
                            equipped_tool_stats.energy_efficiency,
                        );
                        let buff_strength_diff = util::comparison(
                            tool_stats.buff_strength,
                            equipped_tool_stats.buff_strength,
                        );

                        let tool_durability =
                            util::item_durability(item).unwrap_or(Item::MAX_DURABILITY);
                        let equipped_durability =
                            util::item_durability(equipped_item).unwrap_or(Item::MAX_DURABILITY);
                        let durability_diff =
                            util::comparison(tool_durability, equipped_durability);

                        let mut diff_text = |text: String, color, id_index| {
                            widget::Text::new(&text)
                                .align_middle_y_of(state.ids.stats[id_index])
                                .right_from(state.ids.stats[id_index], H_PAD)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(style)
                                .color(color)
                                .set(state.ids.diffs[id_index], ui)
                        };

                        if diff.power.abs() > f32::EPSILON {
                            let text = format!("{} {:.1}", &power_diff.0, &diff.power * 10.0);
                            diff_text(text, power_diff.1, 0)
                        }
                        if diff.speed.abs() > f32::EPSILON {
                            let text = format!("{} {:+.0}", &speed_diff.0, &diff.speed * 100.0);
                            diff_text(text, speed_diff.1, 1)
                        }
                        if diff.effect_power.abs() > f32::EPSILON {
                            let text = format!(
                                "{} {:+.0}",
                                &effect_power_diff.0,
                                &diff.effect_power * 100.0
                            );
                            diff_text(text, effect_power_diff.1, 2)
                        }
                        if diff.range.abs() > f32::EPSILON {
                            let text = format!("{} {:.1}%", &range_diff.0, &diff.range * 100.0);
                            diff_text(text, range_diff.1, 3)
                        }
                        if diff.energy_efficiency.abs() > f32::EPSILON {
                            let text = format!(
                                "{} {:.1}%",
                                &energy_efficiency_diff.0,
                                &diff.energy_efficiency * 100.0
                            );
                            diff_text(text, energy_efficiency_diff.1, 4)
                        }
                        if diff.buff_strength.abs() > f32::EPSILON {
                            let text = format!(
                                "{} {:.1}%",
                                &buff_strength_diff.0,
                                &diff.buff_strength * 100.0
                            );
                            diff_text(text, buff_strength_diff.1, 5)
                        }
                        if tool_durability != equipped_durability && item.has_durability() {
                            let text = format!(
                                "{} {}",
                                &durability_diff.0,
                                tool_durability as i32 - equipped_durability as i32
                            );
                            diff_text(text, durability_diff.1, 6)
                        }
                    }
                }
            },
            ItemKind::Armor(armor) => {
                let armor_stats = armor.stats(self.msm, item.stats_durability_multiplier());

                let mut stat_text = |text: String, i: usize| {
                    widget::Text::new(&text)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .and(|t| {
                            if i == 0 {
                                t.x_align_to(
                                    state.ids.item_frame,
                                    conrod_core::position::Align::Start,
                                )
                                .down_from(state.ids.item_frame, V_PAD)
                            } else {
                                t.down_from(state.ids.stats[i - 1], V_PAD_STATS)
                            }
                        })
                        .set(state.ids.stats[i], ui);
                };

                let mut index = 0;

                if armor_stats.protection.is_some() {
                    stat_text(
                        format!(
                            "{} : {}",
                            i18n.get_msg("common-stats-armor"),
                            util::protec2string(
                                armor_stats.protection.unwrap_or(Protection::Normal(0.0))
                            )
                        ),
                        index,
                    );
                    index += 1;
                }

                // Poise res
                if armor_stats.poise_resilience.is_some() {
                    stat_text(
                        format!(
                            "{} : {}",
                            i18n.get_msg("common-stats-poise_res"),
                            util::protec2string(
                                armor_stats
                                    .poise_resilience
                                    .unwrap_or(Protection::Normal(0.0))
                            )
                        ),
                        index,
                    );
                    index += 1;
                }

                // Max Energy
                if armor_stats.energy_max.is_some() {
                    stat_text(
                        format!(
                            "{} : {:.1}",
                            i18n.get_msg("common-stats-energy_max"),
                            armor_stats.energy_max.unwrap_or(0.0)
                        ),
                        index,
                    );
                    index += 1;
                }

                // Energy Recovery
                if armor_stats.energy_reward.is_some() {
                    stat_text(
                        format!(
                            "{} : {:.1}%",
                            i18n.get_msg("common-stats-energy_reward"),
                            armor_stats.energy_reward.map_or(0.0, |x| x * 100.0)
                        ),
                        index,
                    );
                    index += 1;
                }

                // Precision Power
                if armor_stats.precision_power.is_some() {
                    stat_text(
                        format!(
                            "{} : {:.3}",
                            i18n.get_msg("common-stats-precision_power"),
                            armor_stats.precision_power.unwrap_or(0.0)
                        ),
                        index,
                    );
                    index += 1;
                }

                // Stealth
                if armor_stats.stealth.is_some() {
                    stat_text(
                        format!(
                            "{} : {:.3}",
                            i18n.get_msg("common-stats-stealth"),
                            armor_stats.stealth.unwrap_or(0.0)
                        ),
                        index,
                    );
                    index += 1;
                }

                // Slots
                if item.num_slots() > 0 {
                    stat_text(
                        format!(
                            "{} : {}",
                            i18n.get_msg("common-stats-slots"),
                            item.num_slots()
                        ),
                        index,
                    );
                    index += 1;
                }

                if let Some(durability) = util::item_durability(item) {
                    stat_text(
                        format!(
                            "{} : {}/{}",
                            i18n.get_msg("common-stats-durability"),
                            durability,
                            Item::MAX_DURABILITY
                        ),
                        index,
                    );
                }

                if let Some(equipped_item) = equipped_item {
                    if let ItemKind::Armor(equipped_armor) = &*equipped_item.kind() {
                        let equipped_stats = equipped_armor
                            .stats(self.msm, equipped_item.stats_durability_multiplier());
                        let diff = armor_stats - equipped_stats;
                        let protection_diff = util::option_comparison(
                            &armor_stats.protection,
                            &equipped_stats.protection,
                        );
                        let poise_res_diff = util::option_comparison(
                            &armor_stats.poise_resilience,
                            &equipped_stats.poise_resilience,
                        );
                        let energy_max_diff = util::option_comparison(
                            &armor_stats.energy_max,
                            &equipped_stats.energy_max,
                        );
                        let energy_reward_diff = util::option_comparison(
                            &armor_stats.energy_reward,
                            &equipped_stats.energy_reward,
                        );
                        let precision_power_diff = util::option_comparison(
                            &armor_stats.precision_power,
                            &equipped_stats.precision_power,
                        );
                        let stealth_diff =
                            util::option_comparison(&armor_stats.stealth, &equipped_stats.stealth);

                        let armor_durability = util::item_durability(item);
                        let equipped_durability = util::item_durability(equipped_item);
                        let durability_diff =
                            util::option_comparison(&armor_durability, &equipped_durability);

                        let mut diff_text = |text: String, color, id_index| {
                            widget::Text::new(&text)
                                .align_middle_y_of(state.ids.stats[id_index])
                                .right_from(state.ids.stats[id_index], H_PAD)
                                .graphics_for(id)
                                .parent(id)
                                .with_style(style)
                                .color(color)
                                .set(state.ids.diffs[id_index], ui)
                        };

                        let mut index = 0;
                        if let Some(p_diff) = diff.protection {
                            if p_diff != Protection::Normal(0.0) {
                                let text = format!(
                                    "{} {}",
                                    &protection_diff.0,
                                    util::protec2string(p_diff)
                                );
                                diff_text(text, protection_diff.1, index);
                            }
                        }
                        index += armor_stats.protection.is_some() as usize;

                        if let Some(p_r_diff) = diff.poise_resilience {
                            if p_r_diff != Protection::Normal(0.0) {
                                let text = format!(
                                    "{} {}",
                                    &poise_res_diff.0,
                                    util::protec2string(p_r_diff)
                                );
                                diff_text(text, poise_res_diff.1, index);
                            }
                        }
                        index += armor_stats.poise_resilience.is_some() as usize;

                        if let Some(e_m_diff) = diff.energy_max {
                            if e_m_diff.abs() > Energy::ENERGY_EPSILON {
                                let text = format!("{} {:.1}", &energy_max_diff.0, e_m_diff);
                                diff_text(text, energy_max_diff.1, index);
                            }
                        }
                        index += armor_stats.energy_max.is_some() as usize;

                        if let Some(e_r_diff) = diff.energy_reward {
                            if e_r_diff.abs() > Energy::ENERGY_EPSILON {
                                let text =
                                    format!("{} {:.1}", &energy_reward_diff.0, e_r_diff * 100.0);
                                diff_text(text, energy_reward_diff.1, index);
                            }
                        }
                        index += armor_stats.energy_reward.is_some() as usize;

                        if let Some(p_p_diff) = diff.precision_power {
                            if p_p_diff != 0.0_f32 {
                                let text = format!("{} {:.3}", &precision_power_diff.0, p_p_diff);
                                diff_text(text, precision_power_diff.1, index);
                            }
                        }
                        index += armor_stats.precision_power.is_some() as usize;

                        if let Some(s_diff) = diff.stealth {
                            if s_diff != 0.0_f32 {
                                let text = format!("{} {:.3}", &stealth_diff.0, s_diff);
                                diff_text(text, stealth_diff.1, index);
                            }
                        }
                        index += armor_stats.stealth.is_some() as usize;

                        if armor_durability != equipped_durability && item.has_durability() {
                            let diff = armor_durability.unwrap_or(Item::MAX_DURABILITY) as i32
                                - equipped_durability.unwrap_or(Item::MAX_DURABILITY) as i32;
                            let text = format!("{} {}", &durability_diff.0, diff);
                            diff_text(text, durability_diff.1, index);
                        }
                    }
                }
            },
            ItemKind::Consumable { effects, .. } => {
                for (i, desc) in util::consumable_desc(effects, i18n).iter().enumerate() {
                    if i == 0 {
                        widget::Text::new(desc)
                            .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                            .graphics_for(id)
                            .parent(id)
                            .with_style(self.style.desc)
                            .color(text_color)
                            .down_from(state.ids.item_frame, V_PAD)
                            .set(state.ids.stats[0], ui);
                    } else {
                        widget::Text::new(desc)
                            .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                            .graphics_for(id)
                            .parent(id)
                            .with_style(self.style.desc)
                            .color(text_color)
                            .down_from(state.ids.stats[i - 1], V_PAD_STATS)
                            .set(state.ids.stats[i], ui);
                    }
                }
            },
            ItemKind::ModularComponent(mc) => {
                if let Some(stats) = mc.tool_stats(item.components(), self.msm) {
                    let is_primary = matches!(mc, ModularComponent::ToolPrimaryComponent { .. });

                    // Power
                    let power_text = if is_primary {
                        format!(
                            "{} : {:.1}",
                            i18n.get_msg("common-stats-power"),
                            stats.power * 10.0
                        )
                    } else {
                        format!(
                            "{} : x{:.2}",
                            i18n.get_msg("common-stats-power"),
                            stats.power
                        )
                    };
                    widget::Text::new(&power_text)
                        .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.item_frame, V_PAD)
                        .set(state.ids.stats[0], ui);

                    // Speed
                    let speed_text = if is_primary {
                        format!(
                            "{} : {:+.0}%",
                            i18n.get_msg("common-stats-speed"),
                            (stats.speed - 1.0) * 100.0
                        )
                    } else {
                        format!(
                            "{} : x{:.2}",
                            i18n.get_msg("common-stats-speed"),
                            stats.speed
                        )
                    };
                    widget::Text::new(&speed_text)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[0], V_PAD_STATS)
                        .set(state.ids.stats[1], ui);

                    // Effect Power
                    // TODO: Allow effect power to have different terminology based on what it is
                    // affecting.
                    let effect_power_text = if is_primary {
                        format!(
                            "{} : {:+.0}%",
                            i18n.get_msg("common-stats-effect-power"),
                            (stats.effect_power - 1.0) * 100.0
                        )
                    } else {
                        format!(
                            "{} : x{:.2}",
                            i18n.get_msg("common-stats-effect-power"),
                            stats.effect_power
                        )
                    };
                    widget::Text::new(&effect_power_text)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[1], V_PAD_STATS)
                        .set(state.ids.stats[2], ui);

                    // Range
                    let range_text = if is_primary {
                        format!(
                            "{} : {:.0}%",
                            i18n.get_msg("common-stats-range"),
                            (stats.range - 1.0) * 100.0
                        )
                    } else {
                        format!(
                            "{} : x{:.2}",
                            i18n.get_msg("common-stats-range"),
                            stats.range
                        )
                    };
                    widget::Text::new(&range_text)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[2], V_PAD_STATS)
                        .set(state.ids.stats[3], ui);

                    // Energy Efficiency
                    let energy_eff_text = if is_primary {
                        format!(
                            "{} : {:.0}%",
                            i18n.get_msg("common-stats-energy_efficiency"),
                            (stats.energy_efficiency - 1.0) * 100.0
                        )
                    } else {
                        format!(
                            "{} : x{:.2}",
                            i18n.get_msg("common-stats-energy_efficiency"),
                            stats.energy_efficiency
                        )
                    };
                    widget::Text::new(&energy_eff_text)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[3], V_PAD_STATS)
                        .set(state.ids.stats[4], ui);

                    // Buff Strength
                    let buff_str_text = if is_primary {
                        format!(
                            "{} : {:.0}%",
                            i18n.get_msg("common-stats-buff_strength"),
                            (stats.buff_strength - 1.0) * 100.0
                        )
                    } else {
                        format!(
                            "{} : x{:.2}",
                            i18n.get_msg("common-stats-buff_strength"),
                            stats.buff_strength
                        )
                    };
                    widget::Text::new(&buff_str_text)
                        .graphics_for(id)
                        .parent(id)
                        .with_style(self.style.desc)
                        .color(text_color)
                        .down_from(state.ids.stats[4], V_PAD_STATS)
                        .set(state.ids.stats[5], ui);
                }
            },
            _ => (),
        }

        // Description
        if !desc.is_empty() {
            widget::Text::new(&format!("\"{}\"", &desc))
                .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.desc)
                .color(conrod_core::color::GREY)
                .down_from(
                    if stats_count > 0 {
                        state.ids.stats[state.ids.stats.len() - 1]
                    } else {
                        state.ids.item_frame
                    },
                    V_PAD,
                )
                .w(text_w)
                .set(state.ids.desc, ui);
        }

        // Price display
        if let Some((buy, sell, factor)) =
            util::price_desc(self.prices, item.item_definition_id(), i18n)
        {
            widget::Text::new(&buy)
                .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.desc)
                .color(Color::Rgba(factor, 1.0 - factor, 0.00, 1.0))
                .down_from(
                    if !desc.is_empty() {
                        state.ids.desc
                    } else if stats_count > 0 {
                        state.ids.stats[state.ids.stats.len() - 1]
                    } else {
                        state.ids.item_frame
                    },
                    V_PAD,
                )
                .w(text_w)
                .set(state.ids.prices_buy, ui);

            widget::Text::new(&sell)
                .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.desc)
                .color(Color::Rgba(1.0 - factor, factor, 0.00, 1.0))
                .down_from(state.ids.prices_buy, V_PAD_STATS)
                .w(text_w)
                .set(state.ids.prices_sell, ui);

            //Tooltips for trade mini-tutorial
            widget::Text::new(&format!(
                "{}\n{}",
                i18n.get_msg("hud-trade-tooltip_hint_1"),
                i18n.get_msg("hud-trade-tooltip_hint_2"),
            ))
            .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
            .graphics_for(id)
            .parent(id)
            .with_style(self.style.desc)
            .color(Color::Rgba(255.0, 255.0, 255.0, 1.0))
            .down_from(state.ids.prices_sell, V_PAD_STATS)
            .w(text_w)
            .set(state.ids.tooltip_hints, ui);
        }
    }

    /// Default width is based on the description font size unless the text is
    /// small enough to fit on a single line
    fn default_x_dimension(&self, _ui: &Ui) -> Dimension { Dimension::Absolute(WIDTH) }

    fn default_y_dimension(&self, ui: &Ui) -> Dimension {
        let item = &self.item;

        let (_, desc) = util::item_text(item, self.localized_strings, self.item_i18n);

        let (text_w, _image_w) = self.text_image_width(WIDTH);

        // Item frame
        let frame_h = ICON_SIZE[1] + V_PAD;

        // Stats
        let stats_count = util::line_count(self.item, self.msm, self.localized_strings);
        let stat_h = if stats_count > 0 {
            widget::Text::new("placeholder")
                .with_style(self.style.desc)
                .get_h(ui)
                .unwrap_or(0.0)
                * stats_count as f64
                + (stats_count - 1) as f64 * V_PAD_STATS
                + V_PAD
        } else {
            0.0
        };

        // Description
        let desc_h: f64 = if !desc.is_empty() {
            widget::Text::new(&format!("\"{}\"", &desc))
                .with_style(self.style.desc)
                .w(text_w)
                .get_h(ui)
                .unwrap_or(0.0)
                + V_PAD
        } else {
            0.0
        };

        // Price
        let price_h: f64 = if let Some((buy, sell, _)) = util::price_desc(
            self.prices,
            item.item_definition_id(),
            self.localized_strings,
        ) {
            // Get localized tooltip strings (gotten here because these should
            // only show if in a trade- aka if buy/sell prices are present)
            let tt_hint_1 = self.localized_strings.get_msg("hud-trade-tooltip_hint_1");
            let tt_hint_2 = self.localized_strings.get_msg("hud-trade-tooltip_hint_2");

            widget::Text::new(&format!("{}\n{}\n{}\n{}", buy, sell, tt_hint_1, tt_hint_2))
                .with_style(self.style.desc)
                .w(text_w)
                .get_h(ui)
                .unwrap_or(0.0)
                + V_PAD * 2.0
        } else {
            0.0
        };

        // extra padding to fit frame top padding
        let height = frame_h + stat_h + desc_h + price_h + V_PAD + 5.0;
        Dimension::Absolute(height)
    }
}

impl<'a> Colorable for ItemTooltip<'a> {
    builder_method!(color { style.color = Some(Color) });
}
