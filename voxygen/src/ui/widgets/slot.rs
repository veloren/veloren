//! A widget for selecting a single value along some linear range.
use crate::hud::animate_by_pulse;
use conrod_core::{
    builder_methods, image,
    input::{keyboard::ModifierKey, state::mouse},
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
    fn image_ids(key: &Self::ImageKey, source: &I) -> Vec<image::Id>;
}

pub trait SumSlot: Sized + PartialEq + Copy + Send + 'static {
    fn drag_size(&self) -> Option<[f64; 2]>;
}

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
    pub pulse: f32,
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
            self.selected_slot,
            self.filled_slot,
            content_size,
            self.selected_content_scale,
            self.amount_font,
            self.amount_font_size,
            self.amount_margins,
            self.amount_text_color,
            self.content_source,
            self.image_source,
            self.pulse,
        )
        .wh([wh[0] as f64, wh[1] as f64])
        .and_then(self.background_color, |s, c| s.with_background_color(c))
        .and_then(self.slot_manager.as_mut(), |s, m| s.with_manager(m))
    }
}

#[derive(Clone, Copy)]
enum ManagerState<K> {
    Dragging(
        widget::Id,
        K,
        image::Id,
        /// Amount of items being dragged in the stack.
        Option<u32>,
    ),
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
    // Dropped half of the stack
    SplitDropped(K),
    // Dragged half of the stack
    SplitDragged(K, K),
    // Clicked while selected
    Used(K),
    // {Shift,Ctrl}-clicked
    Request { slot: K, auto_quantity: bool },
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
    pub mouse_over_slot: Option<S>,
    // Si prefixes settings
    use_prefixes: bool,
    prefix_switch_point: u32,
    /* TODO(heyzoos) Will be useful for whoever works on rendering the number of items "in
     * hand".
     *
     * drag_amount_id: widget::Id,
     * drag_amount_shadow_id: widget::Id, */

    /* Asset ID pointing to a font set.
     * amount_font: font::Id, */

    /* Specifies the size of the font used to display number of items held in
     * a stack when dragging.
     * amount_font_size: u32, */

    /* Specifies how much space should be used in the margins of the item
     * amount relative to the slot.
     * amount_margins: Vec2<f32>, */

    /* Specifies the color of the text used to display the number of items held
     * in a stack when dragging.
     * amount_text_color: Color, */
}

