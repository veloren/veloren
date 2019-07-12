//! A widget for selecting a single value along some linear range.
use conrod_core::{
    builder_methods, image,
    position::Range,
    utils,
    widget::{self, Image, Rectangle},
    widget_ids, Color, Colorable, Positionable, Rect, Sizeable, UiCell, Widget, WidgetCommon,
};

/// Linear value selection.
///
/// If the slider's width is greater than its height, it will automatically become a horizontal
/// slider, otherwise it will be a vertical slider.
///
/// Its reaction is triggered if the value is updated or if the mouse button is released while
/// the cursor is above the rectangle.
#[derive(WidgetCommon)]
pub struct ImageFrame {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    // TODO: use type that is not just an array
    // Edge images [t, b, r, l]
    edges: [image::Id; 4],
    edge_src_rects: [Option<Rect>; 4],
    // Corner images [tr, tl, br, bl]
    corners: [image::Id; 4],
    corner_src_rects: [Option<Rect>; 4],
    // Center
    center: Center,
    // Thickness of the frame border, determines the size used for the edge and corner images
    border_size: BorderSize,
    // Color to apply to all images making up the image frame
    color: Option<Color>,
    // TODO: would it be useful to have an optional close button be a part of this?
}

enum Center {
    Plain(Color),
    Image(image::Id, Option<Rect>),
}
impl From<Color> for Center {
    fn from(color: Color) -> Self {
        Center::Plain(color)
    }
}
impl From<image::Id> for Center {
    fn from(image: image::Id) -> Self {
        Center::Image(image, None)
    }
}
impl From<(image::Id, Rect)> for Center {
    fn from((image, src_rect): (image::Id, Rect)) -> Self {
        Center::Image(image, Some(src_rect))
    }
}

struct BorderSize {
    top: f64,
    bottom: f64,
    right: f64,
    left: f64,
}
impl From<f64> for BorderSize {
    fn from(width: f64) -> Self {
        BorderSize {
            top: width,
            bottom: width,
            right: width,
            left: width,
        }
    }
}
impl From<[f64; 2]> for BorderSize {
    fn from([vertical, horizontal]: [f64; 2]) -> Self {
        BorderSize {
            top: horizontal,
            bottom: horizontal,
            right: vertical,
            left: vertical,
        }
    }
}
impl From<[f64; 4]> for BorderSize {
    fn from(vals: [f64; 4]) -> Self {
        BorderSize {
            top: vals[0],
            bottom: vals[1],
            right: vals[2],
            left: vals[3],
        }
    }
}

widget_ids! {
    struct Ids {
        center_plain,
        center_image,
        right,
        top_right,
        top,
        top_left,
        left,
        bottom_left,
        bottom,
        bottom_right,
    }
}

/// Represents the state of the ImageFrame widget.
pub struct State {
    ids: Ids,
}

impl ImageFrame {
    fn new(
        edges: [image::Id; 4],
        corners: [image::Id; 4],
        center: impl Into<Center>,
        border_size: impl Into<BorderSize>,
    ) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            edges,
            edge_src_rects: [None; 4],
            corners,
            corner_src_rects: [None; 4],
            center: center.into(),
            border_size: border_size.into(),
            color: None,
        }
    }

    builder_methods! {
        pub edge_src_rects { edge_src_rects = [Option<Rect>; 4] }
        pub corner_src_rects { corner_src_rects = [Option<Rect>; 4] }
    }
}

