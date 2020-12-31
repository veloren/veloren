use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::{ItemImgs, ItemKey::Tool},
    Show, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    i18n::Localization,
    ui::{fonts::Fonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

use client::{self, Client};
use common::comp::{
    item::tool::ToolKind,
    skills::{self, Skill},
};
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
        skill_sword_combo_0,
        skill_sword_combo_1,
        skill_sword_combo_2,
        skill_sword_combo_3,
        skill_sword_dash_0,
        skill_sword_dash_1,
        skill_sword_dash_2,
        skill_sword_dash_3,
        skill_sword_dash_4,
        skill_sword_dash_5,
        skill_sword_spin_0,
        skill_sword_spin_1,
        skill_sword_spin_2,
        skill_sword_spin_3,
        skill_sword_spin_4,
        skill_sword_passive_0,
        axe_render,
        skill_axe_combo_0,
        skill_axe_combo_1,
        skill_axe_combo_2,
        skill_axe_combo_3,
        skill_axe_spin_0,
        skill_axe_spin_1,
        skill_axe_spin_2,
        skill_axe_spin_3,
        skill_axe_spin_4,
        skill_axe_leap_0,
        skill_axe_leap_1,
        skill_axe_leap_2,
        skill_axe_leap_3,
        skill_axe_leap_4,
        hammer_render,
        skill_hammer_combo_0,
        skill_hammer_combo_1,
        skill_hammer_combo_2,
        skill_hammer_combo_3,
        skill_hammer_charged_0,
        skill_hammer_charged_1,
        skill_hammer_charged_2,
        skill_hammer_charged_3,
        skill_hammer_leap_0,
        skill_hammer_leap_1,
        skill_hammer_leap_2,
        skill_hammer_leap_3,
        skill_hammer_leap_4,
        skill_hammer_leap_5,
        bow_render,
        skill_bow_basic_0,
        skill_bow_basic_1,
        skill_bow_charged_0,
        skill_bow_charged_1,
        skill_bow_charged_2,
        skill_bow_charged_3,
        skill_bow_charged_4,
        skill_bow_charged_5,
        skill_bow_repeater_0,
        skill_bow_repeater_1,
        skill_bow_repeater_2,
        skill_bow_repeater_3,
        skill_bow_repeater_4,
        skill_bow_passive_0,
        staff_render,
        skill_staff_basic_0,
        skill_staff_basic_1,
        skill_staff_basic_2,
        skill_staff_basic_3,
        skill_staff_beam_0,
        skill_staff_beam_1,
        skill_staff_beam_2,
        skill_staff_beam_3,
        skill_staff_shockwave_0,
        skill_staff_shockwave_1,
        skill_staff_shockwave_2,
        skill_staff_shockwave_3,
        skill_staff_shockwave_4,
        sceptre_render,
        skill_sceptre_beam_0,
        skill_sceptre_beam_1,
        skill_sceptre_beam_2,
        skill_sceptre_beam_3,
        skill_sceptre_beam_4,
        skill_sceptre_beam_5,
        skill_sceptre_bomb_0,
        skill_sceptre_bomb_1,
        skill_sceptre_bomb_2,
        skill_sceptre_bomb_3,
        skill_sceptre_bomb_4,
        general_combat_render,
        skill_general_stat_0,
        skill_general_stat_1,
        skill_general_tree_0,
        skill_general_tree_1,
        skill_general_tree_2,
        skill_general_tree_3,
        skill_general_tree_4,
        skill_general_tree_5,
        skill_general_roll_0,
        skill_general_roll_1,
        skill_general_roll_2,
        skill_general_roll_3,
    }
}

#[derive(WidgetCommon)]
pub struct Diary<'a> {
    show: &'a Show,
    _client: &'a Client,

    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    created_btns_top_l: usize,
    created_btns_top_r: usize,
    created_btns_bot_l: usize,
    created_btns_bot_r: usize,
    example_skill_count: usize,
}

