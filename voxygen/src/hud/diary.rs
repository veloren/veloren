use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::{animate_by_pulse, ItemImgs, ItemKey::Tool},
    Show, CRITICAL_HP_COLOR, HP_COLOR, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN, XP_COLOR,
};
use crate::{
    i18n::Localization,
    ui::{fonts::Fonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
};
use conrod_core::{
    color,
    image::Id,
    widget::{self, button, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

use client::{self, Client};
use common::comp::{
    item::tool::ToolKind,
    skills::{self, Skill},
    Stats,
};

widget_ids! {
    pub struct Ids {
        frame,
        bg,
        icon,
        close,
        title,
        content_align,
        exp_bar_bg,
        exp_bar_frame,
        exp_bar_content_align,
        exp_bar_content,
        exp_bar_rank,
        exp_bar_txt,
        tree_title_txt,
        lock_imgs[],
        available_pts_txt,
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
        skill_sword_combo_4,
        skill_sword_dash_0,
        skill_sword_dash_1,
        skill_sword_dash_2,
        skill_sword_dash_3,
        skill_sword_dash_4,
        skill_sword_dash_5,
        skill_sword_dash_6,
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
        skill_axe_combo_4,
        skill_axe_spin_0,
        skill_axe_spin_1,
        skill_axe_spin_2,
        skill_axe_spin_3,
        skill_axe_spin_4,
        skill_axe_spin_5,
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
        skill_hammer_combo_4,
        skill_hammer_charged_0,
        skill_hammer_charged_1,
        skill_hammer_charged_2,
        skill_hammer_charged_3,
        skill_hammer_charged_4,
        skill_hammer_leap_0,
        skill_hammer_leap_1,
        skill_hammer_leap_2,
        skill_hammer_leap_3,
        skill_hammer_leap_4,
        skill_hammer_leap_5,
        bow_render,
        skill_bow_basic_0,
        skill_bow_basic_1,
        skill_bow_basic_2,
        skill_bow_charged_0,
        skill_bow_charged_1,
        skill_bow_charged_2,
        skill_bow_charged_3,
        skill_bow_charged_4,
        skill_bow_charged_5,
        skill_bow_charged_6,
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
        skill_staff_basic_4,
        skill_staff_beam_0,
        skill_staff_beam_1,
        skill_staff_beam_2,
        skill_staff_beam_3,
        skill_staff_beam_4,
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
        skill_sceptre_beam_6,
        skill_sceptre_bomb_0,
        skill_sceptre_bomb_1,
        skill_sceptre_bomb_2,
        skill_sceptre_bomb_3,
        skill_sceptre_bomb_4,
        skill_sceptre_bomb_5,
        general_combat_render_0,
        general_combat_render_1,
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
    stats: &'a Stats,

    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    pulse: f32,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    created_btns_top_l: usize,
    created_btns_top_r: usize,
    created_btns_bot_l: usize,
    created_btns_bot_r: usize,
    hovering_exp_bar: bool,
}

impl<'a> Diary<'a> {
    pub fn new(
        show: &'a Show,
        _client: &'a Client,
        stats: &'a Stats,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        pulse: f32,
    ) -> Self {
        Self {
            show,
            _client,
            stats,
            imgs,
            item_imgs,
            fonts,
            localized_strings,
            rot_imgs,
            tooltip_manager,
            pulse,
            common: widget::CommonBuilder::default(),
            created_btns_top_l: 0,
            created_btns_top_r: 0,
            created_btns_bot_l: 0,
            created_btns_bot_r: 0,
            hovering_exp_bar: false,
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

pub type SelectedSkillTree = skills::SkillGroupKind;

const TREES: [&str; 7] = [
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
    ChangeSkillTree(SelectedSkillTree),
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
        let frame_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8; //Animation timer
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
                .resize(TREES.len(), &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.weapon_imgs
                .resize(TREES.len(), &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.lock_imgs
                .resize(TREES.len(), &mut ui.widget_id_generator())
        });
        for i in TREES.iter().copied().enumerate() {
            let locked = !skill_tree_from_str(i.1)
                .map_or(false, |st| self.stats.skill_set.contains_skill_group(st));

            // Background weapon image
            let img = Image::new(match i.1 {
                "General Combat" => self.imgs.swords_crossed,
                "Sword" => self.imgs.sword,
                "Hammer" => self.imgs.hammer,
                "Axe" => self.imgs.axe,
                "Sceptre" => self.imgs.sceptre,
                "Bow" => self.imgs.bow,
                "Fire Staff" => self.imgs.staff,
                _ => self.imgs.nothing,
            });

            let img = if i.0 == 0 {
                img.top_left_with_margins_on(state.content_align, 10.0, 5.0)
            } else {
                img.down_from(state.weapon_btns[i.0 - 1], 5.0)
            };
            let tooltip_txt = if !locked {
                ""
            } else {
                &self.localized_strings.get("hud.skill.not_unlocked")
            };
            img.w_h(50.0, 50.0).set(state.weapon_imgs[i.0], ui);
            // Lock Image
            if locked {
                Image::new(self.imgs.lock)
                    .w_h(50.0, 50.0)
                    .middle_of(state.weapon_imgs[i.0])
                    .graphics_for(state.weapon_imgs[i.0])
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.8)))
                    .set(state.lock_imgs[i.0], ui);
            }
            // Weapon icons
            let available_pts = skill_tree_from_str(i.1)
                .map(|st| {
                    (
                        st,
                        self.stats.skill_set.available_sp(st),
                        self.stats.skill_set.earned_sp(st),
                    )
                })
                .map_or(false, |(st, a_pts, e_pts)| {
                    a_pts > 0 && (e_pts - a_pts) < st.total_skill_point_cost()
                });
            if Button::image(
                if skill_tree_from_str(i.1).map_or(false, |st| st == *sel_tab || available_pts) {
                    self.imgs.wpn_icon_border_pressed
                } else {
                    self.imgs.wpn_icon_border
                },
            )
            .w_h(50.0, 50.0)
            .hover_image(match skill_tree_from_str(i.1).map(|st| st == *sel_tab) {
                Some(true) => self.imgs.wpn_icon_border_pressed,
                Some(false) => self.imgs.wpn_icon_border_mo,
                None => self.imgs.wpn_icon_border,
            })
            .press_image(match skill_tree_from_str(i.1).map(|st| st == *sel_tab) {
                Some(true) => self.imgs.wpn_icon_border_pressed,
                Some(false) => self.imgs.wpn_icon_border_press,
                None => self.imgs.wpn_icon_border,
            })
            .middle_of(state.weapon_imgs[i.0])
            .image_color(
                if skill_tree_from_str(i.1).map_or(false, |st| st != *sel_tab && available_pts) {
                    Color::Rgba(0.92, 0.76, 0.0, frame_ani)
                } else {
                    TEXT_COLOR
                },
            )
            .with_tooltip(
                self.tooltip_manager,
                i.1,
                &tooltip_txt,
                &diary_tooltip,
                TEXT_COLOR,
            )
            .set(state.weapon_btns[i.0], ui)
            .was_clicked()
            {
                events.push(skill_tree_from_str(i.1).map_or(Event::Close, Event::ChangeSkillTree))
            }
        }
        // Exp Bars and Rank Display
        let current_exp = self.stats.skill_set.experience(*sel_tab) as f64;
        let max_exp = self.stats.skill_set.skill_point_cost(*sel_tab) as f64;
        let exp_percentage = current_exp / max_exp;
        let rank = self.stats.skill_set.earned_sp(*sel_tab);
        let rank_txt = format!("{}", rank);
        let exp_txt = format!("{}/{}", current_exp, max_exp);
        let available_pts = self.stats.skill_set.available_sp(*sel_tab);
        let available_pts_txt = format!("{}", available_pts);
        Image::new(self.imgs.diary_exp_bg)
            .w_h(480.0, 76.0)
            .mid_bottom_with_margin_on(state.content_align, 10.0)
            .set(state.exp_bar_bg, ui);
        Rectangle::fill_with([400.0, 40.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.exp_bar_bg, 32.0, 40.0)
            .set(state.exp_bar_content_align, ui);
        Image::new(self.imgs.bar_content)
            .w_h(400.0 * exp_percentage, 40.0)
            .top_left_with_margins_on(state.exp_bar_content_align, 0.0, 0.0)
            .color(Some(XP_COLOR))
            .set(state.exp_bar_content, ui);
        Image::new(self.imgs.diary_exp_frame)
            .w_h(480.0, 76.0)
            .color(Some(UI_HIGHLIGHT_0))
            .middle_of(state.exp_bar_bg)
            .set(state.exp_bar_frame, ui);
        // Show EXP bar text on hover
        self.hovering_exp_bar = ui
            .widget_input(state.exp_bar_frame)
            .mouse()
            .map_or(false, |m| m.is_over());
        if self.hovering_exp_bar {
            Text::new(&exp_txt)
                .mid_top_with_margin_on(state.exp_bar_frame, 47.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .graphics_for(state.exp_bar_frame)
                .set(state.exp_bar_txt, ui);
        }
        Text::new(&rank_txt)
            .mid_top_with_margin_on(state.exp_bar_frame, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(28))
            .color(TEXT_COLOR)
            .set(state.exp_bar_rank, ui);
        Text::new(
            &self
                .localized_strings
                .get("hud.skill.sp_available")
                .replace("{number}", &available_pts_txt),
        )
        .mid_top_with_margin_on(state.content_align, 700.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(28))
        .color(if available_pts > 0 {
            Color::Rgba(0.92, 0.76, 0.0, frame_ani)
        } else {
            TEXT_COLOR
        })
        .set(state.available_pts_txt, ui);
        let tree_title = match sel_tab {
            SelectedSkillTree::General => "General Combat",
            SelectedSkillTree::Weapon(ToolKind::Sword) => "Sword",
            SelectedSkillTree::Weapon(ToolKind::Hammer) => "Hammer",
            SelectedSkillTree::Weapon(ToolKind::Axe) => "Axe",
            SelectedSkillTree::Weapon(ToolKind::Sceptre) => "Healing Sceptre",
            SelectedSkillTree::Weapon(ToolKind::Bow) => "Bow",
            SelectedSkillTree::Weapon(ToolKind::Staff) => "Fire Staff",
            _ => "Unknown",
        };
        Text::new(&tree_title)
            .mid_top_with_margin_on(state.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.tree_title_txt, ui);
        // Skill Trees
        // Alignment Placing
        let x = 200.0;
        let y = 100.0;
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
            SelectedSkillTree::General => 2,
            SelectedSkillTree::Weapon(ToolKind::Sword) => 5,
            SelectedSkillTree::Weapon(ToolKind::Axe) => 5,
            SelectedSkillTree::Weapon(ToolKind::Hammer) => 5,
            SelectedSkillTree::Weapon(ToolKind::Bow) => 3,
            SelectedSkillTree::Weapon(ToolKind::Staff) => 5,
            SelectedSkillTree::Weapon(ToolKind::Sceptre) => 7,
            _ => 0,
        };
        let skills_top_r = match sel_tab {
            SelectedSkillTree::General => 6,
            SelectedSkillTree::Weapon(ToolKind::Sword) => 7,
            SelectedSkillTree::Weapon(ToolKind::Axe) => 6,
            SelectedSkillTree::Weapon(ToolKind::Hammer) => 5,
            SelectedSkillTree::Weapon(ToolKind::Bow) => 7,
            SelectedSkillTree::Weapon(ToolKind::Staff) => 5,
            SelectedSkillTree::Weapon(ToolKind::Sceptre) => 6,
            _ => 0,
        };
        let skills_bot_l = match sel_tab {
            SelectedSkillTree::General => 4,
            SelectedSkillTree::Weapon(ToolKind::Sword) => 5,
            SelectedSkillTree::Weapon(ToolKind::Axe) => 5,
            SelectedSkillTree::Weapon(ToolKind::Hammer) => 6,
            SelectedSkillTree::Weapon(ToolKind::Bow) => 5,
            SelectedSkillTree::Weapon(ToolKind::Staff) => 5,
            _ => 0,
        };
        let skills_bot_r = match sel_tab {
            SelectedSkillTree::Weapon(ToolKind::Sword) => 1,
            SelectedSkillTree::Weapon(ToolKind::Bow) => 1,
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
        let offset_0 = 22.0;
        let offset_1 = -122.0;
        let offset_2 = offset_1 - -20.0;
        while self.created_btns_top_l < skills_top_l {
            let mut img = Button::image(self.imgs.wpn_icon_border_skills).w_h(80.0, 100.0);
            match self.created_btns_top_l {
                0 => img = img.middle_of(state.skills_top_l_align), // Central Skill
                1 => img = img.up_from(state.skills_top_l[0], offset_0), // 12:00
                2 => img = img.down_from(state.skills_top_l[0], offset_0), // 6:00
                3 => img = img.left_from(state.skills_top_l[0], offset_0), // 3:00
                4 => img = img.right_from(state.skills_top_l[0], offset_0), // 9:00
                5 => img = img.top_left_with_margins_on(state.skills_top_l[0], offset_1, offset_2), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_top_l[0], offset_1, offset_2), /* 1:30 */
                7 => {
                    img = img.bottom_left_with_margins_on(state.skills_top_l[0], offset_1, offset_2)
                }, /* 4:30 */
                8 => {
                    img =
                        img.bottom_right_with_margins_on(state.skills_top_l[0], offset_1, offset_2)
                }, /* 7:30 */
                _ => {},
            }
            img.set(state.skills_top_l[self.created_btns_top_l], ui);
            self.created_btns_top_l += 1;
        }
        // TOP-RIGHT Skills
        while self.created_btns_top_r < skills_top_r {
            let mut img = Button::image(self.imgs.wpn_icon_border_skills).w_h(80.0, 100.0);
            match self.created_btns_top_r {
                0 => img = img.middle_of(state.skills_top_r_align), // Central Skill
                1 => img = img.up_from(state.skills_top_r[0], offset_0), // 12:00
                2 => img = img.down_from(state.skills_top_r[0], offset_0), // 6:00
                3 => img = img.left_from(state.skills_top_r[0], offset_0), // 3:00
                4 => img = img.right_from(state.skills_top_r[0], offset_0), // 9:00
                5 => img = img.top_left_with_margins_on(state.skills_top_r[0], offset_1, offset_2), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_top_r[0], offset_1, offset_2), /* 1:30 */
                7 => {
                    img = img.bottom_left_with_margins_on(state.skills_top_r[0], offset_1, offset_2)
                }, /* 4:30 */
                8 => {
                    img =
                        img.bottom_right_with_margins_on(state.skills_top_r[0], offset_1, offset_2)
                }, /* 7:30 */
                _ => {},
            }
            img.set(state.skills_top_r[self.created_btns_top_r], ui);
            self.created_btns_top_r += 1;
        }
        // BOTTOM-LEFT Skills
        while self.created_btns_bot_l < skills_bot_l {
            let mut img = Button::image(self.imgs.wpn_icon_border_skills).w_h(80.0, 100.0);
            match self.created_btns_bot_l {
                0 => img = img.middle_of(state.skills_bot_l_align), // Central Skill
                1 => img = img.up_from(state.skills_bot_l[0], offset_0), // 12:00
                2 => img = img.down_from(state.skills_bot_l[0], offset_0), // 6:00
                3 => img = img.left_from(state.skills_bot_l[0], offset_0), // 3:00
                4 => img = img.right_from(state.skills_bot_l[0], offset_0), // 9:00
                5 => img = img.top_left_with_margins_on(state.skills_bot_l[0], offset_1, offset_2), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_bot_l[0], offset_1, offset_2), /* 1:30 */
                7 => {
                    img = img.bottom_left_with_margins_on(state.skills_bot_l[0], offset_1, offset_2)
                }, /* 4:30 */
                8 => {
                    img =
                        img.bottom_right_with_margins_on(state.skills_bot_l[0], offset_1, offset_2)
                }, /* 7:30 */
                _ => {},
            }
            img.set(state.skills_bot_l[self.created_btns_bot_l], ui);
            self.created_btns_bot_l += 1;
        }
        // BOTTOM-RIGHT Skills
        while self.created_btns_bot_r < skills_bot_r {
            let mut img = Image::new(self.imgs.wpn_icon_border_skills).w_h(80.0, 100.0);
            match self.created_btns_bot_r {
                0 => img = img.middle_of(state.skills_bot_r_align), // Central Skill
                1 => img = img.up_from(state.skills_bot_r[0], offset_0), // 12:00
                2 => img = img.down_from(state.skills_bot_r[0], offset_0), // 6:00
                3 => img = img.left_from(state.skills_bot_r[0], offset_0), // 3:00
                4 => img = img.right_from(state.skills_bot_r[0], offset_0), // 9:00
                5 => img = img.top_left_with_margins_on(state.skills_bot_r[0], offset_1, offset_2), /* 10:30 */
                6 => img = img.top_right_with_margins_on(state.skills_bot_r[0], offset_1, offset_2), /* 1:30 */
                7 => {
                    img = img.bottom_left_with_margins_on(state.skills_bot_r[0], offset_1, offset_2)
                }, /* 4:30 */
                8 => {
                    img =
                        img.bottom_right_with_margins_on(state.skills_bot_r[0], offset_1, offset_2)
                }, /* 7:30 */
                _ => {},
            }
            img.set(state.skills_bot_r[self.created_btns_bot_r], ui);
            self.created_btns_bot_r += 1;
        }
        // Skill-Icons and Functionality
        // Art dimensions
        let art_size = [320.0, 320.0];
        match sel_tab {
            SelectedSkillTree::General => {
                use skills::{GeneralSkill::*, RollSkill::*, SkillGroupKind::*};
                use ToolKind::*;
                // General Combat
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(Tool("example_general_combat_left".to_string())),
                    self.pulse,
                ))
                .wh(art_size)
                .middle_of(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .set(state.general_combat_render_0, ui);
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(Tool("example_general_combat_right".to_string())),
                    self.pulse,
                ))
                .wh(art_size)
                .middle_of(state.general_combat_render_0)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .set(state.general_combat_render_1, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                let skill = Skill::General(HealthIncrease);
                if create_skill_button(
                    self.imgs.health_plus_skill,
                    state.skills_top_l[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.inc_health_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.inc_health"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_stat_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::General(EnergyIncrease);
                if create_skill_button(
                    self.imgs.stamina_plus_skill,
                    state.skills_top_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.inc_stam_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.inc_stam"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_stat_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Top right skills
                let skill = Skill::UnlockGroup(Weapon(Sword));
                if create_skill_button(
                    self.imgs.unlock_sword_skill,
                    state.skills_top_r[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.unlck_sword_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.unlck_sword"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_tree_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::UnlockGroup(Weapon(Axe));
                if create_skill_button(
                    self.imgs.unlock_axe_skill,
                    state.skills_top_r[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.unlck_axe_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.unlck_axe"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_tree_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::UnlockGroup(Weapon(Hammer));
                if create_skill_button(
                    self.imgs.unlock_hammer_skill,
                    state.skills_top_r[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.unlck_hammer_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.unlck_hammer"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_tree_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::UnlockGroup(Weapon(Bow));
                if create_skill_button(
                    self.imgs.unlock_bow_skill,
                    state.skills_top_r[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.unlck_bow_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.unlck_bow"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_tree_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::UnlockGroup(Weapon(Staff));
                if create_skill_button(
                    self.imgs.unlock_staff_skill0,
                    state.skills_top_r[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.unlck_staff_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.unlck_staff"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_tree_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::UnlockGroup(Weapon(Sceptre));
                if create_skill_button(
                    self.imgs.unlock_sceptre_skill,
                    state.skills_top_r[5],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.unlck_sceptre_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.unlck_sceptre"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_tree_5, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Bottom left skills
                let skill = Skill::Roll(ImmuneMelee);
                if create_skill_button(
                    self.imgs.skill_dodge_skill,
                    state.skills_bot_l[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.dodge_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.dodge"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_roll_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Roll(Cost);
                if create_skill_button(
                    self.imgs.utility_cost_skill,
                    state.skills_bot_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.roll_stamina_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.roll_stamina"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_roll_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Roll(Strength);
                if create_skill_button(
                    self.imgs.utility_speed_skill,
                    state.skills_bot_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.roll_speed_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.roll_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_roll_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Roll(Duration);
                if create_skill_button(
                    self.imgs.utility_amount_skill,
                    state.skills_bot_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.roll_dur_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.roll_dur"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_general_roll_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
            },
            SelectedSkillTree::Weapon(ToolKind::Sword) => {
                use skills::SwordSkill::*;
                // Sword
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(Tool("example_sword".to_string())),
                    self.pulse,
                ))
                .wh(art_size)
                .middle_of(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .set(state.sword_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                Button::image(self.imgs.twohsword_m1)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_l[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self.localized_strings.get("hud.skill.sw_trip_str_title"),
                        &self.localized_strings.get("hud.skill.sw_trip_str"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_combo_0, ui);
                let skill = Skill::Sword(TsCombo);
                if create_skill_button(
                    self.imgs.physical_combo_skill,
                    state.skills_top_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.sw_trip_str_combo_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_trip_str_combo"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_combo_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(TsDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_top_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.sw_trip_str_dmg_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_trip_str_dmg"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_combo_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(TsSpeed);
                if create_skill_button(
                    self.imgs.physical_speed_skill,
                    state.skills_top_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_trip_str_sp_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_trip_str_sp"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_combo_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(TsRegen);
                if create_skill_button(
                    self.imgs.physical_energy_regen_skill,
                    state.skills_top_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.sw_trip_str_reg_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_trip_str_reg"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_combo_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Top right skills
                Button::image(self.imgs.twohsword_m2)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_r[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self.localized_strings.get("hud.skill.sw_dash_title"),
                        &self.localized_strings.get("hud.skill.sw_dash"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sword_dash_0, ui);
                let skill = Skill::Sword(DDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_top_r[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_dash_dmg_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_dash_dmg"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_dash_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(DDrain);
                if create_skill_button(
                    self.imgs.physical_energy_drain_skill,
                    state.skills_top_r[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_dash_drain_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_dash_drain"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_dash_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(DCost);
                if create_skill_button(
                    self.imgs.physical_cost_skill,
                    state.skills_top_r[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_dash_cost_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_dash_cost"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_dash_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(DSpeed);
                if create_skill_button(
                    self.imgs.physical_speed_skill,
                    state.skills_top_r[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_dash_speed_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_dash_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_dash_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(DInfinite);
                if create_skill_button(
                    self.imgs.physical_infinite_skill,
                    state.skills_top_r[5],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_dash_inf_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_dash_inf"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_dash_5, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(DScaling);
                if create_skill_button(
                    self.imgs.physical_amount_skill,
                    state.skills_top_r[6],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_dash_scale_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_dash_scale"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_dash_6, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Bottom left skills
                let skill = Skill::Sword(UnlockSpin);
                if create_skill_button(
                    self.imgs.sword_whirlwind,
                    state.skills_bot_l[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_spin_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_spin"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_spin_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(SDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_bot_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_spin_dmg_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_spin_dmg"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_spin_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(SSpeed);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_bot_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_spin_spd_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_spin_spd"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_spin_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(SCost);
                if create_skill_button(
                    self.imgs.physical_cost_skill,
                    state.skills_bot_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_spin_cost_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_spin_cost"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_spin_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sword(SSpins);
                if create_skill_button(
                    self.imgs.physical_amount_skill,
                    state.skills_bot_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_spin_spins_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_spin_spins"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_spin_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Bottom right skills
                let skill = Skill::Sword(InterruptingAttacks);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_bot_r[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sw_interrupt_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sw_interrupt"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sword_passive_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
            },
            SelectedSkillTree::Weapon(ToolKind::Axe) => {
                use skills::AxeSkill::*;
                // Axe
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(Tool("example_axe".to_string())),
                    self.pulse,
                ))
                .wh(art_size)
                .middle_of(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .set(state.axe_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                Button::image(self.imgs.twohaxe_m1)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_l[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self
                            .localized_strings
                            .get("hud.skill.axe_double_strike_title"),
                        &self.localized_strings.get("hud.skill.axe_double_strike"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_combo_0, ui);
                let skill = Skill::Axe(DsCombo);
                if create_skill_button(
                    self.imgs.physical_combo_skill,
                    state.skills_top_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_double_strike_combo_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.axe_double_strike_combo"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_combo_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(DsDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_top_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_double_strike_damage_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.axe_double_strike_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_combo_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(DsSpeed);
                if create_skill_button(
                    self.imgs.physical_speed_skill,
                    state.skills_top_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_double_strike_speed_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.axe_double_strike_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_combo_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(DsRegen);
                if create_skill_button(
                    self.imgs.physical_energy_regen_skill,
                    state.skills_top_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_double_strike_regen_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.axe_double_strike_regen"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_combo_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Top right skills
                Button::image(self.imgs.axespin)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_r[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self.localized_strings.get("hud.skill.axe_spin_title"),
                        &self.localized_strings.get("hud.skill.axe_spin"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_axe_spin_0, ui);
                let skill = Skill::Axe(SInfinite);
                if create_skill_button(
                    self.imgs.physical_infinite_skill,
                    state.skills_top_r[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_infinite_axe_spin_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.axe_infinite_axe_spin"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_spin_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(SDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_top_r[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_spin_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.axe_spin_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_spin_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(SHelicopter);
                if create_skill_button(
                    self.imgs.physical_helicopter_skill,
                    state.skills_top_r[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_spin_helicopter_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.axe_spin_helicopter"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_spin_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(SSpeed);
                if create_skill_button(
                    self.imgs.physical_speed_skill,
                    state.skills_top_r[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.axe_spin_speed_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.axe_spin_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_spin_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(SCost);
                if create_skill_button(
                    self.imgs.physical_cost_skill,
                    state.skills_top_r[5],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.axe_spin_cost_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.axe_spin_cost"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_spin_5, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Bottom left skills
                let skill = Skill::Axe(UnlockLeap);
                if create_skill_button(
                    self.imgs.skill_axe_leap_slash,
                    state.skills_bot_l[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_unlock_leap_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.axe_unlock_leap"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_leap_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(LDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_bot_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_leap_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.axe_leap_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_leap_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(LKnockback);
                if create_skill_button(
                    self.imgs.physical_knockback_skill,
                    state.skills_bot_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_leap_knockback_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.axe_leap_knockback"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_leap_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(LCost);
                if create_skill_button(
                    self.imgs.physical_cost_skill,
                    state.skills_bot_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.axe_leap_cost_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.axe_leap_cost"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_leap_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Axe(LDistance);
                if create_skill_button(
                    self.imgs.physical_distance_skill,
                    state.skills_bot_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.axe_leap_distance_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.axe_leap_distance"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_axe_leap_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
            },
            SelectedSkillTree::Weapon(ToolKind::Hammer) => {
                use skills::HammerSkill::*;
                // Hammer
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(Tool("example_hammer".to_string())),
                    self.pulse,
                ))
                .wh(art_size)
                .middle_of(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .set(state.hammer_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                Button::image(self.imgs.twohhammer_m1)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_l[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self
                            .localized_strings
                            .get("hud.skill.hmr_single_strike_title"),
                        &self.localized_strings.get("hud.skill.hmr_single_strike"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_combo_0, ui);
                let skill = Skill::Hammer(SsKnockback);
                if create_skill_button(
                    self.imgs.physical_knockback_skill,
                    state.skills_top_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_single_strike_knockback_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.hmr_single_strike_knockback"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_combo_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(SsDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_top_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_single_strike_damage_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.hmr_single_strike_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_combo_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(SsSpeed);
                if create_skill_button(
                    self.imgs.physical_speed_skill,
                    state.skills_top_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_single_strike_speed_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.hmr_single_strike_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_combo_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(SsRegen);
                if create_skill_button(
                    self.imgs.physical_energy_regen_skill,
                    state.skills_top_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_single_strike_regen_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.hmr_single_strike_regen"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_combo_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Top right skills
                Button::image(self.imgs.hammergolf)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_r[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self
                            .localized_strings
                            .get("hud.skill.hmr_charged_melee_title"),
                        &self.localized_strings.get("hud.skill.hmr_charged_melee"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_hammer_charged_0, ui);
                let skill = Skill::Hammer(CKnockback);
                if create_skill_button(
                    self.imgs.physical_knockback_skill,
                    state.skills_top_r[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_charged_melee_knockback_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.hmr_charged_melee_knockback"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_charged_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(CDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_top_r[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_charged_melee_damage_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.hmr_charged_melee_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_charged_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(CDrain);
                if create_skill_button(
                    self.imgs.physical_energy_drain_skill,
                    state.skills_top_r[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_charged_melee_nrg_drain_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.hmr_charged_melee_nrg_drain"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_charged_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(CSpeed);
                if create_skill_button(
                    self.imgs.physical_amount_skill,
                    state.skills_top_r[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_charged_rate_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.hmr_charged_rate"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_charged_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Bottom left skills
                let skill = Skill::Hammer(UnlockLeap);
                if create_skill_button(
                    self.imgs.hammerleap,
                    state.skills_bot_l[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_unlock_leap_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.hmr_unlock_leap"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_leap_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(LDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_bot_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_leap_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.hmr_leap_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_leap_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(LKnockback);
                if create_skill_button(
                    self.imgs.physical_knockback_skill,
                    state.skills_bot_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_leap_knockback_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.hmr_leap_knockback"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_leap_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(LCost);
                if create_skill_button(
                    self.imgs.physical_cost_skill,
                    state.skills_bot_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.hmr_leap_cost_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.hmr_leap_cost"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_leap_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(LDistance);
                if create_skill_button(
                    self.imgs.physical_distance_skill,
                    state.skills_bot_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_leap_distance_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.hmr_leap_distance"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_leap_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Hammer(LRange);
                if create_skill_button(
                    self.imgs.physical_radius_skill,
                    state.skills_bot_l[5],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.hmr_leap_radius_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.hmr_leap_radius"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_hammer_leap_5, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
            },
            SelectedSkillTree::Weapon(ToolKind::Bow) => {
                use skills::BowSkill::*;
                // Bow
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(Tool("example_bow".to_string())),
                    self.pulse,
                ))
                .wh(art_size)
                .middle_of(state.content_align)
                .set(state.bow_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                Button::image(self.imgs.bow_m1)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_l[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self.localized_strings.get("hud.skill.bow_title"),
                        &self.localized_strings.get("hud.skill.bow"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_basic_0, ui);
                let skill = Skill::Bow(BDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_top_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.bow_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_basic_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(BRegen);
                if create_skill_button(
                    self.imgs.physical_energy_regen_skill,
                    state.skills_top_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_energy_regen_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_energy_regen"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_basic_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Top right skills
                Button::image(self.imgs.bow_m2)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_r[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self.localized_strings.get("hud.skill.bow_charged_title"),
                        &self.localized_strings.get("hud.skill.bow_charged"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_bow_charged_0, ui);
                let skill = Skill::Bow(CDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_top_r[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_charged_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_charged_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_charged_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(CDrain);
                if create_skill_button(
                    self.imgs.physical_energy_drain_skill,
                    state.skills_top_r[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_charged_drain_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_charged_drain"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_charged_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(CProjSpeed);
                if create_skill_button(
                    self.imgs.physical_projectile_speed_skill,
                    state.skills_top_r[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_charged_projectile_speed_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.bow_charged_projectile_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_charged_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(CSpeed);
                if create_skill_button(
                    self.imgs.physical_speed_skill,
                    state.skills_top_r[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_charged_speed_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_charged_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_charged_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(CMove);
                if create_skill_button(
                    self.imgs.physical_speed_skill,
                    state.skills_top_r[5],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_charged_move_speed_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.bow_charged_move_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_charged_5, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(CKnockback);
                if create_skill_button(
                    self.imgs.physical_knockback_skill,
                    state.skills_top_r[6],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_charged_knockback_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.bow_charged_knockback"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_charged_6, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Bottom left skills
                let skill = Skill::Bow(UnlockRepeater);
                if create_skill_button(
                    self.imgs.skill_bow_jump_burst,
                    state.skills_bot_l[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_repeater_unlock_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_repeater_unlock"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_repeater_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(RDamage);
                if create_skill_button(
                    self.imgs.physical_damage_skill,
                    state.skills_bot_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_repeater_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_repeater_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_repeater_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(RGlide);
                if create_skill_button(
                    self.imgs.physical_helicopter_skill,
                    state.skills_bot_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_repeater_glide_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_repeater_glide"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_repeater_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(RCost);
                if create_skill_button(
                    self.imgs.physical_cost_skill,
                    state.skills_bot_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_repeater_cost_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_repeater_cost"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_repeater_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Bow(RArrows);
                if create_skill_button(
                    self.imgs.physical_amount_skill,
                    state.skills_bot_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_arrow_count_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_arrow_count"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_repeater_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Bottom right skills
                let skill = Skill::Bow(ProjSpeed);
                if create_skill_button(
                    self.imgs.physical_projectile_speed_skill,
                    state.skills_bot_r[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.bow_projectile_speed_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.bow_projectile_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_bow_passive_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
            },
            SelectedSkillTree::Weapon(ToolKind::Staff) => {
                use skills::StaffSkill::*;
                // Staff
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(Tool("example_staff_fire".to_string())),
                    self.pulse,
                ))
                .wh(art_size)
                .middle_of(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .set(state.staff_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                Button::image(self.imgs.fireball)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_l[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self.localized_strings.get("hud.skill.st_fireball_title"),
                        &self.localized_strings.get("hud.skill.st_fireball"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_basic_0, ui);
                let skill = Skill::Staff(BExplosion);
                if create_skill_button(
                    self.imgs.magic_explosion_skill,
                    state.skills_top_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.st_explosion_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_explosion"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_basic_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(BDamage);
                if create_skill_button(
                    self.imgs.magic_damage_skill,
                    state.skills_top_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.st_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_basic_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(BRegen);
                if create_skill_button(
                    self.imgs.magic_energy_regen_skill,
                    state.skills_top_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_stamina_regen_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_stamina_regen"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_basic_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(BRadius);
                if create_skill_button(
                    self.imgs.magic_radius_skill,
                    state.skills_top_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_explosion_radius_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_explosion_radius"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_basic_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Top right skills
                Button::image(self.imgs.flamethrower)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_r[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self
                            .localized_strings
                            .get("hud.skill.st_flamethrower_title"),
                        &self.localized_strings.get("hud.skill.st_flamethrower"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_staff_beam_0, ui);
                let skill = Skill::Staff(FDamage);
                if create_skill_button(
                    self.imgs.magic_damage_skill,
                    state.skills_top_r[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_flamethrower_damage_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.st_flamethrower_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_beam_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(FDrain);
                if create_skill_button(
                    self.imgs.magic_energy_drain_skill,
                    state.skills_top_r[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_energy_drain_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_energy_drain"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_beam_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(FRange);
                if create_skill_button(
                    self.imgs.magic_radius_skill,
                    state.skills_top_r[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_flamethrower_range_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.st_flamethrower_range"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_beam_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(FVelocity);
                if create_skill_button(
                    self.imgs.magic_projectile_speed_skill,
                    state.skills_top_r[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_flame_velocity_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_flame_velocity"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_beam_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                // Bottom left skills
                let skill = Skill::Staff(UnlockShockwave);
                if create_skill_button(
                    self.imgs.fire_aoe,
                    state.skills_bot_l[0],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_shockwave_unlock_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_shockwave_unlock"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_shockwave_0, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(SDamage);
                if create_skill_button(
                    self.imgs.magic_damage_skill,
                    state.skills_bot_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_shockwave_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_shockwave_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_shockwave_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(SKnockback);
                if create_skill_button(
                    self.imgs.magic_knockback_skill,
                    state.skills_bot_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_shockwave_knockback_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.st_shockwave_knockback"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_shockwave_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(SCost);
                if create_skill_button(
                    self.imgs.magic_cost_skill,
                    state.skills_bot_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_shockwave_cost_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_shockwave_cost"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_shockwave_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Staff(SRange);
                if create_skill_button(
                    self.imgs.magic_radius_skill,
                    state.skills_bot_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.st_shockwave_range_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.st_shockwave_range"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_staff_shockwave_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
            },
            SelectedSkillTree::Weapon(ToolKind::Sceptre) => {
                use skills::SceptreSkill::*;
                // Sceptre
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(Tool("example_sceptre".to_string())),
                    self.pulse,
                ))
                .wh(art_size)
                .middle_of(state.content_align)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .set(state.sceptre_render, ui);
                // Top Left skills
                //        5 1 6
                //        3 0 4
                //        8 2 7
                Button::image(self.imgs.skill_sceptre_heal)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_l[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self.localized_strings.get("hud.skill.sc_beam_title"),
                        &self.localized_strings.get("hud.skill.sc_beam"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_beam_0, ui);
                let skill = Skill::Sceptre(BHeal);
                if create_skill_button(
                    self.imgs.heal_heal_skill,
                    state.skills_top_l[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sc_beam_heal_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_beam_heal"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_beam_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sceptre(BDamage);
                if create_skill_button(
                    self.imgs.heal_damage_skill,
                    state.skills_top_l[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sc_beam_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_beam_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_beam_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sceptre(BRegen);
                if create_skill_button(
                    self.imgs.heal_energy_regen_skill,
                    state.skills_top_l[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.sc_energy_regen_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_energy_regen"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_beam_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sceptre(BRange);
                if create_skill_button(
                    self.imgs.heal_radius_skill,
                    state.skills_top_l[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sc_range_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_range"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_beam_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sceptre(BLifesteal);
                if create_skill_button(
                    self.imgs.heal_lifesteal_skill,
                    state.skills_top_l[5],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.sc_lifesteal_efficiency_title"),
                    &add_sp_cost_tooltip(
                        &self
                            .localized_strings
                            .get("hud.skill.sc_lifesteal_efficiency"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_beam_5, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sceptre(BCost);
                if create_skill_button(
                    self.imgs.heal_cost_skill,
                    state.skills_top_l[6],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sc_heal_cost_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_heal_cost"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_beam_6, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                Button::image(self.imgs.skill_sceptre_heal)
                    .w_h(74.0, 74.0)
                    .mid_top_with_margin_on(state.skills_top_r[0], 3.0)
                    .with_tooltip(
                        self.tooltip_manager,
                        &self.localized_strings.get("hud.skill.sc_healbomb_title"),
                        &self.localized_strings.get("hud.skill.sc_healbomb"),
                        &diary_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.skill_sceptre_bomb_0, ui);
                // Top right skills
                let skill = Skill::Sceptre(PHeal);
                if create_skill_button(
                    self.imgs.heal_heal_skill,
                    state.skills_top_r[1],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sc_heal_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_heal"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_bomb_1, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sceptre(PDamage);
                if create_skill_button(
                    self.imgs.heal_damage_skill,
                    state.skills_top_r[2],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sc_damage_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_damage"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_bomb_2, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sceptre(PRadius);
                if create_skill_button(
                    self.imgs.heal_radius_skill,
                    state.skills_top_r[3],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sc_radius_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_radius"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_bomb_3, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sceptre(PCost);
                if create_skill_button(
                    self.imgs.heal_cost_skill,
                    state.skills_top_r[4],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self.localized_strings.get("hud.skill.sc_energy_cost_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_energy_cost"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_bomb_4, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
                let skill = Skill::Sceptre(PProjSpeed);
                if create_skill_button(
                    self.imgs.heal_projectile_speed_skill,
                    state.skills_top_r[5],
                    &self.stats.skill_set,
                    skill,
                    self.fonts,
                    &get_skill_label(skill, &self.stats.skill_set),
                )
                .with_tooltip(
                    self.tooltip_manager,
                    &self
                        .localized_strings
                        .get("hud.skill.sc_projectile_speed_title"),
                    &add_sp_cost_tooltip(
                        &self.localized_strings.get("hud.skill.sc_projectile_speed"),
                        skill,
                        &self.stats.skill_set,
                        &self.localized_strings,
                    ),
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.skill_sceptre_bomb_5, ui)
                .was_clicked()
                {
                    events.push(Event::UnlockSkill(skill));
                };
            },
            _ => {},
        }

        events
    }
}

fn create_skill_button<'a>(
    image: Id,
    state: widget::Id,
    skill_set: &'a skills::SkillSet,
    skill: Skill,
    fonts: &'a Fonts,
    label: &'a str,
) -> Button<'a, button::Image> {
    Button::image(image)
        .w_h(74.0, 74.0)
        .mid_top_with_margin_on(state, 3.0)
        .label(label)
        .label_y(conrod_core::position::Relative::Scalar(-47.0))
        .label_x(conrod_core::position::Relative::Scalar(0.0))
        .label_color(if skill_set.is_at_max_level(skill) {
            TEXT_COLOR
        } else if skill_set.sufficient_skill_points(skill) {
            HP_COLOR
        } else {
            CRITICAL_HP_COLOR
        })
        .label_font_size(fonts.cyri.scale(15))
        .label_font_id(fonts.cyri.conrod_id)
        .image_color(if skill_set.prerequisites_met(skill) {
            TEXT_COLOR
        } else {
            Color::Rgba(0.41, 0.41, 0.41, 0.7)
        })
}

fn get_skill_label(skill: Skill, skill_set: &skills::SkillSet) -> String {
    if skill_set.prerequisites_met(skill) {
        format!(
            "{}/{}",
            skill_set.skill_level(skill).map_or(0, |l| l.unwrap_or(1)),
            skill.max_level().unwrap_or(1)
        )
    } else {
        "".to_string()
    }
}

fn skill_tree_from_str(string: &str) -> Option<SelectedSkillTree> {
    match string {
        "General Combat" => Some(SelectedSkillTree::General),
        "Sword" => Some(SelectedSkillTree::Weapon(ToolKind::Sword)),
        "Hammer" => Some(SelectedSkillTree::Weapon(ToolKind::Hammer)),
        "Axe" => Some(SelectedSkillTree::Weapon(ToolKind::Axe)),
        "Sceptre" => Some(SelectedSkillTree::Weapon(ToolKind::Sceptre)),
        "Bow" => Some(SelectedSkillTree::Weapon(ToolKind::Bow)),
        "Fire Staff" => Some(SelectedSkillTree::Weapon(ToolKind::Staff)),
        _ => None,
    }
}

fn add_sp_cost_tooltip<'a>(
    tooltip: &'a str,
    skill: Skill,
    skill_set: &'a skills::SkillSet,
    localized_strings: &'a Localization,
) -> String {
    match skill_set.skill_level(skill) {
        Ok(level) if level == skill.max_level() => tooltip.replace("{SP}", ""),
        _ => tooltip.replace(
            "{SP}",
            &localized_strings
                .get("hud.skill.req_sp")
                .replace("{number}", &format!("{}", skill_set.skill_cost(skill))),
        ),
    }
}
