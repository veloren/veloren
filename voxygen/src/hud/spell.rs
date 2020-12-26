use super::{img_ids::Imgs, Show, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN};
use crate::{i18n::Localization, ui::fonts::Fonts};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

use client::{self, Client};
use inline_tweak::*;

widget_ids! {
    pub struct Ids {
        frame,
        bg,
        icon,
        close,
        title,
        content_align,
        weapon_imgs[],
        weapon_btns[],
        skills_top_l_align,
        skills_top_r_align,
        skills_bot_l_align,
        skills_bot_r_align,
        skills_top_l[],
        skills_top_r[],
        skills_bot_l[],
        skills_bot_r[],
        sword_render,
        sword_skill_0,
        sword_sill_1,

    }
}

#[derive(WidgetCommon)]
pub struct Spell<'a> {
    show: &'a Show,
    _client: &'a Client,

    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    created_btns_top_l: usize,
    created_btns_top_r: usize,
    created_btns_bot_l: usize,
    created_btns_bot_r: usize,
    example_skill_count: usize,
}

impl<'a> Spell<'a> {
    pub fn new(
        show: &'a Show,
        _client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            show,
            _client,
            imgs,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
            created_btns_top_l: 0,
            created_btns_top_r: 0,
            created_btns_bot_l: 0,
            created_btns_bot_r: 0,
            example_skill_count: 0,
        }
    }
}

/*pub struct State {
    ids: Ids,
}*/

/*pub enum DiaryTab {
    SkillTrees,
    Achievements,
}*/

pub enum SelectedSkillTree {
    None,
    Sword,
    Hammer,
    Axe,
    Sceptre,
    Bow,
    StaffFire,
}

const WEAPONS: [&str; 6] = ["Sword", "Hammer", "Axe", "Sceptre", "Bow", "Fire Staff"];

pub enum Event {
    Close,
    ChangeWeaponTree(SelectedSkillTree),
}

impl<'a> Widget for Spell<'a> {
    type Event = Vec<Event>;
    type State = Ids;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State { Ids::new(id_gen) }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(mut self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id: _, state, ui, ..
        } = args;
        let mut events = Vec::new();
        let sel_tab = &self.show.skilltreetab;
        // Frame
        Image::new(self.imgs.diary_bg)
            .w_h(1202.0, 886.0)
            .mid_top_with_margin_on(ui.window, 5.0)
            .color(Some(UI_MAIN))
            .set(state.bg, ui);

        Image::new(self.imgs.diary_frame)
            .w_h(1202.0, 886.0)
            .middle_of(state.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.frame, ui);