impl<'a> Diary<'a> {
    pub fn new(
        show: &'a Show,
        _client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
    ) -> Self {
        Self {
            show,
            _client,
            imgs,
            item_imgs,
            fonts,
            localized_strings,
            rot_imgs,
            tooltip_manager,
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
    GeneralCombat,
}

const WEAPONS: [&str; 7] = [
    "General Combat",
    "Sword",
    "Hammer",
    "Axe",
    "Sceptre",
    "Bow",
    "Fire Staff",
];

pub enum Event {
    Close,
    ChangeWeaponTree(SelectedSkillTree),
    UnlockSkill(Skill),
}

impl<'a> Widget for Diary<'a> {
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
        // Tooltips
        let diary_tooltip = Tooltip::new({
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
                "General Combat" => self.imgs.not_found,
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
                "General Combat" => match sel_tab {
                    SelectedSkillTree::GeneralCombat => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border,
                },
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
                "General Combat" => match sel_tab {
                    SelectedSkillTree::GeneralCombat => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_mo,
                },
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
                "General Combat" => match sel_tab {
                    SelectedSkillTree::GeneralCombat => self.imgs.wpn_icon_border_pressed,
                    _ => self.imgs.wpn_icon_border_press,
                },
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
                    "General Combat" => {
                        events.push(Event::ChangeWeaponTree(SelectedSkillTree::GeneralCombat))
                    },
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
            SelectedSkillTree::GeneralCombat => 2,
            SelectedSkillTree::Sword => 4,
            SelectedSkillTree::Axe => 4,
            SelectedSkillTree::Hammer => 4,
            SelectedSkillTree::Bow => 2,
            SelectedSkillTree::StaffFire => 4,
            SelectedSkillTree::Sceptre => 6,
            _ => 0,
        };
        let skills_top_r = match sel_tab {
            SelectedSkillTree::GeneralCombat => 6,
            SelectedSkillTree::Sword => 6,
            SelectedSkillTree::Axe => 5,
            SelectedSkillTree::Hammer => 4,
            SelectedSkillTree::Bow => 6,
            SelectedSkillTree::StaffFire => 4,
            SelectedSkillTree::Sceptre => 5,
            _ => 0,
        };
        let skills_bot_l = match sel_tab {
            SelectedSkillTree::GeneralCombat => 4,
            SelectedSkillTree::Sword => 5,
            SelectedSkillTree::Axe => 5,
            SelectedSkillTree::Hammer => 6,
            SelectedSkillTree::Bow => 5,
            SelectedSkillTree::StaffFire => 5,
            _ => 0,
        };
        let skills_bot_r = match sel_tab {
            SelectedSkillTree::Sword => 1,
            SelectedSkillTree::Bow => 1,
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
                5 => img = img.top_left_with_margins_on(state.skills_top_l[0], -84.0, -84.0), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_top_l[0], -84.0, -84.0), /* 1:30 */
                7 => img = img.bottom_left_with_margins_on(state.skills_top_l[0], -84.0, -84.0), /* 4:30 */
                8 => img = img.bottom_right_with_margins_on(state.skills_top_l[0], -84.0, -84.0), /* 7:30 */
                _ => {},
            }
            img.set(state.skills_top_l[self.created_btns_top_l], ui);
            self.created_btns_top_l += 1;
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
                5 => img = img.top_left_with_margins_on(state.skills_top_r[0], -84.0, -84.0), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_top_r[0], -84.0, -84.0), /* 1:30 */
                7 => img = img.bottom_left_with_margins_on(state.skills_top_r[0], -84.0, -84.0), /* 4:30 */
                8 => img = img.bottom_right_with_margins_on(state.skills_top_r[0], -84.0, -84.0), /* 7:30 */
                _ => {},
            }
            img.set(state.skills_top_r[self.created_btns_top_r], ui);
            self.created_btns_top_r += 1;
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
                5 => img = img.top_left_with_margins_on(state.skills_bot_l[0], -84.0, -84.0), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_bot_l[0], -84.0, -84.0), /* 1:30 */
                7 => img = img.bottom_left_with_margins_on(state.skills_bot_l[0], -84.0, -84.0), /* 4:30 */
                8 => img = img.bottom_right_with_margins_on(state.skills_bot_l[0], -84.0, -84.0), /* 7:30 */
                _ => {},
            }
            img.set(state.skills_bot_l[self.created_btns_bot_l], ui);
            self.created_btns_bot_l += 1;
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
                5 => btn = btn.top_left_with_margins_on(state.skills_bot_r[0], -84.0, -84.0), /* 10:30 */
                6 => btn = btn.top_right_with_margins_on(state.skills_bot_r[0], -84.0, -84.0), /* 1:30 */
                7 => btn = btn.bottom_left_with_margins_on(state.skills_bot_r[0], -84.0, -84.0), /* 4:30 */
                8 => btn = btn.bottom_right_with_margins_on(state.skills_bot_r[0], -84.0, -84.0), /* 7:30 */
                _ => {},
            }
            btn.set(state.skills_bot_r[self.created_btns_bot_r], ui);
            self.created_btns_bot_r += 1;
        }
        // Skill-Icons and Functionality
        // Art dimensions
        let art_size = [tweak!(320.0), tweak!(320.0)];
        match sel_tab {
            SelectedSkillTree::GeneralCombat => {
                use skills::{GeneralSkill::*, RollSkill::*, SkillGroupType::*};
                use ToolKind::*;
                // General Combat
                Image::new(self.imgs.not_found)
                    .wh(art_size)
                    .middle_of(state.content_align)
                    .graphics_for(state.content_align)
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, tweak!(1.0))))
                    .set(state.general_combat_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Increase Health",
                        "Increases health",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_stat_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::General(HealthIncrease)));
                };
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Increase Energy",
                        "Increases energy",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_stat_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::General(EnergyIncrease)));
                };
                // Top right skills
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Unlock Sword",
                        "Unlocks sword skill tree",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_tree_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::UnlockGroup(Weapon(Sword))));
                };
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Unlock Axe",
                        "Unlocks axe skill tree",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_tree_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::UnlockGroup(Weapon(Axe))));
                };
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Unlock Hammer",
                        "Unlocks hammer skill tree",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_tree_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::UnlockGroup(Weapon(Hammer))));
                };
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Unlock Bow",
                        "Unlocks bow skill tree",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_tree_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::UnlockGroup(Weapon(Bow))));
                };
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Unlock Staff",
                        "Unlocks staff skill tree",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_tree_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::UnlockGroup(Weapon(Staff))));
                };
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[5])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Unlock Sceptre",
                        "Unlocks sceptre skill tree",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_tree_5, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::UnlockGroup(Weapon(Sceptre))));
                };
                // Bottom left skills
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Dodge",
                        "Ground-yeeting dodges melee attacks",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_roll_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Roll(ImmuneMelee)));
                };
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Cost",
                        "Decreases cost of ground-yeeting yourself",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_roll_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Roll(Cost)));
                };
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Strength",
                        "Increases how far you ground-yeet yourself",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_roll_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Roll(Strength)));
                };
                if Button::image(self.imgs.not_found)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Duration",
                        "Increases for how long you ground-yeet yourself",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_general_roll_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Roll(Duration)));
                };
            },
            SelectedSkillTree::Sword => {
                use skills::SwordSkill::*;
                // Sword
                Image::new(
                    self.item_imgs
                        .img_id_or_not_found_img(Tool("example_sword".to_string())),
                )
                .wh(art_size)
                .middle_of(state.content_align)
                .graphics_for(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, tweak!(1.0))))
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
                    .with_tooltip(
                        self.tooltip_manager,
                        "Triple Strike Combo",
                        "Unlocks combo scaling on triple strike",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_combo_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(TsCombo)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Triple Strike Damage",
                        "Increases damage scaling on triple strike",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_combo_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(TsDamage)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Triple Strike Speed",
                        "Increases attack speed scaling on triple strike",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_combo_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(TsSpeed)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Triple Strike Regen",
                        "Increases enery regen scaling on triple strike",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_combo_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(TsRegen)));
                };
                // Top right skills
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Dash Damage",
                        "Increases initial damage of the dash",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_dash_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(DDamage)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Dash Drain",
                        "Decreases the rate energy is drained while dashing",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_dash_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(DDrain)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Dash Cost",
                        "Decreases the initial cost of the dash",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_dash_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(DCost)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Dash Speed",
                        "Increases how fast you go while dashing",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_dash_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(DSpeed)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Dash Infinite",
                        "Allows you to dash for as long as you have energy",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_dash_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(DInfinite)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[5])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Dash Scaling",
                        "Increases how much the damage scales by over the course of the dash",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_dash_5, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(DScaling)));
                };
                // Bottom left skills
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Spin Unlock",
                        "Unlocks the sword spin",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_spin_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(SUnlockSpin)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Spin Damage",
                        "Increases the damage done",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_spin_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(SDamage)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Spin Speed",
                        "Increase the speed at which you spin",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_spin_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(SSpeed)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Spin Cost",
                        "Decreases the energy cost of each spin",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_spin_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(SCost)));
                };
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Spin Spins",
                        "Increases the number of times you can spin",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_spin_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(SSpins)));
                };
                // Bottom right skills
                if Button::image(self.imgs.sword_whirlwind)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_r[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Interrupting Attacks",
                        "Allows you to immediately cancel an attack with another attack",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_passive_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sword(InterruptingAttacks)));
                };
            },
            SelectedSkillTree::Axe => {
                use skills::AxeSkill::*;
                // Axe
                Image::new(
                    self.item_imgs
                        .img_id_or_not_found_img(Tool("example_axe".to_string())),
                )
                .wh(art_size)
                .middle_of(state.content_align)
                .graphics_for(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, tweak!(1.0))))
                .set(state.axe_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Double Strike Combo",
                        "Unlocks a second strike",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_combo_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(DsCombo)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Double Strike Damage",
                        "Increases damage scaling in combo",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_combo_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(DsDamage)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Double Strike Speed",
                        "Increases speed scaling in combo",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_combo_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(DsSpeed)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Double Strike Regen",
                        "Increases energy regen scaling in combo",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_combo_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(DsRegen)));
                };
                // Top right skills
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Infinite Axe Spin",
                        "Spin for as long as you have energy",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_spin_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(SInfinite)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Spin Damage",
                        "Increases the daamge each spin does",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_spin_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(SDamage)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Spin Helicopter",
                        "You fall a little slower while spinning",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_spin_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(SHelicopter)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Spin Speed",
                        "Increases your spins per minute",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_spin_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(SSpeed)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Spin Cost",
                        "Increases your spin per energy efficiency",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_spin_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(SCost)));
                };
                // Bottom left skills
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Unlock Leap",
                        "Unlocks a leap spin",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_leap_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(LUnlockLeap)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Leap Damage",
                        "Increases damage of leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_leap_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(LDamage)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Leap Knockback",
                        "Increases knockback from leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_leap_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(LKnockback)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Leap Cost",
                        "Decreases cost of leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_leap_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(LCost)));
                };
                if Button::image(self.imgs.axespin)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Leap Distance",
                        "Increases distance of leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_leap_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Axe(LDistance)));
                };
            },
            SelectedSkillTree::Hammer => {
                use skills::HammerSkill::*;
                // Hammer
                Image::new(
                    self.item_imgs
                        .img_id_or_not_found_img(Tool("example_hammer".to_string())),
                )
                .wh(art_size)
                .middle_of(state.content_align)
                .graphics_for(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, tweak!(1.0))))
                .set(state.hammer_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Single Strike Knockback",
                        "Increaes yeet potential of swings",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_combo_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(SsKnockback)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Single Strike Damage",
                        "Increases damage scaling in combo",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_combo_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(SsDamage)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Single Strike Speed",
                        "Increases speed scaling in combo",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_combo_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(SsSpeed)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Single Strike Regen",
                        "Increases energy regen scaling in combo",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_combo_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(SsRegen)));
                };
                // Top right skills
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charged Melee Knockback",
                        "Massively increases yeet potential of swing",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_charged_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(CKnockback)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charged Melee Damage",
                        "Increases the daamge of the charged swing",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_charged_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(CDamage)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charged Melee Energy Drain",
                        "Decreases the rate energy drains when charging",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_charged_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(CDrain)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charge Rate",
                        "Increases the rate that you charge the swing",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_charged_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(CSpeed)));
                };
                // Bottom left skills
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Unlock Leap",
                        "Unlocks a leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_leap_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(LUnlockLeap)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Leap Damage",
                        "Increases damage of leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_leap_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(LDamage)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Leap Knockback",
                        "Increases knockback from leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_leap_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(LKnockback)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Leap Cost",
                        "Decreases cost of leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_leap_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(LCost)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Leap Distance",
                        "Increases distance of leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_leap_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(LDistance)));
                };
                if Button::image(self.imgs.hammergolf)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[5])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Leap Radius",
                        "Increases attack radius on ground slam",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_leap_5, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Hammer(LRange)));
                };
            },
            SelectedSkillTree::Bow => {
                use skills::BowSkill::*;
                // Bow
                Image::new(
                    self.item_imgs
                        .img_id_or_not_found_img(Tool("example_bow".to_string())),
                )
                .wh(art_size)
                .middle_of(state.content_align)
                .graphics_for(state.content_align)
                .set(state.bow_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Damage",
                        "Increases damage",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_basic_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(BDamage)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Energy Regen",
                        "Increases energy regen",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_basic_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(BRegen)));
                };
                // Top right skills
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charged Damage",
                        "Increases how much damage scales by as it is charged",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_charged_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(CDamage)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charged Drain",
                        "Decreases the rate energy is drained while charging",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_charged_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(CDrain)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charged Projectile Speed",
                        "Increases yeet potential applied to arrow while charging",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_charged_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(CProjSpeed)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charged Speed",
                        "Increases the rate that you charge the attack",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_charged_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(CSpeed)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charged Move Speed",
                        "Increases how fast you can shuffle while charging the attack",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_charged_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(CMove)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[5])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Charged Knockback",
                        "Yeet enemies further",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_charged_5, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(CKnockback)));
                };
                // Bottom left skills
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Repeater Unlock",
                        "Unlocks the ability to leap in the arrow",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_repeater_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(UnlockRepeater)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Repeater Damage",
                        "Increases the damage done",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_repeater_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(RDamage)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Repeater Glide",
                        "Glide further while repeatering",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_repeater_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(RGlide)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Repeater Cost",
                        "Decreases the energy cost to become a gliding repeater",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_repeater_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(RCost)));
                };
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Arrow Count",
                        "Yeet more arrows when you leap",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_repeater_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(RArrows)));
                };
                // Bottom right skills
                if Button::image(self.imgs.bow_m1)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_r[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Projectile Speed",
                        "Allows you to yeet arrows further, faster",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_passive_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Bow(ProjSpeed)));
                };
            },
            SelectedSkillTree::StaffFire => {
                use skills::StaffSkill::*;
                // Staff
                Image::new(
                    self.item_imgs
                        .img_id_or_not_found_img(Tool("example_staff_fire".to_string())),
                )
                .wh(art_size)
                .middle_of(state.content_align)
                .graphics_for(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, tweak!(1.0))))
                .set(state.staff_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Explosion",
                        "When fire just isn't enough",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_basic_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(BExplosion)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Damage",
                        "Increases damage",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_basic_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(BDamage)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Energy Regen",
                        "Increases energy regen",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_basic_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(BRegen)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Explosion Radius",
                        "Bigger is better",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_basic_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(BRadius)));
                };
                // Top right skills
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Flamethrower Damage",
                        "Increases damage",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_beam_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(FDamage)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Energy Drain",
                        "Decreases the rate energy is drained",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_beam_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(FDrain)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Flamethrower Range",
                        "For when the flames just won't reach",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_beam_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(FRange)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Flame Velocity",
                        "Gets the fire there faster",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_beam_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(FVelocity)));
                };
                // Bottom left skills
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Shockwave Unlock",
                        "Unlocks the ability to yeet enemies away using fire",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_shockwave_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(UnlockShockwave)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Shockwave Damage",
                        "Increases the damage done",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_shockwave_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(SDamage)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Shockwave Knockback",
                        "Increases yeet potential",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_shockwave_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(SKnockback)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Shockwave Cost",
                        "Decreases the energy cost to yeet helpless villagers",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_shockwave_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(SCost)));
                };
                if Button::image(self.imgs.fireball)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_bot_l[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Shockwave Range",
                        "Yeet things that used to be out of reach",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_shockwave_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Staff(SRange)));
                };
            },
            SelectedSkillTree::Sceptre => {
                use skills::SceptreSkill::*;
                // Sceptre
                Image::new(
                    self.item_imgs
                        .img_id_or_not_found_img(Tool("example_sceptre".to_string())),
                )
                .wh(art_size)
                .middle_of(state.content_align)
                .graphics_for(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, tweak!(1.0))))
                .set(state.sceptre_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Beam Heal",
                        "Increased healing from the beam",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_beam_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(BHeal)));
                };
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Damage",
                        "Increases damage",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_beam_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(BDamage)));
                };
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Energy Regen",
                        "Increases energy regen from damage",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_beam_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(BRegen)));
                };
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Range",
                        "Longer beam",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_beam_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(BRange)));
                };
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Lifesteal Efficiency",
                        "Thieve more health",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_beam_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(BLifesteal)));
                };
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_l[5])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Heal Cost",
                        "Use less energy when healing",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_beam_5, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(BCost)));
                };
                // Top right skills
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[0])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Heal",
                        "Increases healing",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_bomb_0, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(PHeal)));
                };
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[1])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Damage",
                        "Increases damage",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_bomb_1, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(PDamage)));
                };
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[2])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Radius",
                        "Increases radius",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_bomb_2, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(PRadius)));
                };
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[3])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Energy Cost",
                        "Decreases energy cost of bomb",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_bomb_3, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(PCost)));
                };
                if Button::image(self.imgs.heal_0)
                    .w_h(tweak!(74.0), tweak!(74.0))
                    .middle_of(state.skills_top_r[4])
                    .label(&self.example_skill_count.to_string())
                    .label_y(conrod_core::position::Relative::Scalar(tweak!(-28.0)))
                    .label_x(conrod_core::position::Relative::Scalar(tweak!(32.0)))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(16)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .with_tooltip(
                        self.tooltip_manager,
                        "Projectile Speed",
                        "Yeets it faster",
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_bomb_4, ui)
                    .was_clicked()
                {
                    events.push(Event::UnlockSkill(Skill::Sceptre(PProjSpeed)));
                };
            },
            _ => {},
        }

        events
    }
}
