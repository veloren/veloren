use super::img_ids::Imgs;
use crate::{GlobalState, game_input::GameInput, settings::Button, window::ControllerType};
use conrod_core::{
    Positionable, Sizeable, UiCell, Widget,
    image::Id as ConrodImageId,
    widget::{Id as WidgetId, Image},
};
use gilrs::Button as GilButton;

/// stores the current maximum number of controller inputs for a single action
pub struct LayerIconIds {
    pub main: WidgetId,
    pub modifier1: WidgetId,
    pub modifier2: WidgetId,
}

pub struct IconHandler<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
}

/// returns the left trigger (dark) icon based on controller type
pub fn fetch_skillbar_gamepad_left(ctrl_type: ControllerType, imgs: &Imgs) -> ConrodImageId {
    match ctrl_type {
        ControllerType::Xbox => imgs.left_trigger_xbox_dark,
        ControllerType::Nintendo => imgs.left_trigger_nin_dark,
        ControllerType::Playstation => imgs.left_trigger_ps_dark,
        _ => imgs.m1_ico,
    }
}

/// returns the right trigger (dark) icon based on controller type
pub fn fetch_skillbar_gamepad_right(ctrl_type: ControllerType, imgs: &Imgs) -> ConrodImageId {
    match ctrl_type {
        ControllerType::Xbox => imgs.right_trigger_xbox_dark,
        ControllerType::Nintendo => imgs.right_trigger_nin_dark,
        ControllerType::Playstation => imgs.right_trigger_ps_dark,
        _ => imgs.m2_ico,
    }
}

impl<'a> IconHandler<'a> {
    pub fn new(global_state: &'a GlobalState, imgs: &'a Imgs) -> Self {
        Self { global_state, imgs }
    }

    /// when no icon needs to be displayed, uses the widgets to generate
    /// transparant objects to prevent issues with conrod
    ///
    /// input:
    /// `size` - size of icon to be rendered
    /// `target_id` - conrod id of text that icon is rendered next to
    /// `current_id` - collection of conrod ids reserved for icons (stored in
    /// LayerIconIds) `ui` - UiCell object
    pub fn set_controller_icons_left_none(
        &self,
        size: f64,
        target_id: WidgetId,
        current_id: &LayerIconIds,
        ui: &mut UiCell,
    ) {
        // draw invisible placeholders for all widgets
        let mut anchor_id = target_id;
        let ids = [current_id.main, current_id.modifier1, current_id.modifier2];
        for &icon_id in ids.iter() {
            Image::new(self.imgs.transparant_button)
                .w_h(size, size)
                .mid_left_with_margin_on(anchor_id, -21.0)
                .set(icon_id, ui);
            anchor_id = icon_id;
        }
    }

    /// renders the controller icons for a given GameInput positioned left of
    /// the given widget
    ///
    /// input:
    /// `input` - which input is being displayed
    /// `size` - size of icon
    /// `target_id` - conrod id of text that icon is rendered next to
    /// `current_id` - collection of conrod ids reserved for icons (stored in
    /// LayerIconIds) `ui` - UiCell object
    pub fn set_controller_icons_left(
        &self,
        input: GameInput,
        size: f64,
        target_id: WidgetId,
        current_id: &LayerIconIds,
        ui: &mut UiCell,
    ) -> u8 {
        let main_id = current_id.main;
        let mod1_id = current_id.modifier1;
        let mod2_id = current_id.modifier2;

        let mut icon_draw_list: Vec<(WidgetId, Button)> = Vec::with_capacity(3);
        let unknown = Button::Simple(GilButton::Unknown);
        let mut count: u8 = 0; // # of valid icons displayed

        // prioritize game layers over game buttons
        if let Some(layer_input) = self
            .global_state
            .settings
            .controller
            .get_layer_button_binding(input)
        {
            // draw main layer button
            icon_draw_list.push((main_id, layer_input.button));
            count += 1;

            // draw modifier buttons if any
            let mod1 = layer_input.mod1;
            let mut mod1_valid = false;
            let mod2 = layer_input.mod2;
            let mut mod2_valid = false;

            // place valid buttons first, then place invalid buttons after
            if mod1 != unknown {
                icon_draw_list.push((mod1_id, mod1));
                mod1_valid = true;
                count += 1;
            }
            if mod2 != unknown {
                icon_draw_list.push((mod2_id, mod2));
                mod2_valid = true;
                count += 1;
            }

            // invalid buttons will be rendered as an empty widget because conrod needs all
            // defined widgets to be used (or it will cause issues)
            if !mod1_valid {
                icon_draw_list.push((mod1_id, unknown));
            }
            if !mod2_valid {
                icon_draw_list.push((mod2_id, unknown));
            }
        } else if let Some(button_input) = self
            .global_state
            .settings
            .controller
            .get_game_button_binding(input)
        {
            // draw only button
            icon_draw_list.push((main_id, button_input));
            count += 1;

            // modifiers will be a transparant object to consume conrod modifier widget IDs
            icon_draw_list.push((mod1_id, Button::Simple(GilButton::Unknown)));
            icon_draw_list.push((mod2_id, Button::Simple(GilButton::Unknown)));
        }

        // draw transparant objects if no icons were rendered
        if icon_draw_list.is_empty() {
            self.set_controller_icons_left_none(size, target_id, current_id, ui);
        } else {
            // draw icons
            let mut anchor_id = target_id;
            for (icon_id, button) in icon_draw_list.iter() {
                // draw invisibile placeholder if button is unknown
                if *button == unknown {
                    Image::new(self.imgs.transparant_button)
                        .w_h(size, size)
                        .mid_left_with_margin_on(anchor_id, -21.0)
                        .set(*icon_id, ui);
                } else {
                    Image::new(self.get_controller_icon_id(*button))
                        .w_h(size, size)
                        .mid_left_with_margin_on(anchor_id, -21.0)
                        .set(*icon_id, ui);
                }
                anchor_id = *icon_id;
            }
        }

        count
    }

