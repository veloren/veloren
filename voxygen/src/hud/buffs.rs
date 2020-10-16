use super::{
    img_ids::{Imgs, ImgsRot},
    BUFF_COLOR, DEBUFF_COLOR, TEXT_COLOR,
};
use crate::{
    hud::{get_buff_info, BuffPosition},
    i18n::VoxygenLocalization,
    ui::{fonts::ConrodVoxygenFonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
    GlobalState,
};

use crate::hud::BuffInfo;
use common::comp::{BuffId, Buffs};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle},
    widget_ids, Color, Positionable, Sizeable, Widget, WidgetCommon,
};
use inline_tweak::*;
use std::time::Duration;
widget_ids! {
    struct Ids {
        align,
        buffs_align,
        debuffs_align,
        buff_test,
        debuff_test,
        buffs[],
        buff_timers[],
        debuffs[],
        debuff_timers[],
    }
}

#[derive(WidgetCommon)]
pub struct BuffsBar<'a> {
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    buffs: &'a Buffs,
    pulse: f32,
    global_state: &'a GlobalState,
}

impl<'a> BuffsBar<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        buffs: &'a Buffs,
        pulse: f32,
        global_state: &'a GlobalState,
    ) -> Self {
        Self {
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            rot_imgs,
            tooltip_manager,
            localized_strings,
            buffs,
            pulse,
            global_state,
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    RemoveBuff(BuffId),
}

impl<'a> Widget for BuffsBar<'a> {
    type Event = Vec<Event>;
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
        let mut event = Vec::new();
        let localized_strings = self.localized_strings;
        let buffs = self.buffs;
        let buff_ani = ((self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8) + 0.5; //Animation timer
        let buff_position = self.global_state.settings.gameplay.buff_position;
        let buffs_tooltip = Tooltip::new({
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
        if let BuffPosition::Bar = buff_position {
            // Alignment
            Rectangle::fill_with([484.0, 100.0], color::TRANSPARENT)
                .mid_bottom_with_margin_on(ui.window, tweak!(92.0))
                .set(state.ids.align, ui);
            Rectangle::fill_with([484.0 / 2.0, 90.0], color::TRANSPARENT)
                .bottom_left_with_margins_on(state.ids.align, 0.0, 0.0)
                .set(state.ids.debuffs_align, ui);
            Rectangle::fill_with([484.0 / 2.0, 90.0], color::TRANSPARENT)
                .bottom_right_with_margins_on(state.ids.align, 0.0, 0.0)
                .set(state.ids.buffs_align, ui);

            // Buffs and Debuffs
            // Create two vecs to display buffs and debuffs separately
            let mut buffs_vec = Vec::<BuffInfo>::new();
            let mut debuffs_vec = Vec::<BuffInfo>::new();
            for buff in buffs.active_buffs.clone() {
                let info = get_buff_info(buff);
                if info.is_buff {
                    buffs_vec.push(info);
                } else {
                    debuffs_vec.push(info);
                }
            }
            if state.ids.buffs.len() < buffs_vec.len() {
                state.update(|state| {
                    state
                        .ids
                        .buffs
                        .resize(buffs_vec.len(), &mut ui.widget_id_generator())
                });
            };
            if state.ids.debuffs.len() < debuffs_vec.len() {
                state.update(|state| {
                    state
                        .ids
                        .debuffs
                        .resize(debuffs_vec.len(), &mut ui.widget_id_generator())
                });
            };
            if state.ids.buff_timers.len() < buffs_vec.len() {
                state.update(|state| {
                    state
                        .ids
                        .buff_timers
                        .resize(buffs_vec.len(), &mut ui.widget_id_generator())
                });
            };
            if state.ids.debuff_timers.len() < debuffs_vec.len() {
                state.update(|state| {
                    state
                        .ids
                        .debuff_timers
                        .resize(debuffs_vec.len(), &mut ui.widget_id_generator())
                });
            };
            let pulsating_col = Color::Rgba(1.0, 1.0, 1.0, buff_ani);
            let norm_col = Color::Rgba(1.0, 1.0, 1.0, 1.0);
            // Create Buff Widgets
            for (i, buff) in buffs_vec.iter().enumerate() {
                if i < 22 {
                    // Limit displayed buffs
                    let max_duration = match buff.id {
                        BuffId::Regeneration { duration, .. } => duration.unwrap().as_secs_f32(),
                        _ => 10.0,
                    };
                    let current_duration = buff.dur;
                    let duration_percentage = (current_duration / max_duration * 1000.0) as u32; // Percentage to determine which frame of the timer overlay is displayed
                    let buff_img = match buff.id {
                        BuffId::Regeneration { .. } => self.imgs.buff_plus_0,
                        _ => self.imgs.missing_icon,
                    };
                    let buff_widget = Image::new(buff_img).w_h(20.0, 20.0);
                    // Sort buffs into rows of 11 slots
                    let x = i % 11;
                    let y = i / 11;
                    let buff_widget = buff_widget.bottom_left_with_margins_on(
                        state.ids.buffs_align,
                        0.0 + y as f64 * (21.0),
                        0.0 + x as f64 * (21.0),
                    );
                    buff_widget
                        .color(if current_duration < 10.0 {
                            Some(pulsating_col)
                        } else {
                            Some(norm_col)
                        })
                        .set(state.ids.buffs[i], ui);
                    // Create Buff tooltip
                    let title = match buff.id {
                        BuffId::Regeneration { .. } => {
                            *&localized_strings.get("buff.title.heal_test")
                        },
                        _ => *&localized_strings.get("buff.title.missing"),
                    };
                    let remaining_time = if current_duration == 10e6 as f32 {
                        "Permanent".to_string()
                    } else {
                        format!("Remaining: {:.0}s", current_duration)
                    };
                    let click_to_remove = format!("<{}>", &localized_strings.get("buff.remove"));
                    let desc_txt = match buff.id {
                        BuffId::Regeneration { .. } => {
                            *&localized_strings.get("buff.desc.heal_test")
                        },
                        _ => *&localized_strings.get("buff.desc.missing"),
                    };
                    let desc = format!("{}\n\n{}\n\n{}", desc_txt, remaining_time, click_to_remove);
                    // Timer overlay
                    if Button::image(match duration_percentage as u64 {
                        875..=1000 => self.imgs.nothing, // 8/8
                        750..=874 => self.imgs.buff_0,   // 7/8
                        625..=749 => self.imgs.buff_1,   // 6/8
                        500..=624 => self.imgs.buff_2,   // 5/8
                        375..=499 => self.imgs.buff_3,   // 4/8
                        250..=374 => self.imgs.buff_4,   //3/8
                        125..=249 => self.imgs.buff_5,   // 2/8
                        0..=124 => self.imgs.buff_6,     // 1/8
                        _ => self.imgs.nothing,
                    })
                    .w_h(20.0, 20.0)
                    .middle_of(state.ids.buffs[i])
                    .with_tooltip(
                        self.tooltip_manager,
                        title,
                        &desc,
                        &buffs_tooltip,
                        BUFF_COLOR,
                    )
                    .set(state.ids.buff_timers[i], ui)
                    .was_clicked()
                    {
                        event.push(Event::RemoveBuff(buff.id));
                    };
                };
            }
            // Create Debuff Widgets
            for (i, debuff) in debuffs_vec.iter().enumerate() {
                if i < 22 {
                    // Limit displayed buffs

                    let max_duration = match debuff.id {
                        BuffId::Bleeding { duration, .. } => {
                            duration.unwrap_or(Duration::from_secs(60)).as_secs_f32()
                        },
                        BuffId::Cursed { duration, .. } => {
                            duration.unwrap_or(Duration::from_secs(60)).as_secs_f32()
                        },

                        _ => 10.0,
                    };
                    let current_duration = debuff.dur;
                    let duration_percentage = current_duration / max_duration * 1000.0; // Percentage to determine which frame of the timer overlay is displayed           
                    let debuff_img = match debuff.id {
                        BuffId::Bleeding { .. } => self.imgs.debuff_bleed_0,
                        BuffId::Cursed { .. } => self.imgs.debuff_skull_0,
                        _ => self.imgs.missing_icon,
                    };
                    let debuff_widget = Image::new(debuff_img).w_h(20.0, 20.0);
                    // Sort buffs into rows of 11 slots
                    let x = i % 11;
                    let y = i / 11;
                    let debuff_widget = debuff_widget.bottom_right_with_margins_on(
                        state.ids.debuffs_align,
                        0.0 + y as f64 * (21.0),
                        0.0 + x as f64 * (21.0),
                    );

                    debuff_widget
                        .color(if current_duration < 10.0 {
                            Some(pulsating_col)
                        } else {
                            Some(norm_col)
                        })
                        .set(state.ids.debuffs[i], ui);
                    // Create Debuff tooltip
                    let title = match debuff.id {
                        BuffId::Bleeding { .. } => {
                            *&localized_strings.get("debuff.title.bleed_test")
                        },
                        _ => *&localized_strings.get("buff.title.missing"),
                    };
                    let remaining_time = if current_duration == 10e6 as f32 {
                        "Permanent".to_string()
                    } else {
                        format!("Remaining: {:.0}s", current_duration)
                    };
                    let desc_txt = match debuff.id {
                        BuffId::Bleeding { .. } => {
                            *&localized_strings.get("debuff.desc.bleed_test")
                        },
                        _ => *&localized_strings.get("debuff.desc.missing"),
                    };
                    let desc = format!("{}\n\n{}", desc_txt, remaining_time);
                    Image::new(match duration_percentage as u64 {
                        875..=1000 => self.imgs.nothing, // 8/8
                        750..=874 => self.imgs.buff_0,   // 7/8
                        625..=749 => self.imgs.buff_1,   // 6/8
                        500..=624 => self.imgs.buff_2,   // 5/8
                        375..=499 => self.imgs.buff_3,   // 4/8
                        250..=374 => self.imgs.buff_4,   //3/8
                        125..=249 => self.imgs.buff_5,   // 2/8
                        0..=124 => self.imgs.buff_6,     // 1/8
                        _ => self.imgs.nothing,
                    })
                    .w_h(20.0, 20.0)
                    .middle_of(state.ids.debuffs[i])
                    .with_tooltip(
                        self.tooltip_manager,
                        title,
                        &desc,
                        &buffs_tooltip,
                        DEBUFF_COLOR,
                    )
                    .set(state.ids.debuff_timers[i], ui);
                };
            }
        }
        if let BuffPosition::Map = buff_position {
            // Alignment
            Rectangle::fill_with([tweak!(300.0), tweak!(280.0)], color::RED)
                .top_right_with_margins_on(ui.window, tweak!(5.0), tweak!(270.0))
                .set(state.ids.align, ui);
        }
        event
    }
}
