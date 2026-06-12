use super::img_ids::Imgs;
use crate::{Settings, game_input::GameInput, settings::Button, window::ControllerType};
use conrod_core::image::Id as ConrodImageId;
use gilrs::Button as GilButton;

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

/// represents an input that has no binding.
pub const UNBOUND_KEY: &str = ":none:";

/// gets a string output for the controller input
///
/// a multi-input action will return like ":mod2: + "mod1: + :main:"
pub fn get_controller_input_string(
    input: GameInput,
    settings: &Settings,
    ctrl: ControllerType,
) -> Option<String> {
    let mut icon_tags = Vec::new();
    let unknown = Button::Simple(GilButton::Unknown);

    // extract just the button name
    let get_tag = |b: Button| {
        let mut name = match b {
            Button::Simple(inner) => format!("{:?}", inner),
            _ => format!("{:?}", b),
        };
        match ctrl {
            ControllerType::Xbox => name.push_str("_x"),
            ControllerType::Nintendo => name.push_str("_n"),
            ControllerType::Playstation => name.push_str("_p"),
            _ => {},
        }
        format!(":{}:", name)
    };

    // prioritize game layers over game buttons
    if let Some(layer) = settings.controller.get_layer_button_binding(input) {
        // add modifiers if they aren't Unknown
        if layer.mod2 != unknown {
            icon_tags.push(get_tag(layer.mod2));
        }
        if layer.mod1 != unknown {
            icon_tags.push(get_tag(layer.mod1));
        }

        // add the main layer button
        icon_tags.push(get_tag(layer.button));
    } else if let Some(button) = settings.controller.get_game_button_binding(input) {
        icon_tags.push(get_tag(button));
    }

    if icon_tags.is_empty() {
        None
    } else {
        Some(icon_tags.join(" + ").to_lowercase())
    }
}

/// returns a ConrodImageId for valid strings
pub fn get_controller_icon_id_from_string(name: &str, imgs: &Imgs) -> ConrodImageId {
    // TODO: either gilrs or we have to swap nintendo buttons to be accurate. Figure
    // it out when controller type detection is working.
    match name {
        "south" | "south_x" | "south_n" => imgs.south_button_a,
        "south_p" => imgs.south_button_ps_cross,
        "east" | "east_x" | "east_n" => imgs.east_button_b,
        "east_p" => imgs.east_button_ps_circle,
        "west" | "west_x" | "west_n" => imgs.west_button_x,
        "west_p" => imgs.west_button_ps_square,
        "north" | "north_x" | "north_n" => imgs.north_button_y,
        "north_p" => imgs.north_button_ps_triangle,
        "leftaxis" | "leftaxis_x" | "leftaxis_n" | "leftaxis_p" => imgs.left_axis,
        "rightaxis" | "rightaxis_x" | "rightaxis_n" | "rightaxis_p" => imgs.right_axis,
        "leftthumb" | "leftthumb_x" | "leftthumb_n" | "leftthumb_p" => imgs.left_axis_button,
        "rightthumb" | "rightthumb_x" | "rightthumb_n" | "rightthumb_p" => imgs.right_axis_button,
        // trigger is shoulder
        "lefttrigger" | "lefttrigger_x" => imgs.left_shoulder_xbox_lb,
        "lefttrigger_n" => imgs.left_shoulder_nin_l,
        "lefttrigger_p" => imgs.left_shoulder_ps_l1,
        "righttrigger" | "righttrigger_x" => imgs.right_shoulder_xbox_rb,
        "righttrigger_n" => imgs.right_shoulder_nin_r,
        "righttrigger_p" => imgs.right_shoulder_ps_r1,
        // trigger2 is trigger
        "lefttrigger2" | "lefttrigger2_x" => imgs.left_trigger_xbox_lt,
        "lefttrigger2_n" => imgs.left_trigger_nin_zl,
        "lefttrigger2_p" => imgs.left_trigger_ps_l2,
        "righttrigger2" | "righttrigger2_x" => imgs.right_trigger_xbox_rt,
        "righttrigger2_n" => imgs.right_trigger_nin_zr,
        "righttrigger2_p" => imgs.right_trigger_ps_r2,
        "dpaddown" | "dpaddown_x" | "dpaddown_n" => imgs.dpad_down,
        "dpaddown_p" => imgs.dpad_down_ps,
        "dpadleft" | "dpadleft_x" | "dpadleft_n" => imgs.dpad_left,
        "dpadleft_p" => imgs.dpad_left_ps,
        "dpadright" | "dpadright_x" | "dpadright_n" => imgs.dpad_right,
        "dpadright_p" => imgs.dpad_right_ps,
        "dpadup" | "dpadup_x" | "dpadup_n" => imgs.dpad_up,
        "dpadup_p" => imgs.dpad_up_ps,
        "start" | "start_x" => imgs.start_button_xbox,
        "start_n" => imgs.start_button_nin,
        "start_p" => imgs.start_button_ps,
        "select" | "select_x" => imgs.select_button_xbox,
        "select_n" => imgs.select_button_nin,
        "select_p" => imgs.select_button_ps,
        "mode" => imgs.start_button_xbox, // TODO: add `mode` button for xbox, nin, ps
        // gilrs supports c and z buttons, so here they are (I'm not making custom icons though)
        "c" => imgs.south_button_a,
        "z" => imgs.east_button_b,
        _ => imgs.no_button,
    }
}