    fn get_controller_icon_id(&self, binding: Button) -> ConrodImageId {
        let controller_type = self.global_state.window.controller_type();

        let icon_id: ConrodImageId = match binding {
            Button::Simple(GilButton::South) => self.get_south(controller_type),
            Button::Simple(GilButton::East) => self.get_east(controller_type),
            Button::Simple(GilButton::West) => self.get_west(controller_type),
            Button::Simple(GilButton::North) => self.get_north(controller_type),
            Button::Simple(GilButton::LeftThumb) => self.get_left_axis_button(),
            Button::Simple(GilButton::RightThumb) => self.get_right_axis_button(),
            Button::Simple(GilButton::LeftTrigger) => self.get_left_shoulder(controller_type),
            Button::Simple(GilButton::RightTrigger) => self.get_right_shoulder(controller_type),
            Button::Simple(GilButton::LeftTrigger2) => self.get_left_trigger(controller_type),
            Button::Simple(GilButton::RightTrigger2) => self.get_right_trigger(controller_type),
            Button::Simple(GilButton::DPadDown) => self.get_dpad_down(controller_type),
            Button::Simple(GilButton::DPadLeft) => self.get_dpad_left(controller_type),
            Button::Simple(GilButton::DPadRight) => self.get_dpad_right(controller_type),
            Button::Simple(GilButton::DPadUp) => self.get_dpad_up(controller_type),
            Button::Simple(GilButton::Start) => self.get_start(controller_type),
            Button::Simple(GilButton::Select) => self.get_select(controller_type),
            _ => self.imgs.no_button,
        };

        icon_id
    }

    fn get_south(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.south_button_a,
            ControllerType::Playstation => self.imgs.south_button_ps_cross,
            _ => self.imgs.south_button_a,
        }
    }

    fn get_east(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.east_button_b,
            ControllerType::Playstation => self.imgs.east_button_ps_circle,
            _ => self.imgs.east_button_b,
        }
    }

    fn get_west(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.west_button_x,
            ControllerType::Playstation => self.imgs.west_button_ps_square,
            _ => self.imgs.west_button_x,
        }
    }

    fn get_north(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.north_button_y,
            ControllerType::Playstation => self.imgs.north_button_ps_triangle,
            _ => self.imgs.north_button_y,
        }
    }

    //fn get_left_axis(&self) -> ConrodImageId { self.imgs.left_axis }

    fn get_left_axis_button(&self) -> ConrodImageId { self.imgs.left_axis_button }

    //fn get_right_axis(&self) -> ConrodImageId { self.imgs.right_axis }

    fn get_right_axis_button(&self) -> ConrodImageId { self.imgs.right_axis_button }

    fn get_left_shoulder(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.left_shoulder_nin_l,
            ControllerType::Playstation => self.imgs.left_shoulder_ps_l1,
            _ => self.imgs.left_shoulder_xbox_lb,
        }
    }

    fn get_right_shoulder(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.right_shoulder_nin_r,
            ControllerType::Playstation => self.imgs.right_shoulder_ps_r1,
            _ => self.imgs.right_shoulder_xbox_rb,
        }
    }

    fn get_left_trigger(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.left_trigger_nin_zl,
            ControllerType::Playstation => self.imgs.left_trigger_ps_l2,
            _ => self.imgs.left_trigger_xbox_lt,
        }
    }

    fn get_right_trigger(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.right_trigger_nin_zr,
            ControllerType::Playstation => self.imgs.right_trigger_ps_r2,
            _ => self.imgs.right_trigger_xbox_rt,
        }
    }

    fn get_dpad_down(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.dpad_down,
            ControllerType::Playstation => self.imgs.dpad_down_ps,
            _ => self.imgs.dpad_down,
        }
    }

    fn get_dpad_left(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.dpad_left,
            ControllerType::Playstation => self.imgs.dpad_left_ps,
            _ => self.imgs.dpad_left,
        }
    }

    fn get_dpad_right(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.dpad_right,
            ControllerType::Playstation => self.imgs.dpad_right_ps,
            _ => self.imgs.dpad_right,
        }
    }

    fn get_dpad_up(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.dpad_up,
            ControllerType::Playstation => self.imgs.dpad_up_ps,
            _ => self.imgs.dpad_up,
        }
    }

    fn get_start(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Nintendo => self.imgs.start_button_nin,
            ControllerType::Playstation => self.imgs.start_button_ps,
            _ => self.imgs.start_button_xbox,
        }
    }

    fn get_select(&self, controller_type: ControllerType) -> ConrodImageId {
        match controller_type {
            ControllerType::Xbox => self.imgs.select_button_xbox,
            ControllerType::Nintendo => self.imgs.select_button_nin,
            ControllerType::Playstation => self.imgs.select_button_ps,
            _ => self.imgs.select_button_xbox,
        }
    }
}
