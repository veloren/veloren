use super::{
    img_ids::{Imgs, ImgsRot},
    BLACK, CRITICAL_HP_COLOR, LOW_HP_COLOR, TEXT_COLOR,
};
use crate::{
    i18n::VoxygenLocalization,
    ui::{fonts::ConrodVoxygenFonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
    window::GameInput,
    GlobalState,
};
use client::Client;
use common::comp::Stats;
use conrod_core::{
    widget::{self, Button, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        bag,
        bag_text,
        bag_text_bg,
        bag_space,
        bag_space_bg,
        bag_show_map,
        map_button,
        map_text,
        map_text_bg,
        settings_button,
        settings_text,
        settings_text_bg,
        social_button,
        social_button_bg,
        social_text,
        social_text_bg,
        spellbook_button,
        spellbook_button_bg,
        spellbook_text,
        spellbook_text_bg,
        crafting_button,
        crafting_button_bg,
        crafting_text,
        crafting_text_bg,

    }
}
const TOOLTIP_UPSHIFT: f64 = 40.0;
#[derive(WidgetCommon)]
pub struct Buttons<'a> {
    client: &'a Client,
    show_bag: bool,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    global_state: &'a GlobalState,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    stats: &'a Stats,
}

impl<'a> Buttons<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        client: &'a Client,
        show_bag: bool,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        global_state: &'a GlobalState,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        stats: &'a Stats,
    ) -> Self {
        Self {
            client,
            show_bag,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            global_state,
            rot_imgs,
            tooltip_manager,
            localized_strings,
            stats,
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    ToggleBag,
    ToggleSettings,
    ToggleMap,
    ToggleSocial,
    ToggleSpell,
    ToggleCrafting,
}

impl<'a> Widget for Buttons<'a> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let invs = self.client.inventories();
        let inventory = match invs.get(self.client.entity()) {
            Some(inv) => inv,
            None => return None,
        };
        let localized_strings = self.localized_strings;
        let button_tooltip = Tooltip::new({
            // Edge images [t, b, r, l]
            // Corner images [tr, tl, br, bl]
            let edge = &self.rot_imgs.tt_side;
            let corner = &self.rot_imgs.tt_corner;
            ImageFrame::new(
                [edge.cw180, edge.none, edge.cw270, edge.cw90],
                [corner.none, corner.cw270, corner.cw90, corner.cw180],
                Color::Rgba(0.08, 0.07, 0.04, 1.0),
                5.0,
            )
        })
        .title_font_size(self.fonts.cyri.scale(15))
        .parent(ui.window)
        .desc_font_size(self.fonts.cyri.scale(12))
        .title_text_color(TEXT_COLOR)
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);
        // Bag
        if Button::image(if !self.show_bag {
            self.imgs.bag
        } else {
            self.imgs.bag_open
        })
        .bottom_right_with_margins_on(ui.window, 5.0, 5.0)
        .hover_image(if !self.show_bag {
            self.imgs.bag_hover
        } else {
            self.imgs.bag_open_hover
        })
        .press_image(if !self.show_bag {
            self.imgs.bag_press
        } else {
            self.imgs.bag_open_press
        })
        .w_h(420.0 / 10.0, 480.0 / 10.0)
        .with_tooltip(
            self.tooltip_manager,
            &localized_strings
                .get("hud.bag.inventory")
                .replace("{playername}", &self.stats.name.to_string().as_str()),
            "",
            &button_tooltip,
        )
        .bottom_offset(55.0)
        .set(state.ids.bag, ui)
        .was_clicked()
        {
            return Some(Event::ToggleBag);
        };
        if let Some(bag) = &self
            .global_state
            .settings
            .controls
            .get_binding(GameInput::Bag)
        {
            Text::new(bag.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.bag, 0.0, 0.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(BLACK)
                .set(state.ids.bag_text_bg, ui);
            Text::new(bag.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.bag_text_bg, 1.0, 1.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.bag_text, ui);
        }
        if !self.show_bag {
            let space_used = inventory.amount;
            let space_max = inventory.slots.len();
            let bag_space = format!("{}/{}", space_used, space_max);
            let bag_space_percentage = space_used as f32 / space_max as f32;
            Text::new(&bag_space)
                .mid_top_with_margin_on(state.ids.bag, -15.0)
                .font_size(12)
                .font_id(self.fonts.cyri.conrod_id)
                .color(BLACK)
                .set(state.ids.bag_space_bg, ui);
            Text::new(&bag_space)
                .top_left_with_margins_on(state.ids.bag_space_bg, -1.0, -1.0)
                .font_size(12)
                .font_id(self.fonts.cyri.conrod_id)
                .color(if bag_space_percentage < 0.8 {
                    TEXT_COLOR
                } else if bag_space_percentage < 1.0 {
                    LOW_HP_COLOR
                } else {
                    CRITICAL_HP_COLOR
                })
                .set(state.ids.bag_space, ui);
        }
        // Settings
        if Button::image(self.imgs.settings)
            .w_h(29.0, 25.0)
            .bottom_right_with_margins_on(ui.window, 5.0, 57.0)
            .hover_image(self.imgs.settings_hover)
            .press_image(self.imgs.settings_press)
            .with_tooltip(
                self.tooltip_manager,
                &localized_strings.get("common.settings"),
                "",
                &button_tooltip,
            )
            .bottom_offset(TOOLTIP_UPSHIFT)
            .set(state.ids.settings_button, ui)
            .was_clicked()
        {
            return Some(Event::ToggleSettings);
        };
        if let Some(settings) = &self
            .global_state
            .settings
            .controls
            .get_binding(GameInput::Settings)
        {
            Text::new(settings.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.settings_button, 0.0, 0.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(BLACK)
                .set(state.ids.settings_text_bg, ui);
            Text::new(settings.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.settings_text_bg, 1.0, 1.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.settings_text, ui);
        };

        // Social
        if Button::image(self.imgs.social)
            .w_h(25.0, 25.0)
            .left_from(state.ids.settings_button, 10.0)
            .hover_image(self.imgs.social_hover)
            .press_image(self.imgs.social_press)
            .with_tooltip(
                self.tooltip_manager,
                &localized_strings.get("hud.social"),
                "",
                &button_tooltip,
            )
            .bottom_offset(TOOLTIP_UPSHIFT)
            .set(state.ids.social_button, ui)
            .was_clicked()
        {
            return Some(Event::ToggleSocial);
        }
        if let Some(social) = &self
            .global_state
            .settings
            .controls
            .get_binding(GameInput::Social)
        {
            Text::new(social.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.social_button, 0.0, 0.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(BLACK)
                .set(state.ids.social_text_bg, ui);
            Text::new(social.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.social_text_bg, 1.0, 1.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.social_text, ui);
        };
        // Map
        if Button::image(self.imgs.map_button)
            .w_h(22.0, 25.0)
            .left_from(state.ids.social_button, 10.0)
            .hover_image(self.imgs.map_hover)
            .press_image(self.imgs.map_press)
            .with_tooltip(
                self.tooltip_manager,
                &localized_strings.get("hud.map.map_title"),
                "",
                &button_tooltip,
            )
            .bottom_offset(TOOLTIP_UPSHIFT)
            .set(state.ids.map_button, ui)
            .was_clicked()
        {
            return Some(Event::ToggleMap);
        };
        if let Some(map) = &self
            .global_state
            .settings
            .controls
            .get_binding(GameInput::Map)
        {
            Text::new(map.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.map_button, 0.0, 0.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(BLACK)
                .set(state.ids.map_text_bg, ui);
            Text::new(map.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.map_text_bg, 1.0, 1.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.map_text, ui);
        }

        // Spellbook
        if Button::image(self.imgs.spellbook_button)
            .w_h(28.0, 25.0)
            .left_from(state.ids.map_button, 10.0)
            .hover_image(self.imgs.spellbook_hover)
            .press_image(self.imgs.spellbook_press)
            .with_tooltip(
                self.tooltip_manager,
                &localized_strings.get("hud.spell"),
                "",
                &button_tooltip,
            )
            .bottom_offset(TOOLTIP_UPSHIFT)
            .set(state.ids.spellbook_button, ui)
            .was_clicked()
        {
            return Some(Event::ToggleSpell);
        }
        if let Some(spell) = &self
            .global_state
            .settings
            .controls
            .get_binding(GameInput::Spellbook)
        {
            Text::new(spell.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.spellbook_button, 0.0, 0.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(BLACK)
                .set(state.ids.spellbook_text_bg, ui);
            Text::new(spell.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.spellbook_text_bg, 1.0, 1.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.spellbook_text, ui);
        }
        // Crafting
        if Button::image(self.imgs.crafting_icon)
            .w_h(25.0, 25.0)
            .left_from(state.ids.spellbook_button, 10.0)
            .hover_image(self.imgs.crafting_icon_hover)
            .press_image(self.imgs.crafting_icon_press)
            .with_tooltip(
                self.tooltip_manager,
                &localized_strings.get("hud.crafting"),
                "",
                &button_tooltip,
            )
            .bottom_offset(TOOLTIP_UPSHIFT)
            .set(state.ids.crafting_button, ui)
            .was_clicked()
        {
            return Some(Event::ToggleCrafting);
        }
        if let Some(crafting) = &self
            .global_state
            .settings
            .controls
            .get_binding(GameInput::Crafting)
        {
            Text::new(crafting.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.crafting_button, 0.0, 0.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(BLACK)
                .set(state.ids.crafting_text_bg, ui);
            Text::new(crafting.to_string().as_str())
                .bottom_right_with_margins_on(state.ids.crafting_text_bg, 1.0, 1.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.crafting_text, ui);
        }
        None
    }
}
