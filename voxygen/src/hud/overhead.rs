use super::{
    img_ids::Imgs, DEFAULT_NPC, ENEMY_HP_COLOR, FACTION_COLOR, GROUP_COLOR, GROUP_MEMBER, HP_COLOR,
    LOW_HP_COLOR, REGION_COLOR, SAY_COLOR, STAMINA_COLOR, TELL_COLOR, TEXT_BG, TEXT_COLOR,
};
use crate::{
    hud::get_buff_info,
    i18n::Localization,
    settings::GameplaySettings,
    ui::{fonts::Fonts, Ingameable},
};
use common::comp::{BuffKind, Buffs, Energy, Health, SpeechBubble, SpeechBubbleType, Stats};
use conrod_core::{
    color,
    position::Align,
    widget::{self, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
const MAX_BUBBLE_WIDTH: f64 = 250.0;

widget_ids! {
    struct Ids {
        // Speech bubble
        speech_bubble_text,
        speech_bubble_shadow,
        speech_bubble_top_left,
        speech_bubble_top,
        speech_bubble_top_right,
        speech_bubble_left,
        speech_bubble_mid,
        speech_bubble_right,
        speech_bubble_bottom_left,
        speech_bubble_bottom,
        speech_bubble_bottom_right,
        speech_bubble_tail,
        speech_bubble_icon,

        // Name
        name_bg,
        name,

        // HP
        level,
        level_skull,
        health_bar,
        health_bar_bg,
        health_txt,
        mana_bar,
        health_bar_fg,

        // Buffs
        buffs_align,
        buffs[],
        buff_timers[],
    }
}

#[derive(Clone, Copy)]
pub struct Info<'a> {
    pub name: &'a str,
    pub stats: &'a Stats,
    pub health: &'a Health,
    pub buffs: &'a Buffs,
    pub energy: Option<&'a Energy>,
}

/// Determines whether to show the healthbar
pub fn should_show_healthbar(health: &Health) -> bool { health.current() != health.maximum() }

/// ui widget containing everything that goes over a character's head
/// (Speech bubble, Name, Level, HP/energy bars, etc.)
#[derive(WidgetCommon)]
pub struct Overhead<'a> {
    info: Option<Info<'a>>,
    bubble: Option<&'a SpeechBubble>,
    own_level: u32,
    in_group: bool,
    settings: &'a GameplaySettings,
    pulse: f32,
    i18n: &'a Localization,
    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Overhead<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        info: Option<Info<'a>>,
        bubble: Option<&'a SpeechBubble>,
        own_level: u32,
        in_group: bool,
        settings: &'a GameplaySettings,
        pulse: f32,
        i18n: &'a Localization,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
    ) -> Self {
        Self {
            info,
            bubble,
            own_level,
            in_group,
            settings,
            pulse,
            i18n,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Ingameable for Overhead<'a> {
    fn prim_count(&self) -> usize {
        // Number of conrod primitives contained in the overhead display. TODO maybe
        // this could be done automatically?
        // - 2 Text::new for name
        //
        // If HP Info is shown:
        // - 1 for level: either Text or Image
        // - 3 for HP + fg + bg
        // - 1 for HP text
        // - If there's mana
        //   - 1 Rect::new for mana
        // If there are Buffs
        // - 1 Alignment Rectangle
        // - 10 + 10 Buffs and Timer Overlays (only if there is no speech bubble)
        // If there's a speech bubble
        // - 2 Text::new for speech bubble
        // - 1 Image::new for icon
        // - 10 Image::new for speech bubble (9-slice + tail)
        self.info.map_or(0, |info| {
            2 + 1
                + if self.bubble.is_none() {
                    info.buffs.kinds.len().min(10) * 2
                } else {
                    0
                }
                + if should_show_healthbar(info.health) {
                    5 + if info.energy.is_some() { 1 } else { 0 }
                } else {
                    0
                }
        }) + if self.bubble.is_some() { 13 } else { 0 }
    }
}

impl<'a> Widget for Overhead<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;
        const BARSIZE: f64 = 2.0; // Scaling
        const MANA_BAR_HEIGHT: f64 = BARSIZE * 1.5;
        const MANA_BAR_Y: f64 = MANA_BAR_HEIGHT / 2.0;
        if let Some(Info {
            name,
            stats,
            health,
            buffs,
            energy,
        }) = self.info
        {
            // Used to set healthbar colours based on hp_percentage
            let hp_percentage = health.current() as f64 / health.maximum() as f64 * 100.0;
            // Compare levels to decide if a skull is shown
            let level_comp = stats.level.level() as i64 - self.own_level as i64;
            let health_current = (health.current() / 10) as f64;
            let health_max = (health.maximum() / 10) as f64;
            let name_y = if (health_current - health_max).abs() < 1e-6 {
                MANA_BAR_Y + 20.0
            } else if level_comp > 9 && !self.in_group {
                MANA_BAR_Y + 38.0
            } else {
                MANA_BAR_Y + 32.0
            };
            let font_size = if hp_percentage.abs() > 99.9 { 24 } else { 20 };
            // Show K for numbers above 10^3 and truncate them
            // Show M for numbers above 10^6 and truncate them
            let health_cur_txt = match health_current as u32 {
                0..=999 => format!("{:.0}", health_current.max(1.0)),
                1000..=999999 => format!("{:.0}K", (health_current / 1000.0).max(1.0)),
                _ => format!("{:.0}M", (health_current as f64 / 1.0e6).max(1.0)),
            };
            let health_max_txt = match health_max as u32 {
                0..=999 => format!("{:.0}", health_max.max(1.0)),
                1000..=999999 => format!("{:.0}K", (health_max / 1000.0).max(1.0)),
                _ => format!("{:.0}M", (health_max as f64 / 1.0e6).max(1.0)),
            };
            // Buffs
            // Alignment
            let buff_count = buffs.kinds.len().min(11);
            Rectangle::fill_with([168.0, 100.0], color::TRANSPARENT)
                .x_y(-1.0, name_y + 60.0)
                .parent(id)
                .set(state.ids.buffs_align, ui);

            let gen = &mut ui.widget_id_generator();
            if state.ids.buffs.len() < buff_count {
                state.update(|state| state.ids.buffs.resize(buff_count, gen));
            };
            if state.ids.buff_timers.len() < buff_count {
                state.update(|state| state.ids.buff_timers.resize(buff_count, gen));
            };

            let buff_ani = ((self.pulse * 4.0).cos() * 0.5 + 0.8) + 0.5; //Animation timer
            let pulsating_col = Color::Rgba(1.0, 1.0, 1.0, buff_ani);
            let norm_col = Color::Rgba(1.0, 1.0, 1.0, 1.0);
            // Create Buff Widgets
            if self.bubble.is_none() {
                state
                    .ids
                    .buffs
                    .iter()
                    .copied()
                    .zip(state.ids.buff_timers.iter().copied())
                    .zip(buffs.iter_active().map(get_buff_info))
                    .enumerate()
                    .for_each(|(i, ((id, timer_id), buff))| {
                        // Limit displayed buffs
                        let max_duration = buff.data.duration;
                        let current_duration = buff.dur;
                        let duration_percentage = current_duration.map_or(1000.0, |cur| {
                            max_duration.map_or(1000.0, |max| {
                                cur.as_secs_f32() / max.as_secs_f32() * 1000.0
                            })
                        }) as u32; // Percentage to determine which frame of the timer overlay is displayed
                        let buff_img = match buff.kind {
                            BuffKind::Regeneration { .. } => self.imgs.buff_plus_0,
                            BuffKind::Saturation { .. } => self.imgs.buff_saturation_0,
                            BuffKind::Bleeding { .. } => self.imgs.debuff_bleed_0,
                            BuffKind::Cursed { .. } => self.imgs.debuff_skull_0,
                            BuffKind::Potion { .. } => self.imgs.buff_potion_0,
                            BuffKind::CampfireHeal { .. } => self.imgs.buff_campfire_heal_0,
                        };
                        let buff_widget = Image::new(buff_img).w_h(20.0, 20.0);
                        // Sort buffs into rows of 5 slots
                        let x = i % 5;
                        let y = i / 5;
                        let buff_widget = buff_widget.bottom_left_with_margins_on(
                            state.ids.buffs_align,
                            0.0 + y as f64 * (21.0),
                            0.0 + x as f64 * (21.0),
                        );
                        buff_widget
                            .color(
                                if current_duration.map_or(false, |cur| cur.as_secs_f32() < 10.0) {
                                    Some(pulsating_col)
                                } else {
                                    Some(norm_col)
                                },
                            )
                            .set(id, ui);

                        Image::new(match duration_percentage as u64 {
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
                        .w_h(20.0, 20.0)
                        .middle_of(id)
                        .set(timer_id, ui);
                    });
            }
            // Name
            Text::new(name)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(font_size)
                .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                .x_y(-1.0, name_y)
                .parent(id)
                .set(state.ids.name_bg, ui);
            Text::new(name)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(font_size)
                .color(if self.in_group {
                    GROUP_MEMBER
                /*} else if targets player { //TODO: Add a way to see if the entity is trying to attack the player, their pet(s) or a member of their group and recolour their nametag accordingly
                DEFAULT_NPC*/
                } else {
                    DEFAULT_NPC
                })
                .x_y(0.0, name_y + 1.0)
                .parent(id)
                .set(state.ids.name, ui);

            if should_show_healthbar(health) {
                // Show HP Bar
                let hp_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 1.0; //Animation timer
                let crit_hp_color: Color = Color::Rgba(0.93, 0.59, 0.03, hp_ani);

                // Background
                Image::new(self.imgs.enemy_health_bg)
                .w_h(84.0 * BARSIZE, 10.0 * BARSIZE)
                .x_y(0.0, MANA_BAR_Y + 6.5) //-25.5)
                .color(Some(Color::Rgba(0.1, 0.1, 0.1, 0.8)))
                .parent(id)
                .set(state.ids.health_bar_bg, ui);

                // % HP Filling
                Image::new(self.imgs.enemy_bar)
                    .w_h(73.0 * (hp_percentage / 100.0) * BARSIZE, 6.0 * BARSIZE)
                    .x_y(
                        (4.5 + (hp_percentage / 100.0 * 36.45 - 36.45)) * BARSIZE,
                        MANA_BAR_Y + 7.5,
                    )
                    .color(if self.in_group {
                        // Different HP bar colors only for group members
                        Some(match hp_percentage {
                            x if (0.0..25.0).contains(&x) => crit_hp_color,
                            x if (25.0..50.0).contains(&x) => LOW_HP_COLOR,
                            _ => HP_COLOR,
                        })
                    } else {
                        Some(ENEMY_HP_COLOR)
                    })
                    .parent(id)
                    .set(state.ids.health_bar, ui);
                let mut txt = format!("{}/{}", health_cur_txt, health_max_txt);
                if health.is_dead {
                    txt = self.i18n.get("hud.group.dead").to_string()
                };
                Text::new(&txt)
                    .mid_top_with_margin_on(state.ids.health_bar_bg, 2.0)
                    .font_size(10)
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .parent(id)
                    .set(state.ids.health_txt, ui);

                // % Mana Filling
                if let Some(energy) = energy {
                    let energy_factor = energy.current() as f64 / energy.maximum() as f64;

                    Rectangle::fill_with(
                        [72.0 * energy_factor * BARSIZE, MANA_BAR_HEIGHT],
                        STAMINA_COLOR,
                    )
                    .x_y(
                        ((3.5 + (energy_factor * 36.5)) - 36.45) * BARSIZE,
                        MANA_BAR_Y, //-32.0,
                    )
                    .parent(id)
                    .set(state.ids.mana_bar, ui);
                }

                // Foreground
                Image::new(self.imgs.enemy_health)
                .w_h(84.0 * BARSIZE, 10.0 * BARSIZE)
                .x_y(0.0, MANA_BAR_Y + 6.5) //-25.5)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.99)))
                .parent(id)
                .set(state.ids.health_bar_fg, ui);

                // Level
                const LOW: Color = Color::Rgba(0.54, 0.81, 0.94, 0.4);
                const HIGH: Color = Color::Rgba(1.0, 0.0, 0.0, 1.0);
                const EQUAL: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
                // Change visuals of the level display depending on the player level/opponent
                // level
                let level_comp = stats.level.level() as i64 - self.own_level as i64;
                // + 10 level above player -> skull
                // + 5-10 levels above player -> high
                // -5 - +5 levels around player level -> equal
                // - 5 levels below player -> low
                if level_comp > 9 && !self.in_group {
                    let skull_ani = ((self.pulse * 0.7/* speed factor */).cos() * 0.5 + 0.5) * 10.0; //Animation timer
                    Image::new(if skull_ani as i32 == 1 && rand::random::<f32>() < 0.9 {
                        self.imgs.skull_2
                    } else {
                        self.imgs.skull
                    })
                    .w_h(18.0 * BARSIZE, 18.0 * BARSIZE)
                    .x_y(-39.0 * BARSIZE, MANA_BAR_Y + 7.0)
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                    .parent(id)
                    .set(state.ids.level_skull, ui);
                } else {
                    let fnt_size = match stats.level.level() {
                        0..=9 => 15,
                        10..=99 => 12,
                        100..=999 => 9,
                        _ => 2,
                    };
                    Text::new(&format!("{}", stats.level.level()))
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(fnt_size)
                        .color(if level_comp > 4 {
                            HIGH
                        } else if level_comp < -5 {
                            LOW
                        } else {
                            EQUAL
                        })
                        .x_y(-37.0 * BARSIZE, MANA_BAR_Y + 9.0)
                        .parent(id)
                        .set(state.ids.level, ui);
                }
            }
        }

        // Speech bubble
        if let Some(bubble) = self.bubble {
            let dark_mode = self.settings.speech_bubble_dark_mode;
            let localizer = |s: &str, i| -> String { self.i18n.get_variation(&s, i).to_string() };
            let bubble_contents: String = bubble.message(localizer);
            let (text_color, shadow_color) = bubble_color(&bubble, dark_mode);
            let mut text = Text::new(&bubble_contents)
                .color(text_color)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(18)
                .up_from(state.ids.name, 26.0)
                .x_align_to(state.ids.name, Align::Middle)
                .parent(id);

            if let Some(w) = text.get_w(ui) {
                if w > MAX_BUBBLE_WIDTH {
                    text = text.w(MAX_BUBBLE_WIDTH);
                }
            }
            Image::new(if dark_mode {
                self.imgs.dark_bubble_top_left
            } else {
                self.imgs.speech_bubble_top_left
            })
            .w_h(16.0, 16.0)
            .top_left_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_top_left, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_top
            } else {
                self.imgs.speech_bubble_top
            })
            .h(16.0)
            .padded_w_of(state.ids.speech_bubble_text, -4.0)
            .mid_top_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_top, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_top_right
            } else {
                self.imgs.speech_bubble_top_right
            })
            .w_h(16.0, 16.0)
            .top_right_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_top_right, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_left
            } else {
                self.imgs.speech_bubble_left
            })
            .w(16.0)
            .padded_h_of(state.ids.speech_bubble_text, -4.0)
            .mid_left_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_left, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_mid
            } else {
                self.imgs.speech_bubble_mid
            })
            .padded_wh_of(state.ids.speech_bubble_text, -4.0)
            .top_left_with_margin_on(state.ids.speech_bubble_text, -4.0)
            .parent(id)
            .set(state.ids.speech_bubble_mid, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_right
            } else {
                self.imgs.speech_bubble_right
            })
            .w(16.0)
            .padded_h_of(state.ids.speech_bubble_text, -4.0)
            .mid_right_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_right, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_bottom_left
            } else {
                self.imgs.speech_bubble_bottom_left
            })
            .w_h(16.0, 16.0)
            .bottom_left_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_bottom_left, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_bottom
            } else {
                self.imgs.speech_bubble_bottom
            })
            .h(16.0)
            .padded_w_of(state.ids.speech_bubble_text, -4.0)
            .mid_bottom_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_bottom, ui);
            Image::new(if dark_mode {
                self.imgs.dark_bubble_bottom_right
            } else {
                self.imgs.speech_bubble_bottom_right
            })
            .w_h(16.0, 16.0)
            .bottom_right_with_margin_on(state.ids.speech_bubble_text, -20.0)
            .parent(id)
            .set(state.ids.speech_bubble_bottom_right, ui);
            let tail = Image::new(if dark_mode {
                self.imgs.dark_bubble_tail
            } else {
                self.imgs.speech_bubble_tail
            })
            .parent(id)
            .mid_bottom_with_margin_on(state.ids.speech_bubble_text, -32.0);

            if dark_mode {
                tail.w_h(22.0, 13.0)
            } else {
                tail.w_h(22.0, 28.0)
            }
            .set(state.ids.speech_bubble_tail, ui);

            let mut text_shadow = Text::new(&bubble_contents)
                .color(shadow_color)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(18)
                .x_relative_to(state.ids.speech_bubble_text, 1.0)
                .y_relative_to(state.ids.speech_bubble_text, -1.0)
                .parent(id);
            // Move text to front (conrod depth is lowest first; not a z-index)
            text.depth(text_shadow.get_depth() - 1.0)
                .set(state.ids.speech_bubble_text, ui);
            if let Some(w) = text_shadow.get_w(ui) {
                if w > MAX_BUBBLE_WIDTH {
                    text_shadow = text_shadow.w(MAX_BUBBLE_WIDTH);
                }
            }
            text_shadow.set(state.ids.speech_bubble_shadow, ui);
            let icon = if self.settings.speech_bubble_icon {
                bubble_icon(&bubble, &self.imgs)
            } else {
                self.imgs.nothing
            };
            Image::new(icon)
                .w_h(16.0, 16.0)
                .top_left_with_margin_on(state.ids.speech_bubble_text, -16.0)
                // TODO: Figure out whether this should be parented.
                // .parent(id)
                .set(state.ids.speech_bubble_icon, ui);
        }
    }
}

