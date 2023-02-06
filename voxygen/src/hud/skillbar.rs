use super::{
    hotbar,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
    slots, util, BarNumbers, HudInfo, ShortcutNumbers, BLACK, CRITICAL_HP_COLOR, HP_COLOR,
    LOW_HP_COLOR, POISEBAR_TICK_COLOR, POISE_COLOR, QUALITY_EPIC, QUALITY_LEGENDARY, STAMINA_COLOR,
    TEXT_COLOR, TEXT_VELORITE, UI_HIGHLIGHT_0, XP_COLOR,
};
use crate::{
    game_input::GameInput,
    hud::{animation::animation_timer, ComboFloater, Position, PositionSpecifier},
    ui::{
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
        ImageFrame, ItemTooltip, ItemTooltipManager, ItemTooltipable, Tooltip, TooltipManager,
        Tooltipable,
    },
    window::KeyMouse,
    GlobalState,
};
use i18n::Localization;
use std::borrow::Cow;

use client::{self, Client};
use common::comp::{
    self,
    ability::{AbilityInput, Stance},
    item::{
        tool::{AbilityContext, ToolKind},
        ItemDesc, MaterialStatManifest,
    },
    skillset::SkillGroupKind,
    Ability, ActiveAbilities, Body, CharacterState, Combo, Energy, Health, Inventory, Poise,
    PoiseState, SkillSet,
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, UiCell, Widget, WidgetCommon,
};
use vek::*;

widget_ids! {
    struct Ids {
        // Death message
        death_message_1,
        death_message_2,
        death_message_1_bg,
        death_message_2_bg,
        death_bg,
        // Level up message
        level_up,
        level_down,
        level_align,
        level_message,
        level_message_bg,
        // Hurt BG
        hurt_bg,
        // Skillbar
        alignment,
        bg,
        frame,
        bg_health,
        frame_health,
        bg_energy,
        frame_energy,
        bg_poise,
        frame_poise,
        m1_ico,
        m2_ico,
        // Level
        level_bg,
        level,
        // HP-Bar
        hp_alignment,
        hp_filling,
        hp_decayed,
        hp_txt_alignment,
        hp_txt_bg,
        hp_txt,
        decay_overlay,
        // Energy-Bar
        energy_alignment,
        energy_filling,
        energy_txt_alignment,
        energy_txt_bg,
        energy_txt,
        // Poise-Bar
        poise_alignment,
        poise_filling,
        poise_ticks[],
        poise_txt_alignment,
        poise_txt_bg,
        poise_txt,
        // Exp-Bar
        exp_frame_bg,
        exp_frame,
        exp_filling,
        exp_img_frame_bg,
        exp_img_frame,
        exp_img,
        exp_lvl,
        spellbook_txt_bg,
        spellbook_txt,
        sp_arrow,
        sp_arrow_txt_bg,
        sp_arrow_txt,
        //Bag Button
        bag_frame_bg,
        bag_frame,
        bag_filling,
        bag_img_frame_bg,
        bag_img_frame,
        bag_img,
        bag_space_bg,
        bag_space,
        bag_progress,
        bag_numbers_alignment,
        bag_text_bg,
        bag_text,
        // Combo Counter
        combo_align,
        combo_bg,
        combo,
        // Slots
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
    }
}

#[derive(Clone, Copy)]
struct SlotEntry {
    slot: hotbar::Slot,
    widget_id: widget::Id,
    position: PositionSpecifier,
    game_input: GameInput,
    shortcut_position: PositionSpecifier,
    shortcut_position_bg: PositionSpecifier,
    shortcut_widget_ids: (widget::Id, widget::Id),
}

