use super::{
    img_ids::{Imgs, ImgsRot},
    BUFF_COLOR, DEBUFF_COLOR, TEXT_COLOR,
};
use crate::{
    hud::{get_buff_image, get_buff_info, BuffPosition},
    i18n::Localization,
    ui::{fonts::Fonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
    GlobalState,
};

use common::comp::{BuffKind, Buffs, Energy, Health};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
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
        buff_txts[],
    }
}

#[derive(WidgetCommon)]
pub struct BuffsBar<'a> {
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    localized_strings: &'a Localization,
    buffs: &'a Buffs,
    pulse: f32,
    global_state: &'a GlobalState,
    health: &'a Health,
    energy: &'a Energy,
}

impl<'a> BuffsBar<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        localized_strings: &'a Localization,
        buffs: &'a Buffs,
        pulse: f32,
        global_state: &'a GlobalState,
        health: &'a Health,
        energy: &'a Energy,
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
            health,
            energy,
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    RemoveBuff(BuffKind),
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

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let mut event = Vec::new();
        let localized_strings = self.localized_strings;
        let buffs = self.buffs;
        let buff_ani = ((self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8) + 0.5; //Animation timer
        let pulsating_col = Color::Rgba(1.0, 1.0, 1.0, buff_ani);
        let norm_col = Color::Rgba(1.0, 1.0, 1.0, 1.0);
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
            let show_health = self.health.current() != self.health.maximum();
            let show_stamina = self.energy.current() != self.energy.maximum();
            let offset = if show_health && show_stamina {
                140.0
            } else if show_health || show_stamina {
                95.0
            } else {
                55.0
            };
            // Alignment
            Rectangle::fill_with([484.0, 100.0], color::TRANSPARENT)
                .mid_bottom_with_margin_on(ui.window, offset)
                .set(state.ids.align, ui);
            Rectangle::fill_with([484.0 / 2.0, 90.0], color::TRANSPARENT)
                .bottom_left_with_margins_on(state.ids.align, 0.0, 0.0)
                .set(state.ids.debuffs_align, ui);
            Rectangle::fill_with([484.0 / 2.0, 90.0], color::TRANSPARENT)
                .bottom_right_with_margins_on(state.ids.align, 0.0, 0.0)
                .set(state.ids.buffs_align, ui);

            // Buffs and Debuffs
            let (buff_count, debuff_count) = buffs.iter_active().map(get_buff_info).fold(
                (0, 0),
                |(buff_count, debuff_count), info| {
                    if info.is_buff {
                        (buff_count + 1, debuff_count)
                    } else {
                        (buff_count, debuff_count + 1)
                    }
                },
            );
            // Limit displayed buffs
            let buff_count = buff_count.min(12);
            let debuff_count = debuff_count.min(12);

            let gen = &mut ui.widget_id_generator();
            if state.ids.buffs.len() < buff_count {
                state.update(|state| state.ids.buffs.resize(buff_count, gen));
            };
            if state.ids.debuffs.len() < debuff_count {
                state.update(|state| state.ids.debuffs.resize(debuff_count, gen));
            };
            if state.ids.buff_timers.len() < buff_count {
                state.update(|state| state.ids.buff_timers.resize(buff_count, gen));
            };
            if state.ids.debuff_timers.len() < debuff_count {
                state.update(|state| state.ids.debuff_timers.resize(debuff_count, gen));
            };

            // Create Buff Widgets
            let mut buff_vec = state
                .ids
                .buffs
                .iter()
                .copied()
                .zip(state.ids.buff_timers.iter().copied())
                .zip(
                    buffs
                        .iter_active()
                        .map(get_buff_info)
                        .filter(|info| info.is_buff),
                )
                .collect::<Vec<_>>();

            // Sort the buffs by kind
            buff_vec.sort_by_key(|((_id, _timer_id), buff)| std::cmp::Reverse(buff.kind));

            buff_vec
                .iter()
                .enumerate()
                .for_each(|(i, ((id, timer_id), buff))| {
                    let max_duration = buff.data.duration;
                    let current_duration = buff.dur;
                    let duration_percentage = current_duration.map_or(1000.0, |cur| {
                        max_duration
                            .map_or(1000.0, |max| cur.as_secs_f32() / max.as_secs_f32() * 1000.0)
                    }) as u32; // Percentage to determine which frame of the timer overlay is displayed
                    let buff_img = get_buff_image(buff.kind, self.imgs);
                    let buff_widget = Image::new(buff_img).w_h(40.0, 40.0);
                    // Sort buffs into rows of 11 slots
                    let x = i % 6;
                    let y = i / 6;
                    let buff_widget = buff_widget.bottom_left_with_margins_on(
                        state.ids.buffs_align,
                        0.0 + y as f64 * (41.0),
                        1.5 + x as f64 * (43.0),
                    );
                    buff_widget
                        .color(
                            if current_duration.map_or(false, |cur| cur.as_secs_f32() < 10.0) {
                                Some(pulsating_col)
                            } else {
                                Some(norm_col)
                            },
                        )
                        .set(*id, ui);
                    // Create Buff tooltip
                    let title = match buff.kind {
                        BuffKind::Regeneration { .. } => localized_strings.get("buff.title.heal"),
                        BuffKind::Saturation { .. } => {
                            localized_strings.get("buff.title.saturation")
                        },
                        BuffKind::Potion { .. } => localized_strings.get("buff.title.potion"),
                        BuffKind::CampfireHeal { .. } => {
                            localized_strings.get("buff.title.campfire_heal")
                        },
                        BuffKind::IncreaseMaxHealth { .. } => {
                            localized_strings.get("buff.title.IncreaseMaxHealth")
                        },
                        BuffKind::IncreaseMaxEnergy { .. } => {
                            localized_strings.get("buff.title.staminaup")
                        },
                        _ => localized_strings.get("buff.title.missing"),
                    };
                    let remaining_time = if current_duration.is_none() {
                        "Permanent".to_string()
                    } else {
                        format!("Remaining: {:.0}s", current_duration.unwrap().as_secs_f32())
                    };
                    let click_to_remove = format!("<{}>", &localized_strings.get("buff.remove"));
                    let desc_txt = match buff.kind {
                        BuffKind::Regeneration { .. } => localized_strings.get("buff.desc.heal"),
                        BuffKind::Saturation { .. } => {
                            localized_strings.get("buff.desc.saturation")
                        },
                        BuffKind::Potion { .. } => localized_strings.get("buff.desc.potion"),
                        BuffKind::CampfireHeal { .. } => {
                            localized_strings.get("buff.desc.campfire_heal")
                        },
                        BuffKind::IncreaseMaxHealth { .. } => {
                            localized_strings.get("buff.desc.IncreaseMaxHealth")
                        },
                        BuffKind::IncreaseMaxEnergy { .. } => {
                            localized_strings.get("buff.desc.IncreaseMaxEnergy")
                        },
                        _ => localized_strings.get("buff.desc.missing"),
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
                    .w_h(40.0, 40.0)
                    .middle_of(*id)
                    .with_tooltip(
                        self.tooltip_manager,
                        title,
                        &desc,
                        &buffs_tooltip,
                        BUFF_COLOR,
                    )
                    .set(*timer_id, ui)
                    .was_clicked()
                    {
                        event.push(Event::RemoveBuff(buff.kind));
                    };
                });

            // Create Debuff Widgets
            let mut debuff_vec = state
                .ids
                .debuffs
                .iter()
                .copied()
                .zip(state.ids.debuff_timers.iter().copied())
                .zip(
                    buffs
                        .iter_active()
                        .map(get_buff_info)
                        .filter(|info| !info.is_buff),
                )
                .collect::<Vec<_>>();

            // Sort the debuffs by kind
            debuff_vec.sort_by_key(|((_id, _timer_id), debuff)| debuff.kind);

            debuff_vec
                .iter()
                .enumerate()
                .for_each(|(i, ((id, timer_id), debuff))| {
                    let max_duration = debuff.data.duration;
                    let current_duration = debuff.dur;
                    let duration_percentage = current_duration.map_or(1000.0, |cur| {
                        max_duration
                            .map_or(1000.0, |max| cur.as_secs_f32() / max.as_secs_f32() * 1000.0)
                    }) as u32; // Percentage to determine which frame of the timer overlay is displayed
                    let debuff_img = match debuff.kind {
                        BuffKind::Bleeding { .. } => self.imgs.debuff_bleed_0,
                        BuffKind::Cursed { .. } => self.imgs.debuff_skull_0,
                        _ => self.imgs.missing_icon,
                    };
                    let debuff_widget = Image::new(debuff_img).w_h(40.0, 40.0);
                    // Sort buffs into rows of 11 slots
                    let x = i % 6;
                    let y = i / 6;
                    let debuff_widget = debuff_widget.bottom_right_with_margins_on(
                        state.ids.debuffs_align,
                        0.0 + y as f64 * (41.0),
                        1.5 + x as f64 * (43.0),
                    );

                    debuff_widget
                        .color(
                            if current_duration.map_or(false, |cur| cur.as_secs_f32() < 10.0) {
                                Some(pulsating_col)
                            } else {
                                Some(norm_col)
                            },
                        )
                        .set(*id, ui);
                    // Create Debuff tooltip
                    let title = match debuff.kind {
                        BuffKind::Bleeding { .. } => localized_strings.get("debuff.title.bleed"),
                        _ => localized_strings.get("buff.title.missing"),
                    };
                    let remaining_time = if current_duration.is_none() {
                        "Permanent".to_string()
                    } else {
                        format!("Remaining: {:.0}s", current_duration.unwrap().as_secs_f32())
                    };
                    let desc_txt = match debuff.kind {
                        BuffKind::Bleeding { .. } => localized_strings.get("debuff.desc.bleed"),
                        _ => localized_strings.get("debuff.desc.missing"),
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
                    .w_h(40.0, 40.0)
                    .middle_of(*id)
                    .with_tooltip(
                        self.tooltip_manager,
                        title,
                        &desc,
                        &buffs_tooltip,
                        DEBUFF_COLOR,
                    )
                    .set(*timer_id, ui);
                });
        }

        if let BuffPosition::Map = buff_position {
            // Alignment
            Rectangle::fill_with([210.0, 210.0], color::TRANSPARENT)
                .top_right_with_margins_on(ui.window, 5.0, 270.0)
                .set(state.ids.align, ui);

            // Buffs and Debuffs
            let buff_count = buffs.kinds.len().min(11);
            // Limit displayed buffs
            let buff_count = buff_count.min(20);

            let gen = &mut ui.widget_id_generator();
            if state.ids.buffs.len() < buff_count {
                state.update(|state| state.ids.buffs.resize(buff_count, gen));
            };
            if state.ids.buff_timers.len() < buff_count {
                state.update(|state| state.ids.buff_timers.resize(buff_count, gen));
            };
            if state.ids.buff_txts.len() < buff_count {
                state.update(|state| state.ids.buff_txts.resize(buff_count, gen));
            };

            // Create Buff Widgets

            let mut buff_vec = state
                .ids
                .buffs
                .iter()
                .copied()
                .zip(state.ids.buff_timers.iter().copied())
                .zip(state.ids.buff_txts.iter().copied())
                .zip(buffs.iter_active().map(get_buff_info))
                .collect::<Vec<_>>();

            // Sort the buffs by kind
            buff_vec.sort_by_key(|((_id, _timer_id), txt_id)| std::cmp::Reverse(txt_id.kind));

            buff_vec
                .iter()
                .enumerate()
                .for_each(|(i, (((id, timer_id), txt_id), buff))| {
                    let max_duration = buff.data.duration;
                    let current_duration = buff.dur;
                    // Percentage to determine which frame of the timer overlay is displayed
                    let duration_percentage = current_duration.map_or(1000.0, |cur| {
                        max_duration
                            .map_or(1000.0, |max| cur.as_secs_f32() / max.as_secs_f32() * 1000.0)
                    }) as u32;
                    let buff_img = get_buff_image(buff.kind, &self.imgs);
                    let buff_widget = Image::new(buff_img).w_h(40.0, 40.0);
                    // Sort buffs into rows of 6 slots
                    let x = i % 6;
                    let y = i / 6;
                    let buff_widget = buff_widget.top_right_with_margins_on(
                        state.ids.align,
                        0.0 + y as f64 * (54.0),
                        0.0 + x as f64 * (42.0),
                    );
                    buff_widget
                        .color(
                            if current_duration.map_or(false, |cur| cur.as_secs_f32() < 10.0) {
                                Some(pulsating_col)
                            } else {
                                Some(norm_col)
                            },
                        )
                        .set(*id, ui);
                    // Create Buff tooltip
                    let title = match buff.kind {
                        BuffKind::Regeneration { .. } => localized_strings.get("buff.title.heal"),
                        BuffKind::Saturation { .. } => {
                            localized_strings.get("buff.title.saturation")
                        },
                        BuffKind::Potion { .. } => localized_strings.get("buff.title.potion"),
                        BuffKind::Bleeding { .. } => localized_strings.get("debuff.title.bleed"),
                        BuffKind::CampfireHeal { .. } => {
                            localized_strings.get("buff.title.campfire_heal")
                        },
                        _ => localized_strings.get("buff.title.missing"),
                    };
                    let remaining_time = if current_duration.is_none() {
                        "".to_string()
                    } else {
                        format!("{:.0}s", current_duration.unwrap().as_secs_f32())
                    };
                    let click_to_remove = format!("<{}>", &localized_strings.get("buff.remove"));
                    let desc_txt = match buff.kind {
                        BuffKind::Regeneration { .. } => localized_strings.get("buff.desc.heal"),
                        BuffKind::Saturation { .. } => {
                            localized_strings.get("buff.desc.saturation")
                        },
                        BuffKind::Potion { .. } => localized_strings.get("buff.desc.potion"),
                        BuffKind::Bleeding { .. } => localized_strings.get("debuff.desc.bleed"),
                        BuffKind::CampfireHeal { .. } => {
                            localized_strings.get("buff.desc.campfire_heal")
                        },
                        _ => localized_strings.get("buff.desc.missing"),
                    };
                    let desc = if buff.is_buff {
                        format!("{}\n\n{}", desc_txt, click_to_remove)
                    } else {
                        desc_txt.to_string()
                    };
                    // Timer overlay
                    if Button::image(match duration_percentage as u64 {
                        875..=1000 => self.imgs.nothing, // 8/8
                        750..=874 => self.imgs.buff_0,   // 7/8
                        625..=749 => self.imgs.buff_1,   // 6/8
                        500..=624 => self.imgs.buff_2,   // 5/8
                        375..=499 => self.imgs.buff_3,   // 4/8
                        250..=374 => self.imgs.buff_4,   // 3/8
                        125..=249 => self.imgs.buff_5,   // 2/8
                        0..=124 => self.imgs.buff_6,     // 1/8
                        _ => self.imgs.nothing,
                    })
                    .w_h(40.0, 40.0)
                    .middle_of(*id)
                    .with_tooltip(
                        self.tooltip_manager,
                        title,
                        &desc,
                        &buffs_tooltip,
                        if buff.is_buff {
                            BUFF_COLOR
                        } else {
                            DEBUFF_COLOR
                        },
                    )
                    .set(*timer_id, ui)
                    .was_clicked()
                    {
                        event.push(Event::RemoveBuff(buff.kind));
                    }
                    Text::new(&remaining_time)
                        .down_from(*timer_id, 1.0)
                        .font_size(self.fonts.cyri.scale(10))
                        .font_id(self.fonts.cyri.conrod_id)
                        .graphics_for(*timer_id)
                        .color(TEXT_COLOR)
                        .set(*txt_id, ui);
                });
        }
        event
    }
}
