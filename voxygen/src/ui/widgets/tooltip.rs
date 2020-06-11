use super::image_frame::ImageFrame;
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

pub struct TooltipManager {
    tooltip_id: widget::Id,
    state: HoverState,
    // How long before a tooltip is displayed when hovering
    hover_dur: Duration,
    // How long it takes a tooltip to disappear
    fade_dur: Duration,
    // Current scaling of the ui
    logical_scale_factor: f64,
}
impl TooltipManager {
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
                HoverState::Hovering(hover) if um_id == hover.0 || um_id == self.tooltip_id => (),
                HoverState::Hovering(hover) => {
                    self.state =
                        HoverState::Fading(Instant::now(), hover, Some((Instant::now(), um_id)))
                },
                HoverState::Fading(_, _, Some((_, id)))
                    if um_id == id || um_id == self.tooltip_id => {},
                HoverState::Fading(start, hover, _) => {
                    self.state = HoverState::Fading(start, hover, Some((Instant::now(), um_id)))
                },
                HoverState::Start(_, id) if um_id == id || um_id == self.tooltip_id => (),
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
        tooltip: &Tooltip,
        title_text: &str,
        desc_text: &str,
        img_id: Option<image::Id>,
        image_dims: Option<(f64, f64)>,
        src_id: widget::Id,
        bottom_offset: f64,
        x_offset: f64,
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
                .image(img_id)
                .image_dims(image_dims);

            let [t_w, t_h] = tooltip.get_wh(ui).unwrap_or([0.0, 0.0]);
            let [m_x, m_y] = [mouse_pos[0], mouse_pos[1]];
            let (w_w, w_h) = (ui.win_w, ui.win_h);

            // Determine position based on size and mouse position
            // Flow to the bottom right of the mouse
            let x = (m_x + t_w / 2.0).min(w_w / 2.0 - t_w / 2.0 + x_offset);
            let y = (m_y - mp_h - t_h / 2.0).max(-w_h / 2.0 + t_h / 2.0 + bottom_offset);
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

pub struct Tooltipped<'a, W> {
    inner: W,
    tooltip_manager: &'a mut TooltipManager,
    title_text: &'a str,
    desc_text: &'a str,
    img_id: Option<image::Id>,
    image_dims: Option<(f64, f64)>,
    // Offsets limit of bottom of tooltip
    bottom_offset: Option<f64>,
    x_offset: Option<f64>,
    tooltip: &'a Tooltip<'a>,
}
impl<'a, W: Widget> Tooltipped<'a, W> {
    pub fn tooltip_image(mut self, img_id: image::Id) -> Self {
        self.img_id = Some(img_id);
        self
    }

    pub fn tooltip_image_dims(mut self, dims: (f64, f64)) -> Self {
        self.image_dims = Some(dims);
        self
    }

    pub fn x_offset(mut self, off: f64) -> Self {
        self.x_offset = Some(off);
        self
    }

    pub fn bottom_offset(mut self, off: f64) -> Self {
        self.bottom_offset = Some(off);
        self
    }

    pub fn set(self, id: widget::Id, ui: &mut UiCell) -> W::Event {
        let event = self.inner.set(id, ui);
        self.tooltip_manager.set_tooltip(
            self.tooltip,
            self.title_text,
            self.desc_text,
            self.img_id,
            self.image_dims,
            id,
            self.bottom_offset.unwrap_or(0.0),
            self.x_offset.unwrap_or(0.0),
            ui,
        );
        event
    }
}