fn slot_entries(state: &State, slot_offset: f64) -> [SlotEntry; 10] {
    use PositionSpecifier::*;

    [
        // 1th - 5th slots
        SlotEntry {
            slot: hotbar::Slot::One,
            widget_id: state.ids.slot1,
            position: BottomLeftWithMarginsOn(state.ids.frame, 0.0, 0.0),
            game_input: GameInput::Slot1,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot1_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot1, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot1_text, state.ids.slot1_text_bg),
        },
        SlotEntry {
            slot: hotbar::Slot::Two,
            widget_id: state.ids.slot2,
            position: RightFrom(state.ids.slot1, slot_offset),
            game_input: GameInput::Slot2,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot2_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot2, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot2_text, state.ids.slot2_text_bg),
        },
        SlotEntry {
            slot: hotbar::Slot::Three,
            widget_id: state.ids.slot3,
            position: RightFrom(state.ids.slot2, slot_offset),
            game_input: GameInput::Slot3,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot3_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot3, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot3_text, state.ids.slot3_text_bg),
        },
        SlotEntry {
            slot: hotbar::Slot::Four,
            widget_id: state.ids.slot4,
            position: RightFrom(state.ids.slot3, slot_offset),
            game_input: GameInput::Slot4,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot4_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot4, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot4_text, state.ids.slot4_text_bg),
        },
        SlotEntry {
            slot: hotbar::Slot::Five,
            widget_id: state.ids.slot5,
            position: RightFrom(state.ids.slot4, slot_offset),
            game_input: GameInput::Slot5,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot5_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot5, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot5_text, state.ids.slot5_text_bg),
        },
        // 6th - 10th slots
        SlotEntry {
            slot: hotbar::Slot::Six,
            widget_id: state.ids.slot6,
            position: RightFrom(state.ids.m2_slot_bg, slot_offset),
            game_input: GameInput::Slot6,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot6_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot6, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot6_text, state.ids.slot6_text_bg),
        },
        SlotEntry {
            slot: hotbar::Slot::Seven,
            widget_id: state.ids.slot7,
            position: RightFrom(state.ids.slot6, slot_offset),
            game_input: GameInput::Slot7,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot7_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot7, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot7_text, state.ids.slot7_text_bg),
        },
        SlotEntry {
            slot: hotbar::Slot::Eight,
            widget_id: state.ids.slot8,
            position: RightFrom(state.ids.slot7, slot_offset),
            game_input: GameInput::Slot8,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot8_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot8, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot8_text, state.ids.slot8_text_bg),
        },
        SlotEntry {
            slot: hotbar::Slot::Nine,
            widget_id: state.ids.slot9,
            position: RightFrom(state.ids.slot8, slot_offset),
            game_input: GameInput::Slot9,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot9_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot9, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot9_text, state.ids.slot9_text_bg),
        },
        SlotEntry {
            slot: hotbar::Slot::Ten,
            widget_id: state.ids.slot10,
            position: RightFrom(state.ids.slot9, slot_offset),
            game_input: GameInput::Slot10,
            shortcut_position: BottomLeftWithMarginsOn(state.ids.slot10_text_bg, 1.0, 1.0),
            shortcut_position_bg: TopRightWithMarginsOn(state.ids.slot10, 3.0, 5.0),
            shortcut_widget_ids: (state.ids.slot10_text, state.ids.slot10_text_bg),
        },
    ]
}

pub enum Event {
    OpenDiary(SkillGroupKind),
    OpenBag,
}

#[derive(WidgetCommon)]
pub struct Skillbar<'a> {
    client: &'a Client,
    info: &'a HudInfo,
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    rot_imgs: &'a ImgsRot,
    health: &'a Health,
    inventory: &'a Inventory,
    energy: &'a Energy,
    poise: &'a Poise,
    skillset: &'a SkillSet,
    active_abilities: Option<&'a ActiveAbilities>,
    body: &'a Body,
    // character_state: &'a CharacterState,
    // controller: &'a ControllerInputs,
    hotbar: &'a hotbar::State,
    tooltip_manager: &'a mut TooltipManager,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    slot_manager: &'a mut slots::SlotManager,
    localized_strings: &'a Localization,
    pulse: f32,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    msm: &'a MaterialStatManifest,
    combo_floater: Option<ComboFloater>,
    context: AbilityContext,
    combo: Option<&'a Combo>,
    char_state: Option<&'a CharacterState>,
    stance: Option<&'a Stance>,
}