        // Icon
        Image::new(self.imgs.spellbook_button)
            .w_h(30.0, 27.0)
            .top_left_with_margins_on(state.frame, 8.0, 8.0)
            .set(state.icon, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.frame, 0.0, 0.0)
            .set(state.close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // Title
        Text::new(&self.localized_strings.get("hud.diary"))
            .mid_top_with_margin_on(state.frame, 3.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(29))
            .color(TEXT_COLOR)
            .set(state.title, ui);

        // Content Alignment
        Rectangle::fill_with([599.0 * 2.0, 419.0 * 2.0], color::TRANSPARENT)
            .mid_top_with_margin_on(state.frame, 46.0)
            .set(state.content_align, ui);

        // Contents

        // Skill Trees

        // Skill Tree Selection
        state.update(|s| {
            s.weapon_btns
                .resize(WEAPONS.len(), &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.weapon_imgs
                .resize(WEAPONS.len(), &mut ui.widget_id_generator())
        });
        for i in WEAPONS.iter().copied().enumerate() {
            // Background weapon image
            let img = Image::new(match i.1 {
                "Sword" => self.imgs.sword,
                "Hammer" => self.imgs.hammer,
                "Axe" => self.imgs.axe,
                "Sceptre" => self.imgs.sceptre,
                "Bow" => self.imgs.bow,
                "Fire Staff" => self.imgs.staff,
                _ => self.imgs.nothing,
            });

            let img = if i.0 == 0 {
                img.top_left_with_margins_on(state.content_align, tweak!(10.0), tweak!(5.0))
            } else {
                img.down_from(state.weapon_btns[i.0 - 1], tweak!(5.0))
            };

            img.w_h(tweak!(50.0), tweak!(50.0))
                .set(state.weapon_imgs[i.0], ui);
            // Weapon icons
            if Button::image(match i.1 {
                "Sword" => match sel_tab {
                    SelectedSkillTree::Sword => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border,
                },
                "Hammer" => match sel_tab {
                    SelectedSkillTree::Hammer => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border,
                },
                "Axe" => match sel_tab {
                    SelectedSkillTree::Axe => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border,
                },
                "Sceptre" => match sel_tab {
                    SelectedSkillTree::Sceptre => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border,
                },
                "Bow" => match sel_tab {
                    SelectedSkillTree::Bow => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border,
                },
                "Fire Staff" => match sel_tab {
                    SelectedSkillTree::StaffFire => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border,
                },
                _ => self.imgs.wpn_icon_border,
            })
            .w_h(tweak!(50.0), tweak!(50.0))
            .hover_image(match i.1 {
                "Sword" => match sel_tab {
                    SelectedSkillTree::Sword => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_mo,
                },
                "Hammer" => match sel_tab {
                    SelectedSkillTree::Hammer => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_mo,
                },
                "Axe" => match sel_tab {
                    SelectedSkillTree::Axe => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_mo,
                },
                "Sceptre" => match sel_tab {
                    SelectedSkillTree::Sceptre => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_mo,
                },
                "Bow" => match sel_tab {
                    SelectedSkillTree::Bow => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_mo,
                },
                "Fire Staff" => match sel_tab {
                    SelectedSkillTree::StaffFire => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_mo,
                },
                _ => self.imgs.wpn_icon_border,
            })
            .press_image(match i.1 {
                "Sword" => match sel_tab {
                    SelectedSkillTree::Sword => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_press,
                },
                "Hammer" => match sel_tab {
                    SelectedSkillTree::Hammer => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_press,
                },
                "Axe" => match sel_tab {
                    SelectedSkillTree::Axe => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_press,
                },
                "Sceptre" => match sel_tab {
                    SelectedSkillTree::Sceptre => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_press,
                },
                "Bow" => match sel_tab {
                    SelectedSkillTree::Bow => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_press,
                },
                "Fire Staff" => match sel_tab {
                    SelectedSkillTree::StaffFire => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_press,
                },
                _ => self.imgs.wpn_icon_border,
            })
            .middle_of(state.weapon_imgs[i.0])
            .set(state.weapon_btns[i.0], ui)
            .was_clicked()
            {
                match i.1 {
                    "Sword" => events.push(Event::ChangeWeaponTree(SelectedSkillTree::Sword)),
                    "Hammer" => events.push(Event::ChangeWeaponTree(SelectedSkillTree::Hammer)),
                    "Axe" => events.push(Event::ChangeWeaponTree(SelectedSkillTree::Axe)),
                    "Sceptre" => events.push(Event::ChangeWeaponTree(SelectedSkillTree::Sceptre)),
                    "Bow" => events.push(Event::ChangeWeaponTree(SelectedSkillTree::Bow)),
                    "Fire Staff" => {
                        events.push(Event::ChangeWeaponTree(SelectedSkillTree::StaffFire))
                    },
                    _ => events.push(Event::ChangeWeaponTree(SelectedSkillTree::None)),
                }
            }
        }

        // Skill Trees
        // Alignment Placing
        let x = tweak!(200.0);
        let y = tweak!(100.0);
        // Alignment rectangles for skills
        Rectangle::fill_with([124.0 * 2.0, 124.0 * 2.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.content_align, y, x)
            .set(state.skills_top_l_align, ui);
        Rectangle::fill_with([124.0 * 2.0, 124.0 * 2.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.content_align, y, x)
            .set(state.skills_top_r_align, ui);
        Rectangle::fill_with([124.0 * 2.0, 124.0 * 2.0], color::TRANSPARENT)
            .bottom_left_with_margins_on(state.content_align, y, x)
            .set(state.skills_bot_l_align, ui);
        Rectangle::fill_with([124.0 * 2.0, 124.0 * 2.0], color::TRANSPARENT)
            .bottom_right_with_margins_on(state.content_align, y, x)
            .set(state.skills_bot_r_align, ui);
        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = match sel_tab {
            SelectedSkillTree::Sword => 4,
            SelectedSkillTree::Bow => 1,
            _ => 0,
        };
        let skills_top_r = match sel_tab {
            SelectedSkillTree::Sword => 5,
            _ => 0,
        };
        let skills_bot_l = match sel_tab {
            SelectedSkillTree::Sword => 3,
            SelectedSkillTree::Bow => 2,
            _ => 0,
        };
        let skills_bot_r = match sel_tab {
            SelectedSkillTree::Sword => 1,
            _ => 0,
        };
        // Update widget id array len
        state.update(|s| {
            s.skills_top_l
                .resize(skills_top_l, &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.skills_top_r
                .resize(skills_top_r, &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.skills_bot_l
                .resize(skills_bot_l, &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.skills_bot_r
                .resize(skills_bot_r, &mut ui.widget_id_generator())
        });
        // Create Background Images to place skill icons on them later
        // Create central skill first, others around it:
        //
        //        5 1 6
        //        3 0 4
        //        8 2 7
        //
        //
        // TOP-LEFT Skills
        while self.created_btns_top_l < skills_top_l {
            let mut img = Button::image(self.imgs.wpn_icon_border).w_h(80.0, 80.0);
            match self.created_btns_top_l {
                0 => img = img.middle_of(state.skills_top_l_align), // Central Skill
                1 => img = img.up_from(state.skills_top_l[0], 4.0), // 12:00
                2 => img = img.down_from(state.skills_top_l[0], 4.0), // 6:00
                3 => img = img.left_from(state.skills_top_l[0], 4.0), // 3:00
                4 => img = img.right_from(state.skills_top_l[0], 4.0), // 9:00
                5 => img = img.top_left_with_margins_on(state.skills_top_l[0], -41.0, -41.0), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_top_l[0], -41.0, -41.0), /* 1:30 */
                7 => img = img.bottom_left_with_margins_on(state.skills_top_l[0], -41.0, -41.0), /* 4:30 */
                8 => img = img.bottom_right_with_margins_on(state.skills_top_l[0], -41.0, -41.0), /* 7:30 */
                _ => {},
            }
            img.set(state.skills_top_l[self.created_btns_top_l], ui);
            self.created_btns_top_l = self.created_btns_top_l + 1;
        }
        // TOP-RIGHT Skills
        while self.created_btns_top_r < skills_top_r {
            let mut img = Button::image(self.imgs.wpn_icon_border).w_h(80.0, 80.0);
            match self.created_btns_top_r {
                0 => img = img.middle_of(state.skills_top_r_align), // Central Skill
                1 => img = img.up_from(state.skills_top_r[0], 4.0), // 12:00
                2 => img = img.down_from(state.skills_top_r[0], 4.0), // 6:00
                3 => img = img.left_from(state.skills_top_r[0], 4.0), // 3:00
                4 => img = img.right_from(state.skills_top_r[0], 4.0), // 9:00
                5 => img = img.top_left_with_margins_on(state.skills_top_r[0], -41.0, -41.0), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_top_r[0], -41.0, -41.0), /* 1:30 */
                7 => img = img.bottom_left_with_margins_on(state.skills_top_r[0], -41.0, -41.0), /* 4:30 */
                8 => img = img.bottom_right_with_margins_on(state.skills_top_r[0], -41.0, -41.0), /* 7:30 */
                _ => {},
            }
            img.set(state.skills_top_r[self.created_btns_top_r], ui);
            self.created_btns_top_r = self.created_btns_top_r + 1;
        }
        // BOTTOM-LEFT Skills
        while self.created_btns_bot_l < skills_bot_l {
            let mut img = Button::image(self.imgs.wpn_icon_border).w_h(80.0, 80.0);
            match self.created_btns_bot_l {
                0 => img = img.middle_of(state.skills_bot_l_align), // Central Skill
                1 => img = img.up_from(state.skills_bot_l[0], 4.0), // 12:00
                2 => img = img.down_from(state.skills_bot_l[0], 4.0), // 6:00
                3 => img = img.left_from(state.skills_bot_l[0], 4.0), // 3:00
                4 => img = img.right_from(state.skills_bot_l[0], 4.0), // 9:00
                5 => img = img.top_left_with_margins_on(state.skills_bot_l[0], -41.0, -41.0), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_bot_l[0], -41.0, -41.0), /* 1:30 */
                7 => img = img.bottom_left_with_margins_on(state.skills_bot_l[0], -41.0, -41.0), /* 4:30 */
                8 => img = img.bottom_right_with_margins_on(state.skills_bot_l[0], -41.0, -41.0), /* 7:30 */
                _ => {},
            }
            img.set(state.skills_bot_l[self.created_btns_bot_l], ui);
            self.created_btns_bot_l = self.created_btns_bot_l + 1;
        }
        // BOTTOM-RIGHT Skills
        while self.created_btns_bot_r < skills_bot_r {
            let mut btn = Image::new(self.imgs.wpn_icon_border).w_h(80.0, 80.0);
            match self.created_btns_bot_r {
                0 => btn = btn.middle_of(state.skills_bot_r_align), // Central Skill
                1 => btn = btn.up_from(state.skills_bot_r[0], 4.0), // 12:00
                2 => btn = btn.down_from(state.skills_bot_r[0], 4.0), // 6:00
                3 => btn = btn.left_from(state.skills_bot_r[0], 4.0), // 3:00
                4 => btn = btn.right_from(state.skills_bot_r[0], 4.0), // 9:00
                5 => btn = btn.top_left_with_margins_on(state.skills_bot_r[0], -41.0, -41.0), /* 10:30 */
                6 => btn = btn.top_right_with_margins_on(state.skills_bot_r[0], -41.0, -41.0), /* 1:30 */
                7 => btn = btn.bottom_left_with_margins_on(state.skills_bot_r[0], -41.0, -41.0), /* 4:30 */
                8 => btn = btn.bottom_right_with_margins_on(state.skills_bot_r[0], -41.0, -41.0), /* 7:30 */
                _ => {},
            }
            btn.set(state.skills_bot_r[self.created_btns_bot_r], ui);
            self.created_btns_bot_r = self.created_btns_bot_r + 1;
        }
        // Actual Skill-Icons and Functionality
        match sel_tab {
            SelectedSkillTree::Sword => {
                // Sword
                // fancy bg art
                let art_scale = tweak!(0.6);
                Image::new(self.imgs.sword_render)
                    .w_h(222.0 * art_scale, 818.0 * art_scale)
                    .middle_of(state.content_align)
                    .graphics_for(state.content_align)
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, tweak!(0.2))))
                    .set(state.sword_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .floating(true)
                    .set(state.sword_skill_0, ui)
                    .was_clicked()
                {
                    self.example_skill_count = self.example_skill_count + 1;
                };
            },
            _ => {},
        }

        events
    }
}
