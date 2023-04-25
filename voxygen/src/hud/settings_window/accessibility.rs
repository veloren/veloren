use crate::{
    hud::{img_ids::Imgs, TEXT_COLOR},
    session::settings_change::{Accessibility as AccessibilityChange, Accessibility::*},
    ui::{fonts::Fonts, ToggleButton},
    GlobalState, render::RenderMode,
};
use conrod_core::{
    color,
    widget::{self, Rectangle, Text},
    widget_ids, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;

widget_ids! {
    struct Ids {
        window,
        window_r,
        flashing_lights_button,
        flashing_lights_label,
        flashing_lights_info_label,
    }
}

#[derive(WidgetCommon)]
pub struct Accessibility<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Accessibility<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            global_state,
            imgs,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Widget for Accessibility<'a> {
    type Event = Vec<AccessibilityChange>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Accessibility::update");
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

        Rectangle::fill_with(args.rect.dim(), color::TRANSPARENT)
            .xy(args.rect.xy())
            .graphics_for(args.id)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.window, ui);
        Rectangle::fill_with([args.rect.w() / 2.0, args.rect.h()], color::TRANSPARENT)
            .top_right()
            .parent(state.ids.window)
            .set(state.ids.window_r, ui);

        // Get render mode
        let render_mode = &self.global_state.settings.graphics.render_mode;

        // Disable flashing lights
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-flashing_lights"),
        )
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .top_left_with_margins_on(state.ids.window, 10.0, 10.0)
        .color(TEXT_COLOR)
        .set(state.ids.flashing_lights_label, ui);

        let flashing_lights_enabled = ToggleButton::new(
            self.global_state
                .settings
                .graphics
                .render_mode
                .flashing_lights_enabled,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.flashing_lights_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.flashing_lights_button, ui);

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-flashing_lights_info"),
        )
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .right_from(state.ids.flashing_lights_label, 32.0)
        .color(TEXT_COLOR)
        .set(state.ids.flashing_lights_info_label, ui);

        if render_mode.flashing_lights_enabled != flashing_lights_enabled {
            events.push(ChangeRenderMode(Box::new(RenderMode {
                flashing_lights_enabled,
                ..render_mode.clone()
            })));
        }

        events
    }
}
