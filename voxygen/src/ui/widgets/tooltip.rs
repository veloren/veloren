use conrod_core::{
    builder_method, builder_methods, input::global::Global, text, widget, widget_ids, Color,
    Colorable, FontSize, Positionable, Sizeable, UiCell, Widget, WidgetCommon, WidgetStyle,
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
                }
                HoverState::Fading(_, _, Some((_, id)))
                    if um_id == id || um_id == self.tooltip_id => {}
                HoverState::Fading(start, hover, _) => {
                    self.state = HoverState::Fading(start, hover, Some((Instant::now(), um_id)))
                }
                HoverState::Start(_, id) if um_id == id || um_id == self.tooltip_id => (),
                HoverState::Start(_, _) | HoverState::None => {
                    self.state = HoverState::Start(Instant::now(), um_id)
                }
            }
        } else {
            match self.state {
                HoverState::Hovering(hover) => {
                    self.state = HoverState::Fading(Instant::now(), hover, None)
                }
                HoverState::Fading(start, hover, Some((_, _))) => {
                    self.state = HoverState::Fading(start, hover, None)
                }
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
    fn set_tooltip(&mut self, tooltip: Tooltip, src_id: widget::Id, ui: &mut UiCell) {
        let tooltip_id = self.tooltip_id;
        let mp_h = MOUSE_PAD_Y / self.logical_scale_factor;

        let tooltip = |transparency, mouse_pos: [f64; 2], ui: &mut UiCell| {
            let [t_w, t_h] = tooltip.get_wh(ui).unwrap_or([0.0, 0.0]);
            let [m_x, m_y] = mouse_pos;
            let (w_w, w_h) = (ui.win_w, ui.win_h);

            // Determine position based on size and mouse position
            // Flow to the bottom right of the mouse
            let x = (m_x + t_w / 2.0).min(w_w / 2.0 - t_w / 2.0);
            let y = (m_y - mp_h - t_h / 2.0).max(-w_h / 2.0 + t_h / 2.0);
            tooltip
                .floating(true)
                .transparency(transparency)
                .x_y(x, y)
                .set(tooltip_id, ui);
        };

        match self.state {
            HoverState::Hovering(hover) => tooltip(1.0, hover.1, ui),
            HoverState::Fading(start, hover, _) => tooltip(
                (1.0f32 - start.elapsed().as_millis() as f32 / self.hover_dur.as_millis() as f32)
                    .max(0.0),
                hover.1,
                ui,
            ),
            HoverState::Start(start, id) if id == src_id && start.elapsed() > self.hover_dur => {
                let xy = ui.global_input().current.mouse.xy;
                self.state = HoverState::Hovering(Hover(id, xy));
                tooltip(1.0, xy, ui);
            }
            HoverState::Start(_, _) | HoverState::None => (),
        }
    }
}

pub struct Tooltipped<'a, W> {
    inner: W,
    tooltip_manager: &'a mut TooltipManager,
    tooltip: Tooltip<'a>,
}
impl<'a, W: Widget> Tooltipped<'a, W> {
    pub fn set(self, id: widget::Id, ui: &mut UiCell) -> W::Event {
        let event = self.inner.set(id, ui);
        self.tooltip_manager.set_tooltip(self.tooltip, id, ui);
        event
    }
}

pub trait Tooltipable {
    // If `Tooltip` is expensive to construct accept a closure here instead.
    fn with_tooltip<'a>(
        self,
        tooltip_manager: &'a mut TooltipManager,
        tooltip: Tooltip<'a>,
    ) -> Tooltipped<'a, Self>
    where
        Self: std::marker::Sized;
}
impl<W: Widget> Tooltipable for W {
    fn with_tooltip<'a>(
        self,
        tooltip_manager: &'a mut TooltipManager,
        tooltip: Tooltip<'a>,
    ) -> Tooltipped<'a, W> {
        Tooltipped {
            inner: self,
            tooltip_manager,
            tooltip,
        }
    }
}

/// A widget for displaying tooltips
#[derive(Clone, WidgetCommon)]
pub struct Tooltip<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    title_text: &'a str,
    desc_text: &'a str,
    style: Style,
    transparency: f32,
}

#[derive(Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {
    #[conrod(default = "theme.background_color")]
    pub color: Option<Color>,
    title: widget::text::Style,
    desc: widget::text::Style,
    // add background imgs here
}

widget_ids! {
    struct Ids {
        title,
        desc,
        back_rect,
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Tooltip<'a> {
    pub fn new(title: &'a str, desc: &'a str) -> Self {
        Tooltip {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            title_text: title,
            desc_text: desc,
            transparency: 1.0,
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

    // TODO: add method(s) to make children widgets and use that to determine height in height function (and in update to draw the widgets)

    /// Specify the font used for displaying the text.
    pub fn font_id(mut self, font_id: text::font::Id) -> Self {
        self.style.title.font_id = Some(Some(font_id));
        self.style.desc.font_id = Some(Some(font_id));
        self
    }

    builder_methods! {
        pub title_text_color { style.title.color = Some(Color) }
        pub desc_text_color { style.desc.color = Some(Color) }
        pub title_font_size { style.title.font_size = Some(FontSize) }
        pub desc_font_size { style.desc.font_size = Some(FontSize) }
        pub title_justify { style.title.justify = Some(text::Justify) }
        pub desc_justify { style.desc.justify = Some(text::Justify) }
        transparency { transparency = f32 }
    }
}

impl<'a> Widget for Tooltip<'a> {
    type State = State;
    type Style = Style;
    type Event = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) {
        let widget::UpdateArgs {
            id,
            state,
            rect,
            style,
            ui,
            ..
        } = args;

        // Apply transparency
        let color = style.color(ui.theme()).alpha(self.transparency);

        // Background rectangle
        widget::Rectangle::fill(rect.dim())
            .xy(rect.xy())
            .graphics_for(id)
            .parent(id)
            .color(color)
            .set(state.ids.back_rect, ui);

        // Title of tooltip
        widget::Text::new(self.title_text)
            .w(rect.w())
            .graphics_for(id)
            .parent(id)
            .top_left_with_margins_on(state.ids.back_rect, 5.0, 5.0)
            .with_style(self.style.title)
            // Apply transparency
            .color(style.title.color(ui.theme()).alpha(self.transparency))
            .set(state.ids.title, ui);

        // Description of tooltip
        widget::Text::new(self.desc_text)
            .w(rect.w())
            .graphics_for(id)
            .parent(id)
            .down_from(state.ids.title, 10.0)
            .with_style(self.style.desc)
            // Apply transparency
            .color(style.desc.color(ui.theme()).alpha(self.transparency))
            .set(state.ids.desc, ui);
    }
}

impl<'a> Colorable for Tooltip<'a> {
    builder_method!(color { style.color = Some(Color) });
}