fn bubble_color(bubble: &SpeechBubble, dark_mode: bool) -> (Color, Color) {
    let light_color = match bubble.icon {
        SpeechBubbleType::Tell => TELL_COLOR,
        SpeechBubbleType::Say => SAY_COLOR,
        SpeechBubbleType::Region => REGION_COLOR,
        SpeechBubbleType::Group => GROUP_COLOR,
        SpeechBubbleType::Faction => FACTION_COLOR,
        SpeechBubbleType::World
        | SpeechBubbleType::Quest
        | SpeechBubbleType::Trade
        | SpeechBubbleType::None => TEXT_COLOR,
    };
    if dark_mode {
        (light_color, TEXT_BG)
    } else {
        (TEXT_BG, light_color)
    }
}

fn bubble_icon(sb: &SpeechBubble, imgs: &Imgs) -> conrod_core::image::Id {
    match sb.icon {
        // One for each chat mode
        SpeechBubbleType::Tell => imgs.chat_tell_small,
        SpeechBubbleType::Say => imgs.chat_say_small,
        SpeechBubbleType::Region => imgs.chat_region_small,
        SpeechBubbleType::Group => imgs.chat_group_small,
        SpeechBubbleType::Faction => imgs.chat_faction_small,
        SpeechBubbleType::World => imgs.chat_world_small,
        SpeechBubbleType::Quest => imgs.nothing, // TODO not implemented
        SpeechBubbleType::Trade => imgs.nothing, // TODO not implemented
        SpeechBubbleType::None => imgs.nothing,  // No icon (default for npcs)
    }
}
