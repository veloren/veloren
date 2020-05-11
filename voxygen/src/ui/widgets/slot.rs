//! A widget for selecting a single value along some linear range.
use conrod_core::{
    builder_methods, image,
    input::state::mouse,
    text::font,
    widget::{self, Image, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
use vek::*;

const AMOUNT_SHADOW_OFFSET: [f64; 2] = [1.0, 1.0];

pub trait SlotKey<C, I>: Copy {
    type ImageKey: PartialEq + Send + 'static;
    /// Returns an Option since the slot could be empty
    fn image_key(&self, source: &C) -> Option<(Self::ImageKey, Option<Color>)>;
    fn amount(&self, source: &C) -> Option<u32>;
    fn image_id(key: &Self::ImageKey, source: &I) -> image::Id;
}

pub trait SumSlot: Sized + PartialEq + Copy + Send + 'static {}

pub struct ContentSize {
    // Width divided by height
    pub width_height_ratio: f32,
    // Max fraction of slot widget size that each side can be
    pub max_fraction: f32,
}

pub struct SlotMaker<'a, C, I, S: SumSlot> {
    pub empty_slot: image::Id,
    pub filled_slot: image::Id,
    pub selected_slot: image::Id,
    // Is this useful?
    pub background_color: Option<Color>,
    pub content_size: ContentSize,
    // How to scale content size relative to base content size when selected
    pub selected_content_scale: f32,
    pub amount_font: font::Id,
    pub amount_font_size: u32,
    pub amount_margins: Vec2<f32>,
    pub amount_text_color: Color,
    pub content_source: &'a C,
    pub image_source: &'a I,
    pub slot_manager: Option<&'a mut SlotManager<S>>,
}

impl<'a, C, I, S> SlotMaker<'a, C, I, S>
where
    S: SumSlot,
{
    pub fn fabricate<K: SlotKey<C, I> + Into<S>>(
        &mut self,
        contents: K,
        wh: [f32; 2],
    ) -> Slot<K, C, I, S> {
        let content_size = {
            let ContentSize {
                max_fraction,
                width_height_ratio,
            } = self.content_size;
            let w_max = max_fraction * wh[0];
            let h_max = max_fraction * wh[1];
            let max_ratio = w_max / h_max;
            let (w, h) = if max_ratio > width_height_ratio {
                (width_height_ratio * h_max, w_max)
            } else {
                (w_max, w_max / width_height_ratio)
            };
            Vec2::new(w, h)
        };
        Slot::new(
            contents,
            self.empty_slot,
            self.filled_slot,
            self.selected_slot,
            content_size,
            self.selected_content_scale,
            self.amount_font,
            self.amount_font_size,
            self.amount_margins,
            self.amount_text_color,
            self.content_source,
            self.image_source,
        )
        .wh([wh[0] as f64, wh[1] as f64])
        .and_then(self.background_color, |s, c| s.with_background_color(c))
        .and_then(self.slot_manager.as_mut(), |s, m| s.with_manager(m))
    }
}

#[derive(Clone, Copy)]
enum ManagerState<K> {
    Dragging(widget::Id, K, image::Id),
    Selected(widget::Id, K),
    Idle,
}

enum Interaction {
    Selected,
    Dragging,
    None,
}

pub enum Event<K> {
    // Dragged to another slot
    Dragged(K, K),
    // Dragged to open space
    Dropped(K),
    // Clicked while selected
    Used(K),
}
// Handles interactions with slots
pub struct SlotManager<S: SumSlot> {
    state: ManagerState<S>,
    // Rebuilt every frame
    slot_ids: Vec<widget::Id>,
    // Rebuilt every frame
    slots: Vec<S>,
    events: Vec<Event<S>>,
    // Widget id for dragging image
    drag_id: widget::Id,
    // Size to display dragged content
    // Note: could potentially be specialized for each slot if needed
    drag_img_size: Vec2<f32>,
}

impl<S> SlotManager<S>
where
    S: SumSlot,
{
    pub fn new(mut gen: widget::id::Generator, drag_img_size: Vec2<f32>) -> Self {
        Self {
            state: ManagerState::Idle,
            slot_ids: Vec::new(),
            slots: Vec::new(),
            events: Vec::new(),
            drag_id: gen.next(),
            drag_img_size,
        }
    }

    pub fn maintain(&mut self, ui: &mut conrod_core::UiCell) -> Vec<Event<S>> {
        // Clear
        let slot_ids = std::mem::replace(&mut self.slot_ids, Vec::new());
        let slots = std::mem::replace(&mut self.slots, Vec::new());

        // Detect drops by of selected item by clicking in empty space
        if let ManagerState::Selected(_, slot) = self.state {
            if ui.widget_input(ui.window).clicks().left().next().is_some() {
                self.state = ManagerState::Idle;
                self.events.push(Event::Dropped(slot));
            }
        }

        // If dragging and mouse is released check if there is a slot widget under the
        // mouse
        if let ManagerState::Dragging(_, slot, content_img) = &self.state {
            let content_img = *content_img;
            let input = &ui.global_input().current;
            if let mouse::ButtonPosition::Up = input.mouse.buttons.left() {
                // Get widget under the mouse
                if let Some(id) = input.widget_under_mouse {
                    // If over the window widget drop the contents
                    if id == ui.window {
                        self.events.push(Event::Dropped(*slot));
                    } else if let Some(idx) = slot_ids.iter().position(|slot_id| *slot_id == id) {
                        // If widget is a slot widget swap with it
                        let (from, to) = (*slot, slots[idx]);
                        // Don't drag if it is the same slot
                        if from != to {
                            self.events.push(Event::Dragged(from, to));
                        }
                    }
                }
                // Mouse released stop dragging
                self.state = ManagerState::Idle;
            }
            // Draw image of contents being dragged
            let [mouse_x, mouse_y] = input.mouse.xy;
            let size = self.drag_img_size.map(|e| e as f64).into_array();
            super::ghost_image::GhostImage::new(content_img)
                .wh(size)
                .xy([mouse_x, mouse_y])
                .set(self.drag_id, ui);
        }

        std::mem::replace(&mut self.events, Vec::new())
    }

    fn update(
        &mut self,
        widget: widget::Id,
        slot: S,
        ui: &conrod_core::Ui,
        content_img: Option<image::Id>,
    ) -> Interaction {
        // Add to list of slots
        self.slot_ids.push(widget);
        self.slots.push(slot);

        let filled = content_img.is_some();
        // If the slot is no longer filled deselect it or cancel dragging
        match &self.state {
            ManagerState::Selected(id, _) | ManagerState::Dragging(id, _, _)
                if *id == widget && !filled =>
            {
                self.state = ManagerState::Idle;
            }
            _ => (),
        }

        // If this is the selected/dragged widget make sure the slot value is up to date
        match &mut self.state {
            ManagerState::Selected(id, stored_slot)
            | ManagerState::Dragging(id, stored_slot, _)
                if *id == widget =>
            {
                *stored_slot = slot
            },
            _ => (),
        }

        let input = ui.widget_input(widget);
        // TODO: make more robust wrt multiple events in the same frame (eg event order
        // may matter) TODO: handle taps as well
        let click_count = input.clicks().left().count();
        if click_count > 0 {
            let odd_num_clicks = click_count % 2 == 1;
            self.state = if let ManagerState::Selected(id, other_slot) = self.state {
                if id != widget {
                    // Swap
                    if slot != other_slot {
                        self.events.push(Event::Dragged(other_slot, slot));
                    }
                    if click_count == 1 {
                        ManagerState::Idle
                    } else if click_count == 2 {
                        // Was clicked again
                        ManagerState::Selected(widget, slot)
                    } else {
                        // Clicked more than once after swap, use and deselect
                        self.events.push(Event::Used(slot));
                        ManagerState::Idle
                    }
                } else {
                    // Clicked widget was already selected
                    // Deselect and emit use if clicked while selected
                    self.events.push(Event::Used(slot));
                    ManagerState::Idle
                }
            } else {
                // No widgets were selected
                if odd_num_clicks && filled {
                    ManagerState::Selected(widget, slot)
                } else {
                    // Selected and then deselected with one or more clicks
                    ManagerState::Idle
                }
            };
        }

        // Use on right click if not dragging
        if input.clicks().right().next().is_some() {
            match self.state {
                ManagerState::Selected(_, _) | ManagerState::Idle => {
                    self.events.push(Event::Used(slot));
                    // If something is selected, deselect
                    self.state = ManagerState::Idle;
                },
                ManagerState::Dragging(_, _, _) => {},
            }
        }

        // If not dragging and there is a drag event on this slot start dragging
        if input.drags().left().next().is_some()
            && !matches!(self.state, ManagerState::Dragging(_, _, _))
        {
            // Start dragging if widget is filled
            if let Some(img) = content_img {
                self.state = ManagerState::Dragging(widget, slot, img);
            }
        }

        // Determine whether this slot is being interacted with
        match self.state {
            ManagerState::Selected(id, _) if id == widget => Interaction::Selected,
            ManagerState::Dragging(id, _, _) if id == widget => Interaction::Dragging,
            _ => Interaction::None,
        }
    }

    /// Returns Some(slot) if a slot is selected
    pub fn selected(&self) -> Option<S> {
        if let ManagerState::Selected(_, s) = self.state {
            Some(s)
        } else {
            None
        }
    }

    /// Sets the SlotManager into an idle state
    pub fn idle(&mut self) { self.state = ManagerState::Idle; }
}

#[derive(WidgetCommon)]
pub struct Slot<'a, K: SlotKey<C, I> + Into<S>, C, I, S: SumSlot> {
    slot_key: K,

    // Images for slot background and frame
    empty_slot: image::Id,
    filled_slot: image::Id,
    selected_slot: image::Id,
    background_color: Option<Color>,

    // Size of content image
    content_size: Vec2<f32>,
    selected_content_scale: f32,

    icon: Option<(image::Id, Vec2<f32>, Option<Color>)>,

    // Amount styling
    amount_font: font::Id,
    amount_font_size: u32,
    amount_margins: Vec2<f32>,
    amount_text_color: Color,

    slot_manager: Option<&'a mut SlotManager<S>>,
    // Should we just pass in the ImageKey?
    content_source: &'a C,
    image_source: &'a I,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

widget_ids! {
    // Note: icon, amount, and amount_bg are not always used. Is there any cost to having them?
    struct Ids {
        background,
        icon,
        amount,
        amount_bg,
        content,
    }
}

/// Represents the state of the Slot widget.
pub struct State<K> {
    ids: Ids,
    cached_image: Option<(K, image::Id)>,
}

impl<'a, K, C, I, S> Slot<'a, K, C, I, S>
where
    K: SlotKey<C, I> + Into<S>,
    S: SumSlot,
{
    builder_methods! {
        pub with_background_color { background_color = Some(Color) }
    }

    pub fn with_manager(mut self, slot_manager: &'a mut SlotManager<S>) -> Self {
        self.slot_manager = Some(slot_manager);
        self
    }

    pub fn with_icon(mut self, img: image::Id, size: Vec2<f32>, color: Option<Color>) -> Self {
        self.icon = Some((img, size, color));
        self
    }

    fn new(
        slot_key: K,
        empty_slot: image::Id,
        filled_slot: image::Id,
        selected_slot: image::Id,
        content_size: Vec2<f32>,
        selected_content_scale: f32,
        amount_font: font::Id,
        amount_font_size: u32,
        amount_margins: Vec2<f32>,
        amount_text_color: Color,
        content_source: &'a C,
        image_source: &'a I,
    ) -> Self {
        Self {
            slot_key,
            empty_slot,
            filled_slot,
            selected_slot,
            background_color: None,
            content_size,
            selected_content_scale,
            icon: None,
            amount_font,
            amount_font_size,
            amount_margins,
            amount_text_color,
            slot_manager: None,
            content_source,
            image_source,
            common: widget::CommonBuilder::default(),
        }
    }
}

impl<'a, K, C, I, S> Widget for Slot<'a, K, C, I, S>
where
    K: SlotKey<C, I> + Into<S>,
    S: SumSlot,
{
    type Event = ();
    type State = State<K::ImageKey>;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            cached_image: None,
        }
    }

    fn style(&self) -> Self::Style { () }

    /// Update the state of the Slider.
    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            rect,
            ui,
            ..
        } = args;
        let Slot {
            slot_key,
            empty_slot,
            filled_slot,
            selected_slot,
            background_color,
            content_size,
            selected_content_scale,
            icon,
            amount_font,
            amount_font_size,
            amount_margins,
            amount_text_color,
            content_source,
            image_source,
            ..
        } = self;

        // If the key changed update the cached image id
        let (image_key, content_color) = slot_key
            .image_key(content_source)
            .map_or((None, None), |(i, c)| (Some(i), c));
        if state.cached_image.as_ref().map(|c| &c.0) != image_key.as_ref() {
            state.update(|state| {
                state.cached_image = image_key.map(|key| {
                    let image_id = K::image_id(&key, &image_source);
                    (key, image_id)
                });
            });
        }

        // Get image ids
        let content_image = state.cached_image.as_ref().map(|c| c.1);
        // Get whether this slot is selected
        let interaction = self.slot_manager.map_or(Interaction::None, |m| {
            m.update(id, slot_key.into(), ui, content_image)
        });
        // No content if it is being dragged
        let content_image = if let Interaction::Dragging = interaction {
            None
        } else {
            content_image
        };
        // Go back to getting image ids
        let slot_image = if let Interaction::Selected = interaction {
            selected_slot
        } else if content_image.is_some() {
            filled_slot
        } else {
            empty_slot
        };

        // Get amount (None => no amount text)
        let amount = if let Interaction::Dragging = interaction {
            None // Don't show amount if being dragged
        } else {
            slot_key.amount(content_source)
        };

        // Get slot widget dimensions and position
        let (x, y, w, h) = rect.x_y_w_h();

        // Draw slot frame/background
        Image::new(slot_image)
            .x_y(x, y)
            .w_h(w, h)
            .parent(id)
            .graphics_for(id)
            .color(background_color)
            .set(state.ids.background, ui);

        // Draw icon (only when there is not content)
        // Note: this could potentially be done by the user instead
        if let (Some((icon_image, size, color)), true) = (icon, content_image.is_none()) {
            let wh = size.map(|e| e as f64).into_array();
            Image::new(icon_image)
                .x_y(x, y)
                .wh(wh)
                .parent(id)
                .graphics_for(id)
                .color(color)
                .set(state.ids.icon, ui);
        }

        // Draw contents
        if let Some(content_image) = content_image {
            Image::new(content_image)
                .x_y(x, y)
                .wh((content_size
                    * if let Interaction::Selected = interaction {
                        selected_content_scale
                    } else {
                        1.0
                    })
                .map(|e| e as f64)
                .into_array())
                .color(content_color)
                .parent(id)
                .graphics_for(id)
                .set(state.ids.content, ui);
        }

        // Draw amount
        if let Some(amount) = amount {
            let amount = format!("{}", &amount);
            // Text shadow
            Text::new(&amount)
                .font_id(amount_font)
                .font_size(amount_font_size)
                .bottom_right_with_margins_on(
                    state.ids.content,
                    amount_margins.x as f64,
                    amount_margins.y as f64,
                )
                .parent(id)
                .graphics_for(id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.amount_bg, ui);
            Text::new(&amount)
                .parent(id)
                .graphics_for(id)
                .bottom_left_with_margins_on(
                    state.ids.amount_bg,
                    AMOUNT_SHADOW_OFFSET[0],
                    AMOUNT_SHADOW_OFFSET[1],
                )
                .font_id(amount_font)
                .font_size(amount_font_size)
                .color(amount_text_color)
                .set(state.ids.amount, ui);
        }
    }
}
