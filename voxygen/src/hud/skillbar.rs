use super::{
    hotbar,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
    slots, BarNumbers, ShortcutNumbers, Show, XpBar, BLACK, CRITICAL_HP_COLOR, HP_COLOR,
    LOW_HP_COLOR, MANA_COLOR, TEXT_COLOR, XP_COLOR,
};
use crate::{
    i18n::VoxygenLocalization,
    ui::{
        fonts::ConrodVoxygenFonts,
        slot::{ContentSize, SlotMaker},
        ImageFrame, Tooltip, TooltipManager, Tooltipable,
    },
    window::GameInput,
    GlobalState,
};
use common::comp::{
    item::{
        tool::{DebugKind, StaffKind, Tool, ToolKind},
        ItemKind,
    },
    CharacterState, ControllerInputs, Energy, Inventory, Loadout, Stats,
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
use std::time::{Duration, Instant};
use vek::*;

widget_ids! {
    struct Ids {
        death_message_1,
        death_message_2,
        death_message_1_bg,
        death_message_2_bg,
        level_text,
        next_level_text,
        xp_bar_mid,
        xp_bar_mid_top,
        xp_bar_left,
        xp_bar_left_top,
        xp_bar_right,
        xp_bar_right_top,
        xp_bar_filling,
        xp_bar_filling_top,
        hotbar_align,
        xp_bar_subdivision,
        m1_slot,
        m1_slot_bg,
        m1_text,
        m1_text_bg,
        m1_slot_act,
        m1_content,
        m2_slot,
        m2_slot_bg,
        m2_text,
        m2_text_bg,
        m2_slot_act,
        m2_content,
        slot1,
        slot1_text,
        slot1_text_bg,
        //slot1_act,
        slot2,
        slot2_text,
        slot2_text_bg,
        slot3,
        slot3_text,
        slot3_text_bg,
        slot4,
        slot4_text,
        slot4_text_bg,
        slot5,
        slot5_text,
        slot5_text_bg,
        slot6,
        slot6_text,
        slot6_text_bg,
        slot7,
        slot7_text,
        slot7_text_bg,
        slot8,
        slot8_text,
        slot8_text_bg,
        slot9,
        slot9_text,
        slot9_text_bg,
        slot10,
        slot10_text,
        slot10_text_bg,
        healthbar_bg,
        healthbar_filling,
        health_text,
        health_text_bg,
        energybar_bg,
        energybar_filling,
        energy_text,
        energy_text_bg,
        level_up,
        level_down,
        level_align,
        level_message,
        level_message_bg,
        death_bg,
        hurt_bg,
    }
}

pub enum ResourceType {
    Mana,
    /*Rage,
     *Focus, */
}
#[derive(WidgetCommon)]
pub struct Skillbar<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a ConrodVoxygenFonts,
    rot_imgs: &'a ImgsRot,
    stats: &'a Stats,
    loadout: &'a Loadout,
    energy: &'a Energy,
    character_state: &'a CharacterState,
    controller: &'a ControllerInputs,
    inventory: &'a Inventory,
    hotbar: &'a hotbar::State,
    tooltip_manager: &'a mut TooltipManager,
    slot_manager: &'a mut slots::SlotManager,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    pulse: f32,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    current_resource: ResourceType,
    show: &'a Show,
}

impl<'a> Skillbar<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a ConrodVoxygenFonts,
        rot_imgs: &'a ImgsRot,
        stats: &'a Stats,
        loadout: &'a Loadout,
        energy: &'a Energy,
        character_state: &'a CharacterState,
        pulse: f32,
        controller: &'a ControllerInputs,
        inventory: &'a Inventory,
        hotbar: &'a hotbar::State,
        tooltip_manager: &'a mut TooltipManager,
        slot_manager: &'a mut slots::SlotManager,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        show: &'a Show,
    ) -> Self {
        Self {
            global_state,
            imgs,
            item_imgs,
            fonts,
            rot_imgs,
            stats,
            loadout,
            energy,
            current_resource: ResourceType::Mana,
            common: widget::CommonBuilder::default(),
            character_state,
            pulse,
            controller,
            inventory,
            hotbar,
            tooltip_manager,
            slot_manager,
            localized_strings,
            show,
        }
    }
}

pub struct State {
    ids: Ids,

    last_xp_value: u32,
    last_level: u32,
    last_update_xp: Instant,
    last_update_level: Instant,
}