pub trait Tooltipable {
    // If `Tooltip` is expensive to construct accept a closure here instead.
    fn with_tooltip<'a>(
        self,
        tooltip_manager: &'a mut TooltipManager,
        title_text: &'a str,
        desc_text: &'a str,
        tooltip: &'a Tooltip<'a>,
    ) -> Tooltipped<'a, Self>
    where
        Self: std::marker::Sized;
}
impl<W: Widget> Tooltipable for W {
    fn with_tooltip<'a>(
        self,
        tooltip_manager: &'a mut TooltipManager,
        title_text: &'a str,
        desc_text: &'a str,
        tooltip: &'a Tooltip<'a>,
    ) -> Tooltipped<'a, W> {
        Tooltipped {
            inner: self,
            tooltip_manager,
            title_text,
            desc_text,
            img_id: None,
            image_dims: None,
            bottom_offset: None,
            x_offset: None,
            tooltip,
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
const DEFAULT_CHAR_W: f64 = 30.0;
/// Text vertical spacing factor to account for overhanging text
const TEXT_SPACE_FACTOR: f64 = 0.35;

/// A widget for displaying tooltips
#[derive(Clone, WidgetCommon)]
pub struct Tooltip<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    title_text: &'a str,
    desc_text: &'a str,
    image: Option<image::Id>,
    image_dims: Option<(f64, f64)>,
    style: Style,
    transparency: f32,
    image_frame: ImageFrame,
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
        desc,
        image_frame,
        image,
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Tooltip<'a> {
    builder_methods! {
        pub title_text_color { style.title.color = Some(Color) }
        pub desc_text_color { style.desc.color = Some(Color) }
        pub title_font_size { style.title.font_size = Some(FontSize) }
        pub desc_font_size { style.desc.font_size = Some(FontSize) }
        pub title_justify { style.title.justify = Some(text::Justify) }
        pub desc_justify { style.desc.justify = Some(text::Justify) }
        image { image = Option<image::Id> }
        title { title_text = &'a str }
        desc { desc_text = &'a str }
        image_dims { image_dims = Option<(f64, f64)> }
        transparency { transparency = f32 }
    }

    pub fn new(image_frame: ImageFrame) -> Self {
        Tooltip {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            title_text: "",
            desc_text: "",
            transparency: 1.0,
            image_frame,
            image: None,
            image_dims: None,
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

impl<'a> Widget for Tooltip<'a> {
    type Event = ();
    type State = State;
    type Style = Style;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style { self.style.clone() }

    #[allow(clippy::collapsible_if)] // TODO: Pending review in #587
    fn update(self, args: widget::UpdateArgs<Self>) {
        let widget::UpdateArgs {
            id,
            state,
            rect,
            style,
            ui,
            ..
        } = args;

        // Widths
        let (text_w, image_w) = self.text_image_width(rect.w());

        // Apply transparency
        let color = style.color(ui.theme()).alpha(self.transparency);

        // Background image frame
        self.image_frame
            .wh(rect.dim())
            .xy(rect.xy())
            .graphics_for(id)
            .parent(id)
            .color(color)
            .set(state.ids.image_frame, ui);

        // Image
        if let Some(img_id) = self.image {
            widget::Image::new(img_id)
                .w_h(image_w, self.image_dims.map_or(image_w, |(_, h)| h))
                .graphics_for(id)
                .parent(id)
                .color(Some(color))
                .top_left_with_margins_on(state.ids.image_frame, V_PAD, H_PAD)
                .set(state.ids.image, ui);
        }

        // Spacing for overhanging text
        let title_space = self.style.title.font_size(&ui.theme) as f64 * TEXT_SPACE_FACTOR;

        // Title of tooltip
        if !self.title_text.is_empty() {
            let title = widget::Text::new(self.title_text)
                .w(text_w)
                .graphics_for(id)
                .parent(id)
                .with_style(self.style.title)
                // Apply transparency
                .color(style.title.color(ui.theme()).alpha(self.transparency));

            if self.image.is_some() {
                title
                    .right_from(state.ids.image, H_PAD)
                    .align_top_of(state.ids.image)
            } else {
                title.top_left_with_margins_on(state.ids.image_frame, V_PAD, H_PAD)
            }
            .set(state.ids.title, ui);
        }

        // Description of tooltip
        let desc = widget::Text::new(self.desc_text)
            .w(text_w)
            .graphics_for(id)
            .parent(id)
            // Apply transparency
            .color(style.desc.color(ui.theme()).alpha(self.transparency))
            .with_style(self.style.desc);

        if !self.title_text.is_empty() {
            desc.down_from(state.ids.title, V_PAD * 0.5 + title_space)
                .align_left_of(state.ids.title)
        } else {
            if self.image.is_some() {
                desc.right_from(state.ids.image, H_PAD)
                    .align_top_of(state.ids.image)
            } else {
                desc.top_left_with_margins_on(state.ids.image_frame, V_PAD, H_PAD)
            }
        }
        .set(state.ids.desc, ui);
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
        let title_h = if self.title_text.is_empty() {
            0.0
        } else {
            widget::Text::new(self.title_text)
                .with_style(self.style.title)
                .w(text_w)
                .get_h(ui)
                .unwrap_or(0.0)
                + self.style.title.font_size(&ui.theme) as f64 * TEXT_SPACE_FACTOR
                + 0.5 * V_PAD
        };
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
        let height = (title_h + desc_h).max(image_h) + 2.0 * V_PAD;
        Dimension::Absolute(height)
    }
}

impl<'a> Colorable for Tooltip<'a> {
    builder_method!(color { style.color = Some(Color) });
}