impl Widget for ImageFrame {
    type State = State;
    type Style = ();
    type Event = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    /// Update the state of the ImageFrame
    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            rect,
            ui,
            ..
        } = args;

        let (frame_w, frame_h) = rect.w_h();

        let t_height = self.border_size.top.min(frame_h);
        let b_height = self.border_size.bottom.min(frame_h);
        let r_width = self.border_size.right.min(frame_w);
        let l_width = self.border_size.left.min(frame_w);
        let inner_width = (frame_w - r_width - l_width).max(0.0);
        let inner_height = (frame_h - t_height - b_height).max(0.0);

        let r_rect = Rect::from_xy_dim(
            [rect.x() + (inner_width + r_width) / 2.0, rect.y()],
            [r_width, inner_height],
        );
        let tr_rect = Rect::from_xy_dim(
            [
                rect.x() + (inner_width + r_width) / 2.0,
                rect.y() + (inner_height + t_height) / 2.0,
            ],
            [r_width, t_height],
        );
        let t_rect = Rect::from_xy_dim(
            [rect.x(), rect.y() + (inner_height + t_height) / 2.0],
            [inner_width, t_height],
        );
        let tl_rect = Rect::from_xy_dim(
            [
                rect.x() - (inner_width + l_width) / 2.0,
                rect.y() + (inner_height + t_height) / 2.0,
            ],
            [l_width, t_height],
        );
        let l_rect = Rect::from_xy_dim(
            [rect.x() - (inner_width + l_width) / 2.0, rect.y()],
            [l_width, inner_height],
        );
        let bl_rect = Rect::from_xy_dim(
            [
                rect.x() - (inner_width + l_width) / 2.0,
                rect.y() - (inner_height + b_height) / 2.0,
            ],
            [l_width, b_height],
        );
        let b_rect = Rect::from_xy_dim(
            [rect.x(), rect.y() - (inner_height + b_height) / 2.0],
            [inner_width, b_height],
        );
        let br_rect = Rect::from_xy_dim(
            [
                rect.x() + (inner_width + r_width) / 2.0,
                rect.y() - (inner_height + b_height) / 2.0,
            ],
            [r_width, b_height],
        );

        let maybe_color = self.color;
        let set_image = |image_id, rect: Rect, maybe_src_rect, widget_id, ui: &mut UiCell| {
            Image::new(image_id)
                .xy(rect.xy())
                .wh(rect.dim())
                .parent(id)
                .graphics_for(id)
                .color(maybe_color)
                .set(widget_id, ui);
        };
        // Right edge
        set_image(
            self.edges[2],
            r_rect,
            self.edge_src_rects[2],
            state.ids.right,
            ui,
        );
        // Top-right corner
        set_image(
            self.corners[0],
            tr_rect,
            self.corner_src_rects[0],
            state.ids.top_right,
            ui,
        );
        // Top edge
        set_image(
            self.edges[0],
            t_rect,
            self.edge_src_rects[0],
            state.ids.top,
            ui,
        );
        // Top-left corner
        set_image(
            self.corners[1],
            tl_rect,
            self.corner_src_rects[1],
            state.ids.top_left,
            ui,
        );
        // Left edge
        set_image(
            self.edges[3],
            l_rect,
            self.edge_src_rects[3],
            state.ids.left,
            ui,
        );
        // Bottom-left corner
        set_image(
            self.corners[3],
            bl_rect,
            self.corner_src_rects[3],
            state.ids.bottom_left,
            ui,
        );
        // Bottom edge
        set_image(
            self.edges[1],
            b_rect,
            self.edge_src_rects[1],
            state.ids.bottom,
            ui,
        );
        // Bottom-right corner
        set_image(
            self.corners[2],
            br_rect,
            self.corner_src_rects[2],
            state.ids.bottom_right,
            ui,
        );

        // Center,
        match self.center {
            Center::Plain(color) => {
                Rectangle::fill_with([inner_width, inner_height], color)
                    .xy(rect.xy())
                    .parent(id)
                    .graphics_for(id)
                    .and_then(maybe_color, |w, c| w.color(c))
                    .set(state.ids.center_plain, ui);
            }
            Center::Image(image_id, maybe_src_rect) => {
                set_image(
                    image_id,
                    Rect::from_xy_dim(rect.xy(), [inner_width, inner_height]),
                    maybe_src_rect,
                    state.ids.center_image,
                    ui,
                );
            }
        }
    }
}

impl Colorable for ImageFrame {
    fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}