impl<'a> Widget for Skillbar<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),

            last_xp_value: 0,
            last_level: 1,
            last_update_xp: Instant::now(),
            last_update_level: Instant::now(),
        }
    }

    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let level = (self.stats.level.level()).to_string();
        let next_level = (self.stats.level.level() + 1).to_string();

        let exp_percentage = (self.stats.exp.current() as f64) / (self.stats.exp.maximum() as f64);

        let hp_percentage =
            self.stats.health.current() as f64 / self.stats.health.maximum() as f64 * 100.0;
        let energy_percentage = self.energy.current() as f64 / self.energy.maximum() as f64 * 100.0;

        let scale = 2.0;

        let bar_values = self.global_state.settings.gameplay.bar_numbers;
        let shortcuts = self.global_state.settings.gameplay.shortcut_numbers;

        const BG_COLOR_2: Color = Color::Rgba(0.0, 0.0, 0.0, 0.99);
        let hp_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8; //Animation timer
        let crit_hp_color: Color = Color::Rgba(0.79, 0.19, 0.17, hp_ani);

        let localized_strings = self.localized_strings;

        // Level Up Message
        if !self.show.intro {
            let current_level = self.stats.level.level();
            const FADE_IN_LVL: f32 = 1.0;
            const FADE_HOLD_LVL: f32 = 3.0;
            const FADE_OUT_LVL: f32 = 2.0;
            // Fade
            // Check if no other popup is displayed and a new one is needed
            if state.last_update_level.elapsed()
                > Duration::from_secs_f32(FADE_IN_LVL + FADE_HOLD_LVL + FADE_OUT_LVL)
                && state.last_level != current_level
            {
                // Update last_value
                state.update(|s| s.last_level = current_level);
                state.update(|s| s.last_update_level = Instant::now());
            };

            let seconds_level = state.last_update_level.elapsed().as_secs_f32();
            let fade_level = if current_level == 1 {
                0.0
            } else if seconds_level < FADE_IN_LVL {
                seconds_level / FADE_IN_LVL
            } else if seconds_level < FADE_IN_LVL + FADE_HOLD_LVL {
                1.0
            } else {
                (1.0 - (seconds_level - FADE_IN_LVL - FADE_HOLD_LVL) / FADE_OUT_LVL).max(0.0)
            };
            // Contents
            Rectangle::fill_with([82.0 * 4.0, 40.0 * 4.0], color::TRANSPARENT)
                .mid_top_with_margin_on(ui.window, 300.0)
                .set(state.ids.level_align, ui);
            let level_up_text = &localized_strings
                .get("char_selection.level_fmt")
                .replace("{level_nb}", &self.stats.level.level().to_string());
            Text::new(&level_up_text)
                .middle_of(state.ids.level_align)
                .font_size(self.fonts.cyri.scale(30))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, fade_level))
                .set(state.ids.level_message_bg, ui);
            Text::new(&level_up_text)
                .bottom_left_with_margins_on(state.ids.level_message_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(30))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(1.0, 1.0, 1.0, fade_level))
                .set(state.ids.level_message, ui);
            Image::new(self.imgs.level_up)
                .w_h(82.0 * 4.0, 9.0 * 4.0)
                .mid_top_with_margin_on(state.ids.level_align, 0.0)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade_level)))
                .graphics_for(state.ids.level_align)
                .set(state.ids.level_up, ui);
            Image::new(self.imgs.level_down)
                .w_h(82.0 * 4.0, 9.0 * 4.0)
                .mid_bottom_with_margin_on(state.ids.level_align, 0.0)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade_level)))
                .graphics_for(state.ids.level_align)
                .set(state.ids.level_down, ui);
        }
        // Death message
        if self.stats.is_dead {
            if let Some(key) = self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Respawn)
            {
                Text::new(&localized_strings.get("hud.you_died"))
                    .middle_of(ui.window)
                    .font_size(self.fonts.cyri.scale(50))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                    .set(state.ids.death_message_1_bg, ui);
                Text::new(
                    &localized_strings
                        .get("hud.press_key_to_respawn")
                        .replace("{key}", key.to_string().as_str()),
                )
                .mid_bottom_with_margin_on(state.ids.death_message_1_bg, -120.0)
                .font_size(self.fonts.cyri.scale(30))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.death_message_2_bg, ui);
                Text::new(&localized_strings.get("hud.you_died"))
                    .bottom_left_with_margins_on(state.ids.death_message_1_bg, 2.0, 2.0)
                    .font_size(self.fonts.cyri.scale(50))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(CRITICAL_HP_COLOR)
                    .set(state.ids.death_message_1, ui);
                Text::new(
                    &localized_strings
                        .get("hud.press_key_to_respawn")
                        .replace("{key}", key.to_string().as_str()),
                )
                .bottom_left_with_margins_on(state.ids.death_message_2_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(30))
                .font_id(self.fonts.cyri.conrod_id)
                .color(CRITICAL_HP_COLOR)
                .set(state.ids.death_message_2, ui);
            }
        }
        // Experience-Bar
        match self.global_state.settings.gameplay.xp_bar {
            XpBar::Always => {
                // Constant Display of the Exp Bar at the bottom of the screen
                Image::new(self.imgs.xp_bar_mid)
                    .w_h(80.0 * scale, 10.0 * scale)
                    .mid_bottom_with_margin_on(ui.window, 2.0)
                    .set(state.ids.xp_bar_mid, ui);
                Image::new(self.imgs.xp_bar_right)
                    .w_h(100.0 * scale, 10.0 * scale)
                    .right_from(state.ids.xp_bar_mid, 0.0)
                    .set(state.ids.xp_bar_right, ui);
                Image::new(self.imgs.xp_bar_left)
                    .w_h(100.0 * scale, 10.0 * scale)
                    .left_from(state.ids.xp_bar_mid, 0.0)
                    .set(state.ids.xp_bar_left, ui);
                Image::new(self.imgs.bar_content)
                    .w_h(260.0 * scale * exp_percentage, 5.0 * scale)
                    .color(Some(XP_COLOR))
                    .top_left_with_margins_on(state.ids.xp_bar_left, 2.0 * scale, 10.0 * scale)
                    .set(state.ids.xp_bar_filling, ui);
                // Level Display
                if self.stats.level.level() < 10 {
                    Text::new(&level)
                        .bottom_left_with_margins_on(
                            state.ids.xp_bar_left,
                            3.5 * scale,
                            4.0 * scale,
                        )
                        .font_size(self.fonts.cyri.scale(10))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right,
                            3.5 * scale,
                            4.0 * scale,
                        )
                        .font_size(self.fonts.cyri.scale(10))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.next_level_text, ui);
                } else if self.stats.level.level() < 100 {
                    // Change offset and fontsize for levels > 9
                    Text::new(&level)
                        .bottom_left_with_margins_on(
                            state.ids.xp_bar_left,
                            3.5 * scale,
                            3.0 * scale,
                        )
                        .font_size(self.fonts.cyri.scale(9))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right,
                            3.5 * scale,
                            3.0 * scale,
                        )
                        .font_size(self.fonts.cyri.scale(9))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.next_level_text, ui);
                } else {
                    // Change offset and fontsize for levels > 9
                    Text::new(&level)
                        .bottom_left_with_margins_on(
                            state.ids.xp_bar_left,
                            3.5 * scale,
                            2.5 * scale,
                        )
                        .font_size(self.fonts.cyri.scale(8))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right,
                            3.5 * scale,
                            2.5 * scale,
                        )
                        .font_size(self.fonts.cyri.scale(8))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.next_level_text, ui);
                }
                // M1 Slot
                Image::new(self.imgs.skillbar_slot_big)
                    .w_h(40.0 * scale, 40.0 * scale)
                    .top_left_with_margins_on(state.ids.xp_bar_mid, -40.0 * scale, 0.0)
                    .set(state.ids.m1_slot, ui);
            },
            XpBar::OnGain => {
                // Displays the Exp Bar at the top of the screen when exp is gained and fades it
                // out afterwards
                const FADE_IN_XP: f32 = 1.0;
                const FADE_HOLD_XP: f32 = 3.0;
                const FADE_OUT_XP: f32 = 2.0;
                let current_xp = self.stats.exp.current();
                // Check if no other popup is displayed and a new one is needed
                if state.last_update_xp.elapsed()
                    > Duration::from_secs_f32(FADE_IN_XP + FADE_HOLD_XP + FADE_OUT_XP)
                    && state.last_xp_value != current_xp
                {
                    // Update last_value
                    state.update(|s| s.last_xp_value = current_xp);
                    state.update(|s| s.last_update_xp = Instant::now());
                }

                let seconds_xp = state.last_update_xp.elapsed().as_secs_f32();
                let fade_xp = if current_xp == 0 {
                    0.0
                } else if seconds_xp < FADE_IN_XP {
                    seconds_xp / FADE_IN_XP
                } else if seconds_xp < FADE_IN_XP + FADE_HOLD_XP {
                    1.0
                } else {
                    (1.0 - (seconds_xp - FADE_IN_XP - FADE_HOLD_XP) / FADE_OUT_XP).max(0.0)
                };
                // Hotbar parts
                Image::new(self.imgs.xp_bar_mid)
                    .w_h(80.0 * scale * 1.5, 10.0 * scale * 1.5)
                    .mid_top_with_margin_on(ui.window, 20.0)
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade_xp)))
                    .set(state.ids.xp_bar_mid_top, ui);
                Image::new(self.imgs.xp_bar_right)
                    .w_h(100.0 * scale * 1.5, 10.0 * scale * 1.5)
                    .right_from(state.ids.xp_bar_mid_top, 0.0)
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade_xp)))
                    .set(state.ids.xp_bar_right_top, ui);
                Image::new(self.imgs.xp_bar_left)
                    .w_h(100.0 * scale * 1.5, 10.0 * scale * 1.5)
                    .left_from(state.ids.xp_bar_mid_top, 0.0)
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade_xp)))
                    .set(state.ids.xp_bar_left_top, ui);
                Image::new(self.imgs.bar_content)
                    .w_h(260.0 * scale * 1.5 * exp_percentage, 6.0 * scale * 1.5)
                    .color(Some(Color::Rgba(0.59, 0.41, 0.67, fade_xp)))
                    .top_left_with_margins_on(
                        state.ids.xp_bar_left_top,
                        2.0 * scale * 1.5,
                        10.0 * scale * 1.5,
                    )
                    .set(state.ids.xp_bar_filling_top, ui);
                // Level Display
                if self.stats.level.level() < 10 {
                    Text::new(&level)
                        .bottom_left_with_margins_on(
                            state.ids.xp_bar_left_top,
                            3.0 * scale * 1.5,
                            4.0 * scale * 1.5,
                        )
                        .font_size(self.fonts.cyri.scale(17))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right_top,
                            3.0 * scale * 1.5,
                            4.0 * scale * 1.5,
                        )
                        .font_size(self.fonts.cyri.scale(15))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.next_level_text, ui);
                } else if self.stats.level.level() < 100 {
                    // Change offset and fontsize for levels > 9
                    Text::new(&level)
                        .bottom_left_with_margins_on(
                            state.ids.xp_bar_left_top,
                            3.0 * scale * 1.5,
                            3.0 * scale * 1.5,
                        )
                        .font_size(self.fonts.cyri.scale(15))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right_top,
                            3.0 * scale * 1.5,
                            3.0 * scale * 1.5,
                        )
                        .font_size(self.fonts.cyri.scale(15))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.next_level_text, ui);
                } else {
                    // Change offset and fontsize for levels > 9
                    Text::new(&level)
                        .bottom_left_with_margins_on(
                            state.ids.xp_bar_left_top,
                            3.0 * scale * 1.5,
                            2.75 * scale * 1.5,
                        )
                        .font_size(self.fonts.cyri.scale(12))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right_top,
                            3.0 * scale * 1.5,
                            2.75 * scale * 1.5,
                        )
                        .font_size(self.fonts.cyri.scale(12))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.next_level_text, ui);
                }
                // Alignment for hotbar
                Rectangle::fill_with([80.0 * scale, 1.0], color::TRANSPARENT)
                    .mid_bottom_with_margin_on(ui.window, 9.0)
                    .set(state.ids.hotbar_align, ui);
                // M1 Slot

                match self.character_state {
                    CharacterState::BasicMelee { .. } => {
                        if self.controller.primary.is_pressed() {
                            let fade_pulse = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.6; //Animation timer;
                            Image::new(self.imgs.skillbar_slot_big)
                                .w_h(40.0 * scale, 40.0 * scale)
                                .top_left_with_margins_on(
                                    state.ids.hotbar_align,
                                    -40.0 * scale,
                                    0.0,
                                )
                                .set(state.ids.m1_slot, ui);
                            Image::new(self.imgs.skillbar_slot_big_act)
                                .w_h(40.0 * scale, 40.0 * scale)
                                .middle_of(state.ids.m1_slot)
                                .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade_pulse)))
                                .floating(true)
                                .set(state.ids.m1_slot_act, ui);
                        } else {
                            Image::new(self.imgs.skillbar_slot_big)
                                .w_h(40.0 * scale, 40.0 * scale)
                                .top_left_with_margins_on(
                                    state.ids.hotbar_align,
                                    -40.0 * scale,
                                    0.0,
                                )
                                .set(state.ids.m1_slot, ui);
                        }
                    },
                    _ => {
                        Image::new(self.imgs.skillbar_slot_big)
                            .w_h(40.0 * scale, 40.0 * scale)
                            .top_left_with_margins_on(state.ids.hotbar_align, -40.0 * scale, 0.0)
                            .set(state.ids.m1_slot, ui);
                    },
                }
            },
        }
        // M1 Slot
        Image::new(self.imgs.skillbar_slot_big_bg)
            .w_h(38.0 * scale, 38.0 * scale)
            .color(
                match self.loadout.active_item.as_ref().map(|i| &i.item.kind) {
                    Some(ItemKind::Tool(Tool { kind, .. })) => match kind {
                        ToolKind::Bow(_) => Some(BG_COLOR_2),
                        ToolKind::Staff(_) => Some(BG_COLOR_2),
                        _ => Some(BG_COLOR_2),
                    },
                    _ => Some(BG_COLOR_2),
                },
            )
            .middle_of(state.ids.m1_slot)
            .set(state.ids.m1_slot_bg, ui);
        Button::image(
            match self.loadout.active_item.as_ref().map(|i| &i.item.kind) {
                Some(ItemKind::Tool(Tool { kind, .. })) => match kind {
                    ToolKind::Sword(_) => self.imgs.twohsword_m1,
                    ToolKind::Hammer(_) => self.imgs.twohhammer_m1,
                    ToolKind::Axe(_) => self.imgs.twohaxe_m1,
                    ToolKind::Bow(_) => self.imgs.bow_m1,
                    ToolKind::Staff(_) => self.imgs.staff_m1,
                    ToolKind::Debug(DebugKind::Boost) => self.imgs.flyingrod_m1,
                    _ => self.imgs.nothing,
                },
                _ => self.imgs.nothing,
            },
        ) // Insert Icon here
        .w(
            match self.loadout.active_item.as_ref().map(|i| &i.item.kind) {
                Some(ItemKind::Tool(Tool { kind, .. })) => match kind {
                    ToolKind::Bow(_) => 30.0 * scale,
                    ToolKind::Staff(_) => 32.0 * scale,
                    _ => 38.0 * scale,
                },
                _ => 38.0 * scale,
            },
        )
        .h(
            match self.loadout.active_item.as_ref().map(|i| &i.item.kind) {
                Some(ItemKind::Tool(Tool { kind, .. })) => match kind {
                    ToolKind::Bow(_) => 30.0 * scale,
                    ToolKind::Staff(_) => 32.0 * scale,
                    _ => 38.0 * scale,
                },
                _ => 38.0 * scale,
            },
        )
        .middle_of(state.ids.m1_slot_bg)
        .set(state.ids.m1_content, ui);
        // M2 Slot
        match self.character_state {
            CharacterState::BasicMelee { .. } => {
                let fade_pulse = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.6; //Animation timer;
                if self.controller.secondary.is_pressed() {
                    Image::new(self.imgs.skillbar_slot_big)
                        .w_h(40.0 * scale, 40.0 * scale)
                        .right_from(state.ids.m1_slot, 0.0)
                        .set(state.ids.m2_slot, ui);
                    Image::new(self.imgs.skillbar_slot_big_act)
                        .w_h(40.0 * scale, 40.0 * scale)
                        .middle_of(state.ids.m2_slot)
                        .color(Some(Color::Rgba(1.0, 1.0, 1.0, fade_pulse)))
                        .floating(true)
                        .set(state.ids.m2_slot_act, ui);
                } else {
                    Image::new(self.imgs.skillbar_slot_big)
                        .w_h(40.0 * scale, 40.0 * scale)
                        .right_from(state.ids.m1_slot, 0.0)
                        .set(state.ids.m2_slot, ui);
                }
            },
            _ => {
                Image::new(self.imgs.skillbar_slot_big)
                    .w_h(40.0 * scale, 40.0 * scale)
                    .right_from(state.ids.m1_slot, 0.0)
                    .set(state.ids.m2_slot, ui);
            },
        }

        Image::new(self.imgs.skillbar_slot_big_bg)
            .w_h(38.0 * scale, 38.0 * scale)
            .color(
                match self.loadout.active_item.as_ref().map(|i| &i.item.kind) {
                    Some(ItemKind::Tool(Tool { kind, .. })) => match kind {
                        ToolKind::Bow(_) => Some(BG_COLOR_2),
                        ToolKind::Staff(_) => Some(BG_COLOR_2),
                        _ => Some(BG_COLOR_2),
                    },
                    _ => Some(BG_COLOR_2),
                },
            )
            .middle_of(state.ids.m2_slot)
            .set(state.ids.m2_slot_bg, ui);
        Button::image(
            match self.loadout.active_item.as_ref().map(|i| &i.item.kind) {
                Some(ItemKind::Tool(Tool { kind, .. })) => match kind {
                    ToolKind::Sword(_) => self.imgs.charge,
                    ToolKind::Hammer(_) => self.imgs.nothing,
                    ToolKind::Axe(_) => self.imgs.nothing,
                    ToolKind::Bow(_) => self.imgs.nothing,
                    ToolKind::Staff(StaffKind::Sceptre) => self.imgs.heal_0,
                    ToolKind::Staff(_) => self.imgs.staff_m2,
                    ToolKind::Debug(DebugKind::Boost) => self.imgs.flyingrod_m2,
                    _ => self.imgs.nothing,
                },
                _ => self.imgs.nothing,
            },
        ) // Insert Icon here
        .w(
            match self.loadout.active_item.as_ref().map(|i| &i.item.kind) {
                Some(ItemKind::Tool(Tool { kind, .. })) => match kind {
                    ToolKind::Staff(_) => 30.0 * scale,
                    _ => 38.0 * scale,
                },
                _ => 38.0 * scale,
            },
        )
        .h(
            match self.loadout.active_item.as_ref().map(|i| &i.item.kind) {
                Some(ItemKind::Tool(Tool { kind, .. })) => match kind {
                    ToolKind::Staff(_) => 30.0 * scale,
                    _ => 38.0 * scale,
                },
                _ => 38.0 * scale,
            },
        )
        .middle_of(state.ids.m2_slot_bg)
        .image_color(
            match self.loadout.active_item.as_ref().map(|i| &i.item.kind) {
                Some(ItemKind::Tool(Tool { kind, .. })) => match kind {
                    ToolKind::Sword(_) => {
                        if self.energy.current() as f64 >= 200.0 {
                            Color::Rgba(1.0, 1.0, 1.0, 1.0)
                        } else {
                            Color::Rgba(0.3, 0.3, 0.3, 0.8)
                        }
                    },
                    ToolKind::Staff(StaffKind::Sceptre) => {
                        if self.energy.current() as f64 >= 400.0 {
                            Color::Rgba(1.0, 1.0, 1.0, 1.0)
                        } else {
                            Color::Rgba(0.3, 0.3, 0.3, 0.8)
                        }
                    },
                    _ => Color::Rgba(1.0, 1.0, 1.0, 1.0),
                },
                _ => Color::Rgba(1.0, 1.0, 1.0, 1.0),
            },
        )
        .set(state.ids.m2_content, ui);
        // Slots
        let content_source = (self.hotbar, self.inventory, self.loadout, self.energy); // TODO: avoid this
        let image_source = (self.item_imgs, self.imgs);

        let mut slot_maker = SlotMaker {
            // TODO: is a separate image needed for the frame?
            empty_slot: self.imgs.skillbar_slot,
            filled_slot: self.imgs.skillbar_slot,
            selected_slot: self.imgs.skillbar_slot_act,
            background_color: None,
            content_size: ContentSize {
                width_height_ratio: 1.0,
                max_fraction: 0.8, /* Changes the item image size by setting a maximum fraction
                                    * of either the width or height */
            },
            selected_content_scale: 1.0,
            amount_font: self.fonts.cyri.conrod_id,
            amount_margins: Vec2::new(1.0, 1.0),
            amount_font_size: self.fonts.cyri.scale(12),
            amount_text_color: TEXT_COLOR,
            content_source: &content_source,
            image_source: &image_source,
            slot_manager: Some(self.slot_manager),
        };
        let item_tooltip = Tooltip::new({
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
        // Helper
        let tooltip_text = |slot| {
            content_source
                .0
                .get(slot)
                .and_then(|content| match content {
                    hotbar::SlotContents::Inventory(i) => content_source
                        .1
                        .get(i)
                        .map(|item| (item.name(), item.description())),
                    hotbar::SlotContents::Ability3 => content_source
                        .2
                        .active_item
                        .as_ref()
                        .map(|i| &i.item.kind)
                        .and_then(|kind| match kind {
                            ItemKind::Tool(Tool { kind, .. }) => match kind {
                                ToolKind::Staff(_) => Some((
                                    "Firebomb",
                                    "\nWhirls a big fireball into the air. \nExplodes the ground \
                                     and does\na big amount of damage",
                                )),
                                ToolKind::Debug(DebugKind::Boost) => Some((
                                    "Possessing Arrow",
                                    "\nShoots a poisonous arrow.\nLets you control your target.",
                                )),
                                _ => None,
                            },
                            _ => None,
                        }),
                })
        };

        const SLOT_TOOLTIP_UPSHIFT: f64 = 70.0;
        //Slot 5
        let slot = slot_maker
            .fabricate(hotbar::Slot::Five, [20.0 * scale as f32; 2])
            .bottom_left_with_margins_on(state.ids.m1_slot, 0.0, -20.0 * scale);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::Five) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot5, ui);
        } else {
            slot.set(state.ids.slot5, ui);
        }
        // Slot 4
        let slot = slot_maker
            .fabricate(hotbar::Slot::Four, [20.0 * scale as f32; 2])
            .left_from(state.ids.slot5, 0.0);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::Four) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot4, ui);
        } else {
            slot.set(state.ids.slot4, ui);
        }
        // Slot 3
        let slot = slot_maker
            .fabricate(hotbar::Slot::Three, [20.0 * scale as f32; 2])
            .left_from(state.ids.slot4, 0.0);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::Three) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot3, ui);
        } else {
            slot.set(state.ids.slot3, ui);
        }
        // Slot 2
        let slot = slot_maker
            .fabricate(hotbar::Slot::Two, [20.0 * scale as f32; 2])
            .left_from(state.ids.slot3, 0.0);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::Two) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot2, ui);
        } else {
            slot.set(state.ids.slot2, ui);
        }
        // Slot 1
        slot_maker.empty_slot = self.imgs.skillbar_slot_l;
        slot_maker.filled_slot = self.imgs.skillbar_slot_l;
        slot_maker.selected_slot = self.imgs.skillbar_slot_l_act;
        let slot = slot_maker
            .fabricate(hotbar::Slot::One, [20.0 * scale as f32; 2])
            .left_from(state.ids.slot2, 0.0);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::One) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot1, ui);
        } else {
            slot.set(state.ids.slot1, ui);
        }
        // Slot 6
        slot_maker.empty_slot = self.imgs.skillbar_slot;
        slot_maker.filled_slot = self.imgs.skillbar_slot;
        slot_maker.selected_slot = self.imgs.skillbar_slot_act;
        let slot = slot_maker
            .fabricate(hotbar::Slot::Six, [20.0 * scale as f32; 2])
            .bottom_right_with_margins_on(state.ids.m2_slot, 0.0, -20.0 * scale);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::Six) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot6, ui);
        } else {
            slot.set(state.ids.slot6, ui);
        }
        // Slot 7
        let slot = slot_maker
            .fabricate(hotbar::Slot::Seven, [20.0 * scale as f32; 2])
            .right_from(state.ids.slot6, 0.0);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::Seven) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot7, ui);
        } else {
            slot.set(state.ids.slot7, ui);
        }
        // Slot 8
        let slot = slot_maker
            .fabricate(hotbar::Slot::Eight, [20.0 * scale as f32; 2])
            .right_from(state.ids.slot7, 0.0);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::Eight) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot8, ui);
        } else {
            slot.set(state.ids.slot8, ui);
        }
        // Slot 9
        let slot = slot_maker
            .fabricate(hotbar::Slot::Nine, [20.0 * scale as f32; 2])
            .right_from(state.ids.slot8, 0.0);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::Nine) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot9, ui);
        } else {
            slot.set(state.ids.slot9, ui);
        }
        // Quickslot
        slot_maker.empty_slot = self.imgs.skillbar_slot_r;
        slot_maker.filled_slot = self.imgs.skillbar_slot_r;
        slot_maker.selected_slot = self.imgs.skillbar_slot_r_act;
        let slot = slot_maker
            .fabricate(hotbar::Slot::Ten, [20.0 * scale as f32; 2])
            .right_from(state.ids.slot9, 0.0);
        if let Some((title, desc)) = tooltip_text(hotbar::Slot::Ten) {
            slot.with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                .bottom_offset(SLOT_TOOLTIP_UPSHIFT)
                .set(state.ids.slot10, ui);
        } else {
            slot.set(state.ids.slot10, ui);
        }

        // Shortcuts
        if let ShortcutNumbers::On = shortcuts {
            if let Some(slot1) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot1)
            {
                Text::new(slot1.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot1, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot1_text_bg, ui);
                Text::new(slot1.to_string().as_str())
                    .bottom_left_with_margins_on(state.ids.slot1_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot1_text, ui);
            }
            if let Some(slot2) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot2)
            {
                Text::new(slot2.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot2, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot2_text_bg, ui);
                Text::new(slot2.to_string().as_str())
                    .bottom_left_with_margins_on(state.ids.slot2_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot2_text, ui);
            }
            if let Some(slot3) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot3)
            {
                Text::new(slot3.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot3, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot3_text_bg, ui);
                Text::new(slot3.to_string().as_str())
                    .bottom_left_with_margins_on(state.ids.slot3_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot3_text, ui);
            }
            if let Some(slot4) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot4)
            {
                Text::new(slot4.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot4, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot4_text_bg, ui);
                Text::new(slot4.to_string().as_str())
                    .bottom_left_with_margins_on(state.ids.slot4_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot4_text, ui);
            }
            if let Some(slot5) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot5)
            {
                Text::new(slot5.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot5, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot5_text_bg, ui);
                Text::new(slot5.to_string().as_str())
                    .bottom_left_with_margins_on(state.ids.slot5_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot5_text, ui);
            }
            if let Some(m1) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Primary)
            {
                Text::new(m1.to_string().as_str())
                    .top_left_with_margins_on(state.ids.m1_slot, 5.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.m1_text_bg, ui);
                Text::new(m1.to_string().as_str())
                    .bottom_right_with_margins_on(state.ids.m1_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.m1_text, ui);
            }
            if let Some(m2) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Secondary)
            {
                Text::new(m2.to_string().as_str())
                    .top_right_with_margins_on(state.ids.m2_slot, 5.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.m2_text_bg, ui);
                Text::new(m2.to_string().as_str())
                    .bottom_left_with_margins_on(state.ids.m2_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.m2_text, ui);
            }
            if let Some(slot6) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot6)
            {
                Text::new(slot6.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot6, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot6_text_bg, ui);
                Text::new(slot6.to_string().as_str())
                    .bottom_right_with_margins_on(state.ids.slot6_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot6_text, ui);
            }
            if let Some(slot7) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot7)
            {
                Text::new(slot7.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot7, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot7_text_bg, ui);
                Text::new(slot7.to_string().as_str())
                    .bottom_right_with_margins_on(state.ids.slot7_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot7_text, ui);
            }
            if let Some(slot8) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot8)
            {
                Text::new(slot8.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot8, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot8_text_bg, ui);
                Text::new(slot8.to_string().as_str())
                    .bottom_right_with_margins_on(state.ids.slot8_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot8_text, ui);
            }
            if let Some(slot9) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot9)
            {
                Text::new(slot9.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot9, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot9_text_bg, ui);
                Text::new(slot9.to_string().as_str())
                    .bottom_right_with_margins_on(state.ids.slot9_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot9_text, ui);
            }
            if let Some(slot10) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Slot10)
            {
                Text::new(slot10.to_string().as_str())
                    .top_right_with_margins_on(state.ids.slot10, 3.0, 5.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(BLACK)
                    .set(state.ids.slot10_text_bg, ui);
                Text::new(slot10.to_string().as_str())
                    .bottom_right_with_margins_on(state.ids.slot10_text_bg, 1.0, 1.0)
                    .font_size(self.fonts.cyri.scale(8))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.slot10_text, ui);
            }
        };

        // Lifebar
        Image::new(self.imgs.healthbar_bg)
            .w_h(100.0 * scale, 20.0 * scale)
            .top_left_with_margins_on(state.ids.m1_slot, 0.0, -100.0 * scale)
            .set(state.ids.healthbar_bg, ui);
        Image::new(self.imgs.bar_content)
            .w_h(97.0 * scale * hp_percentage / 100.0, 16.0 * scale)
            .color(Some(if hp_percentage <= 20.0 {
                crit_hp_color
            } else if hp_percentage <= 40.0 {
                LOW_HP_COLOR
            } else {
                HP_COLOR
            }))
            .top_right_with_margins_on(state.ids.healthbar_bg, 2.0 * scale, 1.0 * scale)
            .set(state.ids.healthbar_filling, ui);
        // Energybar
        Image::new(self.imgs.energybar_bg)
            .w_h(100.0 * scale, 20.0 * scale)
            .top_right_with_margins_on(state.ids.m2_slot, 0.0, -100.0 * scale)
            .set(state.ids.energybar_bg, ui);
        Image::new(self.imgs.bar_content)
            .w_h(97.0 * scale * energy_percentage / 100.0, 16.0 * scale)
            .top_left_with_margins_on(state.ids.energybar_bg, 2.0 * scale, 1.0 * scale)
            .color(Some(match self.current_resource {
                ResourceType::Mana => MANA_COLOR,
                /*ResourceType::Focus => FOCUS_COLOR,
                 *ResourceType::Rage => RAGE_COLOR, */
            }))
            .set(state.ids.energybar_filling, ui);
        // Bar Text
        // Values
        if let BarNumbers::Values = bar_values {
            let hp_text = format!(
                "{}/{}",
                self.stats.health.current() as u32,
                self.stats.health.maximum() as u32
            );
            Text::new(&hp_text)
                .mid_top_with_margin_on(state.ids.healthbar_bg, 6.0 * scale)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.health_text_bg, ui);
            Text::new(&hp_text)
                .bottom_left_with_margins_on(state.ids.health_text_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.health_text, ui);
            let energy_text = format!(
                "{}/{}",
                self.energy.current() as u32 / 10, /* TODO Fix regeneration with smaller energy
                                                    * numbers instead of dividing by 10 here */
                self.energy.maximum() as u32 / 10
            );
            Text::new(&energy_text)
                .mid_top_with_margin_on(state.ids.energybar_bg, 6.0 * scale)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.energy_text_bg, ui);
            Text::new(&energy_text)
                .bottom_left_with_margins_on(state.ids.energy_text_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.energy_text, ui);
        }
        //Percentages
        if let BarNumbers::Percent = bar_values {
            let hp_text = format!("{}%", hp_percentage as u32);
            Text::new(&hp_text)
                .mid_top_with_margin_on(state.ids.healthbar_bg, 6.0 * scale)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.health_text_bg, ui);
            Text::new(&hp_text)
                .bottom_left_with_margins_on(state.ids.health_text_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.health_text, ui);
            let energy_text = format!("{}%", energy_percentage as u32);
            Text::new(&energy_text)
                .mid_top_with_margin_on(state.ids.energybar_bg, 6.0 * scale)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.energy_text_bg, ui);
            Text::new(&energy_text)
                .bottom_left_with_margins_on(state.ids.energy_text_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.energy_text, ui);
        }
    }

    // Buffs
    // Add debuff slots above the health bar
    // Add buff slots above the mana bar

    // Debuffs
}