impl<'a> Skillbar<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: &'a Client,
        info: &'a HudInfo,
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        rot_imgs: &'a ImgsRot,
        health: &'a Health,
        inventory: &'a Inventory,
        energy: &'a Energy,
        poise: &'a Poise,
        skillset: &'a SkillSet,
        active_abilities: Option<&'a ActiveAbilities>,
        body: &'a Body,
        // character_state: &'a CharacterState,
        pulse: f32,
        // controller: &'a ControllerInputs,
        hotbar: &'a hotbar::State,
        tooltip_manager: &'a mut TooltipManager,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut slots::SlotManager,
        localized_strings: &'a Localization,
        msm: &'a MaterialStatManifest,
        combo_floater: Option<ComboFloater>,
        context: AbilityContext,
        combo: Option<&'a Combo>,
        char_state: Option<&'a CharacterState>,
        stance: Option<&'a Stance>,
    ) -> Self {
        Self {
            client,
            info,
            global_state,
            imgs,
            item_imgs,
            fonts,
            rot_imgs,
            health,
            inventory,
            energy,
            poise,
            skillset,
            active_abilities,
            body,
            common: widget::CommonBuilder::default(),
            // character_state,
            pulse,
            // controller,
            hotbar,
            tooltip_manager,
            item_tooltip_manager,
            slot_manager,
            localized_strings,
            msm,
            combo_floater,
            context,
            combo,
            char_state,
            stance,
        }
    }

    fn create_new_button_with_shadow(
        &self,
        ui: &mut UiCell,
        key_mouse: &KeyMouse,
        button_identifier: widget::Id,
        text_background: widget::Id,
        text: widget::Id,
    ) {
        let key_layout = &self.global_state.window.key_layout;
        let key_desc = key_mouse.display_shortest(key_layout);

        //Create shadow
        Text::new(&key_desc)
            .bottom_right_with_margins_on(button_identifier, 0.0, 0.0)
            .font_size(10)
            .font_id(self.fonts.cyri.conrod_id)
            .color(BLACK)
            .set(text_background, ui);

        //Create button
        Text::new(&key_desc)
            .bottom_right_with_margins_on(text_background, 1.0, 1.0)
            .font_size(10)
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(text, ui);
    }

    fn show_death_message(&self, state: &State, ui: &mut UiCell) {
        let localized_strings = self.localized_strings;
        let key_layout = &self.global_state.window.key_layout;

        if let Some(key) = self
            .global_state
            .settings
            .controls
            .get_binding(GameInput::Respawn)
        {
            Text::new(&self.localized_strings.get_msg("hud-you_died"))
                .middle_of(ui.window)
                .font_size(self.fonts.cyri.scale(50))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.death_message_1_bg, ui);
            let respawn_msg =
                localized_strings.get_msg_ctx("hud-press_key_to_respawn", &i18n::fluent_args! {
                    "key" => key.display_string(key_layout)
                });
            Text::new(&respawn_msg)
                .mid_bottom_with_margin_on(state.ids.death_message_1_bg, -120.0)
                .font_size(self.fonts.cyri.scale(30))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.death_message_2_bg, ui);
            Text::new(&self.localized_strings.get_msg("hud-you_died"))
                .bottom_left_with_margins_on(state.ids.death_message_1_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(50))
                .font_id(self.fonts.cyri.conrod_id)
                .color(CRITICAL_HP_COLOR)
                .set(state.ids.death_message_1, ui);
            Text::new(&respawn_msg)
                .bottom_left_with_margins_on(state.ids.death_message_2_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(30))
                .font_id(self.fonts.cyri.conrod_id)
                .color(CRITICAL_HP_COLOR)
                .set(state.ids.death_message_2, ui);
        }
    }

    fn show_stat_bars(&self, state: &State, ui: &mut UiCell) -> Option<Event> {
        let (hp_percentage, energy_percentage, poise_percentage): (f64, f64, f64) =
            if self.health.is_dead {
                (0.0, 0.0, 0.0)
            } else {
                let max_hp = f64::from(self.health.base_max().max(self.health.maximum()));
                let current_hp = f64::from(self.health.current());
                (
                    current_hp / max_hp * 100.0,
                    f64::from(self.energy.fraction() * 100.0),
                    f64::from(self.poise.fraction() * 100.0),
                )
            };

        // Animation timer
        let hp_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8;
        let crit_hp_color: Color = Color::Rgba(0.79, 0.19, 0.17, hp_ani);
        let bar_values = self.global_state.settings.interface.bar_numbers;
        let show_health = self.global_state.settings.interface.always_show_bars
            || (self.health.current() - self.health.maximum()).abs() > Health::HEALTH_EPSILON;
        let show_energy = self.global_state.settings.interface.always_show_bars
            || (self.energy.current() - self.energy.maximum()).abs() > Energy::ENERGY_EPSILON;
        let show_poise = self.global_state.settings.interface.enable_poise_bar
            && (self.global_state.settings.interface.always_show_bars
                || (self.poise.current() - self.poise.maximum()).abs() > Poise::POISE_EPSILON);
        let decayed_health = 1.0 - self.health.maximum() as f64 / self.health.base_max() as f64;

        if show_health && !self.health.is_dead || decayed_health > 0.0 {
            let offset = 1.0;
            Image::new(self.imgs.health_bg)
                .w_h(484.0, 24.0)
                .mid_top_with_margin_on(state.ids.frame, -offset)
                .set(state.ids.bg_health, ui);
            Rectangle::fill_with([480.0, 18.0], color::TRANSPARENT)
                .top_left_with_margins_on(state.ids.bg_health, 2.0, 2.0)
                .set(state.ids.hp_alignment, ui);
            let health_col = match hp_percentage as u8 {
                0..=20 => crit_hp_color,
                21..=40 => LOW_HP_COLOR,
                _ => HP_COLOR,
            };
            Image::new(self.imgs.bar_content)
                .w_h(480.0 * hp_percentage / 100.0, 18.0)
                .color(Some(health_col))
                .top_left_with_margins_on(state.ids.hp_alignment, 0.0, 0.0)
                .set(state.ids.hp_filling, ui);

            if decayed_health > 0.0 {
                let decay_bar_len = 480.0 * decayed_health;
                Image::new(self.imgs.bar_content)
                    .w_h(decay_bar_len, 18.0)
                    .color(Some(QUALITY_EPIC))
                    .top_right_with_margins_on(state.ids.hp_alignment, 0.0, 0.0)
                    .crop_kids()
                    .set(state.ids.hp_decayed, ui);

                Image::new(self.imgs.decayed_bg)
                    .w_h(480.0, 18.0)
                    .color(Some(Color::Rgba(0.58, 0.29, 0.93, (hp_ani + 0.6).min(1.0))))
                    .top_left_with_margins_on(state.ids.hp_alignment, 0.0, 0.0)
                    .parent(state.ids.hp_decayed)
                    .set(state.ids.decay_overlay, ui);
            }
            Image::new(self.imgs.health_frame)
                .w_h(484.0, 24.0)
                .color(Some(UI_HIGHLIGHT_0))
                .middle_of(state.ids.bg_health)
                .set(state.ids.frame_health, ui);
        }
        if show_energy && !self.health.is_dead {
            let offset = if show_health || decayed_health > 0.0 {
                34.0
            } else {
                1.0
            };
            Image::new(self.imgs.energy_bg)
                .w_h(323.0, 16.0)
                .mid_top_with_margin_on(state.ids.frame, -offset)
                .set(state.ids.bg_energy, ui);
            Rectangle::fill_with([319.0, 10.0], color::TRANSPARENT)
                .top_left_with_margins_on(state.ids.bg_energy, 2.0, 2.0)
                .set(state.ids.energy_alignment, ui);
            Image::new(self.imgs.bar_content)
                .w_h(319.0 * energy_percentage / 100.0, 10.0)
                .color(Some(STAMINA_COLOR))
                .top_left_with_margins_on(state.ids.energy_alignment, 0.0, 0.0)
                .set(state.ids.energy_filling, ui);
            Image::new(self.imgs.energy_frame)
                .w_h(323.0, 16.0)
                .color(Some(UI_HIGHLIGHT_0))
                .middle_of(state.ids.bg_energy)
                .set(state.ids.frame_energy, ui);
        }
        if show_poise && !self.health.is_dead {
            let offset = 17.0;

            let poise_colour = match self.poise.previous_state {
                self::PoiseState::KnockedDown => BLACK,
                self::PoiseState::Dazed => Color::Rgba(0.25, 0.0, 0.15, 1.0),
                self::PoiseState::Stunned => Color::Rgba(0.40, 0.0, 0.30, 1.0),
                self::PoiseState::Interrupted => Color::Rgba(0.55, 0.0, 0.45, 1.0),
                _ => POISE_COLOR,
            };

            Image::new(self.imgs.poise_bg)
                .w_h(323.0, 14.0)
                .mid_top_with_margin_on(state.ids.frame, -offset)
                .set(state.ids.bg_poise, ui);
            Rectangle::fill_with([319.0, 10.0], color::TRANSPARENT)
                .top_left_with_margins_on(state.ids.bg_poise, 2.0, 2.0)
                .set(state.ids.poise_alignment, ui);
            Image::new(self.imgs.bar_content)
                .w_h(319.0 * poise_percentage / 100.0, 10.0)
                .color(Some(poise_colour))
                .top_left_with_margins_on(state.ids.poise_alignment, 0.0, 0.0)
                .set(state.ids.poise_filling, ui);
            for i in 0..state.ids.poise_ticks.len() {
                Image::new(self.imgs.poise_tick)
                    .w_h(3.0, 10.0)
                    .color(Some(POISEBAR_TICK_COLOR))
                    .top_left_with_margins_on(
                        state.ids.poise_alignment,
                        0.0,
                        319.0f64 * (self::Poise::POISE_THRESHOLDS[i] / self.poise.maximum()) as f64,
                    )
                    .set(state.ids.poise_ticks[i], ui);
            }
            Image::new(self.imgs.poise_frame)
                .w_h(323.0, 16.0)
                .color(Some(UI_HIGHLIGHT_0))
                .middle_of(state.ids.bg_poise)
                .set(state.ids.frame_poise, ui);
        }
        // Bag button and indicator
        Image::new(self.imgs.selected_exp_bg)
            .w_h(34.0, 38.0)
            .bottom_right_with_margins_on(state.ids.slot10, 0.0, -37.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
            .set(state.ids.bag_img_frame_bg, ui);

        if Button::image(self.imgs.bag_frame)
            .w_h(34.0, 38.0)
            .middle_of(state.ids.bag_img_frame_bg)
            .set(state.ids.bag_img_frame, ui)
            .was_clicked()
        {
            return Some(Event::OpenBag);
        }
        let invs = self.client.inventories();
        let inventory = match invs.get(self.info.viewpoint_entity) {
            Some(inv) => inv,
            None => return None,
        };

        let space_used = inventory.populated_slots();
        let space_max = inventory.slots().count();
        let bag_space = format!("{}/{}", space_used, space_max);
        let bag_space_percentage = space_used as f64 / space_max as f64;

        // bag filling indicator bar
        Image::new(self.imgs.bar_content)
            .w_h(1.0, 21.0 * bag_space_percentage)
            .color(if bag_space_percentage < 0.6 {
                Some(TEXT_VELORITE)
            } else if bag_space_percentage < 1.0 {
                Some(LOW_HP_COLOR)
            } else {
                Some(CRITICAL_HP_COLOR)
            })
            .graphics_for(state.ids.bag_img_frame)
            .bottom_left_with_margins_on(state.ids.bag_img_frame, 14.0, 2.0)
            .set(state.ids.bag_filling, ui);

        // bag filling text
        Rectangle::fill_with([32.0, 11.0], color::TRANSPARENT)
            .bottom_left_with_margins_on(state.ids.bag_img_frame_bg, 1.0, 2.0)
            .graphics_for(state.ids.bag_img_frame)
            .set(state.ids.bag_numbers_alignment, ui);
        Text::new(&bag_space)
            .middle_of(state.ids.bag_numbers_alignment)
            .font_size(if bag_space.len() < 6 { 9 } else { 8 })
            .font_id(self.fonts.cyri.conrod_id)
            .color(BLACK)
            .graphics_for(state.ids.bag_img_frame)
            .set(state.ids.bag_space_bg, ui);
        Text::new(&bag_space)
            .bottom_right_with_margins_on(state.ids.bag_space_bg, 1.0, 1.0)
            .font_size(if bag_space.len() < 6 { 9 } else { 8 })
            .font_id(self.fonts.cyri.conrod_id)
            .color(if bag_space_percentage < 0.6 {
                TEXT_VELORITE
            } else if bag_space_percentage < 1.0 {
                LOW_HP_COLOR
            } else {
                CRITICAL_HP_COLOR
            })
            .graphics_for(state.ids.bag_img_frame)
            .set(state.ids.bag_space, ui);

        Image::new(self.imgs.bag_ico)
            .w_h(24.0, 24.0)
            .graphics_for(state.ids.bag_img_frame)
            .mid_bottom_with_margin_on(state.ids.bag_img_frame, 13.0)
            .set(state.ids.bag_img, ui);

        if let Some(bag) = &self
            .global_state
            .settings
            .controls
            .get_binding(GameInput::Bag)
        {
            self.create_new_button_with_shadow(
                ui,
                bag,
                state.ids.bag_img,
                state.ids.bag_text_bg,
                state.ids.bag_text,
            );
        }

        // Exp Type and Level Display

        // Unspent SP indicator
        let unspent_sp = self.skillset.has_available_sp();
        if unspent_sp {
            let arrow_ani = animation_timer(self.pulse); //Animation timer
            Image::new(self.imgs.sp_indicator_arrow)
                .w_h(20.0, 11.0)
                .graphics_for(state.ids.exp_img_frame)
                .mid_top_with_margin_on(state.ids.exp_img_frame, -12.0 + arrow_ani as f64)
                .color(Some(QUALITY_LEGENDARY))
                .set(state.ids.sp_arrow, ui);
            Text::new(&self.localized_strings.get_msg("hud-sp_arrow_txt"))
                .mid_top_with_margin_on(state.ids.sp_arrow, -18.0)
                .graphics_for(state.ids.exp_img_frame)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(BLACK)
                .set(state.ids.sp_arrow_txt_bg, ui);
            Text::new(&self.localized_strings.get_msg("hud-sp_arrow_txt"))
                .graphics_for(state.ids.exp_img_frame)
                .bottom_right_with_margins_on(state.ids.sp_arrow_txt_bg, 1.0, 1.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(QUALITY_LEGENDARY)
                .set(state.ids.sp_arrow_txt, ui);
        }

        if self
            .global_state
            .settings
            .interface
            .xp_bar_skillgroup
            .is_some()
        {
            let offset = -81.0;
            let selected_experience = &self
                .global_state
                .settings
                .interface
                .xp_bar_skillgroup
                .unwrap_or(SkillGroupKind::General);
            let current_exp = self.skillset.available_experience(*selected_experience) as f64;
            let max_exp = self.skillset.skill_point_cost(*selected_experience) as f64;
            let exp_percentage = current_exp / max_exp.max(1.0);
            let level = self.skillset.earned_sp(*selected_experience);
            let level_txt = if level > 0 {
                self.skillset.earned_sp(*selected_experience).to_string()
            } else {
                "".to_string()
            };

            // Exp Bar
            Image::new(self.imgs.exp_frame_bg)
                .w_h(594.0, 8.0)
                .mid_top_with_margin_on(state.ids.frame, -offset)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.9)))
                .set(state.ids.exp_frame_bg, ui);
            Image::new(self.imgs.exp_frame)
                .w_h(594.0, 8.0)
                .middle_of(state.ids.exp_frame_bg)
                .set(state.ids.exp_frame, ui);

            Image::new(self.imgs.bar_content)
                .w_h(590.0 * exp_percentage, 4.0)
                .color(Some(XP_COLOR))
                .top_left_with_margins_on(state.ids.exp_frame, 2.0, 2.0)
                .set(state.ids.exp_filling, ui);
            // Exp Type and Level Display
            Image::new(self.imgs.selected_exp_bg)
                .w_h(34.0, 38.0)
                .top_left_with_margins_on(state.ids.exp_frame, -39.0, 3.0)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .set(state.ids.exp_img_frame_bg, ui);

            if Button::image(self.imgs.selected_exp)
                .w_h(34.0, 38.0)
                .middle_of(state.ids.exp_img_frame_bg)
                .set(state.ids.exp_img_frame, ui)
                .was_clicked()
            {
                return Some(Event::OpenDiary(*selected_experience));
            }

            Text::new(&level_txt)
                .mid_bottom_with_margin_on(state.ids.exp_img_frame, 2.0)
                .font_size(11)
                .font_id(self.fonts.cyri.conrod_id)
                .color(QUALITY_LEGENDARY)
                .graphics_for(state.ids.exp_img_frame)
                .set(state.ids.exp_lvl, ui);

            Image::new(match selected_experience {
                SkillGroupKind::General => self.imgs.swords_crossed,
                SkillGroupKind::Weapon(ToolKind::Sword) => self.imgs.sword,
                SkillGroupKind::Weapon(ToolKind::Hammer) => self.imgs.hammer,
                SkillGroupKind::Weapon(ToolKind::Axe) => self.imgs.axe,
                SkillGroupKind::Weapon(ToolKind::Sceptre) => self.imgs.sceptre,
                SkillGroupKind::Weapon(ToolKind::Bow) => self.imgs.bow,
                SkillGroupKind::Weapon(ToolKind::Staff) => self.imgs.staff,
                SkillGroupKind::Weapon(ToolKind::Pick) => self.imgs.mining,
                _ => self.imgs.nothing,
            })
            .w_h(24.0, 24.0)
            .graphics_for(state.ids.exp_img_frame)
            .mid_bottom_with_margin_on(state.ids.exp_img_frame, 13.0)
            .set(state.ids.exp_img, ui);

            // Show Shortcut
            if let Some(spell) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Spellbook)
            {
                self.create_new_button_with_shadow(
                    ui,
                    spell,
                    state.ids.exp_img,
                    state.ids.spellbook_txt_bg,
                    state.ids.spellbook_txt,
                );
            }
        } else {
            // Only show Spellbook ico
            Image::new(self.imgs.selected_exp_bg)
                .w_h(34.0, 38.0)
                .bottom_left_with_margins_on(state.ids.slot1, 0.0, -37.0)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .set(state.ids.exp_img_frame_bg, ui);

            if Button::image(self.imgs.selected_exp)
                .w_h(34.0, 38.0)
                .middle_of(state.ids.exp_img_frame_bg)
                .set(state.ids.exp_img_frame, ui)
                .was_clicked()
            {
                return Some(Event::OpenDiary(SkillGroupKind::General));
            }

            Image::new(self.imgs.spellbook_ico0)
                .w_h(24.0, 24.0)
                .graphics_for(state.ids.exp_img_frame)
                .mid_bottom_with_margin_on(state.ids.exp_img_frame, 13.0)
                .set(state.ids.exp_img, ui);

            // Show Shortcut
            if let Some(spell) = &self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Spellbook)
            {
                self.create_new_button_with_shadow(
                    ui,
                    spell,
                    state.ids.exp_img,
                    state.ids.spellbook_txt_bg,
                    state.ids.spellbook_txt,
                );
            }
        }

        // Bar Text
        let bar_text = if self.health.is_dead {
            Some((
                self.localized_strings
                    .get_msg("hud-group-dead")
                    .into_owned(),
                self.localized_strings
                    .get_msg("hud-group-dead")
                    .into_owned(),
                self.localized_strings
                    .get_msg("hud-group-dead")
                    .into_owned(),
            ))
        } else if let BarNumbers::Values = bar_values {
            Some((
                format!(
                    "{}/{}",
                    self.health.current().round().max(1.0) as u32, /* Don't show 0 health for
                                                                    * living players */
                    self.health.maximum().round() as u32
                ),
                format!(
                    "{}/{}",
                    self.energy.current().round() as u32,
                    self.energy.maximum().round() as u32
                ),
                String::new(), // Don't obscure the tick mark
            ))
        } else if let BarNumbers::Percent = bar_values {
            Some((
                format!("{}%", hp_percentage as u32),
                format!("{}%", energy_percentage as u32),
                String::new(), // Don't obscure the tick mark
            ))
        } else {
            None
        };
        if let Some((hp_txt, energy_txt, poise_txt)) = bar_text {
            Text::new(&hp_txt)
                .middle_of(state.ids.frame_health)
                .font_size(self.fonts.cyri.scale(12))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.hp_txt_bg, ui);
            Text::new(&hp_txt)
                .bottom_left_with_margins_on(state.ids.hp_txt_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(12))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.hp_txt, ui);

            Text::new(&energy_txt)
                .middle_of(state.ids.frame_energy)
                .font_size(self.fonts.cyri.scale(12))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.energy_txt_bg, ui);
            Text::new(&energy_txt)
                .bottom_left_with_margins_on(state.ids.energy_txt_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(12))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.energy_txt, ui);

            Text::new(&poise_txt)
                .middle_of(state.ids.frame_poise)
                .font_size(self.fonts.cyri.scale(12))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.poise_txt_bg, ui);
            Text::new(&poise_txt)
                .bottom_left_with_margins_on(state.ids.poise_txt_bg, 2.0, 2.0)
                .font_size(self.fonts.cyri.scale(12))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.poise_txt, ui);
        }
        None
    }

    fn show_slotbar(&mut self, state: &State, ui: &mut UiCell, slot_offset: f64) {
        let shortcuts = self.global_state.settings.interface.shortcut_numbers;
        let key_layout = &self.global_state.window.key_layout;

        // TODO: avoid this
        let content_source = (
            self.hotbar,
            self.inventory,
            self.energy,
            self.skillset,
            self.active_abilities,
            self.body,
            self.context,
            self.combo,
            self.char_state,
            self.stance,
        );

        let image_source = (self.item_imgs, self.imgs);
        let mut slot_maker = SlotMaker {
            // TODO: is a separate image needed for the frame?
            empty_slot: self.imgs.skillbar_slot,
            filled_slot: self.imgs.skillbar_slot,
            selected_slot: self.imgs.inv_slot_sel,
            background_color: None,
            content_size: ContentSize {
                width_height_ratio: 1.0,
                max_fraction: 0.9, /* Changes the item image size by setting a maximum fraction
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
            pulse: self.pulse,
        };

        // Tooltips
        let tooltip = Tooltip::new({
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
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);

        let item_tooltip = ItemTooltip::new(
            {
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
            },
            self.client,
            self.info,
            self.imgs,
            self.item_imgs,
            self.pulse,
            self.msm,
            self.localized_strings,
        )
        .title_font_size(self.fonts.cyri.scale(20))
        .parent(ui.window)
        .desc_font_size(self.fonts.cyri.scale(12))
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);

        let slot_content = |slot| {
            let (hotbar, inventory, ..) = content_source;
            hotbar.get(slot).and_then(|content| match content {
                hotbar::SlotContents::Inventory(i, _) => inventory.get_by_hash(i),
                _ => None,
            })
        };

        // Helper
        let tooltip_text = |slot| {
            let (hotbar, inventory, _, skill_set, active_abilities, _, context, _, _, _) =
                content_source;
            hotbar.get(slot).and_then(|content| match content {
                hotbar::SlotContents::Inventory(i, _) => inventory
                    .get_by_hash(i)
                    .map(|item| (item.name(), Cow::Borrowed(item.description()))),
                hotbar::SlotContents::Ability(i) => active_abilities
                    .and_then(|a| {
                        a.auxiliary_set(Some(inventory), Some(skill_set))
                            .get(i)
                            .and_then(|a| {
                                Ability::from(*a).ability_id(
                                    Some(inventory),
                                    Some(skill_set),
                                    context,
                                )
                            })
                    })
                    .map(|id| util::ability_description(id, self.localized_strings)),
            })
        };

        slot_maker.empty_slot = self.imgs.skillbar_slot;
        slot_maker.selected_slot = self.imgs.skillbar_slot;

        let slots = slot_entries(state, slot_offset);
        for entry in slots {
            let slot = slot_maker
                .fabricate(entry.slot, [40.0; 2])
                .filled_slot(self.imgs.skillbar_slot)
                .position(entry.position);
            // if there is an item attached, show item tooltip
            if let Some(item) = slot_content(entry.slot) {
                slot.with_item_tooltip(
                    self.item_tooltip_manager,
                    core::iter::once(item as &dyn ItemDesc),
                    &None,
                    &item_tooltip,
                )
                .set(entry.widget_id, ui);
            // if we can gather some text to display, show it
            } else if let Some((title, desc)) = tooltip_text(entry.slot) {
                slot.with_tooltip(self.tooltip_manager, &title, &desc, &tooltip, TEXT_COLOR)
                    .set(entry.widget_id, ui);
            // if not, just set slot
            } else {
                slot.set(entry.widget_id, ui);
            }

            // shortcuts
            if let ShortcutNumbers::On = shortcuts {
                if let Some(key) = &self
                    .global_state
                    .settings
                    .controls
                    .get_binding(entry.game_input)
                {
                    let position = entry.shortcut_position;
                    let position_bg = entry.shortcut_position_bg;
                    let (id, id_bg) = entry.shortcut_widget_ids;

                    let key_desc = key.display_shortest(key_layout);
                    // shortcut text
                    Text::new(&key_desc)
                        .position(position)
                        .font_size(self.fonts.cyri.scale(8))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(id, ui);
                    // shortcut background
                    Text::new(&key_desc)
                        .position(position_bg)
                        .font_size(self.fonts.cyri.scale(8))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(BLACK)
                        .set(id_bg, ui);
                }
            }
        }
        // Slot M1
        Image::new(self.imgs.skillbar_slot)
            .w_h(40.0, 40.0)
            .right_from(state.ids.slot5, slot_offset)
            .set(state.ids.m1_slot_bg, ui);

        let primary_ability_id = self.active_abilities.and_then(|a| {
            Ability::from(a.primary).ability_id(
                Some(self.inventory),
                Some(self.skillset),
                self.context,
            )
        });

        let (primary_ability_title, primary_ability_desc) =
            util::ability_description(primary_ability_id.unwrap_or(""), self.localized_strings);

        Button::image(
            primary_ability_id.map_or(self.imgs.nothing, |id| util::ability_image(self.imgs, id)),
        )
        .w_h(36.0, 36.0)
        .middle_of(state.ids.m1_slot_bg)
        .with_tooltip(
            self.tooltip_manager,
            &primary_ability_title,
            &primary_ability_desc,
            &tooltip,
            TEXT_COLOR,
        )
        .set(state.ids.m1_content, ui);
        // Slot M2
        Image::new(self.imgs.skillbar_slot)
            .w_h(40.0, 40.0)
            .right_from(state.ids.m1_slot_bg, slot_offset)
            .set(state.ids.m2_slot_bg, ui);

        let secondary_ability_id = self.active_abilities.and_then(|a| {
            Ability::from(a.secondary).ability_id(
                Some(self.inventory),
                Some(self.skillset),
                self.context,
            )
        });

        let (secondary_ability_title, secondary_ability_desc) =
            util::ability_description(secondary_ability_id.unwrap_or(""), self.localized_strings);

        Button::image(
            secondary_ability_id.map_or(self.imgs.nothing, |id| util::ability_image(self.imgs, id)),
        )
        .w_h(36.0, 36.0)
        .middle_of(state.ids.m2_slot_bg)
        .image_color(
            if self
                .active_abilities
                .and_then(|a| {
                    a.activate_ability(
                        AbilityInput::Secondary,
                        Some(self.inventory),
                        self.skillset,
                        Some(self.body),
                        self.char_state,
                        self.context,
                    )
                })
                .map_or(false, |(a, _)| {
                    self.energy.current() >= a.energy_cost()
                        && self.combo.map_or(false, |c| c.counter() >= a.combo_cost())
                        && a.ability_meta().requirements.requirements_met(self.stance)
                })
            {
                Color::Rgba(1.0, 1.0, 1.0, 1.0)
            } else {
                Color::Rgba(0.3, 0.3, 0.3, 0.8)
            },
        )
        .with_tooltip(
            self.tooltip_manager,
            &secondary_ability_title,
            &secondary_ability_desc,
            &tooltip,
            TEXT_COLOR,
        )
        .set(state.ids.m2_content, ui);

        // M1 and M2 icons
        Image::new(self.imgs.m1_ico)
            .w_h(16.0, 18.0)
            .mid_bottom_with_margin_on(state.ids.m1_content, -11.0)
            .set(state.ids.m1_ico, ui);
        Image::new(self.imgs.m2_ico)
            .w_h(16.0, 18.0)
            .mid_bottom_with_margin_on(state.ids.m2_content, -11.0)
            .set(state.ids.m2_ico, ui);
    }

    fn show_combo_counter(&self, combo_floater: ComboFloater, state: &State, ui: &mut UiCell) {
        if combo_floater.combo > 0 {
            let combo_txt = format!("{} Combo", combo_floater.combo);
            let combo_cnt = combo_floater.combo as f32;
            let time_since_last_update = comp::combo::COMBO_DECAY_START - combo_floater.timer;
            let alpha = (1.0 - time_since_last_update * 0.2).min(1.0) as f32;
            let fnt_col = Color::Rgba(
                // White -> Yellow -> Red text color gradient depending on count
                (1.0 - combo_cnt / (combo_cnt + 20.0)).max(0.79),
                (1.0 - combo_cnt / (combo_cnt + 80.0)).max(0.19),
                (1.0 - combo_cnt / (combo_cnt + 5.0)).max(0.17),
                alpha,
            );
            // Increase size for higher counts,
            // "flash" on update by increasing the font size by 2.
            let fnt_size = ((14.0 + combo_floater.timer as f32 * 0.8).min(30.0)) as u32
                + if (time_since_last_update) < 0.1 { 2 } else { 0 };

            Rectangle::fill_with([10.0, 10.0], color::TRANSPARENT)
                .middle_of(ui.window)
                .set(state.ids.combo_align, ui);

            Text::new(combo_txt.as_str())
                .mid_bottom_with_margin_on(
                    state.ids.combo_align,
                    -350.0 + time_since_last_update * -8.0,
                )
                .font_size(self.fonts.cyri.scale(fnt_size))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.0, 0.0, 0.0, alpha))
                .set(state.ids.combo_bg, ui);
            Text::new(combo_txt.as_str())
                .bottom_right_with_margins_on(state.ids.combo_bg, 1.0, 1.0)
                .font_size(self.fonts.cyri.scale(fnt_size))
                .font_id(self.fonts.cyri.conrod_id)
                .color(fnt_col)
                .set(state.ids.combo, ui);
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Widget for Skillbar<'a> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(mut self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Skillbar::update");
        let widget::UpdateArgs { state, ui, .. } = args;

        let slot_offset = 3.0;

        // Death message
        if self.health.is_dead {
            self.show_death_message(state, ui);
        }

        // Skillbar

        // Poise bar ticks
        state.update(|s| {
            s.ids.poise_ticks.resize(
                self::Poise::POISE_THRESHOLDS.len(),
                &mut ui.widget_id_generator(),
            )
        });

        // Alignment and BG
        let alignment_size = 40.0 * 12.0 + slot_offset * 11.0;
        Rectangle::fill_with([alignment_size, 80.0], color::TRANSPARENT)
            .mid_bottom_with_margin_on(ui.window, 10.0)
            .set(state.ids.frame, ui);

        // Health, Energy and Poise bars
        let event = self.show_stat_bars(state, ui);

        // Slots
        self.show_slotbar(state, ui, slot_offset);

        // Combo Counter
        if let Some(combo_floater) = self.combo_floater {
            self.show_combo_counter(combo_floater, state, ui);
        }
        event
    }
}
