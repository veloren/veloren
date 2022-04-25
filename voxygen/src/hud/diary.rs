use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::{animate_by_pulse, ItemImgs},
    Position, PositionSpecifier, Show, BLACK, CRITICAL_HP_COLOR, HP_COLOR, TEXT_COLOR,
    UI_HIGHLIGHT_0, UI_MAIN, XP_COLOR,
};
use crate::{
    game_input::GameInput,
    hud::{
        self,
        slots::{AbilitySlot, SlotManager},
        util,
    },
    ui::{
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
        ImageFrame, Tooltip, TooltipManager, Tooltipable,
    },
    GlobalState,
};
use conrod_core::{
    color, image,
    widget::{self, Button, Image, Rectangle, State, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, UiCell, Widget, WidgetCommon,
};
use i18n::Localization;
use vek::*;

use client::{self, Client};
use common::{
    combat,
    comp::{
        self,
        ability::{Ability, ActiveAbilities, AuxiliaryAbility, MAX_ABILITIES},
        inventory::{
            item::{
                item_key::ItemKey,
                tool::{MaterialStatManifest, ToolKind},
                ItemKind,
            },
            slot::EquipSlot,
        },
        skills::{
            self, AxeSkill, BowSkill, ClimbSkill, GeneralSkill, HammerSkill, MiningSkill,
            RollSkill, SceptreSkill, Skill, StaffSkill, SwimSkill, SwordSkill, SKILL_MODIFIERS,
        },
        skillset::{SkillGroupKind, SkillSet},
        Body, Energy, Health, Inventory, Poise,
    },
    consts::{ENERGY_PER_LEVEL, HP_PER_LEVEL},
};
use std::borrow::Cow;

const ART_SIZE: [f64; 2] = [320.0, 320.0];

widget_ids! {
    pub struct Ids {
        frame,
        bg,
        icon,
        close,
        title,
        content_align,
        section_imgs[],
        section_btns[],
        // Skill tree stuffs
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
        skill_bow_shotgun_0,
        skill_bow_shotgun_1,
        skill_bow_shotgun_2,
        skill_bow_shotgun_3,
        skill_bow_shotgun_4,
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
        skill_sceptre_lifesteal_0,
        skill_sceptre_lifesteal_1,
        skill_sceptre_lifesteal_2,
        skill_sceptre_lifesteal_3,
        skill_sceptre_lifesteal_4,
        skill_sceptre_heal_0,
        skill_sceptre_heal_1,
        skill_sceptre_heal_2,
        skill_sceptre_heal_3,
        skill_sceptre_heal_4,
        skill_sceptre_aura_0,
        skill_sceptre_aura_1,
        skill_sceptre_aura_2,
        skill_sceptre_aura_3,
        skill_sceptre_aura_4,
        pick_render,
        skill_pick_m1,
        skill_pick_m1_0,
        skill_pick_m1_1,
        skill_pick_m1_2,
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
        skill_general_climb_0,
        skill_general_climb_1,
        skill_general_climb_2,
        skill_general_swim_0,
        skill_general_swim_1,
        // Ability selection
        spellbook_art,
        sb_page_left_align,
        sb_page_right_align,
        spellbook_skills_bg,
        spellbook_btn,
        spellbook_btn_bg,
        ability_select_title,
        ability_page_left,
        ability_page_right,
        active_abilities[],
        active_abilities_keys[],
        main_weap_select,
        off_weap_select,
        abilities[],
        ability_frames[],
        abilities_dual[],
        ability_titles[],
        ability_descs[],
        dragged_ability,
        // Stats
        stat_names[],
        stat_values[],
    }
}

#[derive(WidgetCommon)]
pub struct Diary<'a> {
    show: &'a Show,
    _client: &'a Client,
    global_state: &'a GlobalState,
    skill_set: &'a SkillSet,
    active_abilities: &'a ActiveAbilities,
    inventory: &'a Inventory,
    health: &'a Health,
    energy: &'a Energy,
    poise: &'a Poise,
    body: &'a Body,
    msm: &'a MaterialStatManifest,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    slot_manager: &'a mut SlotManager,
    pulse: f32,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    created_btns_top_l: usize,
    created_btns_top_r: usize,
    created_btns_bot_l: usize,
    created_btns_bot_r: usize,
}

pub struct DiaryShow {
    pub skilltreetab: SelectedSkillTree,
    pub section: DiarySection,
}

