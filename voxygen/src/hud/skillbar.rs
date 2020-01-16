use super::{
    img_ids::Imgs, BarNumbers, Fonts, ShortcutNumbers, XpBar, CRITICAL_HP_COLOR, HP_COLOR,
    LOW_HP_COLOR, MANA_COLOR, TEXT_COLOR, XP_COLOR,
};
use crate::GlobalState;
use common::comp::{item::Debug, item::ToolData, item::ToolKind, Energy, ItemKind, Stats};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
use std::time::{Duration, Instant};

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
        m1_content,
        m2_slot,
        m2_slot_bg,
        m2_text,
        m2_content,
        slot1,
        slot1_bg,
        slot1_text,
        slot2,
        slot2_bg,
        slot2_text,
        slot3,
        slot3_bg,
        slot3_text,
        slot4,
        slot4_bg,
        slot4_text,
        slot5,
        slot5_bg,
        slot5_text,
        slot6,
        slot6_bg,
        slot6_text,
        slot7,
        slot7_bg,
        slot7_text,
        slot8,
        slot8_bg,
        slot8_text,
        slot9,
        slot9_bg,
        slot9_text,
        slotq,
        slotq_bg,
        slotq_text,
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
        stamina_wheel,
        death_bg,
        hurt_bg,
    }
}

pub enum ResourceType {
    Mana,
    //Rage,
    //Focus,
}
#[derive(WidgetCommon)]
pub struct Skillbar<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    stats: &'a Stats,
    energy: &'a Energy,
    pulse: f32,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    current_resource: ResourceType,
}