impl<S> SlotManager<S>
where
    S: SumSlot,
{
    pub fn new(
        mut gen: widget::id::Generator,
        drag_img_size: Vec2<f32>,
        use_prefixes: bool,
        prefix_switch_point: u32,
        /* TODO(heyzoos) Will be useful for whoever works on rendering the number of items "in
         * hand". amount_font: font::Id,
         * amount_margins: Vec2<f32>,
         * amount_font_size: u32,
         * amount_text_color: Color, */
    ) -> Self {
        Self {
            state: ManagerState::Idle,
            slot_ids: Vec::new(),
            slots: Vec::new(),
            events: Vec::new(),
            drag_id: gen.next(),
            mouse_over_slot: None,
            use_prefixes,
            prefix_switch_point,
            // TODO(heyzoos) Will be useful for whoever works on rendering the number of items "in
            // hand". drag_amount_id: gen.next(),
            // drag_amount_shadow_id: gen.next(),
            // amount_font,
            // amount_font_size,
            // amount_margins,
            // amount_text_color,
            drag_img_size,
        }
    }

    pub fn maintain(&mut self, ui: &mut conrod_core::UiCell) -> Vec<Event<S>> {
        // Clear
        let slot_ids = core::mem::take(&mut self.slot_ids);
        let slots = core::mem::take(&mut self.slots);

        // Detect drops by of selected item by clicking in empty space
        if let ManagerState::Selected(_, slot) = self.state {
            if ui.widget_input(ui.window).clicks().left().next().is_some() {
                self.state = ManagerState::Idle;
                self.events.push(Event::Dropped(slot));
            }
        }

        let input = &ui.global_input().current;
        self.mouse_over_slot = input
            .widget_under_mouse
            .and_then(|x| slot_ids.iter().position(|slot_id| *slot_id == x))
            .map(|x| slots[x]);

        // If dragging and mouse is released check if there is a slot widget under the
        // mouse
        if let ManagerState::Dragging(_, slot, content_img, drag_amount) = &self.state {
            let content_img = *content_img;
            let drag_amount = *drag_amount;

            let dragged_size = if let Some(dragged_size) = slot.drag_size() {
                dragged_size
            } else {
                self.drag_img_size.map(|e| e as f64).into_array()
            };

            // If we are dragging and we right click, drop half the stack
            // on the ground or into the slot under the cursor. This only
            // works with open slots or slots containing the same kind of
            // item.

            if drag_amount.is_some() {
                if let Some(id) = input.widget_under_mouse {
                    if ui.widget_input(id).clicks().right().next().is_some() {
                        if id == ui.window {
                            let temp_slot = *slot;
                            self.events.push(Event::SplitDropped(temp_slot));
                        } else if let Some(idx) = slot_ids.iter().position(|slot_id| *slot_id == id)
                        {
                            let (from, to) = (*slot, slots[idx]);
                            if from != to {
                                self.events.push(Event::SplitDragged(from, to));
                            }
                        }
                    }
                }
            }

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
            super::ghost_image::GhostImage::new(content_img)
                .wh(dragged_size)
                .xy([mouse_x, mouse_y])
                .set(self.drag_id, ui);

            // TODO(heyzoos) Will be useful for whoever works on rendering the
            // number of items "in hand".
            //
            // if let Some(drag_amount) = drag_amount {
            //     Text::new(format!("{}", drag_amount).as_str())
            //         .parent(self.drag_id)
            //         .font_id(self.amount_font)
            //         .font_size(self.amount_font_size)
            //         .bottom_right_with_margins_on(
            //             self.drag_id,
            //             self.amount_margins.x as f64,
            //             self.amount_margins.y as f64,
            //         )
            //         .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            //         .set(self.drag_amount_shadow_id, ui);
            //     Text::new(format!("{}", drag_amount).as_str())
            //         .parent(self.drag_id)
            //         .font_id(self.amount_font)
            //         .font_size(self.amount_font_size)
            //         .bottom_right_with_margins_on(
            //             self.drag_id,
            //             self.amount_margins.x as f64,
            //             self.amount_margins.y as f64,
            //         )
            //         .color(self.amount_text_color)
            //         .set(self.drag_amount_id, ui);
            // }
        }

        core::mem::take(&mut self.events)
    }

    pub fn set_use_prefixes(&mut self, use_prefixes: bool) { self.use_prefixes = use_prefixes; }

    pub fn set_prefix_switch_point(&mut self, prefix_switch_point: u32) {
        self.prefix_switch_point = prefix_switch_point;
    }

    fn update(
        &mut self,
        widget: widget::Id,
        slot: S,
        ui: &conrod_core::Ui,
        content_img: Option<Vec<image::Id>>,
        drag_amount: Option<u32>,
    ) -> Interaction {
        // Add to list of slots
        self.slot_ids.push(widget);
        self.slots.push(slot);

        let filled = content_img.is_some();
        // If the slot is no longer filled deselect it or cancel dragging
        match &self.state {
            ManagerState::Selected(id, _) | ManagerState::Dragging(id, _, _, _)
                if *id == widget && !filled =>
            {
                self.state = ManagerState::Idle;
            },
            _ => (),
        }

        // If this is the selected/dragged widget make sure the slot value is up to date
        match &mut self.state {
            ManagerState::Selected(id, stored_slot)
            | ManagerState::Dragging(id, stored_slot, _, _)
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

        // Translate ctrl-clicks to stack-requests and shift-clicks to
        // individual-requests
        if let Some(click) = input.clicks().left().next() {
            if !matches!(self.state, ManagerState::Dragging(_, _, _, _)) {
                match click.modifiers {
                    ModifierKey::CTRL => {
                        self.events.push(Event::Request {
                            slot,
                            auto_quantity: true,
                        });
                        self.state = ManagerState::Idle;
                    },
                    ModifierKey::SHIFT => {
                        self.events.push(Event::Request {
                            slot,
                            auto_quantity: false,
                        });
                        self.state = ManagerState::Idle;
                    },
                    _ => {},
                }
            }
        }

        // Use on right click if not dragging
        if input.clicks().right().next().is_some() {
            match self.state {
                ManagerState::Selected(_, _) | ManagerState::Idle => {
                    self.events.push(Event::Used(slot));
                    // If something is selected, deselect
                    self.state = ManagerState::Idle;
                },
                ManagerState::Dragging(_, _, _, _) => {},
            }
        }

        // If not dragging and there is a drag event on this slot start dragging
        if input.drags().left().next().is_some()
            && !matches!(self.state, ManagerState::Dragging(_, _, _, _))
        {
            // Start dragging if widget is filled
            if let Some(images) = content_img {
                if !images.is_empty() {
                    self.state = ManagerState::Dragging(widget, slot, images[0], drag_amount);
                }
            }
        }

        // Determine whether this slot is being interacted with
        match self.state {
            ManagerState::Selected(id, _) if id == widget => Interaction::Selected,
            ManagerState::Dragging(id, _, _, _) if id == widget => Interaction::Dragging,
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
    filled_slot: image::Id,
    // Should we just pass in the ImageKey?
    content_source: &'a C,
    image_source: &'a I,

    pulse: f32,

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
    cached_images: Option<(K, Vec<image::Id>)>,
}

impl<'a, K, C, I, S> Slot<'a, K, C, I, S>
where
    K: SlotKey<C, I> + Into<S>,
    S: SumSlot,
{
    builder_methods! {
        pub with_background_color { background_color = Some(Color) }
    }

    #[must_use]
    pub fn with_manager(mut self, slot_manager: &'a mut SlotManager<S>) -> Self {
        self.slot_manager = Some(slot_manager);
        self
    }

    #[must_use]
    pub fn filled_slot(mut self, img: image::Id) -> Self {
        self.filled_slot = img;
        self
    }

    #[must_use]
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
        pulse: f32,
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
            pulse,
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
            cached_images: None,
        }
    }

    fn style(&self) -> Self::Style {}

    /// Update the state of the Slot.
    #[allow(clippy::useless_asref)] // false positive
    fn update(mut self, args: widget::UpdateArgs<Self>) -> Self::Event {
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
        if state.cached_images.as_ref().map(|c| &c.0) != image_key.as_ref() {
            state.update(|state| {
                state.cached_images = image_key.map(|key| {
                    let image_ids = K::image_ids(&key, image_source);
                    (key, image_ids)
                });
            });
        }

        // Get image ids
        let content_images = state.cached_images.as_ref().map(|c| c.1.clone());
        // Get whether this slot is selected
        let interaction = self.slot_manager.as_mut().map_or(Interaction::None, |m| {
            m.update(
                id,
                slot_key.into(),
                ui,
                content_images.clone(),
                slot_key.amount(content_source),
            )
        });
        // No content if it is being dragged
        let content_images = if let Interaction::Dragging = interaction {
            None
        } else {
            content_images
        };
        // Go back to getting image ids
        let slot_image = if let Interaction::Selected = interaction {
            selected_slot
        } else if content_images.is_some() {
            self.filled_slot
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
        if let (Some((icon_image, size, color)), true) = (icon, content_images.is_none()) {
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
        if let Some(content_images) = content_images {
            Image::new(animate_by_pulse(&content_images, self.pulse))
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
            let amount = match self
                .slot_manager
                .as_ref()
                .map_or(true, |sm| sm.use_prefixes)
            {
                true => {
                    let threshold = amount
                        / (u32::pow(
                            10,
                            self.slot_manager
                                .map_or(4, |sm| sm.prefix_switch_point)
                                .saturating_sub(4),
                        ));
                    match amount {
                        amount if threshold >= 1_000_000_000 => {
                            format!("{}G", amount / 1_000_000_000)
                        },
                        amount if threshold >= 1_000_000 => format!("{}M", amount / 1_000_000),
                        amount if threshold >= 1_000 => format!("{}K", amount / 1_000),
                        amount => format!("{}", amount),
                    }
                },
                false => format!("{}", amount),
            };
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