impl Default for DiaryShow {
    fn default() -> Self {
        Self {
            skilltreetab: SelectedSkillTree::General,
            section: DiarySection::SkillTrees,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> Diary<'a> {
    pub fn new(
        show: &'a Show,
        _client: &'a Client,
        global_state: &'a GlobalState,
        skill_set: &'a SkillSet,
        active_abilities: &'a ActiveAbilities,
        inventory: &'a Inventory,
        health: &'a Health,
        energy: &'a Energy,
        poise: &'a Poise,
        body: &'a Body,
        msm: &'a MaterialStatManifest,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        slot_manager: &'a mut SlotManager,
        pulse: f32,
    ) -> Self {
        Self {
            show,
            _client,
            global_state,
            skill_set,
            active_abilities,
            inventory,
            health,
            energy,
            poise,
            body,
            msm,
            imgs,
            item_imgs,
            fonts,
            localized_strings,
            rot_imgs,
            tooltip_manager,
            slot_manager,
            pulse,
            common: widget::CommonBuilder::default(),
            created_btns_top_l: 0,
            created_btns_top_r: 0,
            created_btns_bot_l: 0,
            created_btns_bot_r: 0,
        }
    }
}

pub type SelectedSkillTree = SkillGroupKind;

// TODO: make it enum?
const TREES: [&str; 8] = [
    "General Combat",
    "Sword",
    "Hammer",
    "Axe",
    "Sceptre",
    "Bow",
    "Fire Staff",
    "Mining",
];

// Possible future sections: Bestiary ("Pokedex" of fought enemies), Weapon and
// armour catalogue, Achievements...
const SECTIONS: [&str; 3] = ["Skill-Trees", "Abilities", "Stats"];

pub enum Event {
    Close,
    ChangeSkillTree(SelectedSkillTree),
    UnlockSkill(Skill),
    ChangeSection(DiarySection),
}

#[derive(PartialEq)]
pub enum DiarySection {
    SkillTrees,
    AbilitySelection,
    Stats,
}

pub struct DiaryState {
    ids: Ids,
    ability_page: usize,
}

impl<'a> Widget for Diary<'a> {
    type Event = Vec<Event>;
    type State = DiaryState;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        DiaryState {
            ids: Ids::new(id_gen),
            ability_page: 0,
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(mut self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Diary::update");
        let widget::UpdateArgs { state, ui, .. } = args;
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

        //Animation timer Frame
        let frame_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8;

        Image::new(self.imgs.diary_bg)
            .w_h(1202.0, 886.0)
            .mid_top_with_margin_on(ui.window, 5.0)
            .color(Some(UI_MAIN))
            .set(state.ids.bg, ui);

        Image::new(self.imgs.diary_frame)
            .w_h(1202.0, 886.0)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.frame, ui);

        // Icon
        Image::new(self.imgs.spellbook_button)
            .w_h(30.0, 27.0)
            .top_left_with_margins_on(state.ids.frame, 8.0, 8.0)
            .set(state.ids.icon, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.ids.frame, 0.0, 0.0)
            .set(state.ids.close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // Title
        Text::new(self.localized_strings.get("hud.diary"))
            .mid_top_with_margin_on(state.ids.frame, 3.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(29))
            .color(TEXT_COLOR)
            .set(state.ids.title, ui);

        // Content Alignment
        Rectangle::fill_with([599.0 * 2.0, 419.0 * 2.0], color::TRANSPARENT)
            .mid_top_with_margin_on(state.ids.frame, 46.0)
            .set(state.ids.content_align, ui);

        // Contents
        // Section buttons
        let sel_section = &self.show.diary_fields.section;

        // Update len
        state.update(|s| {
            s.ids
                .section_imgs
                .resize(SECTIONS.len(), &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.ids
                .section_btns
                .resize(SECTIONS.len(), &mut ui.widget_id_generator())
        });
        for (i, section_name) in SECTIONS.iter().copied().enumerate() {
            let section = match section_from_str(section_name) {
                Some(st) => st,
                None => {
                    tracing::warn!("unexpected section name: {}", section_name);
                    continue;
                },
            };
            // Section Icons
            let section_desc = match section_name {
                "Abilities" => "List of your currently available abilities.",
                "Skill-Trees" => "",
                "Stats" => "",
                _ => "",
            };
            let btn_img = {
                let img = match section_name {
                    "Abilities" => self.imgs.spellbook_ico,
                    "Skill-Trees" => self.imgs.skilltree_ico,
                    "Stats" => self.imgs.stats_ico,
                    _ => self.imgs.nothing,
                };
                if i == 0 {
                    Image::new(img).top_left_with_margins_on(state.ids.content_align, 0.0, -50.0)
                } else {
                    Image::new(img).down_from(state.ids.section_btns[i - 1], 5.0)
                }
            };
            btn_img.w_h(50.0, 50.0).set(state.ids.section_imgs[i], ui);
            // Section Buttons
            let border_image = if section == *sel_section {
                self.imgs.wpn_icon_border_pressed
            } else {
                self.imgs.wpn_icon_border
            };

            let hover_image = if section == *sel_section {
                self.imgs.wpn_icon_border_pressed
            } else {
                self.imgs.wpn_icon_border_mo
            };

            let press_image = if section == *sel_section {
                self.imgs.wpn_icon_border_pressed
            } else {
                self.imgs.wpn_icon_border_press
            };
            let section_buttons = Button::image(border_image)
                .w_h(50.0, 50.0)
                .hover_image(hover_image)
                .press_image(press_image)
                .middle_of(state.ids.section_imgs[i])
                .with_tooltip(
                    self.tooltip_manager,
                    section_name,
                    section_desc,
                    &diary_tooltip,
                    TEXT_COLOR,
                )
                .set(state.ids.section_btns[i], ui);
            if section_buttons.was_clicked() {
                events.push(Event::ChangeSection(section))
            }
        }
        match self.show.diary_fields.section {
            DiarySection::SkillTrees => {
                // Skill Trees
                let sel_tab = &self.show.diary_fields.skilltreetab;

                // Skill Tree Selection
                state.update(|s| {
                    s.ids
                        .weapon_btns
                        .resize(TREES.len(), &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .weapon_imgs
                        .resize(TREES.len(), &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .lock_imgs
                        .resize(TREES.len(), &mut ui.widget_id_generator())
                });

                // Draw skillgroup tab's icons
                for (i, skilltree_name) in TREES.iter().copied().enumerate() {
                    let skill_group = match skill_tree_from_str(skilltree_name) {
                        Some(st) => st,
                        None => {
                            tracing::warn!("unexpected tree name: {}", skilltree_name);
                            continue;
                        },
                    };

                    // Check if we have this skill tree unlocked
                    let locked = !self.skill_set.skill_group_accessible(skill_group);

                    // Weapon button image
                    let btn_img = {
                        let img = match skilltree_name {
                            "General Combat" => self.imgs.swords_crossed,
                            "Sword" => self.imgs.sword,
                            "Hammer" => self.imgs.hammer,
                            "Axe" => self.imgs.axe,
                            "Sceptre" => self.imgs.sceptre,
                            "Bow" => self.imgs.bow,
                            "Fire Staff" => self.imgs.staff,
                            "Mining" => self.imgs.mining,
                            _ => self.imgs.nothing,
                        };

                        if i == 0 {
                            Image::new(img).top_left_with_margins_on(
                                state.ids.content_align,
                                10.0,
                                5.0,
                            )
                        } else {
                            Image::new(img).down_from(state.ids.weapon_btns[i - 1], 5.0)
                        }
                    };
                    btn_img.w_h(50.0, 50.0).set(state.ids.weapon_imgs[i], ui);

                    // Lock Image
                    if locked {
                        Image::new(self.imgs.lock)
                            .w_h(50.0, 50.0)
                            .middle_of(state.ids.weapon_imgs[i])
                            .graphics_for(state.ids.weapon_imgs[i])
                            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.8)))
                            .set(state.ids.lock_imgs[i], ui);
                    }

                    // Weapon icons
                    let have_points = {
                        let available = self.skill_set.available_sp(skill_group);
                        let earned = self.skill_set.earned_sp(skill_group);
                        let total_cost = skill_group.total_skill_point_cost();

                        available > 0 && (earned - available) < total_cost
                    };

                    let border_image = if skill_group == *sel_tab || have_points {
                        self.imgs.wpn_icon_border_pressed
                    } else {
                        self.imgs.wpn_icon_border
                    };

                    let hover_image = if skill_group == *sel_tab {
                        self.imgs.wpn_icon_border_pressed
                    } else {
                        self.imgs.wpn_icon_border_mo
                    };

                    let press_image = if skill_group == *sel_tab {
                        self.imgs.wpn_icon_border_pressed
                    } else {
                        self.imgs.wpn_icon_border_press
                    };

                    let color = if skill_group != *sel_tab && have_points {
                        Color::Rgba(0.92, 0.76, 0.0, frame_ani)
                    } else {
                        TEXT_COLOR
                    };

                    let tooltip_txt = if locked {
                        self.localized_strings.get("hud.skill.not_unlocked")
                    } else {
                        ""
                    };

                    let wpn_button = Button::image(border_image)
                        .w_h(50.0, 50.0)
                        .hover_image(hover_image)
                        .press_image(press_image)
                        .middle_of(state.ids.weapon_imgs[i])
                        .image_color(color)
                        .with_tooltip(
                            self.tooltip_manager,
                            skilltree_name,
                            tooltip_txt,
                            &diary_tooltip,
                            TEXT_COLOR,
                        )
                        .set(state.ids.weapon_btns[i], ui);
                    if wpn_button.was_clicked() {
                        events.push(Event::ChangeSkillTree(skill_group))
                    }
                }

                // Exp Bars and Rank Display
                let current_exp = self.skill_set.available_experience(*sel_tab) as f64;
                let max_exp = self.skill_set.skill_point_cost(*sel_tab) as f64;
                let exp_percentage = current_exp / max_exp;
                let rank = self.skill_set.earned_sp(*sel_tab);
                let rank_txt = format!("{}", rank);
                let exp_txt = format!("{}/{}", current_exp, max_exp);
                let available_pts = self.skill_set.available_sp(*sel_tab);
                let available_pts_txt = format!("{}", available_pts);
                Image::new(self.imgs.diary_exp_bg)
                    .w_h(480.0, 76.0)
                    .mid_bottom_with_margin_on(state.ids.content_align, 10.0)
                    .set(state.ids.exp_bar_bg, ui);
                Rectangle::fill_with([400.0, 40.0], color::TRANSPARENT)
                    .top_left_with_margins_on(state.ids.exp_bar_bg, 32.0, 40.0)
                    .set(state.ids.exp_bar_content_align, ui);
                Image::new(self.imgs.bar_content)
                    .w_h(400.0 * exp_percentage, 40.0)
                    .top_left_with_margins_on(state.ids.exp_bar_content_align, 0.0, 0.0)
                    .color(Some(XP_COLOR))
                    .set(state.ids.exp_bar_content, ui);
                Image::new(self.imgs.diary_exp_frame)
                    .w_h(480.0, 76.0)
                    .color(Some(UI_HIGHLIGHT_0))
                    .middle_of(state.ids.exp_bar_bg)
                    .set(state.ids.exp_bar_frame, ui);
                // Show EXP bar text on hover
                if ui
                    .widget_input(state.ids.exp_bar_frame)
                    .mouse()
                    .map_or(false, |m| m.is_over())
                {
                    Text::new(&exp_txt)
                        .mid_top_with_margin_on(state.ids.exp_bar_frame, 47.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(TEXT_COLOR)
                        .graphics_for(state.ids.exp_bar_frame)
                        .set(state.ids.exp_bar_txt, ui);
                }
                Text::new(&rank_txt)
                    .mid_top_with_margin_on(state.ids.exp_bar_frame, match rank {
                        0..=99 => 5.0,
                        100..=999 => 8.0,
                        _ => 10.0,
                    })
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(match rank {
                        0..=99 => 28,
                        100..=999 => 21,
                        _ => 15,
                    }))
                    .color(TEXT_COLOR)
                    .set(state.ids.exp_bar_rank, ui);

                Text::new(
                    &self
                        .localized_strings
                        .get("hud.skill.sp_available")
                        .replace("{number}", &available_pts_txt),
                )
                .mid_top_with_margin_on(state.ids.content_align, 700.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(28))
                .color(if available_pts > 0 {
                    Color::Rgba(0.92, 0.76, 0.0, frame_ani)
                } else {
                    TEXT_COLOR
                })
                .set(state.ids.available_pts_txt, ui);
                // Skill Trees
                // Alignment Placing
                let x = 200.0;
                let y = 100.0;
                // Alignment rectangles for skills
                Rectangle::fill_with([124.0 * 2.0, 124.0 * 2.0], color::TRANSPARENT)
                    .top_left_with_margins_on(state.ids.content_align, y, x)
                    .set(state.ids.skills_top_l_align, ui);
                Rectangle::fill_with([124.0 * 2.0, 124.0 * 2.0], color::TRANSPARENT)
                    .top_right_with_margins_on(state.ids.content_align, y, x)
                    .set(state.ids.skills_top_r_align, ui);
                Rectangle::fill_with([124.0 * 2.0, 124.0 * 2.0], color::TRANSPARENT)
                    .bottom_left_with_margins_on(state.ids.content_align, y, x)
                    .set(state.ids.skills_bot_l_align, ui);
                Rectangle::fill_with([124.0 * 2.0, 124.0 * 2.0], color::TRANSPARENT)
                    .bottom_right_with_margins_on(state.ids.content_align, y, x)
                    .set(state.ids.skills_bot_r_align, ui);

                match sel_tab {
                    SelectedSkillTree::General => {
                        self.handle_general_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Sword) => {
                        self.handle_sword_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Hammer) => {
                        self.handle_hammer_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Axe) => {
                        self.handle_axe_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Sceptre) => {
                        self.handle_sceptre_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Bow) => {
                        self.handle_bow_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Staff) => {
                        self.handle_staff_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Pick) => {
                        self.handle_mining_skills_window(&diary_tooltip, state, ui, events)
                    },
                    _ => events,
                }
            },
            DiarySection::AbilitySelection => {
                use comp::ability::AbilityInput;

                // Background Art
                Image::new(self.imgs.book_bg)
                    .w_h(299.0 * 4.0, 184.0 * 4.0)
                    .mid_top_with_margin_on(state.ids.content_align, 4.0)
                    //.graphics_for(state.ids.content_align)
                    .set(state.ids.spellbook_art, ui);
                Image::new(self.imgs.skills_bg)
                    .w_h(240.0 * 2.0, 40.0 * 2.0)
                    .mid_bottom_with_margin_on(state.ids.content_align, 8.0)
                    .set(state.ids.spellbook_skills_bg, ui);

                Rectangle::fill_with([299.0 * 2.0, 184.0 * 4.0], color::TRANSPARENT)
                    .top_left_with_margins_on(state.ids.spellbook_art, 0.0, 0.0)
                    .set(state.ids.sb_page_left_align, ui);
                Rectangle::fill_with([299.0 * 2.0, 184.0 * 4.0], color::TRANSPARENT)
                    .top_right_with_margins_on(state.ids.spellbook_art, 0.0, 0.0)
                    .set(state.ids.sb_page_right_align, ui);

                // Display all active abilities on bottom of window
                state.update(|s| {
                    s.ids
                        .active_abilities
                        .resize(MAX_ABILITIES, &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .active_abilities_keys
                        .resize(MAX_ABILITIES, &mut ui.widget_id_generator())
                });

                let mut slot_maker = SlotMaker {
                    empty_slot: self.imgs.inv_slot,
                    filled_slot: self.imgs.inv_slot,
                    selected_slot: self.imgs.inv_slot_sel,
                    background_color: Some(UI_MAIN),
                    content_size: ContentSize {
                        width_height_ratio: 1.0,
                        max_fraction: 0.9,
                    },
                    selected_content_scale: 1.067,
                    amount_font: self.fonts.cyri.conrod_id,
                    amount_margins: Vec2::new(-4.0, 0.0),
                    amount_font_size: self.fonts.cyri.scale(12),
                    amount_text_color: TEXT_COLOR,
                    content_source: &(self.active_abilities, self.inventory, self.skill_set),
                    image_source: self.imgs,
                    slot_manager: Some(self.slot_manager),
                    pulse: 0.0,
                };

                for i in 0..MAX_ABILITIES {
                    let ability_id = self
                        .active_abilities
                        .get_ability(
                            AbilityInput::Auxiliary(i),
                            Some(self.inventory),
                            Some(self.skill_set),
                        )
                        .ability_id(Some(self.inventory));
                    let (ability_title, ability_desc) = if let Some(ability_id) = ability_id {
                        util::ability_description(ability_id)
                    } else {
                        ("Drag an ability here to use it.", "")
                    };

                    let image_size = 80.0;
                    let image_offsets = 92.0 * i as f64;

                    let slot = AbilitySlot::Slot(i);
                    let mut ability_slot = slot_maker.fabricate(slot, [image_size; 2]);

                    if i == 0 {
                        ability_slot = ability_slot.top_left_with_margins_on(
                            state.ids.spellbook_skills_bg,
                            0.0,
                            32.0 + image_offsets,
                        );
                    } else {
                        ability_slot =
                            ability_slot.right_from(state.ids.active_abilities[i - 1], 4.0)
                    }
                    ability_slot
                        .with_tooltip(
                            self.tooltip_manager,
                            ability_title,
                            ability_desc,
                            &diary_tooltip,
                            TEXT_COLOR,
                        )
                        .set(state.ids.active_abilities[i], ui);

                    // Display Slot Keybinding
                    let keys = &self.global_state.settings.controls;
                    let key_layout = &self.global_state.window.key_layout;
                    let ability_key = [
                        GameInput::Slot1,
                        GameInput::Slot2,
                        GameInput::Slot3,
                        GameInput::Slot4,
                        GameInput::Slot5,
                    ]
                    .get(i)
                    .and_then(|input| keys.get_binding(*input))
                    .map(|key| key.display_string(key_layout))
                    .unwrap_or_default();

                    Text::new(&ability_key)
                        .top_left_with_margins_on(state.ids.active_abilities[i], 0.0, 4.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(20))
                        .color(TEXT_COLOR)
                        .graphics_for(state.ids.active_abilities[i])
                        .set(state.ids.active_abilities_keys[i], ui);
                }

                let same_weap_kinds = self
                    .inventory
                    .equipped(EquipSlot::ActiveMainhand)
                    .zip(self.inventory.equipped(EquipSlot::ActiveOffhand))
                    .map_or(false, |(a, b)| {
                        if let (ItemKind::Tool(tool_a), ItemKind::Tool(tool_b)) = (&a.kind, &b.kind)
                        {
                            (a.ability_spec(), tool_a.kind) == (b.ability_spec(), tool_b.kind)
                        } else {
                            false
                        }
                    });

                let main_weap_abilities = ActiveAbilities::iter_unlocked_abilities(
                    Some(self.inventory),
                    Some(self.skill_set),
                    EquipSlot::ActiveMainhand,
                )
                .map(AuxiliaryAbility::MainWeapon)
                .map(|a| (Ability::from(a).ability_id(Some(self.inventory)), a));
                let off_weap_abilities = ActiveAbilities::iter_unlocked_abilities(
                    Some(self.inventory),
                    Some(self.skill_set),
                    EquipSlot::ActiveOffhand,
                )
                .map(AuxiliaryAbility::OffWeapon)
                .map(|a| (Ability::from(a).ability_id(Some(self.inventory)), a));

                let abilities: Vec<_> = if same_weap_kinds {
                    // When the weapons have the same ability kind take only the main weapons,
                    main_weap_abilities.collect()
                } else {
                    main_weap_abilities.chain(off_weap_abilities).collect()
                };

                const ABILITIES_PER_PAGE: usize = 12;

                let page_indices = (abilities.len().saturating_sub(1)) / ABILITIES_PER_PAGE;

                if state.ability_page > page_indices {
                    state.update(|s| s.ability_page = 0);
                }

                state.update(|s| {
                    s.ids
                        .abilities
                        .resize(ABILITIES_PER_PAGE, &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .abilities_dual
                        .resize(ABILITIES_PER_PAGE, &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .ability_titles
                        .resize(ABILITIES_PER_PAGE, &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .ability_frames
                        .resize(ABILITIES_PER_PAGE, &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .ability_descs
                        .resize(ABILITIES_PER_PAGE, &mut ui.widget_id_generator())
                });

                // Page button
                // Left Arrow
                let left_arrow = Button::image(if state.ability_page > 0 {
                    self.imgs.arrow_l
                } else {
                    self.imgs.arrow_l_inactive
                })
                .bottom_left_with_margins_on(state.ids.spellbook_art, -83.0, 10.0)
                .w_h(48.0, 55.0);
                // Grey out arrows when inactive
                if state.ability_page > 0 {
                    if left_arrow
                        .hover_image(self.imgs.arrow_l_click)
                        .press_image(self.imgs.arrow_l)
                        .set(state.ids.ability_page_left, ui)
                        .was_clicked()
                    {
                        state.update(|s| s.ability_page -= 1);
                    }
                } else {
                    left_arrow.set(state.ids.ability_page_left, ui);
                }
                // Right Arrow
                let right_arrow = Button::image(if state.ability_page < page_indices {
                    self.imgs.arrow_r
                } else {
                    self.imgs.arrow_r_inactive
                })
                .bottom_right_with_margins_on(state.ids.spellbook_art, -83.0, 10.0)
                .w_h(48.0, 55.0);
                if state.ability_page < page_indices {
                    // Only show right button if not on last page
                    if right_arrow
                        .hover_image(self.imgs.arrow_r_click)
                        .press_image(self.imgs.arrow_r)
                        .set(state.ids.ability_page_right, ui)
                        .was_clicked()
                    {
                        state.update(|s| s.ability_page += 1);
                    };
                } else {
                    right_arrow.set(state.ids.ability_page_right, ui);
                }

                let ability_start = state.ability_page * ABILITIES_PER_PAGE;

                let mut slot_maker = SlotMaker {
                    empty_slot: self.imgs.inv_slot,
                    filled_slot: self.imgs.inv_slot,
                    selected_slot: self.imgs.inv_slot_sel,
                    background_color: Some(UI_MAIN),
                    content_size: ContentSize {
                        width_height_ratio: 1.0,
                        max_fraction: 1.0,
                    },
                    selected_content_scale: 1.067,
                    amount_font: self.fonts.cyri.conrod_id,
                    amount_margins: Vec2::new(-4.0, 0.0),
                    amount_font_size: self.fonts.cyri.scale(12),
                    amount_text_color: TEXT_COLOR,
                    content_source: &(self.active_abilities, self.inventory, self.skill_set),
                    image_source: self.imgs,
                    slot_manager: Some(self.slot_manager),
                    pulse: 0.0,
                };

                for (id_index, (ability_id, ability)) in abilities
                    .iter()
                    .skip(ability_start)
                    .take(ABILITIES_PER_PAGE)
                    .enumerate()
                {
                    let (ability_title, ability_desc) =
                        util::ability_description(ability_id.unwrap_or(""));

                    let (align_state, image_offsets) = if id_index < 6 {
                        (state.ids.sb_page_left_align, 120.0 * id_index as f64)
                    } else {
                        (state.ids.sb_page_right_align, 120.0 * (id_index - 6) as f64)
                    };

                    Image::new(if same_weap_kinds {
                        self.imgs.ability_frame_dual
                    } else {
                        self.imgs.ability_frame
                    })
                    .w_h(566.0, 108.0)
                    .top_left_with_margins_on(align_state, 16.0 + image_offsets, 16.0)
                    .color(Some(UI_HIGHLIGHT_0))
                    .set(state.ids.ability_frames[id_index], ui);

                    let slot = AbilitySlot::Ability(*ability);
                    slot_maker
                        .fabricate(slot, [100.0; 2])
                        .top_left_with_margins_on(align_state, 20.0 + image_offsets, 20.0)
                        .set(state.ids.abilities[id_index], ui);

                    if same_weap_kinds {
                        if let AuxiliaryAbility::MainWeapon(slot) = ability {
                            let ability = AuxiliaryAbility::OffWeapon(*slot);

                            let slot = AbilitySlot::Ability(ability);
                            slot_maker
                                .fabricate(slot, [100.0; 2])
                                .top_right_with_margins_on(align_state, 20.0 + image_offsets, 20.0)
                                .set(state.ids.abilities_dual[id_index], ui);
                        }
                    }
                    // The page width...
                    let text_width = 299.0 * 2.0
                        - if same_weap_kinds && matches!(ability, AuxiliaryAbility::MainWeapon(_)) {
                            // with double the width of an ability image and some padding subtracted
                            // if dual wielding two of the same weapon kind
                            (20.0 + 100.0 + 10.0) * 2.0
                        } else {
                            // or the width of an ability image and some padding subtracted
                            // otherwise
                            20.0 * 2.0 + 100.0
                        };
                    Text::new(ability_title)
                        .top_left_with_margins_on(state.ids.abilities[id_index], 5.0, 110.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(28))
                        .color(TEXT_COLOR)
                        .w(text_width)
                        .graphics_for(state.ids.abilities[id_index])
                        .set(state.ids.ability_titles[id_index], ui);
                    Text::new(ability_desc)
                        .top_left_with_margins_on(state.ids.abilities[id_index], 40.0, 110.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(18))
                        .color(TEXT_COLOR)
                        .w(text_width)
                        .graphics_for(state.ids.abilities[id_index])
                        .set(state.ids.ability_descs[id_index], ui);
                }

                events
            },
            DiarySection::Stats => {
                const STATS: [&str; 13] = [
                    "Hitpoints",
                    "Energy",
                    "Poise",
                    "Combat-Rating",
                    "Protection",
                    "Stun-Resistance",
                    "Crit-Power",
                    "Energy Reward",
                    "Stealth",
                    "Weapon Power",
                    "Weapon Speed",
                    "Weapon Poise",
                    "Weapon Crit-Chance",
                ];

                // Background Art
                Image::new(self.imgs.book_bg)
                    .w_h(299.0 * 4.0, 184.0 * 4.0)
                    .mid_top_with_margin_on(state.ids.content_align, 4.0)
                    .set(state.ids.spellbook_art, ui);

                state.update(|s| {
                    s.ids
                        .stat_names
                        .resize(STATS.len(), &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .stat_values
                        .resize(STATS.len(), &mut ui.widget_id_generator())
                });
                for (i, stat) in STATS.iter().copied().enumerate() {
                    // Stat names
                    let mut txt = Text::new(stat)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(29))
                        .color(BLACK);

                    if i == 0 {
                        txt = txt.top_left_with_margins_on(state.ids.spellbook_art, 20.0, 20.0);
                    } else {
                        txt = txt.down_from(state.ids.stat_names[i - 1], 10.0);
                    };
                    txt.set(state.ids.stat_names[i], ui);

                    let main_weap_stats = self
                        .inventory
                        .equipped(EquipSlot::ActiveMainhand)
                        .and_then(|item| match &item.kind {
                            ItemKind::Tool(tool) => {
                                Some(tool.stats.resolve_stats(self.msm, item.components()))
                            },
                            _ => None,
                        });

                    let off_weap_stats = self
                        .inventory
                        .equipped(EquipSlot::ActiveOffhand)
                        .and_then(|item| match &item.kind {
                            ItemKind::Tool(tool) => {
                                Some(tool.stats.resolve_stats(self.msm, item.components()))
                            },
                            _ => None,
                        });

                    // Stat values
                    let value = match stat {
                        "Hitpoints" => format!("{}", self.health.base_max() as u32),
                        "Energy" => format!("{}", self.energy.base_max() as u32),
                        "Poise" => format!("{}", self.poise.base_max() as u32),
                        "Combat-Rating" => {
                            let cr = combat::combat_rating(
                                self.inventory,
                                self.health,
                                self.energy,
                                self.poise,
                                self.skill_set,
                                *self.body,
                                self.msm,
                            );
                            format!("{:.2}", cr * 10.0)
                        },
                        "Protection" => {
                            let protection = combat::compute_protection(Some(self.inventory));
                            match protection {
                                Some(prot) => format!("{}", prot),
                                None => String::from("Invincible"),
                            }
                        },
                        "Stun-Resistance" => {
                            let stun_res = Poise::compute_poise_damage_reduction(self.inventory);
                            format!("{:.2}%", stun_res * 100.0)
                        },
                        "Crit-Power" => {
                            let critpwr = combat::compute_crit_mult(Some(self.inventory));
                            format!("x{:.2}", critpwr)
                        },
                        "Energy Reward" => {
                            let energy_rew =
                                combat::compute_energy_reward_mod(Some(self.inventory));
                            format!("{:+.0}%", (energy_rew - 1.0) * 100.0)
                        },
                        "Stealth" => {
                            let stealth = combat::compute_stealth_coefficient(Some(self.inventory));
                            format!("{:.2}", stealth)
                        },
                        "Weapon Power" => match (main_weap_stats, off_weap_stats) {
                            (Some(m_stats), Some(o_stats)) => {
                                format!("{}   {}", m_stats.power * 10.0, o_stats.power * 10.0)
                            },
                            (Some(stats), None) | (None, Some(stats)) => {
                                format!("{}", stats.power * 10.0)
                            },
                            (None, None) => String::new(),
                        },
                        "Weapon Speed" => {
                            let spd_fmt = |sp| (sp - 1.0) * 100.0;
                            match (main_weap_stats, off_weap_stats) {
                                (Some(m_stats), Some(o_stats)) => format!(
                                    "{:+.0}%   {:+.0}%",
                                    spd_fmt(m_stats.speed),
                                    spd_fmt(o_stats.speed)
                                ),
                                (Some(stats), None) | (None, Some(stats)) => {
                                    format!("{:+.0}%", spd_fmt(stats.speed))
                                },
                                _ => String::new(),
                            }
                        },
                        "Weapon Poise" => match (main_weap_stats, off_weap_stats) {
                            (Some(m_stats), Some(o_stats)) => {
                                format!(
                                    "{}   {}",
                                    m_stats.effect_power * 10.0,
                                    o_stats.effect_power * 10.0
                                )
                            },
                            (Some(stats), None) | (None, Some(stats)) => {
                                format!("{}", stats.effect_power * 10.0)
                            },
                            (None, None) => String::new(),
                        },
                        "Weapon Crit-Chance" => {
                            let crit_fmt = |cc| cc * 100.0;
                            match (main_weap_stats, off_weap_stats) {
                                (Some(m_stats), Some(o_stats)) => format!(
                                    "{:.1}%   {:.1}%",
                                    crit_fmt(m_stats.crit_chance),
                                    crit_fmt(o_stats.crit_chance)
                                ),
                                (Some(stats), None) | (None, Some(stats)) => {
                                    format!("{:.1}%", crit_fmt(stats.crit_chance))
                                },
                                (None, None) => String::new(),
                            }
                        },
                        unknown => unreachable!("{}", unknown),
                    };

                    let mut number = Text::new(&value)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(29))
                        .color(BLACK);

                    if i == 0 {
                        number = number.right_from(state.ids.stat_names[i], 265.0);
                    } else {
                        number = number.down_from(state.ids.stat_values[i - 1], 10.0);
                    };
                    number.set(state.ids.stat_values[i], ui);
                }

                events
            },
        }
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
        "Mining" => Some(SelectedSkillTree::Weapon(ToolKind::Pick)),
        _ => None,
    }
}

fn section_from_str(string: &str) -> Option<DiarySection> {
    match string {
        "Abilities" => Some(DiarySection::AbilitySelection),
        "Skill-Trees" => Some(DiarySection::SkillTrees),
        "Stats" => Some(DiarySection::Stats),
        _ => None,
    }
}

enum SkillIcon<'a> {
    Unlockable {
        skill: Skill,
        image: image::Id,
        position: PositionSpecifier,
        id: widget::Id,
    },
    Descriptive {
        title: &'a str,
        desc: &'a str,
        image: image::Id,
        position: PositionSpecifier,
        id: widget::Id,
    },
}

impl<'a> Diary<'a> {
    fn handle_general_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        let tree_title = self.localized_strings.get("common.weapons.general");
        Text::new(tree_title)
            .mid_top_with_margin_on(state.ids.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.ids.tree_title_txt, ui);

        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = 2;
        let skills_top_r = 6;
        let skills_bot_l = 4;
        let skills_bot_r = 5;

        self.setup_state_for_skill_icons(
            state,
            ui,
            skills_top_l,
            skills_top_r,
            skills_bot_l,
            skills_bot_r,
        );
        use skills::{GeneralSkill::*, RollSkill::*};
        use SkillGroupKind::*;
        use ToolKind::*;
        // General Combat
        Image::new(animate_by_pulse(
            &self
                .item_imgs
                .img_ids_or_not_found_img(ItemKey::Tool("example_general_combat_left".to_string())),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.content_align)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
        .set(state.ids.general_combat_render_0, ui);

        Image::new(animate_by_pulse(
            &self.item_imgs.img_ids_or_not_found_img(ItemKey::Tool(
                "example_general_combat_right".to_string(),
            )),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.general_combat_render_0)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
        .set(state.ids.general_combat_render_1, ui);

        use PositionSpecifier::MidTopWithMarginOn;
        let skill_buttons = &[
            // Top Left skills
            //        5 1 6
            //        3 0 4
            //        8 2 7
            SkillIcon::Unlockable {
                skill: Skill::General(HealthIncrease),
                image: self.imgs.health_plus_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[0], 3.0),
                id: state.ids.skill_general_stat_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::General(EnergyIncrease),
                image: self.imgs.energy_plus_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[1], 3.0),
                id: state.ids.skill_general_stat_1,
            },
            // Top right skills
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Sword)),
                image: self.imgs.unlock_sword_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[0], 3.0),
                id: state.ids.skill_general_tree_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Axe)),
                image: self.imgs.unlock_axe_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[1], 3.0),
                id: state.ids.skill_general_tree_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Hammer)),
                image: self.imgs.unlock_hammer_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[2], 3.0),
                id: state.ids.skill_general_tree_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Bow)),
                image: self.imgs.unlock_bow_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[3], 3.0),
                id: state.ids.skill_general_tree_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Staff)),
                image: self.imgs.unlock_staff_skill0,
                position: MidTopWithMarginOn(state.ids.skills_top_r[4], 3.0),
                id: state.ids.skill_general_tree_4,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Sceptre)),
                image: self.imgs.unlock_sceptre_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[5], 3.0),
                id: state.ids.skill_general_tree_5,
            },
            // Bottom left skills
            SkillIcon::Descriptive {
                title: "hud.skill.dodge_title",
                desc: "hud.skill.dodge",
                image: self.imgs.skill_dodge_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[0], 3.0),
                id: state.ids.skill_general_roll_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Roll(RollSkill::Cost),
                image: self.imgs.utility_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[1], 3.0),
                id: state.ids.skill_general_roll_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Roll(Strength),
                image: self.imgs.utility_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[2], 3.0),
                id: state.ids.skill_general_roll_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Roll(Duration),
                image: self.imgs.utility_duration_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[3], 3.0),
                id: state.ids.skill_general_roll_3,
            },
            // Bottom right skills
            SkillIcon::Descriptive {
                title: "hud.skill.climbing_title",
                desc: "hud.skill.climbing",
                image: self.imgs.skill_climbing_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_r[0], 3.0),
                id: state.ids.skill_general_climb_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Climb(ClimbSkill::Cost),
                image: self.imgs.utility_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_r[1], 3.0),
                id: state.ids.skill_general_climb_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Climb(ClimbSkill::Speed),
                image: self.imgs.utility_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_r[2], 3.0),
                id: state.ids.skill_general_climb_2,
            },
            SkillIcon::Descriptive {
                title: "hud.skill.swim_title",
                desc: "hud.skill.swim",
                image: self.imgs.skill_swim_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_r[3], 3.0),
                id: state.ids.skill_general_swim_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Swim(SwimSkill::Speed),
                image: self.imgs.utility_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_r[4], 3.0),
                id: state.ids.skill_general_swim_1,
            },
        ];

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip);
        events
    }

    fn handle_sword_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        // Title text
        let tree_title = self.localized_strings.get("common.weapons.sword");

        Text::new(tree_title)
            .mid_top_with_margin_on(state.ids.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.ids.tree_title_txt, ui);

        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = 5;
        let skills_top_r = 7;
        let skills_bot_l = 5;
        let skills_bot_r = 1;

        self.setup_state_for_skill_icons(
            state,
            ui,
            skills_top_l,
            skills_top_r,
            skills_bot_l,
            skills_bot_r,
        );

        // Skill icons and buttons
        use skills::SwordSkill::*;
        // Sword
        Image::new(animate_by_pulse(
            &self
                .item_imgs
                .img_ids_or_not_found_img(ItemKey::Tool("example_sword".to_string())),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.content_align)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
        .set(state.ids.sword_render, ui);
        use PositionSpecifier::MidTopWithMarginOn;
        let skill_buttons = &[
            // Top Left skills
            //        5 1 6
            //        3 0 4
            //        8 2 7
            SkillIcon::Descriptive {
                title: "hud.skill.sw_trip_str_title",
                desc: "hud.skill.sw_trip_str",
                image: self.imgs.twohsword_m1,
                position: MidTopWithMarginOn(state.ids.skills_top_l[0], 3.0),
                id: state.ids.skill_sword_combo_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(TsCombo),
                image: self.imgs.physical_combo_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[1], 3.0),
                id: state.ids.skill_sword_combo_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(TsDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[2], 3.0),
                id: state.ids.skill_sword_combo_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(TsSpeed),
                image: self.imgs.physical_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[3], 3.0),
                id: state.ids.skill_sword_combo_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(TsRegen),
                image: self.imgs.physical_energy_regen_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[4], 3.0),
                id: state.ids.skill_sword_combo_4,
            },
            // Top right skills
            SkillIcon::Descriptive {
                title: "hud.skill.sw_dash_title",
                desc: "hud.skill.sw_dash",
                image: self.imgs.twohsword_m2,
                position: MidTopWithMarginOn(state.ids.skills_top_r[0], 3.0),
                id: state.ids.skill_sword_dash_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(DDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[1], 3.0),
                id: state.ids.skill_sword_dash_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(DDrain),
                image: self.imgs.physical_energy_drain_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[2], 3.0),
                id: state.ids.skill_sword_dash_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(DCost),
                image: self.imgs.physical_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[3], 3.0),
                id: state.ids.skill_sword_dash_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(DSpeed),
                image: self.imgs.physical_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[4], 3.0),
                id: state.ids.skill_sword_dash_4,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(DChargeThrough),
                image: self.imgs.physical_distance_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[5], 3.0),
                id: state.ids.skill_sword_dash_5,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(DScaling),
                image: self.imgs.physical_amount_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[6], 3.0),
                id: state.ids.skill_sword_dash_6,
            },
            // Bottom left skills
            SkillIcon::Unlockable {
                skill: Skill::Sword(UnlockSpin),
                image: self.imgs.sword_whirlwind,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[0], 3.0),
                id: state.ids.skill_sword_spin_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(SDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[1], 3.0),
                id: state.ids.skill_sword_spin_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(SSpeed),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[2], 3.0),
                id: state.ids.skill_sword_spin_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(SCost),
                image: self.imgs.physical_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[3], 3.0),
                id: state.ids.skill_sword_spin_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sword(SSpins),
                image: self.imgs.physical_amount_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[4], 3.0),
                id: state.ids.skill_sword_spin_4,
            },
            // Bottom right skills
            SkillIcon::Unlockable {
                skill: Skill::Sword(InterruptingAttacks),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_r[0], 3.0),
                id: state.ids.skill_sword_passive_0,
            },
        ];

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip);
        events
    }

    fn handle_hammer_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        // Title text
        let tree_title = self.localized_strings.get("common.weapons.hammer");

        Text::new(tree_title)
            .mid_top_with_margin_on(state.ids.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.ids.tree_title_txt, ui);

        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = 5;
        let skills_top_r = 5;
        let skills_bot_l = 6;
        let skills_bot_r = 0;

        self.setup_state_for_skill_icons(
            state,
            ui,
            skills_top_l,
            skills_top_r,
            skills_bot_l,
            skills_bot_r,
        );

        // Skill icons and buttons
        use skills::HammerSkill::*;
        // Hammer
        Image::new(animate_by_pulse(
            &self
                .item_imgs
                .img_ids_or_not_found_img(ItemKey::Tool("example_hammer".to_string())),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.content_align)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
        .set(state.ids.hammer_render, ui);
        use PositionSpecifier::MidTopWithMarginOn;
        let skill_buttons = &[
            // Top Left skills
            //        5 1 6
            //        3 0 4
            //        8 2 7
            SkillIcon::Descriptive {
                title: "hud.skill.hmr_single_strike_title",
                desc: "hud.skill.hmr_single_strike",
                image: self.imgs.twohhammer_m1,
                position: MidTopWithMarginOn(state.ids.skills_top_l[0], 3.0),
                id: state.ids.skill_hammer_combo_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(SsKnockback),
                image: self.imgs.physical_knockback_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[1], 3.0),
                id: state.ids.skill_hammer_combo_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(SsDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[2], 3.0),
                id: state.ids.skill_hammer_combo_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(SsSpeed),
                image: self.imgs.physical_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[3], 3.0),
                id: state.ids.skill_hammer_combo_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(SsRegen),
                image: self.imgs.physical_energy_regen_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[4], 3.0),
                id: state.ids.skill_hammer_combo_4,
            },
            // Top right skills
            SkillIcon::Descriptive {
                title: "hud.skill.hmr_charged_melee_title",
                desc: "hud.skill.hmr_charged_melee",
                image: self.imgs.hammergolf,
                position: MidTopWithMarginOn(state.ids.skills_top_r[0], 3.0),
                id: state.ids.skill_hammer_charged_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(CKnockback),
                image: self.imgs.physical_knockback_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[1], 3.0),
                id: state.ids.skill_hammer_charged_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(CDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[2], 3.0),
                id: state.ids.skill_hammer_charged_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(CDrain),
                image: self.imgs.physical_energy_drain_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[3], 3.0),
                id: state.ids.skill_hammer_charged_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(CSpeed),
                image: self.imgs.physical_amount_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[4], 3.0),
                id: state.ids.skill_hammer_charged_4,
            },
            // Bottom left skills
            SkillIcon::Unlockable {
                skill: Skill::Hammer(UnlockLeap),
                image: self.imgs.hammerleap,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[0], 3.0),
                id: state.ids.skill_hammer_leap_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(LDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[1], 3.0),
                id: state.ids.skill_hammer_leap_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(LKnockback),
                image: self.imgs.physical_knockback_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[2], 3.0),
                id: state.ids.skill_hammer_leap_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(LCost),
                image: self.imgs.physical_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[3], 3.0),
                id: state.ids.skill_hammer_leap_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(LDistance),
                image: self.imgs.physical_distance_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[4], 3.0),
                id: state.ids.skill_hammer_leap_4,
            },
            SkillIcon::Unlockable {
                skill: Skill::Hammer(LRange),
                image: self.imgs.physical_radius_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[5], 3.0),
                id: state.ids.skill_hammer_leap_5,
            },
        ];

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip);
        events
    }

    fn handle_axe_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        // Title text
        let tree_title = self.localized_strings.get("common.weapons.axe");

        Text::new(tree_title)
            .mid_top_with_margin_on(state.ids.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.ids.tree_title_txt, ui);

        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = 5;
        let skills_top_r = 6;
        let skills_bot_l = 5;
        let skills_bot_r = 0;

        self.setup_state_for_skill_icons(
            state,
            ui,
            skills_top_l,
            skills_top_r,
            skills_bot_l,
            skills_bot_r,
        );

        // Skill icons and buttons
        use skills::AxeSkill::*;
        // Axe
        Image::new(animate_by_pulse(
            &self
                .item_imgs
                .img_ids_or_not_found_img(ItemKey::Tool("example_axe".to_string())),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.content_align)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
        .set(state.ids.axe_render, ui);
        use PositionSpecifier::MidTopWithMarginOn;
        let skill_buttons = &[
            // Top Left skills
            //        5 1 6
            //        3 0 4
            //        8 2 7
            SkillIcon::Descriptive {
                title: "hud.skill.axe_double_strike_title",
                desc: "hud.skill.axe_double_strike",
                image: self.imgs.twohaxe_m1,
                position: MidTopWithMarginOn(state.ids.skills_top_l[0], 3.0),
                id: state.ids.skill_axe_combo_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(DsCombo),
                image: self.imgs.physical_combo_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[1], 3.0),
                id: state.ids.skill_axe_combo_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(DsDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[2], 3.0),
                id: state.ids.skill_axe_combo_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(DsSpeed),
                image: self.imgs.physical_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[3], 3.0),
                id: state.ids.skill_axe_combo_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(DsRegen),
                image: self.imgs.physical_energy_regen_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[4], 3.0),
                id: state.ids.skill_axe_combo_4,
            },
            // Top right skills
            SkillIcon::Descriptive {
                title: "hud.skill.axe_spin_title",
                desc: "hud.skill.axe_spin",
                image: self.imgs.axespin,
                position: MidTopWithMarginOn(state.ids.skills_top_r[0], 3.0),
                id: state.ids.skill_axe_spin_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(SInfinite),
                image: self.imgs.physical_infinite_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[1], 3.0),
                id: state.ids.skill_axe_spin_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(SDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[2], 3.0),
                id: state.ids.skill_axe_spin_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(SHelicopter),
                image: self.imgs.physical_helicopter_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[3], 3.0),
                id: state.ids.skill_axe_spin_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(SSpeed),
                image: self.imgs.physical_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[4], 3.0),
                id: state.ids.skill_axe_spin_4,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(SCost),
                image: self.imgs.physical_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[5], 3.0),
                id: state.ids.skill_axe_spin_5,
            },
            // Bottom left skills
            SkillIcon::Unlockable {
                skill: Skill::Axe(UnlockLeap),
                image: self.imgs.skill_axe_leap_slash,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[0], 3.0),
                id: state.ids.skill_axe_leap_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(LDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[1], 3.0),
                id: state.ids.skill_axe_leap_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(LKnockback),
                image: self.imgs.physical_knockback_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[2], 3.0),
                id: state.ids.skill_axe_leap_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(LCost),
                image: self.imgs.physical_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[3], 3.0),
                id: state.ids.skill_axe_leap_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Axe(LDistance),
                image: self.imgs.physical_distance_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[4], 3.0),
                id: state.ids.skill_axe_leap_4,
            },
        ];

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip);
        events
    }

    fn handle_sceptre_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        // Title text
        let tree_title = self.localized_strings.get("common.weapons.sceptre");

        Text::new(tree_title)
            .mid_top_with_margin_on(state.ids.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.ids.tree_title_txt, ui);

        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = 5;
        let skills_top_r = 5;
        let skills_bot_l = 5;
        let skills_bot_r = 0;

        self.setup_state_for_skill_icons(
            state,
            ui,
            skills_top_l,
            skills_top_r,
            skills_bot_l,
            skills_bot_r,
        );

        // Skill icons and buttons
        use skills::SceptreSkill::*;
        // Sceptre
        Image::new(animate_by_pulse(
            &self
                .item_imgs
                .img_ids_or_not_found_img(ItemKey::Tool("example_sceptre".to_string())),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.content_align)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
        .set(state.ids.sceptre_render, ui);
        use PositionSpecifier::MidTopWithMarginOn;
        let skill_buttons = &[
            // Top Left skills
            //        5 1 6
            //        3 0 4
            //        8 2 7
            SkillIcon::Descriptive {
                title: "hud.skill.sc_lifesteal_title",
                desc: "hud.skill.sc_lifesteal",
                image: self.imgs.skill_sceptre_lifesteal,
                position: MidTopWithMarginOn(state.ids.skills_top_l[0], 3.0),
                id: state.ids.skill_sceptre_lifesteal_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(LDamage),
                image: self.imgs.magic_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[1], 3.0),
                id: state.ids.skill_sceptre_lifesteal_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(LRange),
                image: self.imgs.magic_distance_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[2], 3.0),
                id: state.ids.skill_sceptre_lifesteal_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(LLifesteal),
                image: self.imgs.magic_lifesteal_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[3], 3.0),
                id: state.ids.skill_sceptre_lifesteal_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(LRegen),
                image: self.imgs.magic_energy_regen_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[4], 3.0),
                id: state.ids.skill_sceptre_lifesteal_4,
            },
            // Top right skills
            SkillIcon::Descriptive {
                title: "hud.skill.sc_heal_title",
                desc: "hud.skill.sc_heal",
                image: self.imgs.skill_sceptre_heal,
                position: MidTopWithMarginOn(state.ids.skills_top_r[0], 3.0),
                id: state.ids.skill_sceptre_heal_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(HHeal),
                image: self.imgs.heal_heal_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[1], 3.0),
                id: state.ids.skill_sceptre_heal_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(HDuration),
                image: self.imgs.heal_duration_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[2], 3.0),
                id: state.ids.skill_sceptre_heal_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(HRange),
                image: self.imgs.heal_radius_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[3], 3.0),
                id: state.ids.skill_sceptre_heal_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(HCost),
                image: self.imgs.heal_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[4], 3.0),
                id: state.ids.skill_sceptre_heal_4,
            },
            // Bottom left skills
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(UnlockAura),
                image: self.imgs.skill_sceptre_aura,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[0], 3.0),
                id: state.ids.skill_sceptre_aura_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(AStrength),
                image: self.imgs.buff_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[1], 3.0),
                id: state.ids.skill_sceptre_aura_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(ADuration),
                image: self.imgs.buff_duration_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[2], 3.0),
                id: state.ids.skill_sceptre_aura_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(ARange),
                image: self.imgs.buff_radius_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[3], 3.0),
                id: state.ids.skill_sceptre_aura_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Sceptre(ACost),
                image: self.imgs.buff_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[4], 3.0),
                id: state.ids.skill_sceptre_aura_4,
            },
        ];

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip);
        events
    }

    fn handle_bow_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        // Title text
        let tree_title = self.localized_strings.get("common.weapons.bow");

        Text::new(tree_title)
            .mid_top_with_margin_on(state.ids.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.ids.tree_title_txt, ui);

        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = 6;
        let skills_top_r = 4;
        let skills_bot_l = 5;
        let skills_bot_r = 1;

        self.setup_state_for_skill_icons(
            state,
            ui,
            skills_top_l,
            skills_top_r,
            skills_bot_l,
            skills_bot_r,
        );

        // Skill icons and buttons
        use skills::BowSkill::*;
        // Bow
        Image::new(animate_by_pulse(
            &self
                .item_imgs
                .img_ids_or_not_found_img(ItemKey::Tool("example_bow".to_string())),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.content_align)
        .set(state.ids.bow_render, ui);
        use PositionSpecifier::MidTopWithMarginOn;
        let skill_buttons = &[
            // Top Left skills
            //        5 1 6
            //        3 0 4
            //        8 2 7
            SkillIcon::Descriptive {
                title: "hud.skill.bow_charged_title",
                desc: "hud.skill.bow_charged",
                image: self.imgs.bow_m1,
                position: MidTopWithMarginOn(state.ids.skills_top_l[0], 3.0),
                id: state.ids.skill_bow_charged_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(CDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[1], 3.0),
                id: state.ids.skill_bow_charged_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(CRegen),
                image: self.imgs.physical_energy_regen_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[2], 3.0),
                id: state.ids.skill_bow_charged_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(CKnockback),
                image: self.imgs.physical_knockback_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[3], 3.0),
                id: state.ids.skill_bow_charged_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(CSpeed),
                image: self.imgs.physical_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[4], 3.0),
                id: state.ids.skill_bow_charged_4,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(CMove),
                image: self.imgs.physical_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[5], 3.0),
                id: state.ids.skill_bow_charged_5,
            },
            // Top right skills
            SkillIcon::Descriptive {
                title: "hud.skill.bow_repeater_title",
                desc: "hud.skill.bow_repeater",
                image: self.imgs.bow_m2,
                position: MidTopWithMarginOn(state.ids.skills_top_r[0], 3.0),
                id: state.ids.skill_bow_repeater_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(RDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[1], 3.0),
                id: state.ids.skill_bow_repeater_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(RCost),
                image: self.imgs.physical_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[2], 3.0),
                id: state.ids.skill_bow_repeater_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(RSpeed),
                image: self.imgs.physical_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[3], 3.0),
                id: state.ids.skill_bow_repeater_3,
            },
            // Bottom left skills
            SkillIcon::Unlockable {
                skill: Skill::Bow(UnlockShotgun),
                image: self.imgs.skill_bow_jump_burst,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[0], 3.0),
                id: state.ids.skill_bow_shotgun_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(SDamage),
                image: self.imgs.physical_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[1], 3.0),
                id: state.ids.skill_bow_shotgun_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(SCost),
                image: self.imgs.physical_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[2], 3.0),
                id: state.ids.skill_bow_shotgun_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(SArrows),
                image: self.imgs.physical_amount_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[3], 3.0),
                id: state.ids.skill_bow_shotgun_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Bow(SSpread),
                image: self.imgs.physical_explosion_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[4], 3.0),
                id: state.ids.skill_bow_shotgun_4,
            },
            // Bottom right skills
            SkillIcon::Unlockable {
                skill: Skill::Bow(ProjSpeed),
                image: self.imgs.physical_projectile_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_r[0], 3.0),
                id: state.ids.skill_bow_passive_0,
            },
        ];

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip);
        events
    }

    fn handle_staff_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        // Title text
        let tree_title = self.localized_strings.get("common.weapons.staff");

        Text::new(tree_title)
            .mid_top_with_margin_on(state.ids.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.ids.tree_title_txt, ui);

        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = 4;
        let skills_top_r = 5;
        let skills_bot_l = 5;
        let skills_bot_r = 0;

        self.setup_state_for_skill_icons(
            state,
            ui,
            skills_top_l,
            skills_top_r,
            skills_bot_l,
            skills_bot_r,
        );

        // Skill icons and buttons
        use skills::StaffSkill::*;
        // Staff
        Image::new(animate_by_pulse(
            &self
                .item_imgs
                .img_ids_or_not_found_img(ItemKey::Tool("example_staff_fire".to_string())),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.content_align)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
        .set(state.ids.staff_render, ui);

        use PositionSpecifier::MidTopWithMarginOn;
        let skill_buttons = &[
            // Top Left skills
            //        5 1 6
            //        3 0 4
            //        8 2 7
            SkillIcon::Descriptive {
                title: "hud.skill.st_fireball_title",
                desc: "hud.skill.st_fireball",
                image: self.imgs.fireball,
                position: MidTopWithMarginOn(state.ids.skills_top_l[0], 3.0),
                id: state.ids.skill_staff_basic_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(BDamage),
                image: self.imgs.magic_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[1], 3.0),
                id: state.ids.skill_staff_basic_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(BRegen),
                image: self.imgs.magic_energy_regen_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[2], 3.0),
                id: state.ids.skill_staff_basic_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(BRadius),
                image: self.imgs.magic_radius_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[3], 3.0),
                id: state.ids.skill_staff_basic_3,
            },
            // Top right skills
            SkillIcon::Descriptive {
                title: "hud.skill.st_flamethrower_title",
                desc: "hud.skill.st_flamethrower",
                image: self.imgs.flamethrower,
                position: MidTopWithMarginOn(state.ids.skills_top_r[0], 3.0),
                id: state.ids.skill_staff_beam_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(FDamage),
                image: self.imgs.magic_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[1], 3.0),
                id: state.ids.skill_staff_beam_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(FDrain),
                image: self.imgs.magic_energy_drain_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[2], 3.0),
                id: state.ids.skill_staff_beam_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(FRange),
                image: self.imgs.magic_radius_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[3], 3.0),
                id: state.ids.skill_staff_beam_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(FVelocity),
                image: self.imgs.magic_projectile_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_r[4], 3.0),
                id: state.ids.skill_staff_beam_4,
            },
            // Bottom left skills
            SkillIcon::Unlockable {
                skill: Skill::Staff(UnlockShockwave),
                image: self.imgs.fire_aoe,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[0], 3.0),
                id: state.ids.skill_staff_shockwave_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(SDamage),
                image: self.imgs.magic_damage_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[1], 3.0),
                id: state.ids.skill_staff_shockwave_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(SKnockback),
                image: self.imgs.magic_knockback_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[2], 3.0),
                id: state.ids.skill_staff_shockwave_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(SCost),
                image: self.imgs.magic_cost_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[3], 3.0),
                id: state.ids.skill_staff_shockwave_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::Staff(SRange),
                image: self.imgs.magic_radius_skill,
                position: MidTopWithMarginOn(state.ids.skills_bot_l[4], 3.0),
                id: state.ids.skill_staff_shockwave_4,
            },
        ];

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip);
        events
    }

    fn handle_mining_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        // Title text
        let tree_title = self.localized_strings.get("common.tool.mining");

        Text::new(tree_title)
            .mid_top_with_margin_on(state.ids.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.ids.tree_title_txt, ui);

        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = 4;
        let skills_top_r = 0;
        let skills_bot_l = 0;
        let skills_bot_r = 0;

        self.setup_state_for_skill_icons(
            state,
            ui,
            skills_top_l,
            skills_top_r,
            skills_bot_l,
            skills_bot_r,
        );

        // Skill icons and buttons
        use skills::MiningSkill::*;
        // Mining
        Image::new(animate_by_pulse(
            &self
                .item_imgs
                .img_ids_or_not_found_img(ItemKey::Tool("example_pick".to_string())),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.content_align)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
        .set(state.ids.pick_render, ui);

        use PositionSpecifier::MidTopWithMarginOn;
        let skill_buttons = &[
            // Top Left skills
            //        5 1 6
            //        3 0 4
            //        8 2 7
            SkillIcon::Descriptive {
                title: "hud.skill.pick_strike_title",
                desc: "hud.skill.pick_strike",
                image: self.imgs.pickaxe,
                position: MidTopWithMarginOn(state.ids.skills_top_l[0], 3.0),
                id: state.ids.skill_pick_m1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Pick(Speed),
                image: self.imgs.pickaxe_speed_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[1], 3.0),
                id: state.ids.skill_pick_m1_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::Pick(OreGain),
                image: self.imgs.pickaxe_oregain_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[2], 3.0),
                id: state.ids.skill_pick_m1_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::Pick(GemGain),
                image: self.imgs.pickaxe_gemgain_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[3], 3.0),
                id: state.ids.skill_pick_m1_2,
            },
        ];

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip);
        events
    }

    fn handle_skill_buttons(
        &mut self,
        icons: &[SkillIcon],
        ui: &mut UiCell,
        events: &mut Vec<Event>,
        diary_tooltip: &Tooltip,
    ) {
        for icon in icons {
            match icon {
                SkillIcon::Descriptive {
                    title,
                    desc,
                    image,
                    position,
                    id,
                } => {
                    // TODO: shouldn't this be a `Image::new`?
                    Button::image(*image)
                        .w_h(74.0, 74.0)
                        .position(*position)
                        .with_tooltip(
                            self.tooltip_manager,
                            self.localized_strings.get(title),
                            self.localized_strings.get(desc),
                            diary_tooltip,
                            TEXT_COLOR,
                        )
                        .set(*id, ui);
                },
                SkillIcon::Unlockable {
                    skill,
                    image,
                    position,
                    id,
                } => self.create_unlock_skill_button(
                    *skill,
                    *image,
                    *position,
                    *id,
                    ui,
                    events,
                    diary_tooltip,
                ),
            }
        }
    }

    fn setup_state_for_skill_icons(
        &mut self,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        skills_top_l: usize,
        skills_top_r: usize,
        skills_bot_l: usize,
        skills_bot_r: usize,
    ) {
        // Update widget id array len
        state.update(|s| {
            s.ids
                .skills_top_l
                .resize(skills_top_l, &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.ids
                .skills_top_r
                .resize(skills_top_r, &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.ids
                .skills_bot_l
                .resize(skills_bot_l, &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.ids
                .skills_bot_r
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
        let offset_0 = 22.0;
        let offset_1 = -122.0;
        let offset_2 = offset_1 - -20.0;

        let skill_pos = |idx, align, central_skill| {
            use PositionSpecifier::*;
            match idx {
                // Central skill
                0 => MiddleOf(align),
                // 12:00
                1 => UpFrom(central_skill, offset_0),
                // 6:00
                2 => DownFrom(central_skill, offset_0),
                // 3:00
                3 => LeftFrom(central_skill, offset_0),
                // 9:00
                4 => RightFrom(central_skill, offset_0),
                // 10:30
                5 => TopLeftWithMarginsOn(central_skill, offset_1, offset_2),
                // 1:30
                6 => TopRightWithMarginsOn(central_skill, offset_1, offset_2),
                // 4:30
                7 => BottomLeftWithMarginsOn(central_skill, offset_1, offset_2),
                // 7:30
                8 => BottomRightWithMarginsOn(central_skill, offset_1, offset_2),
                buttons => {
                    panic!("{} > 8 position number", buttons);
                },
            }
        };

        // TOP-LEFT Skills
        //
        // TODO: Why this uses while loop on field of struct and not just
        // `for i in 0..skils_top_l`?
        while self.created_btns_top_l < skills_top_l {
            let pos = skill_pos(
                self.created_btns_top_l,
                state.ids.skills_top_l_align,
                state.ids.skills_top_l[0],
            );
            Button::image(self.imgs.wpn_icon_border_skills)
                .w_h(80.0, 100.0)
                .position(pos)
                .set(state.ids.skills_top_l[self.created_btns_top_l], ui);
            self.created_btns_top_l += 1;
        }
        // TOP-RIGHT Skills
        while self.created_btns_top_r < skills_top_r {
            let pos = skill_pos(
                self.created_btns_top_r,
                state.ids.skills_top_r_align,
                state.ids.skills_top_r[0],
            );
            Button::image(self.imgs.wpn_icon_border_skills)
                .w_h(80.0, 100.0)
                .position(pos)
                .set(state.ids.skills_top_r[self.created_btns_top_r], ui);
            self.created_btns_top_r += 1;
        }
        // BOTTOM-LEFT Skills
        while self.created_btns_bot_l < skills_bot_l {
            let pos = skill_pos(
                self.created_btns_bot_l,
                state.ids.skills_bot_l_align,
                state.ids.skills_bot_l[0],
            );
            Button::image(self.imgs.wpn_icon_border_skills)
                .w_h(80.0, 100.0)
                .position(pos)
                .set(state.ids.skills_bot_l[self.created_btns_bot_l], ui);
            self.created_btns_bot_l += 1;
        }
        // BOTTOM-RIGHT Skills
        while self.created_btns_bot_r < skills_bot_r {
            let pos = skill_pos(
                self.created_btns_bot_r,
                state.ids.skills_bot_r_align,
                state.ids.skills_bot_r[0],
            );
            Button::image(self.imgs.wpn_icon_border_skills)
                .w_h(80.0, 100.0)
                .position(pos)
                .set(state.ids.skills_bot_r[self.created_btns_bot_r], ui);
            self.created_btns_bot_r += 1;
        }
    }

    fn create_unlock_skill_button(
        &mut self,
        skill: Skill,
        skill_image: image::Id,
        position: PositionSpecifier,
        widget_id: widget::Id,
        ui: &mut UiCell,
        events: &mut Vec<Event>,
        diary_tooltip: &Tooltip,
    ) {
        let label = if self.skill_set.prerequisites_met(skill) {
            let current = self.skill_set.skill_level(skill).unwrap_or(0);
            let max = skill.max_level();
            format!("{}/{}", current, max)
        } else {
            "".to_owned()
        };

        let label_color = if self.skill_set.is_at_max_level(skill) {
            TEXT_COLOR
        } else if self.skill_set.sufficient_skill_points(skill) {
            HP_COLOR
        } else {
            CRITICAL_HP_COLOR
        };

        let image_color = if self.skill_set.prerequisites_met(skill) {
            TEXT_COLOR
        } else {
            Color::Rgba(0.41, 0.41, 0.41, 0.7)
        };

        let (title, description) = skill_strings(skill, self.localized_strings);
        let description = if description.contains("{SP}") {
            Cow::Owned(self.splice_skill_reqs(skill, &description))
        } else {
            description
        };

        let button = Button::image(skill_image)
            .w_h(74.0, 74.0)
            .position(position)
            .label(&label)
            .label_y(conrod_core::position::Relative::Scalar(-47.0))
            .label_x(conrod_core::position::Relative::Scalar(0.0))
            .label_color(label_color)
            .label_font_size(self.fonts.cyri.scale(15))
            .label_font_id(self.fonts.cyri.conrod_id)
            .image_color(image_color)
            .with_tooltip(
                self.tooltip_manager,
                title,
                &description,
                diary_tooltip,
                TEXT_COLOR,
            )
            .set(widget_id, ui);

        if button.was_clicked() {
            events.push(Event::UnlockSkill(skill));
        };
    }

    fn splice_skill_reqs(&self, skill: Skill, desc: &str) -> String {
        let current_level = self.skill_set.skill_level(skill);
        if matches!(current_level, Ok(level) if level == skill.max_level()) {
            desc.replace("{SP}", "")
        } else {
            let req_sp_text = self.localized_strings.get("hud.skill.req_sp");
            let skill_cost_text = self.skill_set.skill_cost(skill).to_string();
            desc.replace("{SP}", &req_sp_text.replace("{number}", &skill_cost_text))
        }
    }
}

/// Returns skill info as a tuple of title and description.
///
/// Title is ready to use, description may include `"{SP}"` placeholder you
/// will want to handle yourself.
pub fn skill_strings(skill: Skill, i18n: &Localization) -> (&str, Cow<str>) {
    match skill {
        // general tree
        Skill::General(s) => general_skill_strings(s, i18n),
        Skill::UnlockGroup(s) => unlock_skill_strings(s, i18n),
        // weapon trees
        Skill::Sword(s) => sword_skill_strings(s, i18n),
        Skill::Axe(s) => axe_skill_strings(s, i18n),
        Skill::Hammer(s) => hammer_skill_strings(s, i18n),
        Skill::Bow(s) => bow_skill_strings(s, i18n),
        Skill::Staff(s) => staff_skill_strings(s, i18n),
        Skill::Sceptre(s) => sceptre_skill_strings(s, i18n),
        // movement trees
        Skill::Roll(s) => roll_skill_strings(s, i18n),
        Skill::Climb(s) => climb_skill_strings(s, i18n),
        Skill::Swim(s) => swim_skill_strings(s, i18n),
        // mining
        Skill::Pick(s) => mining_skill_strings(s, i18n),
    }
}

fn general_skill_strings(skill: GeneralSkill, i18n: &Localization) -> (&str, Cow<str>) {
    match skill {
        GeneralSkill::HealthIncrease => splice_constant(
            i18n,
            "hud.skill.inc_health_title",
            "hud.skill.inc_health",
            u32::from(HP_PER_LEVEL),
        ),
        GeneralSkill::EnergyIncrease => splice_constant(
            i18n,
            "hud.skill.inc_energy_title",
            "hud.skill.inc_energy",
            u32::from(ENERGY_PER_LEVEL),
        ),
    }
}

fn unlock_skill_strings(group: SkillGroupKind, i18n: &Localization) -> (&str, Cow<str>) {
    match group {
        SkillGroupKind::Weapon(ToolKind::Sword) => {
            localize(i18n, "hud.skill.unlck_sword_title", "hud.skill.unlck_sword")
        },
        SkillGroupKind::Weapon(ToolKind::Axe) => {
            localize(i18n, "hud.skill.unlck_axe_title", "hud.skill.unlck_axe")
        },
        SkillGroupKind::Weapon(ToolKind::Hammer) => localize(
            i18n,
            "hud.skill.unlck_hammer_title",
            "hud.skill.unlck_hammer",
        ),
        SkillGroupKind::Weapon(ToolKind::Bow) => {
            localize(i18n, "hud.skill.unlck_bow_title", "hud.skill.unlck_bow")
        },
        SkillGroupKind::Weapon(ToolKind::Staff) => {
            localize(i18n, "hud.skill.unlck_staff_title", "hud.skill.unlck_staff")
        },
        SkillGroupKind::Weapon(ToolKind::Sceptre) => localize(
            i18n,
            "hud.skill.unlck_sceptre_title",
            "hud.skill.unlck_sceptre",
        ),
        SkillGroupKind::General
        | SkillGroupKind::Weapon(
            ToolKind::Dagger
            | ToolKind::Shield
            | ToolKind::Spear
            | ToolKind::Blowgun
            | ToolKind::Debug
            | ToolKind::Farming
            | ToolKind::Pick
            | ToolKind::Natural
            | ToolKind::Empty,
        ) => {
            tracing::warn!("Requesting title for unlocking unexpected skill group");
            ("", Cow::Owned(String::new()))
        },
    }
}

fn sword_skill_strings(skill: SwordSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.sword_tree;
    match skill {
        // triple strike
        SwordSkill::TsCombo => localize(
            i18n,
            "hud.skill.sw_trip_str_combo_title",
            "hud.skill.sw_trip_str_combo",
        ),
        SwordSkill::TsDamage => localize(
            i18n,
            "hud.skill.sw_trip_str_dmg_title",
            "hud.skill.sw_trip_str_dmg",
        ),
        SwordSkill::TsSpeed => localize(
            i18n,
            "hud.skill.sw_trip_str_sp_title",
            "hud.skill.sw_trip_str_sp",
        ),
        SwordSkill::TsRegen => localize(
            i18n,
            "hud.skill.sw_trip_str_reg_title",
            "hud.skill.sw_trip_str_reg",
        ),
        // dash
        SwordSkill::DDamage => splice_multiplier(
            i18n,
            "hud.skill.sw_dash_dmg_title",
            "hud.skill.sw_dash_dmg",
            modifiers.dash.base_damage,
        ),
        SwordSkill::DDrain => splice_multiplier(
            i18n,
            "hud.skill.sw_dash_drain_title",
            "hud.skill.sw_dash_drain",
            modifiers.dash.energy_drain,
        ),
        SwordSkill::DCost => splice_multiplier(
            i18n,
            "hud.skill.sw_dash_cost_title",
            "hud.skill.sw_dash_cost",
            modifiers.dash.energy_cost,
        ),
        SwordSkill::DSpeed => splice_multiplier(
            i18n,
            "hud.skill.sw_dash_speed_title",
            "hud.skill.sw_dash_speed",
            modifiers.dash.forward_speed,
        ),
        SwordSkill::DChargeThrough => localize(
            i18n,
            "hud.skill.sw_dash_charge_through_title",
            "hud.skill.sw_dash_charge_through",
        ),
        SwordSkill::DScaling => splice_multiplier(
            i18n,
            "hud.skill.sw_dash_scale_title",
            "hud.skill.sw_dash_scale",
            modifiers.dash.scaled_damage,
        ),
        // spin
        SwordSkill::UnlockSpin => localize(i18n, "hud.skill.sw_spin_title", "hud.skill.sw_spin"),
        SwordSkill::SDamage => splice_multiplier(
            i18n,
            "hud.skill.sw_spin_dmg_title",
            "hud.skill.sw_spin_dmg",
            modifiers.spin.base_damage,
        ),
        SwordSkill::SSpeed => splice_multiplier(
            i18n,
            "hud.skill.sw_spin_spd_title",
            "hud.skill.sw_spin_spd",
            modifiers.spin.swing_duration,
        ),
        SwordSkill::SCost => splice_multiplier(
            i18n,
            "hud.skill.sw_spin_cost_title",
            "hud.skill.sw_spin_cost",
            modifiers.spin.energy_cost,
        ),
        SwordSkill::SSpins => splice_constant(
            i18n,
            "hud.skill.sw_spin_spins_title",
            "hud.skill.sw_spin_spins",
            modifiers.spin.num,
        ),
        // independent skills
        SwordSkill::InterruptingAttacks => localize(
            i18n,
            "hud.skill.sw_interrupt_title",
            "hud.skill.sw_interrupt",
        ),
    }
}

fn axe_skill_strings(skill: AxeSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.axe_tree;
    match skill {
        // Double strike upgrades
        AxeSkill::DsCombo => localize(
            i18n,
            "hud.skill.axe_double_strike_combo_title",
            "hud.skill.axe_double_strike_combo",
        ),
        AxeSkill::DsDamage => localize(
            i18n,
            "hud.skill.axe_double_strike_damage_title",
            "hud.skill.axe_double_strike_damage",
        ),
        AxeSkill::DsSpeed => localize(
            i18n,
            "hud.skill.axe_double_strike_speed_title",
            "hud.skill.axe_double_strike_speed",
        ),
        AxeSkill::DsRegen => localize(
            i18n,
            "hud.skill.axe_double_strike_regen_title",
            "hud.skill.axe_double_strike_regen",
        ),
        // Spin upgrades
        AxeSkill::SInfinite => localize(
            i18n,
            "hud.skill.axe_infinite_axe_spin_title",
            "hud.skill.axe_infinite_axe_spin",
        ),
        AxeSkill::SHelicopter => localize(
            i18n,
            "hud.skill.axe_spin_helicopter_title",
            "hud.skill.axe_spin_helicopter",
        ),
        AxeSkill::SDamage => splice_multiplier(
            i18n,
            "hud.skill.axe_spin_damage_title",
            "hud.skill.axe_spin_damage",
            modifiers.spin.base_damage,
        ),
        AxeSkill::SSpeed => splice_multiplier(
            i18n,
            "hud.skill.axe_spin_speed_title",
            "hud.skill.axe_spin_speed",
            modifiers.spin.swing_duration,
        ),
        AxeSkill::SCost => splice_multiplier(
            i18n,
            "hud.skill.axe_spin_cost_title",
            "hud.skill.axe_spin_cost",
            modifiers.spin.energy_cost,
        ),
        // Leap upgrades
        AxeSkill::UnlockLeap => localize(
            i18n,
            "hud.skill.axe_unlock_leap_title",
            "hud.skill.axe_unlock_leap",
        ),
        AxeSkill::LDamage => splice_multiplier(
            i18n,
            "hud.skill.axe_leap_damage_title",
            "hud.skill.axe_leap_damage",
            modifiers.leap.base_damage,
        ),
        AxeSkill::LKnockback => splice_multiplier(
            i18n,
            "hud.skill.axe_leap_knockback_title",
            "hud.skill.axe_leap_knockback",
            modifiers.leap.knockback,
        ),
        AxeSkill::LCost => splice_multiplier(
            i18n,
            "hud.skill.axe_leap_cost_title",
            "hud.skill.axe_leap_cost",
            modifiers.leap.energy_cost,
        ),
        AxeSkill::LDistance => splice_multiplier(
            i18n,
            "hud.skill.axe_leap_distance_title",
            "hud.skill.axe_leap_distance",
            modifiers.leap.leap_strength,
        ),
    }
}

fn hammer_skill_strings(skill: HammerSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.hammer_tree;
    // Single strike upgrades
    match skill {
        HammerSkill::SsKnockback => splice_multiplier(
            i18n,
            "hud.skill.hmr_single_strike_knockback_title",
            "hud.skill.hmr_single_strike_knockback",
            modifiers.single_strike.knockback,
        ),
        HammerSkill::SsDamage => localize(
            i18n,
            "hud.skill.hmr_single_strike_damage_title",
            "hud.skill.hmr_single_strike_damage",
        ),
        HammerSkill::SsSpeed => localize(
            i18n,
            "hud.skill.hmr_single_strike_speed_title",
            "hud.skill.hmr_single_strike_speed",
        ),
        HammerSkill::SsRegen => localize(
            i18n,
            "hud.skill.hmr_single_strike_regen_title",
            "hud.skill.hmr_single_strike_regen",
        ),
        // Charged melee upgrades
        HammerSkill::CDamage => splice_multiplier(
            i18n,
            "hud.skill.hmr_charged_melee_damage_title",
            "hud.skill.hmr_charged_melee_damage",
            modifiers.charged.scaled_damage,
        ),
        HammerSkill::CKnockback => splice_multiplier(
            i18n,
            "hud.skill.hmr_charged_melee_knockback_title",
            "hud.skill.hmr_charged_melee_knockback",
            modifiers.charged.scaled_knockback,
        ),
        HammerSkill::CDrain => splice_multiplier(
            i18n,
            "hud.skill.hmr_charged_melee_nrg_drain_title",
            "hud.skill.hmr_charged_melee_nrg_drain",
            modifiers.charged.energy_drain,
        ),
        HammerSkill::CSpeed => splice_multiplier(
            i18n,
            "hud.skill.hmr_charged_rate_title",
            "hud.skill.hmr_charged_rate",
            modifiers.charged.charge_rate,
        ),
        // Leap upgrades
        HammerSkill::UnlockLeap => localize(
            i18n,
            "hud.skill.hmr_unlock_leap_title",
            "hud.skill.hmr_unlock_leap",
        ),
        HammerSkill::LDamage => splice_multiplier(
            i18n,
            "hud.skill.hmr_leap_damage_title",
            "hud.skill.hmr_leap_damage",
            modifiers.leap.base_damage,
        ),
        HammerSkill::LCost => splice_multiplier(
            i18n,
            "hud.skill.hmr_leap_cost_title",
            "hud.skill.hmr_leap_cost",
            modifiers.leap.energy_cost,
        ),
        HammerSkill::LDistance => splice_multiplier(
            i18n,
            "hud.skill.hmr_leap_distance_title",
            "hud.skill.hmr_leap_distance",
            modifiers.leap.leap_strength,
        ),
        HammerSkill::LKnockback => splice_multiplier(
            i18n,
            "hud.skill.hmr_leap_knockback_title",
            "hud.skill.hmr_leap_knockback",
            modifiers.leap.knockback,
        ),
        HammerSkill::LRange => {
            let title = i18n.get("hud.skill.hmr_leap_radius_title");
            let desc = i18n.get("hud.skill.hmr_leap_radius");
            let desc = desc.replace("{boost}", &format!("{}", modifiers.leap.range));

            (title, Cow::Owned(desc))
        },
    }
}

fn bow_skill_strings(skill: BowSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.bow_tree;
    match skill {
        // Passives
        BowSkill::ProjSpeed => splice_multiplier(
            i18n,
            "hud.skill.bow_projectile_speed_title",
            "hud.skill.bow_projectile_speed",
            modifiers.universal.projectile_speed,
        ),
        // Charged upgrades
        BowSkill::CDamage => splice_multiplier(
            i18n,
            "hud.skill.bow_charged_damage_title",
            "hud.skill.bow_charged_damage",
            modifiers.charged.damage_scaling,
        ),
        BowSkill::CRegen => splice_multiplier(
            i18n,
            "hud.skill.bow_charged_energy_regen_title",
            "hud.skill.bow_charged_energy_regen",
            modifiers.charged.regen_scaling,
        ),
        BowSkill::CKnockback => splice_multiplier(
            i18n,
            "hud.skill.bow_charged_knockback_title",
            "hud.skill.bow_charged_knockback",
            modifiers.charged.knockback_scaling,
        ),
        BowSkill::CSpeed => splice_multiplier(
            i18n,
            "hud.skill.bow_charged_speed_title",
            "hud.skill.bow_charged_speed",
            modifiers.charged.charge_rate,
        ),
        BowSkill::CMove => splice_multiplier(
            i18n,
            "hud.skill.bow_charged_move_title",
            "hud.skill.bow_charged_move",
            modifiers.charged.move_speed,
        ),
        // Repeater upgrades
        BowSkill::RDamage => splice_multiplier(
            i18n,
            "hud.skill.bow_repeater_damage_title",
            "hud.skill.bow_repeater_damage",
            modifiers.repeater.power,
        ),
        BowSkill::RCost => splice_multiplier(
            i18n,
            "hud.skill.bow_repeater_cost_title",
            "hud.skill.bow_repeater_cost",
            modifiers.repeater.energy_cost,
        ),
        BowSkill::RSpeed => splice_multiplier(
            i18n,
            "hud.skill.bow_repeater_speed_title",
            "hud.skill.bow_repeater_speed",
            modifiers.repeater.max_speed,
        ),
        // Shotgun upgrades
        BowSkill::UnlockShotgun => localize(
            i18n,
            "hud.skill.bow_shotgun_unlock_title",
            "hud.skill.bow_shotgun_unlock",
        ),
        BowSkill::SDamage => splice_multiplier(
            i18n,
            "hud.skill.bow_shotgun_damage_title",
            "hud.skill.bow_shotgun_damage",
            modifiers.shotgun.power,
        ),
        BowSkill::SCost => splice_multiplier(
            i18n,
            "hud.skill.bow_shotgun_cost_title",
            "hud.skill.bow_shotgun_cost",
            modifiers.shotgun.energy_cost,
        ),
        BowSkill::SArrows => splice_constant(
            i18n,
            "hud.skill.bow_shotgun_arrow_count_title",
            "hud.skill.bow_shotgun_arrow_count",
            modifiers.shotgun.num_projectiles,
        ),
        BowSkill::SSpread => splice_multiplier(
            i18n,
            "hud.skill.bow_shotgun_spread_title",
            "hud.skill.bow_shotgun_spread",
            modifiers.shotgun.spread,
        ),
    }
}

fn staff_skill_strings(skill: StaffSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.staff_tree;
    match skill {
        // Basic ranged upgrades
        StaffSkill::BDamage => splice_multiplier(
            i18n,
            "hud.skill.st_damage_title",
            "hud.skill.st_damage",
            modifiers.fireball.power,
        ),
        StaffSkill::BRegen => splice_multiplier(
            i18n,
            "hud.skill.st_energy_regen_title",
            "hud.skill.st_energy_regen",
            modifiers.fireball.regen,
        ),
        StaffSkill::BRadius => splice_multiplier(
            i18n,
            "hud.skill.st_explosion_radius_title",
            "hud.skill.st_explosion_radius",
            modifiers.fireball.range,
        ),
        // Flamethrower upgrades
        StaffSkill::FDamage => splice_multiplier(
            i18n,
            "hud.skill.st_flamethrower_damage_title",
            "hud.skill.st_flamethrower_damage",
            modifiers.flamethrower.damage,
        ),
        StaffSkill::FRange => splice_multiplier(
            i18n,
            "hud.skill.st_flamethrower_range_title",
            "hud.skill.st_flamethrower_range",
            modifiers.flamethrower.range,
        ),
        StaffSkill::FDrain => splice_multiplier(
            i18n,
            "hud.skill.st_energy_drain_title",
            "hud.skill.st_energy_drain",
            modifiers.flamethrower.energy_drain,
        ),
        StaffSkill::FVelocity => splice_multiplier(
            i18n,
            "hud.skill.st_flame_velocity_title",
            "hud.skill.st_flame_velocity",
            modifiers.flamethrower.velocity,
        ),
        // Shockwave upgrades
        StaffSkill::UnlockShockwave => localize(
            i18n,
            "hud.skill.st_shockwave_unlock_title",
            "hud.skill.st_shockwave_unlock",
        ),
        StaffSkill::SDamage => splice_multiplier(
            i18n,
            "hud.skill.st_shockwave_damage_title",
            "hud.skill.st_shockwave_damage",
            modifiers.shockwave.damage,
        ),
        StaffSkill::SKnockback => splice_multiplier(
            i18n,
            "hud.skill.st_shockwave_knockback_title",
            "hud.skill.st_shockwave_knockback",
            modifiers.shockwave.knockback,
        ),
        StaffSkill::SRange => splice_multiplier(
            i18n,
            "hud.skill.st_shockwave_range_title",
            "hud.skill.st_shockwave_range",
            modifiers.shockwave.duration,
        ),
        StaffSkill::SCost => splice_multiplier(
            i18n,
            "hud.skill.st_shockwave_cost_title",
            "hud.skill.st_shockwave_cost",
            modifiers.shockwave.energy_cost,
        ),
    }
}

fn sceptre_skill_strings(skill: SceptreSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.sceptre_tree;
    match skill {
        // Lifesteal beam upgrades
        SceptreSkill::LDamage => splice_multiplier(
            i18n,
            "hud.skill.sc_lifesteal_damage_title",
            "hud.skill.sc_lifesteal_damage",
            modifiers.beam.damage,
        ),
        SceptreSkill::LRange => splice_multiplier(
            i18n,
            "hud.skill.sc_lifesteal_range_title",
            "hud.skill.sc_lifesteal_range",
            modifiers.beam.range,
        ),
        SceptreSkill::LLifesteal => splice_multiplier(
            i18n,
            "hud.skill.sc_lifesteal_lifesteal_title",
            "hud.skill.sc_lifesteal_lifesteal",
            modifiers.beam.lifesteal,
        ),
        SceptreSkill::LRegen => splice_multiplier(
            i18n,
            "hud.skill.sc_lifesteal_regen_title",
            "hud.skill.sc_lifesteal_regen",
            modifiers.beam.energy_regen,
        ),
        // Healing aura upgrades
        SceptreSkill::HHeal => splice_multiplier(
            i18n,
            "hud.skill.sc_heal_heal_title",
            "hud.skill.sc_heal_heal",
            modifiers.healing_aura.strength,
        ),
        SceptreSkill::HRange => splice_multiplier(
            i18n,
            "hud.skill.sc_heal_range_title",
            "hud.skill.sc_heal_range",
            modifiers.healing_aura.range,
        ),
        SceptreSkill::HDuration => splice_multiplier(
            i18n,
            "hud.skill.sc_heal_duration_title",
            "hud.skill.sc_heal_duration",
            modifiers.healing_aura.duration,
        ),
        SceptreSkill::HCost => splice_multiplier(
            i18n,
            "hud.skill.sc_heal_cost_title",
            "hud.skill.sc_heal_cost",
            modifiers.healing_aura.energy_cost,
        ),
        // Warding aura upgrades
        SceptreSkill::UnlockAura => localize(
            i18n,
            "hud.skill.sc_wardaura_unlock_title",
            "hud.skill.sc_wardaura_unlock",
        ),
        SceptreSkill::AStrength => splice_multiplier(
            i18n,
            "hud.skill.sc_wardaura_strength_title",
            "hud.skill.sc_wardaura_strength",
            modifiers.warding_aura.strength,
        ),
        SceptreSkill::ADuration => splice_multiplier(
            i18n,
            "hud.skill.sc_wardaura_duration_title",
            "hud.skill.sc_wardaura_duration",
            modifiers.warding_aura.duration,
        ),
        SceptreSkill::ARange => splice_multiplier(
            i18n,
            "hud.skill.sc_wardaura_range_title",
            "hud.skill.sc_wardaura_range",
            modifiers.warding_aura.range,
        ),
        SceptreSkill::ACost => splice_multiplier(
            i18n,
            "hud.skill.sc_wardaura_cost_title",
            "hud.skill.sc_wardaura_cost",
            modifiers.warding_aura.energy_cost,
        ),
    }
}

fn roll_skill_strings(skill: RollSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.general_tree.roll;
    match skill {
        RollSkill::Cost => splice_multiplier(
            i18n,
            "hud.skill.roll_energy_title",
            "hud.skill.roll_energy",
            modifiers.energy_cost,
        ),
        RollSkill::Strength => splice_multiplier(
            i18n,
            "hud.skill.roll_speed_title",
            "hud.skill.roll_speed",
            modifiers.strength,
        ),
        RollSkill::Duration => splice_multiplier(
            i18n,
            "hud.skill.roll_dur_title",
            "hud.skill.roll_dur",
            modifiers.duration,
        ),
    }
}

fn climb_skill_strings(skill: ClimbSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.general_tree.climb;
    match skill {
        ClimbSkill::Cost => splice_multiplier(
            i18n,
            "hud.skill.climbing_cost_title",
            "hud.skill.climbing_cost",
            modifiers.energy_cost,
        ),
        ClimbSkill::Speed => splice_multiplier(
            i18n,
            "hud.skill.climbing_speed_title",
            "hud.skill.climbing_speed",
            modifiers.speed,
        ),
    }
}

fn swim_skill_strings(skill: SwimSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.general_tree.swim;
    match skill {
        SwimSkill::Speed => splice_multiplier(
            i18n,
            "hud.skill.swim_speed_title",
            "hud.skill.swim_speed",
            modifiers.speed,
        ),
    }
}

fn mining_skill_strings(skill: MiningSkill, i18n: &Localization) -> (&str, Cow<str>) {
    let modifiers = SKILL_MODIFIERS.mining_tree;
    match skill {
        MiningSkill::Speed => splice_multiplier(
            i18n,
            "hud.skill.pick_strike_speed_title",
            "hud.skill.pick_strike_speed",
            modifiers.speed,
        ),
        MiningSkill::OreGain => splice_constant(
            i18n,
            "hud.skill.pick_strike_oregain_title",
            "hud.skill.pick_strike_oregain",
            (modifiers.ore_gain * 100.0).round() as u32,
        ),
        MiningSkill::GemGain => splice_constant(
            i18n,
            "hud.skill.pick_strike_gemgain_title",
            "hud.skill.pick_strike_gemgain",
            (modifiers.gem_gain * 100.0).round() as u32,
        ),
    }
}

/// Helper function which takes title i18n key and description i18n key
/// and returns localized title and localized description replacing "{boost}"
/// placeholder with passed constant.
// TODO: do something better when we get to multimodifier skills
fn splice_constant<'loc>(
    i18n: &'loc Localization,
    title: &'loc str,
    desc: &str,
    constant: u32,
) -> (&'loc str, Cow<'loc, str>) {
    let title = i18n.get(title);
    let desc = i18n.get(desc);
    let desc = desc.replace("{boost}", &constant.to_string());

    (title, Cow::Owned(desc))
}

/// Helper function which takes title i18n key and description i18n key
/// and returns localized title and localized description replacing "{boost}"
/// placeholder with absolute value of percentage effect of multiplier.
// TODO: do something better when we get to multimodifier skills
fn splice_multiplier<'loc>(
    i18n: &'loc Localization,
    title: &'loc str,
    desc: &str,
    multipler: f32,
) -> (&'loc str, Cow<'loc, str>) {
    let percentage = hud::multiplier_to_percentage(multipler).abs();

    let title = i18n.get(title);
    let desc = i18n.get(desc);
    let desc = desc.replace("{boost}", &format!("{:.0}", percentage));

    (title, Cow::Owned(desc))
}

// Small helper function to get localized skill text.
fn localize<'loc>(
    i18n: &'loc Localization,
    title: &'loc str,
    desc: &'loc str,
) -> (&'loc str, Cow<'loc, str>) {
    let title = i18n.get(title);
    let desc = i18n.get(desc);

    (title, Cow::Borrowed(desc))
}