impl<'a> Skillbar<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        stats: &'a Stats,
        energy: &'a Energy,
        pulse: f32,
    ) -> Self {
        Self {
            global_state,
            imgs,
            fonts,
            stats,
            energy,
            global_state,
            current_resource: ResourceType::Mana,
            common: widget::CommonBuilder::default(),
            pulse,
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
    type State = State;
    type Style = ();
    type Event = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),

            last_xp_value: 0,
            last_level: 1,
            last_update_xp: Instant::now(),
            last_update_level: Instant::now(),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

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

        const BG_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 0.8);
        const BG_COLOR_2: Color = Color::Rgba(0.0, 0.0, 0.0, 0.99);
        let hp_ani = (self.pulse * 4.0/*speed factor*/).cos() * 0.5 + 0.8; //Animation timer
        let crit_hp_color: Color = Color::Rgba(0.79, 0.19, 0.17, hp_ani);

        // Stamina Wheel
        /*
        let stamina_percentage =
            self.stats.health.current() as f64 / self.stats.health.maximum() as f64 * 100.0;
        if stamina_percentage < 100.0 {
            Image::new(if stamina_percentage <= 0.1 {
                self.imgs.stamina_0
            } else if stamina_percentage < 12.5 {
                self.imgs.stamina_1
            } else if stamina_percentage < 25.0 {
                self.imgs.stamina_2
            } else if stamina_percentage < 37.5 {
                self.imgs.stamina_3
            } else if stamina_percentage < 50.0 {
                self.imgs.stamina_4
            } else if stamina_percentage < 62.5 {
                self.imgs.stamina_5
            } else if stamina_percentage < 75.0 {
                self.imgs.stamina_6
            } else if stamina_percentage < 87.5 {
                self.imgs.stamina_7
            } else {
                self.imgs.stamina_8
            })
            .w_h(37.0 * 3.0, 37.0 * 3.0)
            .mid_bottom_with_margin_on(ui.window, 150.0)
            .set(state.ids.stamina_wheel, ui);
        }
        */

        // Level Up Message

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
        let level_up_text = format!("Level {}", self.stats.level.level() as u32);
        Text::new(&level_up_text)
            .middle_of(state.ids.level_align)
            .font_size(30)
            .font_id(self.fonts.cyri)
            .color(Color::Rgba(0.0, 0.0, 0.0, fade_level))
            .set(state.ids.level_message_bg, ui);
        Text::new(&level_up_text)
            .bottom_left_with_margins_on(state.ids.level_message_bg, 2.0, 2.0)
            .font_size(30)
            .font_id(self.fonts.cyri)
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
        // Death message
        if self.stats.is_dead {
            Text::new("You Died")
                .middle_of(ui.window)
                .font_size(50)
                .font_id(self.fonts.cyri)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.death_message_1_bg, ui);
            Text::new(&format!(
                "Press {:?} to respawn at your Waypoint.\n\
                 \n\
                 Press Enter, type in /waypoint and confirm to set it here.",
                self.global_state.settings.controls.respawn
            ))
            .mid_bottom_with_margin_on(state.ids.death_message_1_bg, -120.0)
            .font_size(30)
            .font_id(self.fonts.cyri)
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(state.ids.death_message_2_bg, ui);
            Text::new("You Died")
                .bottom_left_with_margins_on(state.ids.death_message_1_bg, 2.0, 2.0)
                .font_size(50)
                .font_id(self.fonts.cyri)
                .color(CRITICAL_HP_COLOR)
                .set(state.ids.death_message_1, ui);
            Text::new(&format!(
                "Press {:?} to respawn at your Waypoint.\n\
                 \n\
                 Press Enter, type in /waypoint and confirm to set it here.",
                self.global_state.settings.controls.respawn
            ))
            .bottom_left_with_margins_on(state.ids.death_message_2_bg, 2.0, 2.0)
            .font_size(30)
            .color(CRITICAL_HP_COLOR)
            .set(state.ids.death_message_2, ui);
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
                        .font_size(10)
                        .font_id(self.fonts.cyri)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right,
                            3.5 * scale,
                            4.0 * scale,
                        )
                        .font_size(10)
                        .font_id(self.fonts.cyri)
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
                        .font_size(9)
                        .font_id(self.fonts.cyri)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right,
                            3.5 * scale,
                            3.0 * scale,
                        )
                        .font_size(9)
                        .font_id(self.fonts.cyri)
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
                        .font_size(8)
                        .font_id(self.fonts.cyri)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right,
                            3.5 * scale,
                            2.5 * scale,
                        )
                        .font_size(8)
                        .font_id(self.fonts.cyri)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                        .set(state.ids.next_level_text, ui);
                }
                // M1 Slot
                Image::new(self.imgs.skillbar_slot_big)
                    .w_h(40.0 * scale, 40.0 * scale)
                    .top_left_with_margins_on(state.ids.xp_bar_mid, -40.0 * scale, 0.0)
                    .set(state.ids.m1_slot, ui);
            }
            XpBar::OnGain => {
                // Displays the Exp Bar at the top of the screen when exp is gained and fades it out afterwards
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
                        .font_size(17)
                        .font_id(self.fonts.cyri)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right_top,
                            3.0 * scale * 1.5,
                            4.0 * scale * 1.5,
                        )
                        .font_size(15)
                        .font_id(self.fonts.cyri)
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
                        .font_size(15)
                        .font_id(self.fonts.cyri)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right_top,
                            3.0 * scale * 1.5,
                            3.0 * scale * 1.5,
                        )
                        .font_size(15)
                        .font_id(self.fonts.cyri)
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
                        .font_size(12)
                        .font_id(self.fonts.cyri)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.level_text, ui);
                    Text::new(&next_level)
                        .bottom_right_with_margins_on(
                            state.ids.xp_bar_right_top,
                            3.0 * scale * 1.5,
                            2.75 * scale * 1.5,
                        )
                        .font_size(12)
                        .font_id(self.fonts.cyri)
                        .color(Color::Rgba(1.0, 1.0, 1.0, fade_xp))
                        .set(state.ids.next_level_text, ui);
                }
                // Alignment for hotbar
                Rectangle::fill_with([80.0 * scale, 1.0], color::TRANSPARENT)
                    .mid_bottom_with_margin_on(ui.window, 9.0)
                    .set(state.ids.hotbar_align, ui);
                // M1 Slot
                Image::new(self.imgs.skillbar_slot_big)
                    .w_h(40.0 * scale, 40.0 * scale)
                    .top_left_with_margins_on(state.ids.hotbar_align, -40.0 * scale, 0.0)
                    .set(state.ids.m1_slot, ui);
            }
        }
        // M1 Slot
        Image::new(self.imgs.skillbar_slot_big_bg)
            .w_h(36.0 * scale, 36.0 * scale)
            .color(match self.stats.equipment.main.as_ref().map(|i| &i.kind) {
                Some(ItemKind::Tool(ToolData { kind, .. })) => match kind {
                    ToolKind::Bow => Some(BG_COLOR_2),
                    ToolKind::Staff => Some(BG_COLOR_2),
                    _ => Some(BG_COLOR_2),
                },
                _ => Some(BG_COLOR_2),
            })
            .middle_of(state.ids.m1_slot)
            .set(state.ids.m1_slot_bg, ui);
        Button::image(match self.stats.equipment.main.as_ref().map(|i| &i.kind) {
            Some(ItemKind::Tool(ToolData { kind, .. })) => match kind {
                ToolKind::Sword(_) => self.imgs.twohsword_m1,
                ToolKind::Hammer => self.imgs.twohhammer_m1,
                ToolKind::Axe => self.imgs.twohaxe_m1,
                ToolKind::Bow => self.imgs.bow_m1,
                ToolKind::Staff => self.imgs.staff_m1,
                ToolKind::Debug(Debug::Boost) => self.imgs.flyingrod_m1,
                _ => self.imgs.twohaxe_m1,
            },
            _ => self.imgs.twohaxe_m1,
        }) // Insert Icon here
        .w(match self.stats.equipment.main.as_ref().map(|i| &i.kind) {
            Some(ItemKind::Tool(ToolData { kind, .. })) => match kind {
                ToolKind::Bow => 30.0 * scale,
                ToolKind::Staff => 30.0 * scale,
                _ => 38.0 * scale,
            },
            _ => 38.0 * scale,
        })
        .h(match self.stats.equipment.main.as_ref().map(|i| &i.kind) {
            Some(ItemKind::Tool(ToolData { kind, .. })) => match kind {
                ToolKind::Bow => 30.0 * scale,
                ToolKind::Staff => 36.0 * scale,
                _ => 38.0 * scale,
            },
            _ => 38.0 * scale,
        })
        .middle_of(state.ids.m1_slot_bg)
        .set(state.ids.m1_content, ui);
        // M2 Slot
        Image::new(self.imgs.skillbar_slot_big)
            .w_h(40.0 * scale, 40.0 * scale)
            .right_from(state.ids.m1_slot, 0.0)
            .set(state.ids.m2_slot, ui);
        Image::new(self.imgs.skillbar_slot_big_bg)
            .w_h(36.0 * scale, 36.0 * scale)
            .color(match self.stats.equipment.main.as_ref().map(|i| &i.kind) {
                Some(ItemKind::Tool(ToolData { kind, .. })) => match kind {
                    ToolKind::Bow => Some(BG_COLOR_2),
                    ToolKind::Staff => Some(BG_COLOR_2),
                    _ => Some(BG_COLOR_2),
                },
                _ => Some(BG_COLOR_2),
            })
            .middle_of(state.ids.m2_slot)
            .set(state.ids.m2_slot_bg, ui);
        Button::image(match self.stats.equipment.main.as_ref().map(|i| &i.kind) {
            Some(ItemKind::Tool(ToolData { kind, .. })) => match kind {
                ToolKind::Sword(_) => self.imgs.twohsword_m2,
                ToolKind::Hammer => self.imgs.twohhammer_m2,
                ToolKind::Axe => self.imgs.twohaxe_m2,
                ToolKind::Bow => self.imgs.bow_m2,
                ToolKind::Staff => self.imgs.staff_m2,
                ToolKind::Debug(Debug::Boost) => self.imgs.flyingrod_m2,
                _ => self.imgs.twohaxe_m2,
            },
            _ => self.imgs.twohaxe_m2,
        }) // Insert Icon here
        .w(match self.stats.equipment.main.as_ref().map(|i| &i.kind) {
            Some(ItemKind::Tool(ToolData { kind, .. })) => match kind {
                ToolKind::Bow => 30.0 * scale,
                ToolKind::Staff => 30.0 * scale,
                _ => 38.0 * scale,
            },
            _ => 38.0 * scale,
        })
        .h(match self.stats.equipment.main.as_ref().map(|i| &i.kind) {
            Some(ItemKind::Tool(ToolData { kind, .. })) => match kind {
                ToolKind::Bow => 30.0 * scale,
                ToolKind::Staff => 30.0 * scale,
                _ => 38.0 * scale,
            },
            _ => 38.0 * scale,
        })
        .middle_of(state.ids.m2_slot_bg)
        .set(state.ids.m2_content, ui);
        //Slot 5
        Image::new(self.imgs.skillbar_slot)
            .w_h(20.0 * scale, 20.0 * scale)
            .bottom_left_with_margins_on(state.ids.m1_slot, 0.0, -20.0 * scale)
            .set(state.ids.slot5, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slot5)
            .set(state.ids.slot5_bg, ui);
        // Slot 4
        Image::new(self.imgs.skillbar_slot)
            .w_h(20.0 * scale, 20.0 * scale)
            .left_from(state.ids.slot5, 0.0)
            .set(state.ids.slot4, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slot4)
            .set(state.ids.slot4_bg, ui);
        // Slot 3
        Image::new(self.imgs.skillbar_slot)
            .w_h(20.0 * scale, 20.0 * scale)
            .left_from(state.ids.slot4, 0.0)
            .set(state.ids.slot3, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slot3)
            .set(state.ids.slot3_bg, ui);
        // Slot 2
        Image::new(self.imgs.skillbar_slot)
            .w_h(20.0 * scale, 20.0 * scale)
            .left_from(state.ids.slot3, 0.0)
            .set(state.ids.slot2, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slot2)
            .set(state.ids.slot2_bg, ui);
        // Slot 1
        Image::new(self.imgs.skillbar_slot_l)
            .w_h(20.0 * scale, 20.0 * scale)
            .left_from(state.ids.slot2, 0.0)
            .set(state.ids.slot1, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slot1)
            .set(state.ids.slot1_bg, ui);
        // Slot 6
        Image::new(self.imgs.skillbar_slot)
            .w_h(20.0 * scale, 20.0 * scale)
            .bottom_right_with_margins_on(state.ids.m2_slot, 0.0, -20.0 * scale)
            .set(state.ids.slot6, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slot6)
            .set(state.ids.slot6_bg, ui);
        // Slot 7
        Image::new(self.imgs.skillbar_slot)
            .w_h(20.0 * scale, 20.0 * scale)
            .right_from(state.ids.slot6, 0.0)
            .set(state.ids.slot7, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slot7)
            .set(state.ids.slot7_bg, ui);
        // Slot 8
        Image::new(self.imgs.skillbar_slot)
            .w_h(20.0 * scale, 20.0 * scale)
            .right_from(state.ids.slot7, 0.0)
            .set(state.ids.slot8, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slot8)
            .set(state.ids.slot8_bg, ui);
        // Slot 9
        Image::new(self.imgs.skillbar_slot)
            .w_h(20.0 * scale, 20.0 * scale)
            .right_from(state.ids.slot8, 0.0)
            .set(state.ids.slot9, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slot9)
            .set(state.ids.slot9_bg, ui);
        // Quickslot
        Image::new(self.imgs.skillbar_slot_r)
            .w_h(20.0 * scale, 20.0 * scale)
            .right_from(state.ids.slot9, 0.0)
            .set(state.ids.slotq, ui);
        Image::new(self.imgs.skillbar_slot_bg)
            .w_h(19.0 * scale, 19.0 * scale)
            .color(Some(BG_COLOR))
            .middle_of(state.ids.slotq)
            .set(state.ids.slotq_bg, ui);
        // Shortcuts

        if let ShortcutNumbers::On = shortcuts {
            Text::new("1")
                .top_right_with_margins_on(state.ids.slot1_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slot1_text, ui);
            Text::new("2")
                .top_right_with_margins_on(state.ids.slot2_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slot2_text, ui);
            Text::new("3")
                .top_right_with_margins_on(state.ids.slot3_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slot3_text, ui);
            Text::new("4")
                .top_right_with_margins_on(state.ids.slot4_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slot4_text, ui);
            Text::new("5")
                .top_right_with_margins_on(state.ids.slot5_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slot5_text, ui);
            Text::new("M1")
                .top_left_with_margins_on(state.ids.m1_slot, 5.0, 5.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.m1_text, ui);
            Text::new("M2")
                .top_right_with_margins_on(state.ids.m2_slot, 5.0, 5.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.m2_text, ui);
            Text::new("6")
                .top_left_with_margins_on(state.ids.slot6_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slot6_text, ui);
            Text::new("7")
                .top_left_with_margins_on(state.ids.slot7_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slot7_text, ui);
            Text::new("8")
                .top_left_with_margins_on(state.ids.slot8_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slot8_text, ui);
            Text::new("9")
                .top_left_with_margins_on(state.ids.slot9_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slot9_text, ui);
            Text::new("Q")
                .top_left_with_margins_on(state.ids.slotq_bg, 1.0, 1.0)
                .font_size(8)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.slotq_text, ui);
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
                //ResourceType::Focus => FOCUS_COLOR,
                //ResourceType::Rage => RAGE_COLOR,
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
                .font_size(14)
                .font_id(self.fonts.cyri)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.health_text_bg, ui);
            Text::new(&hp_text)
                .bottom_left_with_margins_on(state.ids.health_text_bg, 2.0, 2.0)
                .font_size(14)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.health_text, ui);
            let energy_text = format!(
                "{}/{}",
                self.energy.current() as u32,
                self.energy.maximum() as u32
            );
            Text::new(&energy_text)
                .mid_top_with_margin_on(state.ids.energybar_bg, 6.0 * scale)
                .font_size(14)
                .font_id(self.fonts.cyri)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.energy_text_bg, ui);
            Text::new(&energy_text)
                .bottom_left_with_margins_on(state.ids.energy_text_bg, 2.0, 2.0)
                .font_size(14)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.energy_text, ui);
        }
        //Percentages
        if let BarNumbers::Percent = bar_values {
            let hp_text = format!("{}%", hp_percentage as u32);
            Text::new(&hp_text)
                .mid_top_with_margin_on(state.ids.healthbar_bg, 6.0 * scale)
                .font_size(14)
                .font_id(self.fonts.cyri)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.health_text_bg, ui);
            Text::new(&hp_text)
                .bottom_left_with_margins_on(state.ids.health_text_bg, 2.0, 2.0)
                .font_size(14)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.health_text, ui);
            let energy_text = format!("{}%", energy_percentage as u32);
            Text::new(&energy_text)
                .mid_top_with_margin_on(state.ids.energybar_bg, 6.0 * scale)
                .font_size(14)
                .font_id(self.fonts.cyri)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .set(state.ids.energy_text_bg, ui);
            Text::new(&energy_text)
                .bottom_left_with_margins_on(state.ids.energy_text_bg, 2.0, 2.0)
                .font_size(14)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.energy_text, ui);
        }
    }

    // Buffs
    // Add debuff slots above the health bar
    // Add buff slots above the mana bar

    // Debuffs
}
