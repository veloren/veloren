use super::{
    BLACK, CRITICAL_HP_COLOR, HP_COLOR, Position, PositionSpecifier, Show, TEXT_COLOR,
    UI_HIGHLIGHT_0, UI_MAIN, XP_COLOR,
    img_ids::{Imgs, ImgsRot},
    item_imgs::{ItemImgs, animate_by_pulse},
};
use crate::{
    GlobalState,
    game_input::GameInput,
    hud::{
        self,
        slots::{AbilitySlot, SlotManager},
        util,
    },
    ui::{
        ImageFrame, Tooltip, TooltipManager, Tooltipable,
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
    },
};
use client::{self, Client};
use common::{
    combat,
    comp::{
        self, Body, CharacterState, Energy, Health, Inventory, Poise, Stats,
        ability::{Ability, ActiveAbilities, AuxiliaryAbility, BASE_ABILITY_LIMIT},
        inventory::{
            item::{
                ItemI18n, ItemKind, MaterialStatManifest,
                item_key::ItemKey,
                tool::{AbilityContext, ToolKind},
            },
            slot::EquipSlot,
        },
        skills::{
            self, AxeSkill, BowSkill, ClimbSkill, HammerSkill, MiningSkill, SKILL_MODIFIERS,
            SceptreSkill, Skill, StaffSkill, SwimSkill, SwordSkill,
        },
        skillset::{SkillGroupKind, SkillSet},
    },
    resources::BattleMode,
    uid::Uid,
};
use conrod_core::{
    Color, Colorable, Labelable, Positionable, Sizeable, UiCell, Widget, WidgetCommon, color,
    image,
    position::Relative,
    widget::{self, Button, Image, Rectangle, State, Text},
    widget_ids,
};
use i18n::Localization;
use std::borrow::Cow;
use strum::{EnumIter, IntoEnumIterator};
use vek::*;
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
        active_bar_checkbox,
        active_bar_checkbox_label,
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
        skills[],
        skill_lock_imgs[],
        sword_bg,
        sword_stance_cleaving_text,
        sword_stance_agile_text,
        sword_stance_crippling_text,
        sword_stance_heavy_text,
        sword_stance_defensive_text,
        sword_stance_cleaving_shadow,
        sword_stance_agile_shadow,
        sword_stance_crippling_shadow,
        sword_stance_heavy_shadow,
        sword_stance_defensive_shadow,
        sword_stance_left_align,
        sword_stance_right_align,
        axe_bg,
        hammer_bg,
        bow_bg,
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
        sword_path_overlay,
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
        // Recipes
        recipe_groups[],
    }
}

#[derive(WidgetCommon)]
pub struct Diary<'a> {
    show: &'a Show,
    client: &'a Client,
    global_state: &'a GlobalState,
    skill_set: &'a SkillSet,
    active_abilities: &'a ActiveAbilities,
    inventory: &'a Inventory,
    char_state: &'a CharacterState,
    health: &'a Health,
    energy: &'a Energy,
    poise: &'a Poise,
    body: &'a Body,
    uid: &'a Uid,
    msm: &'a MaterialStatManifest,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    item_i18n: &'a ItemI18n,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    slot_manager: &'a mut SlotManager,
    pulse: f32,
    context: &'a AbilityContext,
    stats: Option<&'a Stats>,

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

#[expect(clippy::too_many_arguments)]
impl<'a> Diary<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        global_state: &'a GlobalState,
        skill_set: &'a SkillSet,
        active_abilities: &'a ActiveAbilities,
        inventory: &'a Inventory,
        char_state: &'a CharacterState,
        health: &'a Health,
        energy: &'a Energy,
        poise: &'a Poise,
        body: &'a Body,
        uid: &'a Uid,
        msm: &'a MaterialStatManifest,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        item_i18n: &'a ItemI18n,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        slot_manager: &'a mut SlotManager,
        pulse: f32,
        context: &'a AbilityContext,
        stats: Option<&'a Stats>,
    ) -> Self {
        Self {
            show,
            client,
            global_state,
            skill_set,
            active_abilities,
            inventory,
            char_state,
            health,
            energy,
            poise,
            body,
            uid,
            msm,
            imgs,
            item_imgs,
            fonts,
            localized_strings,
            item_i18n,
            rot_imgs,
            tooltip_manager,
            slot_manager,
            pulse,
            context,
            stats,
            common: widget::CommonBuilder::default(),
            created_btns_top_l: 0,
            created_btns_top_r: 0,
            created_btns_bot_l: 0,
            created_btns_bot_r: 0,
        }
    }
}

pub type SelectedSkillTree = SkillGroupKind;

pub enum Event {
    Close,
    ChangeSkillTree(SelectedSkillTree),
    UnlockSkill(Skill),
    ChangeSection(DiarySection),
    SelectExpBar(Option<SkillGroupKind>),
}

// Possible future sections: Bestiary ("Pokedex" of fought enemies), Weapon and
// armour catalogue, Achievements...
#[derive(EnumIter, PartialEq, Eq)]
pub enum DiarySection {
    SkillTrees,
    AbilitySelection,
    Character,
    Recipes,
}

impl DiarySection {
    fn title_key(&self) -> &'static str {
        match self {
            DiarySection::SkillTrees => "hud-diary-sections-skill_trees-title",
            DiarySection::AbilitySelection => "hud-diary-sections-abilities-title",
            DiarySection::Character => "hud-diary-sections-character-title",
            DiarySection::Recipes => "hud-diary-sections-recipes-title",
        }
    }
}

// Represents the SkillGroupKind items
// that have a skill tree in the diary
#[derive(EnumIter, PartialEq, Eq)]
pub enum DiarySkillTree {
    General,
    Sword,
    Axe,
    Hammer,
    Bow,
    Staff,
    Sceptre,
    Pick,
}

impl DiarySkillTree {
    fn title_key(&self) -> &'static str {
        match self {
            DiarySkillTree::General => "hud-skill_tree-general",
            DiarySkillTree::Sword => "hud-skill_tree-sword",
            DiarySkillTree::Axe => "hud-skill_tree-axe",
            DiarySkillTree::Hammer => "hud-skill_tree-hammer",
            DiarySkillTree::Bow => "hud-skill_tree-bow",
            DiarySkillTree::Staff => "hud-skill_tree-staff",
            DiarySkillTree::Sceptre => "hud-skill_tree-sceptre",
            DiarySkillTree::Pick => "hud-skill_tree-mining",
        }
    }

    fn to_skill_group(&self) -> SkillGroupKind {
        match self {
            DiarySkillTree::General => SkillGroupKind::General,
            DiarySkillTree::Sword => SkillGroupKind::Weapon(ToolKind::Sword),
            DiarySkillTree::Axe => SkillGroupKind::Weapon(ToolKind::Axe),
            DiarySkillTree::Hammer => SkillGroupKind::Weapon(ToolKind::Hammer),
            DiarySkillTree::Bow => SkillGroupKind::Weapon(ToolKind::Bow),
            DiarySkillTree::Staff => SkillGroupKind::Weapon(ToolKind::Staff),
            DiarySkillTree::Sceptre => SkillGroupKind::Weapon(ToolKind::Sceptre),
            DiarySkillTree::Pick => SkillGroupKind::Weapon(ToolKind::Pick),
        }
    }
}

pub struct DiaryState {
    ids: Ids,
    ability_page: usize,
    recipe_page: usize,
}

impl Widget for Diary<'_> {
    type Event = Vec<Event>;
    type State = DiaryState;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        DiaryState {
            ids: Ids::new(id_gen),
            ability_page: 0,
            recipe_page: 0,
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
        Text::new(&self.localized_strings.get_msg("hud-diary"))
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

        let sections_len = DiarySection::iter().enumerate().len();

        // Update len
        state.update(|s| {
            s.ids
                .section_imgs
                .resize(sections_len, &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.ids
                .section_btns
                .resize(sections_len, &mut ui.widget_id_generator())
        });

        for (i, section) in DiarySection::iter().enumerate() {
            let section_name = self.localized_strings.get_msg(section.title_key());

            let btn_img = {
                let img = match section {
                    DiarySection::AbilitySelection => self.imgs.spellbook_ico,
                    DiarySection::SkillTrees => self.imgs.skilltree_ico,
                    DiarySection::Character => self.imgs.stats_ico,
                    DiarySection::Recipes => self.imgs.crafting_ico,
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
                    &section_name,
                    "",
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

                let skill_trees_len = DiarySkillTree::iter().enumerate().len();

                // Skill Tree Selection
                state.update(|s| {
                    s.ids
                        .weapon_btns
                        .resize(skill_trees_len, &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .weapon_imgs
                        .resize(skill_trees_len, &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .lock_imgs
                        .resize(skill_trees_len, &mut ui.widget_id_generator())
                });

                // Draw skillgroup tab's icons
                for (i, skill_tree) in DiarySkillTree::iter().enumerate() {
                    let skill_tree_name = self.localized_strings.get_msg(skill_tree.title_key());
                    let skill_group = skill_tree.to_skill_group();

                    // Check if we have this skill tree unlocked
                    let locked = !self.skill_set.skill_group_accessible(skill_group);

                    // Weapon button image
                    let btn_img = {
                        let img = match skill_tree {
                            DiarySkillTree::General => self.imgs.swords_crossed,
                            DiarySkillTree::Sword => self.imgs.sword,
                            DiarySkillTree::Axe => self.imgs.axe,
                            DiarySkillTree::Hammer => self.imgs.hammer,
                            DiarySkillTree::Bow => self.imgs.bow,
                            DiarySkillTree::Staff => self.imgs.staff,
                            DiarySkillTree::Sceptre => self.imgs.sceptre,
                            DiarySkillTree::Pick => self.imgs.mining,
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
                        self.localized_strings.get_msg("hud-skill-not_unlocked")
                    } else {
                        Cow::Borrowed("")
                    };

                    let wpn_button = Button::image(border_image)
                        .w_h(50.0, 50.0)
                        .hover_image(hover_image)
                        .press_image(press_image)
                        .middle_of(state.ids.weapon_imgs[i])
                        .image_color(color)
                        .with_tooltip(
                            self.tooltip_manager,
                            &skill_tree_name,
                            &tooltip_txt,
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
                // Show as Exp bar below skillbar
                let exp_selected =
                    self.global_state.settings.interface.xp_bar_skillgroup == Some(*sel_tab);
                if Button::image(if !exp_selected {
                    self.imgs.checkbox
                } else {
                    self.imgs.checkbox_checked
                })
                .w_h(18.0, 18.0)
                .hover_image(if !exp_selected {
                    self.imgs.checkbox_mo
                } else {
                    self.imgs.checkbox_checked_mo
                })
                .press_image(if !exp_selected {
                    self.imgs.checkbox_press
                } else {
                    self.imgs.checkbox_checked
                })
                .top_right_with_margins_on(state.ids.exp_bar_frame, 50.0, -30.0)
                .set(state.ids.active_bar_checkbox, ui)
                .was_clicked()
                {
                    if self.global_state.settings.interface.xp_bar_skillgroup != Some(*sel_tab) {
                        events.push(Event::SelectExpBar(Some(*sel_tab)));
                    } else {
                        events.push(Event::SelectExpBar(None));
                    }
                }

                Text::new(&self.localized_strings.get_msg("hud-skill-set_as_exp_bar"))
                    .right_from(state.ids.active_bar_checkbox, 10.0)
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .graphics_for(state.ids.active_bar_checkbox)
                    .color(TEXT_COLOR)
                    .set(state.ids.active_bar_checkbox_label, ui);

                // Show EXP bar text on hover
                if ui
                    .widget_input(state.ids.exp_bar_frame)
                    .mouse()
                    .is_some_and(|m| m.is_over())
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

                Text::new(&self.localized_strings.get_msg_ctx(
                    "hud-skill-sp_available",
                    &i18n::fluent_args! {
                        "number" => available_pts,
                    },
                ))
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
                    SelectedSkillTree::Weapon(ToolKind::Axe) => {
                        self.handle_axe_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Hammer) => {
                        self.handle_hammer_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Bow) => {
                        self.handle_bow_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Staff) => {
                        self.handle_staff_skills_window(&diary_tooltip, state, ui, events)
                    },
                    SelectedSkillTree::Weapon(ToolKind::Sceptre) => {
                        self.handle_sceptre_skills_window(&diary_tooltip, state, ui, events)
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
                        .resize(BASE_ABILITY_LIMIT, &mut ui.widget_id_generator())
                });
                state.update(|s| {
                    s.ids
                        .active_abilities_keys
                        .resize(BASE_ABILITY_LIMIT, &mut ui.widget_id_generator())
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
                    content_source: &(
                        self.active_abilities,
                        self.inventory,
                        self.skill_set,
                        self.context,
                        Some(self.char_state),
                        self.stats,
                    ),
                    image_source: self.imgs,
                    slot_manager: Some(self.slot_manager),
                    pulse: 0.0,
                };

                for i in 0..BASE_ABILITY_LIMIT {
                    let ability_id = self
                        .active_abilities
                        .get_ability(
                            AbilityInput::Auxiliary(i),
                            Some(self.inventory),
                            Some(self.skill_set),
                            self.stats,
                        )
                        .ability_id(
                            Some(self.char_state),
                            Some(self.inventory),
                            Some(self.skill_set),
                            self.context,
                        );
                    let (ability_title, ability_desc) = if let Some(ability_id) = ability_id {
                        util::ability_description(ability_id, self.localized_strings)
                    } else {
                        (
                            Cow::Borrowed("Drag an ability here to use it."),
                            Cow::Borrowed(""),
                        )
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
                            &ability_title,
                            &ability_desc,
                            &diary_tooltip,
                            TEXT_COLOR,
                        )
                        .set(state.ids.active_abilities[i], ui);

                    // Display Slot Keybinding
                    let keys = &self.global_state.settings.controls;
                    let ability_key = [
                        GameInput::Slot1,
                        GameInput::Slot2,
                        GameInput::Slot3,
                        GameInput::Slot4,
                        GameInput::Slot5,
                    ]
                    .get(i)
                    .and_then(|input| keys.get_binding(*input))
                    .map(|key| key.display_shortest())
                    .unwrap_or_default();

                    Text::new(&ability_key)
                        .top_left_with_margins_on(state.ids.active_abilities[i], 0.0, 4.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(20))
                        .color(TEXT_COLOR)
                        .graphics_for(state.ids.active_abilities[i])
                        .set(state.ids.active_abilities_keys[i], ui);
                }

                let abilities: Vec<_> = ActiveAbilities::all_available_abilities(
                    Some(self.inventory),
                    Some(self.skill_set),
                )
                .into_iter()
                .map(|a| {
                    (
                        Ability::from(a).ability_id(
                            Some(self.char_state),
                            Some(self.inventory),
                            Some(self.skill_set),
                            self.context,
                        ),
                        a,
                    )
                })
                .collect();

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
                    content_source: &(
                        self.active_abilities,
                        self.inventory,
                        self.skill_set,
                        self.context,
                        Some(self.char_state),
                        self.stats,
                    ),
                    image_source: self.imgs,
                    slot_manager: Some(self.slot_manager),
                    pulse: 0.0,
                };

                let same_weap_kinds = self
                    .inventory
                    .equipped(EquipSlot::ActiveMainhand)
                    .zip(self.inventory.equipped(EquipSlot::ActiveOffhand))
                    .is_some_and(|(a, b)| {
                        if let (ItemKind::Tool(tool_a), ItemKind::Tool(tool_b)) =
                            (&*a.kind(), &*b.kind())
                        {
                            (a.ability_spec(), tool_a.kind) == (b.ability_spec(), tool_b.kind)
                        } else {
                            false
                        }
                    });

                for (id_index, (ability_id, ability)) in abilities
                    .iter()
                    .skip(ability_start)
                    .take(ABILITIES_PER_PAGE)
                    .enumerate()
                {
                    let (ability_title, ability_desc) =
                        util::ability_description(ability_id.unwrap_or(""), self.localized_strings);

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

                    if same_weap_kinds && let AuxiliaryAbility::MainWeapon(slot) = ability {
                        let ability = AuxiliaryAbility::OffWeapon(*slot);

                        let slot = AbilitySlot::Ability(ability);
                        slot_maker
                            .fabricate(slot, [100.0; 2])
                            .top_right_with_margins_on(align_state, 20.0 + image_offsets, 20.0)
                            .set(state.ids.abilities_dual[id_index], ui);
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
                    Text::new(&ability_title)
                        .top_left_with_margins_on(state.ids.abilities[id_index], 5.0, 110.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(28))
                        .color(TEXT_COLOR)
                        .w(text_width)
                        .graphics_for(state.ids.abilities[id_index])
                        .set(state.ids.ability_titles[id_index], ui);
                    Text::new(&ability_desc)
                        .top_left_with_margins_on(state.ids.abilities[id_index], 40.0, 110.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(13))
                        .color(TEXT_COLOR)
                        .w(text_width)
                        .graphics_for(state.ids.abilities[id_index])
                        .set(state.ids.ability_descs[id_index], ui);
                }

                events
            },
            DiarySection::Character => {
                // Background Art
                Image::new(self.imgs.book_bg)
                    .w_h(299.0 * 4.0, 184.0 * 4.0)
                    .mid_top_with_margin_on(state.ids.content_align, 4.0)
                    .set(state.ids.spellbook_art, ui);

                if state.ids.stat_names.len() < STAT_COUNT {
                    state.update(|s| {
                        s.ids
                            .stat_names
                            .resize(STAT_COUNT, &mut ui.widget_id_generator());
                        s.ids
                            .stat_values
                            .resize(STAT_COUNT, &mut ui.widget_id_generator());
                    });
                }

                for (i, stat) in CharacterStat::iter().enumerate() {
                    // Stat names
                    let localized_name = stat.localized_str(self.localized_strings);
                    let mut txt = Text::new(&localized_name)
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
                        .and_then(|item| match &*item.kind() {
                            ItemKind::Tool(tool) => {
                                Some(tool.stats(item.stats_durability_multiplier()))
                            },
                            _ => None,
                        });

                    let off_weap_stats = self
                        .inventory
                        .equipped(EquipSlot::ActiveOffhand)
                        .and_then(|item| match &*item.kind() {
                            ItemKind::Tool(tool) => {
                                Some(tool.stats(item.stats_durability_multiplier()))
                            },
                            _ => None,
                        });

                    let (name, _gender, battle_mode) = self
                        .client
                        .player_list()
                        .get(self.uid)
                        .and_then(|info| info.character.as_ref())
                        .map_or_else(
                            || ("Unknown".to_string(), None, BattleMode::PvP),
                            |character_info| {
                                (
                                    self.localized_strings.get_content(&character_info.name),
                                    character_info.gender,
                                    character_info.battle_mode,
                                )
                            },
                        );

                    // Stat values
                    let value = match stat {
                        CharacterStat::Name => name,
                        CharacterStat::BattleMode => match battle_mode {
                            BattleMode::PvP => "PvP".to_string(),
                            BattleMode::PvE => "PvE".to_string(),
                        },
                        CharacterStat::Waypoint => self
                            .client
                            .waypoint()
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| "Unknown".to_string()),
                        CharacterStat::Hitpoints => format!("{}", self.health.base_max() as u32),
                        CharacterStat::Energy => format!("{}", self.energy.base_max() as u32),
                        CharacterStat::Poise => format!("{}", self.poise.base_max() as u32),
                        CharacterStat::CombatRating => {
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
                        CharacterStat::Protection => {
                            let protection =
                                combat::compute_protection(Some(self.inventory), self.msm);
                            match protection {
                                Some(prot) => format!("{}", prot),
                                None => String::from("Invincible"),
                            }
                        },
                        CharacterStat::StunResistance => {
                            let stun_res = Poise::compute_poise_damage_reduction(
                                Some(self.inventory),
                                self.msm,
                                None,
                                self.stats,
                            );
                            format!("{:.2}%", stun_res * 100.0)
                        },
                        CharacterStat::PrecisionPower => {
                            let precision_power =
                                combat::compute_precision_mult(Some(self.inventory), self.msm);
                            format!("x{:.2}", precision_power)
                        },
                        CharacterStat::EnergyReward => {
                            let energy_rew =
                                combat::compute_energy_reward_mod(Some(self.inventory), self.msm);
                            format!("{:+.0}%", (energy_rew - 1.0) * 100.0)
                        },
                        CharacterStat::Stealth => {
                            let stealth_perception_multiplier =
                                combat::perception_dist_multiplier_from_stealth(
                                    Some(self.inventory),
                                    None,
                                    self.msm,
                                );
                            let txt =
                                format!("{:+.1}%", (1.0 - stealth_perception_multiplier) * 100.0);

                            txt
                        },
                        CharacterStat::WeaponPower => match (main_weap_stats, off_weap_stats) {
                            (Some(m_stats), Some(o_stats)) => {
                                format!("{}   {}", m_stats.power * 10.0, o_stats.power * 10.0)
                            },
                            (Some(stats), None) | (None, Some(stats)) => {
                                format!("{}", stats.power * 10.0)
                            },
                            (None, None) => String::new(),
                        },
                        CharacterStat::WeaponSpeed => {
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
                        CharacterStat::WeaponEffectPower => match (main_weap_stats, off_weap_stats)
                        {
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
                    };

                    let mut number = Text::new(&value)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(29))
                        .color(BLACK);

                    if i == 0 {
                        number = number.right_from(state.ids.stat_names[i], 165.0);
                    } else {
                        number = number.down_from(state.ids.stat_values[i - 1], 10.0);
                    };
                    number.set(state.ids.stat_values[i], ui);
                }

                events
            },
            DiarySection::Recipes => {
                // Background Art
                Image::new(self.imgs.book_bg)
                    .w_h(299.0 * 4.0, 184.0 * 4.0)
                    .mid_top_with_margin_on(state.ids.content_align, 4.0)
                    .set(state.ids.spellbook_art, ui);

                Rectangle::fill_with([299.0 * 2.0, 184.0 * 4.0], color::TRANSPARENT)
                    .top_left_with_margins_on(state.ids.spellbook_art, 0.0, 0.0)
                    .set(state.ids.sb_page_left_align, ui);
                Rectangle::fill_with([299.0 * 2.0, 184.0 * 4.0], color::TRANSPARENT)
                    .top_right_with_margins_on(state.ids.spellbook_art, 0.0, 0.0)
                    .set(state.ids.sb_page_right_align, ui);

                const RECIPES_PER_PAGE: usize = 36;

                let page_index_max =
                    self.inventory.recipe_groups_iter().len().saturating_sub(1) / RECIPES_PER_PAGE;

                if state.recipe_page > page_index_max {
                    state.update(|s| s.recipe_page = 0);
                }

                // Page button
                // Left Arrow
                let left_arrow = Button::image(if state.recipe_page > 0 {
                    self.imgs.arrow_l
                } else {
                    self.imgs.arrow_l_inactive
                })
                .bottom_left_with_margins_on(state.ids.spellbook_art, -83.0, 10.0)
                .w_h(48.0, 55.0);
                // Grey out arrows when inactive
                if state.recipe_page > 0 {
                    if left_arrow
                        .hover_image(self.imgs.arrow_l_click)
                        .press_image(self.imgs.arrow_l)
                        .set(state.ids.ability_page_left, ui)
                        .was_clicked()
                    {
                        state.update(|s| s.recipe_page -= 1);
                    }
                } else {
                    left_arrow.set(state.ids.ability_page_left, ui);
                }
                // Right Arrow
                let right_arrow = Button::image(if state.recipe_page < page_index_max {
                    self.imgs.arrow_r
                } else {
                    self.imgs.arrow_r_inactive
                })
                .bottom_right_with_margins_on(state.ids.spellbook_art, -83.0, 10.0)
                .w_h(48.0, 55.0);
                if state.recipe_page < page_index_max {
                    // Only show right button if not on last page
                    if right_arrow
                        .hover_image(self.imgs.arrow_r_click)
                        .press_image(self.imgs.arrow_r)
                        .set(state.ids.ability_page_right, ui)
                        .was_clicked()
                    {
                        state.update(|s| s.recipe_page += 1);
                    };
                } else {
                    right_arrow.set(state.ids.ability_page_right, ui);
                }

                state.update(|s| {
                    s.ids
                        .recipe_groups
                        .resize(RECIPES_PER_PAGE, &mut ui.widget_id_generator())
                });

                for (i, rg) in self
                    .inventory
                    .recipe_groups_iter()
                    .skip(state.recipe_page * RECIPES_PER_PAGE)
                    .take(RECIPES_PER_PAGE)
                    .enumerate()
                {
                    let (title, _desc) =
                        util::item_text(rg, self.localized_strings, self.item_i18n);

                    let mut text = Text::new(&title)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(29))
                        .color(BLACK);

                    if i == 0 {
                        text =
                            text.top_left_with_margins_on(state.ids.sb_page_left_align, 20.0, 20.0);
                    } else if i == 18 {
                        text = text.top_left_with_margins_on(
                            state.ids.sb_page_right_align,
                            20.0,
                            20.0,
                        );
                    } else {
                        text = text.down_from(state.ids.recipe_groups[i - 1], 10.0);
                    }
                    text.set(state.ids.recipe_groups[i], ui);
                }

                events
            },
        }
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
    Ability {
        skill: Skill,
        ability_id: &'a str,
        position: PositionSpecifier,
    },
}

impl Diary<'_> {
    fn handle_general_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        let tree_title = &self.localized_strings.get_msg("common-weapons-general");
        Text::new(tree_title)
            .mid_top_with_margin_on(state.ids.content_align, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(34))
            .color(TEXT_COLOR)
            .set(state.ids.tree_title_txt, ui);

        // Number of skills per rectangle per weapon, start counting at 0
        // Maximum of 9 skills/8 indices
        let skills_top_l = 6;
        let skills_top_r = 0;
        let skills_bot_l = 0;
        let skills_bot_r = 5;

        self.setup_state_for_skill_icons(
            state,
            ui,
            skills_top_l,
            skills_top_r,
            skills_bot_l,
            skills_bot_r,
        );

        use SkillGroupKind::*;
        use ToolKind::*;
        // General Combat
        Image::new(animate_by_pulse(
            &self.item_imgs.img_ids_or_not_found_img(ItemKey::Simple(
                "example_general_combat_left".to_string(),
            )),
            self.pulse,
        ))
        .wh(ART_SIZE)
        .middle_of(state.ids.content_align)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
        .set(state.ids.general_combat_render_0, ui);

        Image::new(animate_by_pulse(
            &self.item_imgs.img_ids_or_not_found_img(ItemKey::Simple(
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
            // Bottom left skills
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Sword)),
                image: self.imgs.unlock_sword_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[0], 3.0),
                id: state.ids.skill_general_tree_0,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Axe)),
                image: self.imgs.unlock_axe_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[1], 3.0),
                id: state.ids.skill_general_tree_1,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Hammer)),
                image: self.imgs.unlock_hammer_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[2], 3.0),
                id: state.ids.skill_general_tree_2,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Bow)),
                image: self.imgs.unlock_bow_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[3], 3.0),
                id: state.ids.skill_general_tree_3,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Staff)),
                image: self.imgs.unlock_staff_skill0,
                position: MidTopWithMarginOn(state.ids.skills_top_l[4], 3.0),
                id: state.ids.skill_general_tree_4,
            },
            SkillIcon::Unlockable {
                skill: Skill::UnlockGroup(Weapon(Sceptre)),
                image: self.imgs.unlock_sceptre_skill,
                position: MidTopWithMarginOn(state.ids.skills_top_l[5], 3.0),
                id: state.ids.skill_general_tree_5,
            },
            // Bottom right skills
            SkillIcon::Descriptive {
                title: "hud-skill-climbing_title",
                desc: "hud-skill-climbing",
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
                title: "hud-skill-swim_title",
                desc: "hud-skill-swim",
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

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip, state);
        events
    }

    fn handle_sword_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        Image::new(self.imgs.sword_tree_paths)
            .wh([1042.0, 636.0])
            .mid_top_with_margin_on(state.ids.content_align, 55.0)
            .graphics_for(state.ids.content_align)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
            .set(state.ids.sword_path_overlay, ui);

        // Sword
        Image::new(self.imgs.sword_bg)
            .wh([933.0, 615.0])
            .mid_top_with_margin_on(state.ids.content_align, 65.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
            .set(state.ids.sword_bg, ui);

        // Stances:
        // For alignment purposes
        Rectangle::fill_with([169.0, 615.0], color::TRANSPARENT)
            .top_left_of(state.ids.sword_bg)
            .set(state.ids.sword_stance_left_align, ui);
        Rectangle::fill_with([169.0, 615.0], color::TRANSPARENT)
            .top_right_of(state.ids.sword_bg)
            .set(state.ids.sword_stance_right_align, ui);

        // Cleaving
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_cleaving"),
        )
        .mid_top_with_margin_on(state.ids.sword_stance_left_align, -7.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(Color::Rgba(0.94, 0.54, 0.07, 1.0))
        .set(state.ids.sword_stance_cleaving_text, ui);

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_cleaving"),
        )
        .x_y_position_relative_to(
            state.ids.sword_stance_cleaving_text,
            Relative::Scalar(2.5),
            Relative::Scalar(-2.5),
        )
        .depth(1.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(color::BLACK)
        .set(state.ids.sword_stance_cleaving_shadow, ui);

        // Agile
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_agile"),
        )
        .mid_top_with_margin_on(state.ids.sword_bg, -7.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(Color::Rgba(0.81, 0.70, 0.08, 1.0))
        .set(state.ids.sword_stance_agile_text, ui);

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_agile"),
        )
        .x_y_position_relative_to(
            state.ids.sword_stance_agile_text,
            Relative::Scalar(2.5),
            Relative::Scalar(-2.5),
        )
        .depth(1.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(color::BLACK)
        .set(state.ids.sword_stance_agile_shadow, ui);

        // Crippling
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_crippling"),
        )
        .mid_top_with_margin_on(state.ids.sword_stance_right_align, -7.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(Color::Rgba(0.0, 0.52, 0.0, 1.0))
        .set(state.ids.sword_stance_crippling_text, ui);

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_crippling"),
        )
        .x_y_position_relative_to(
            state.ids.sword_stance_crippling_text,
            Relative::Scalar(2.5),
            Relative::Scalar(-2.5),
        )
        .depth(1.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(color::BLACK)
        .set(state.ids.sword_stance_crippling_shadow, ui);

        // Heavy
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_heavy"),
        )
        .mid_bottom_with_margin_on(state.ids.sword_stance_left_align, 272.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(Color::Rgba(0.67, 0.0, 0.0, 1.0))
        .set(state.ids.sword_stance_heavy_text, ui);

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_heavy"),
        )
        .x_y_position_relative_to(
            state.ids.sword_stance_heavy_text,
            Relative::Scalar(2.5),
            Relative::Scalar(-2.5),
        )
        .depth(1.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(color::BLACK)
        .set(state.ids.sword_stance_heavy_shadow, ui);

        // Defensive
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_defensive"),
        )
        .mid_bottom_with_margin_on(state.ids.sword_stance_right_align, 272.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(Color::Rgba(0.10, 0.40, 0.82, 1.0))
        .set(state.ids.sword_stance_defensive_text, ui);

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-skill-sword_stance_defensive"),
        )
        .x_y_position_relative_to(
            state.ids.sword_stance_defensive_text,
            Relative::Scalar(2.5),
            Relative::Scalar(-2.5),
        )
        .depth(1.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(34))
        .color(color::BLACK)
        .set(state.ids.sword_stance_defensive_shadow, ui);

        use PositionSpecifier::TopLeftWithMarginsOn;
        let skill_buttons = &[
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CrescentSlash),
                ability_id: "veloren.core.pseudo_abilities.sword.crescent_slash",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 537.0, 429.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::FellStrike),
                ability_id: "veloren.core.pseudo_abilities.sword.fell_strike",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 457.0, 527.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::Skewer),
                ability_id: "veloren.core.pseudo_abilities.sword.skewer",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 368.0, 527.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::Cascade),
                ability_id: "veloren.core.pseudo_abilities.sword.cascade",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 457.0, 332.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CrossCut),
                ability_id: "veloren.core.pseudo_abilities.sword.cross_cut",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 368.0, 332.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::Finisher),
                ability_id: "veloren.core.pseudo_abilities.sword.finisher",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 263.0, 429.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::HeavySweep),
                ability_id: "common.abilities.sword.heavy_sweep",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 457.0, 2.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::HeavyPommelStrike),
                ability_id: "common.abilities.sword.heavy_pommel_strike",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 457.0, 91.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::AgileQuickDraw),
                ability_id: "common.abilities.sword.agile_quick_draw",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 142.0, 384.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::AgileFeint),
                ability_id: "common.abilities.sword.agile_feint",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 142.0, 472.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::DefensiveRiposte),
                ability_id: "common.abilities.sword.defensive_riposte",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 457.0, 766.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::DefensiveDisengage),
                ability_id: "common.abilities.sword.defensive_disengage",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 457.0, 855.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CripplingGouge),
                ability_id: "common.abilities.sword.crippling_gouge",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 53.0, 766.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CripplingHamstring),
                ability_id: "common.abilities.sword.crippling_hamstring",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 142.0, 766.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CleavingWhirlwindSlice),
                ability_id: "common.abilities.sword.cleaving_whirlwind_slice",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 53.0, 91.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CleavingEarthSplitter),
                ability_id: "common.abilities.sword.cleaving_earth_splitter",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 142.0, 91.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::HeavyFortitude),
                ability_id: "common.abilities.sword.heavy_fortitude",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 368.0, 2.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::HeavyPillarThrust),
                ability_id: "common.abilities.sword.heavy_pillar_thrust",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 368.0, 91.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::AgileDancingEdge),
                ability_id: "common.abilities.sword.agile_dancing_edge",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 53.0, 385.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::AgileFlurry),
                ability_id: "common.abilities.sword.agile_flurry",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 53.0, 473.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::DefensiveStalwartSword),
                ability_id: "common.abilities.sword.defensive_stalwart_sword",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 368.0, 766.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::DefensiveDeflect),
                ability_id: "common.abilities.sword.defensive_deflect",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 368.0, 855.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CripplingEviscerate),
                ability_id: "common.abilities.sword.crippling_eviscerate",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 142.0, 855.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CripplingBloodyGash),
                ability_id: "common.abilities.sword.crippling_bloody_gash",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 53.0, 855.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CleavingBladeFever),
                ability_id: "common.abilities.sword.cleaving_blade_fever",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 53.0, 2.0),
            },
            SkillIcon::Ability {
                skill: Skill::Sword(SwordSkill::CleavingSkySplitter),
                ability_id: "common.abilities.sword.cleaving_sky_splitter",
                position: TopLeftWithMarginsOn(state.ids.sword_bg, 142.0, 2.0),
            },
        ];

        state.update(|s| {
            s.ids
                .skills
                .resize(skill_buttons.len(), &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.ids
                .skill_lock_imgs
                .resize(skill_buttons.len(), &mut ui.widget_id_generator())
        });

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip, state);
        events
    }

    fn handle_axe_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        // Axe
        Image::new(self.imgs.axe_bg)
            .wh([924.0, 619.0])
            .mid_top_with_margin_on(state.ids.content_align, 65.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
            .set(state.ids.axe_bg, ui);

        use PositionSpecifier::TopLeftWithMarginsOn;
        let skill_buttons = &[
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::BrutalSwing),
                ability_id: "common.abilities.axe.brutal_swing",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 387.0, 424.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Berserk),
                ability_id: "common.abilities.axe.berserk",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 287.0, 374.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::RisingTide),
                ability_id: "common.abilities.axe.rising_tide",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 287.0, 474.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::SavageSense),
                ability_id: "common.abilities.axe.savage_sense",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 187.0, 324.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::AdrenalineRush),
                ability_id: "common.abilities.axe.adrenaline_rush",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 187.0, 524.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Execute),
                ability_id: "common.abilities.axe.execute",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 187.0, 424.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Maelstrom),
                ability_id: "common.abilities.axe.maelstrom",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 4.0, 424.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Rake),
                ability_id: "common.abilities.axe.rake",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 507.0, 325.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Bloodfeast),
                ability_id: "common.abilities.axe.bloodfeast",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 387.0, 74.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::FierceRaze),
                ability_id: "common.abilities.axe.fierce_raze",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 387.0, 174.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Furor),
                ability_id: "common.abilities.axe.furor",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 287.0, 24.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Fracture),
                ability_id: "common.abilities.axe.fracture",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 287.0, 224.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Lacerate),
                ability_id: "common.abilities.axe.lacerate",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 287.0, 124.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Riptide),
                ability_id: "common.abilities.axe.riptide",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 104.0, 124.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::SkullBash),
                ability_id: "common.abilities.axe.skull_bash",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 507.0, 523.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Sunder),
                ability_id: "common.abilities.axe.sunder",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 387.0, 674.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Plunder),
                ability_id: "common.abilities.axe.plunder",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 387.0, 774.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Defiance),
                ability_id: "common.abilities.axe.defiance",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 287.0, 624.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Keelhaul),
                ability_id: "common.abilities.axe.keelhaul",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 287.0, 824.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Bulkhead),
                ability_id: "common.abilities.axe.bulkhead",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 287.0, 724.0),
            },
            SkillIcon::Ability {
                skill: Skill::Axe(AxeSkill::Capsize),
                ability_id: "common.abilities.axe.capsize",
                position: TopLeftWithMarginsOn(state.ids.axe_bg, 104.0, 724.0),
            },
        ];

        state.update(|s| {
            s.ids
                .skills
                .resize(skill_buttons.len(), &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.ids
                .skill_lock_imgs
                .resize(skill_buttons.len(), &mut ui.widget_id_generator())
        });

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip, state);
        events
    }

    fn handle_hammer_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        // Hammer
        Image::new(self.imgs.hammer_bg)
            .wh([924.0, 619.0])
            .mid_top_with_margin_on(state.ids.content_align, 65.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
            .set(state.ids.hammer_bg, ui);

        use PositionSpecifier::TopLeftWithMarginsOn;
        let skill_buttons = &[
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::ScornfulSwipe),
                ability_id: "common.abilities.hammer.scornful_swipe",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 455.0, 424.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Tremor),
                ability_id: "common.abilities.hammer.tremor",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 398.0, 172.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::VigorousBash),
                ability_id: "common.abilities.hammer.vigorous_bash",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 398.0, 272.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Retaliate),
                ability_id: "common.abilities.hammer.retaliate",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 284.0, 122.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::SpineCracker),
                ability_id: "common.abilities.hammer.spine_cracker",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 284.0, 222.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Breach),
                ability_id: "common.abilities.hammer.breach",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 284.0, 322.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::IronTempest),
                ability_id: "common.abilities.hammer.iron_tempest",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 170.0, 172.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Upheaval),
                ability_id: "common.abilities.hammer.upheaval",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 170.0, 272.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Thunderclap),
                ability_id: "common.abilities.hammer.thunderclap",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 56.0, 172.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::SeismicShock),
                ability_id: "common.abilities.hammer.seismic_shock",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 56.0, 272.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::HeavyWhorl),
                ability_id: "common.abilities.hammer.heavy_whorl",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 398.0, 576.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Intercept),
                ability_id: "common.abilities.hammer.intercept",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 398.0, 676.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::PileDriver),
                ability_id: "common.abilities.hammer.pile_driver",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 284.0, 526.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::LungPummel),
                ability_id: "common.abilities.hammer.lung_pummel",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 284.0, 626.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::HelmCrusher),
                ability_id: "common.abilities.hammer.helm_crusher",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 284.0, 726.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Rampart),
                ability_id: "common.abilities.hammer.rampart",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 170.0, 576.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Tenacity),
                ability_id: "common.abilities.hammer.tenacity",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 170.0, 676.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Earthshaker),
                ability_id: "common.abilities.hammer.earthshaker",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 56.0, 576.0),
            },
            SkillIcon::Ability {
                skill: Skill::Hammer(HammerSkill::Judgement),
                ability_id: "common.abilities.hammer.judgement",
                position: TopLeftWithMarginsOn(state.ids.hammer_bg, 56.0, 676.0),
            },
        ];

        state.update(|s| {
            s.ids
                .skills
                .resize(skill_buttons.len(), &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.ids
                .skill_lock_imgs
                .resize(skill_buttons.len(), &mut ui.widget_id_generator())
        });

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip, state);
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
        let tree_title = &self.localized_strings.get_msg("common-weapons-sceptre");

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
                .img_ids_or_not_found_img(ItemKey::Simple("example_sceptre".to_string())),
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
                title: "hud-skill-sc_lifesteal_title",
                desc: "hud-skill-sc_lifesteal",
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
                title: "hud-skill-sc_heal_title",
                desc: "hud-skill-sc_heal",
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

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip, state);
        events
    }

    fn handle_bow_skills_window(
        &mut self,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
        ui: &mut UiCell,
        mut events: Vec<Event>,
    ) -> Vec<Event> {
        Image::new(self.imgs.bow_bg)
            .wh([924.0, 619.0])
            .mid_top_with_margin_on(state.ids.content_align, 65.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
            .set(state.ids.bow_bg, ui);

        use PositionSpecifier::TopLeftWithMarginsOn;
        let skill_buttons = &[
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::Foothold),
                ability_id: "common.abilities.bow.foothold",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 424.0, 480.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::SnareShot),
                ability_id: "common.abilities.bow.snare_shot",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 424.0, 368.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::ArdentHunt),
                ability_id: "common.abilities.bow.ardent_hunt",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 310.0, 204.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::HeavyNock),
                ability_id: "common.abilities.bow.heavy_nock",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 310.0, 424.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::Barrage),
                ability_id: "common.abilities.bow.barrage",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 310.0, 644.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::OwlTalon),
                ability_id: "common.abilities.bow.owl_talon",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 196.0, 154.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::EagleEye),
                ability_id: "common.abilities.bow.eagle_eye",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 196.0, 254.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::IgniteArrow),
                ability_id: "common.abilities.bow.ignite_arrow",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 196.0, 374.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::DrenchArrow),
                ability_id: "common.abilities.bow.drench_arrow",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 196.0, 474.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::PiercingGale),
                ability_id: "common.abilities.bow.piercing_gale",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 196.0, 594.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::Scatterburst),
                ability_id: "common.abilities.bow.scatterburst",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 196.0, 694.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::Heartseeker),
                ability_id: "common.abilities.bow.heartseeker",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 82.0, 154.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::Hawkstrike),
                ability_id: "common.abilities.bow.hawkstrike",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 82.0, 254.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::FreezeArrow),
                ability_id: "common.abilities.bow.freeze_arrow",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 82.0, 374.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::JoltArrow),
                ability_id: "common.abilities.bow.jolt_arrow",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 82.0, 474.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::Fusillade),
                ability_id: "common.abilities.bow.fusillade",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 82.0, 594.0),
            },
            SkillIcon::Ability {
                skill: Skill::Bow(BowSkill::DeathVolley),
                ability_id: "common.abilities.bow.death_volley",
                position: TopLeftWithMarginsOn(state.ids.bow_bg, 82.0, 694.0),
            },
        ];

        state.update(|s| {
            s.ids
                .skills
                .resize(skill_buttons.len(), &mut ui.widget_id_generator())
        });
        state.update(|s| {
            s.ids
                .skill_lock_imgs
                .resize(skill_buttons.len(), &mut ui.widget_id_generator())
        });

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip, state);
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
        let tree_title = &self.localized_strings.get_msg("common-weapons-staff");

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
                .img_ids_or_not_found_img(ItemKey::Simple("example_staff_fire".to_string())),
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
                title: "hud-skill-st_fireball_title",
                desc: "hud-skill-st_fireball",
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
                title: "hud-skill-st_flamethrower_title",
                desc: "hud-skill-st_flamethrower",
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

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip, state);
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
        let tree_title = &self.localized_strings.get_msg("common-tool-mining");

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
                .img_ids_or_not_found_img(ItemKey::Simple("example_pick".to_string())),
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
                title: "hud-skill-pick_strike_title",
                desc: "hud-skill-pick_strike",
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

        self.handle_skill_buttons(skill_buttons, ui, &mut events, diary_tooltip, state);
        events
    }

    fn handle_skill_buttons(
        &mut self,
        icons: &[SkillIcon],
        ui: &mut UiCell,
        events: &mut Vec<Event>,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
    ) {
        for (i, icon) in icons.iter().enumerate() {
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
                            &self.localized_strings.get_msg(title),
                            &self.localized_strings.get_msg(desc),
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
                SkillIcon::Ability {
                    skill,
                    ability_id,
                    position,
                } => self.create_unlock_ability_button(
                    *skill,
                    ability_id,
                    *position,
                    i,
                    ui,
                    events,
                    diary_tooltip,
                    state,
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

        let skill_strings = skill_strings(skill);
        let (title, description) =
            skill_strings.localize(self.localized_strings, self.skill_set, skill);

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
                &title,
                &description,
                diary_tooltip,
                TEXT_COLOR,
            )
            .set(widget_id, ui);

        if button.was_clicked() {
            events.push(Event::UnlockSkill(skill));
        };
    }

    fn create_unlock_ability_button(
        &mut self,
        skill: Skill,
        ability_id: &str,
        position: PositionSpecifier,
        widget_index: usize,
        ui: &mut UiCell,
        events: &mut Vec<Event>,
        diary_tooltip: &Tooltip,
        state: &mut State<DiaryState>,
    ) {
        let locked = !self.skill_set.prerequisites_met(skill);
        let owned = self.skill_set.has_skill(skill);
        let image_color = if owned {
            TEXT_COLOR
        } else {
            Color::Rgba(0.41, 0.41, 0.41, 0.7)
        };

        let (title, description) = util::ability_description(ability_id, self.localized_strings);

        let sp_cost = sp(self.localized_strings, self.skill_set, skill);

        let description = format!("{description}\n{sp_cost}");

        let button = Button::image(util::ability_image(self.imgs, ability_id))
            .w_h(76.0, 76.0)
            .position(position)
            .image_color(image_color)
            .with_tooltip(
                self.tooltip_manager,
                &title,
                &description,
                diary_tooltip,
                TEXT_COLOR,
            )
            .set(state.ids.skills[widget_index], ui);

        // Lock Image
        if locked {
            Image::new(self.imgs.lock)
                .w_h(76.0, 76.0)
                .middle_of(state.ids.skills[widget_index])
                .graphics_for(state.ids.skills[widget_index])
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.8)))
                .set(state.ids.skill_lock_imgs[widget_index], ui);
        }

        if button.was_clicked() {
            events.push(Event::UnlockSkill(skill));
        };
    }
}

/// Returns skill info as a tuple of title and description.
///
/// If you want to get localized version, use `SkillStrings::localize` method
fn skill_strings(skill: Skill) -> SkillStrings<'static> {
    match skill {
        // general tree
        Skill::UnlockGroup(s) => unlock_skill_strings(s),
        // weapon trees
        Skill::Staff(s) => staff_skill_strings(s),
        Skill::Sceptre(s) => sceptre_skill_strings(s),
        // movement trees
        Skill::Climb(s) => climb_skill_strings(s),
        Skill::Swim(s) => swim_skill_strings(s),
        // mining
        Skill::Pick(s) => mining_skill_strings(s),
        _ => SkillStrings::plain("", ""),
    }
}

fn unlock_skill_strings(group: SkillGroupKind) -> SkillStrings<'static> {
    match group {
        SkillGroupKind::Weapon(ToolKind::Sword) => {
            SkillStrings::plain("hud-skill-unlck_sword_title", "hud-skill-unlck_sword")
        },
        SkillGroupKind::Weapon(ToolKind::Axe) => {
            SkillStrings::plain("hud-skill-unlck_axe_title", "hud-skill-unlck_axe")
        },
        SkillGroupKind::Weapon(ToolKind::Hammer) => {
            SkillStrings::plain("hud-skill-unlck_hammer_title", "hud-skill-unlck_hammer")
        },
        SkillGroupKind::Weapon(ToolKind::Bow) => {
            SkillStrings::plain("hud-skill-unlck_bow_title", "hud-skill-unlck_bow")
        },
        SkillGroupKind::Weapon(ToolKind::Staff) => {
            SkillStrings::plain("hud-skill-unlck_staff_title", "hud-skill-unlck_staff")
        },
        SkillGroupKind::Weapon(ToolKind::Sceptre) => {
            SkillStrings::plain("hud-skill-unlck_sceptre_title", "hud-skill-unlck_sceptre")
        },
        SkillGroupKind::General
        | SkillGroupKind::Weapon(
            ToolKind::Dagger
            | ToolKind::Shield
            | ToolKind::Spear
            | ToolKind::Blowgun
            | ToolKind::Debug
            | ToolKind::Farming
            | ToolKind::Instrument
            | ToolKind::Throwable
            | ToolKind::Pick
            | ToolKind::Shovel
            | ToolKind::Natural
            | ToolKind::Empty,
        ) => {
            tracing::warn!("Requesting title for unlocking unexpected skill group");
            SkillStrings::Empty
        },
    }
}

fn staff_skill_strings(skill: StaffSkill) -> SkillStrings<'static> {
    let modifiers = SKILL_MODIFIERS.staff_tree;
    match skill {
        // Basic ranged upgrades
        StaffSkill::BDamage => SkillStrings::with_mult(
            "hud-skill-st_damage_title",
            "hud-skill-st_damage",
            modifiers.fireball.power,
        ),
        StaffSkill::BRegen => SkillStrings::with_mult(
            "hud-skill-st_energy_regen_title",
            "hud-skill-st_energy_regen",
            modifiers.fireball.regen,
        ),
        StaffSkill::BRadius => SkillStrings::with_mult(
            "hud-skill-st_explosion_radius_title",
            "hud-skill-st_explosion_radius",
            modifiers.fireball.range,
        ),
        // Flamethrower upgrades
        StaffSkill::FDamage => SkillStrings::with_mult(
            "hud-skill-st_flamethrower_damage_title",
            "hud-skill-st_flamethrower_damage",
            modifiers.flamethrower.damage,
        ),
        StaffSkill::FRange => SkillStrings::with_mult(
            "hud-skill-st_flamethrower_range_title",
            "hud-skill-st_flamethrower_range",
            modifiers.flamethrower.range,
        ),
        StaffSkill::FDrain => SkillStrings::with_mult(
            "hud-skill-st_energy_drain_title",
            "hud-skill-st_energy_drain",
            modifiers.flamethrower.energy_drain,
        ),
        StaffSkill::FVelocity => SkillStrings::with_mult(
            "hud-skill-st_flame_velocity_title",
            "hud-skill-st_flame_velocity",
            modifiers.flamethrower.velocity,
        ),
        // Shockwave upgrades
        StaffSkill::UnlockShockwave => SkillStrings::plain(
            "hud-skill-st_shockwave_unlock_title",
            "hud-skill-st_shockwave_unlock",
        ),
        StaffSkill::SDamage => SkillStrings::with_mult(
            "hud-skill-st_shockwave_damage_title",
            "hud-skill-st_shockwave_damage",
            modifiers.shockwave.damage,
        ),
        StaffSkill::SKnockback => SkillStrings::with_mult(
            "hud-skill-st_shockwave_knockback_title",
            "hud-skill-st_shockwave_knockback",
            modifiers.shockwave.knockback,
        ),
        StaffSkill::SRange => SkillStrings::with_mult(
            "hud-skill-st_shockwave_range_title",
            "hud-skill-st_shockwave_range",
            modifiers.shockwave.duration,
        ),
        StaffSkill::SCost => SkillStrings::with_mult(
            "hud-skill-st_shockwave_cost_title",
            "hud-skill-st_shockwave_cost",
            modifiers.shockwave.energy_cost,
        ),
    }
}

fn sceptre_skill_strings(skill: SceptreSkill) -> SkillStrings<'static> {
    let modifiers = SKILL_MODIFIERS.sceptre_tree;
    match skill {
        // Lifesteal beam upgrades
        SceptreSkill::LDamage => SkillStrings::with_mult(
            "hud-skill-sc_lifesteal_damage_title",
            "hud-skill-sc_lifesteal_damage",
            modifiers.beam.damage,
        ),
        SceptreSkill::LRange => SkillStrings::with_mult(
            "hud-skill-sc_lifesteal_range_title",
            "hud-skill-sc_lifesteal_range",
            modifiers.beam.range,
        ),
        SceptreSkill::LLifesteal => SkillStrings::with_mult(
            "hud-skill-sc_lifesteal_lifesteal_title",
            "hud-skill-sc_lifesteal_lifesteal",
            modifiers.beam.lifesteal,
        ),
        SceptreSkill::LRegen => SkillStrings::with_mult(
            "hud-skill-sc_lifesteal_regen_title",
            "hud-skill-sc_lifesteal_regen",
            modifiers.beam.energy_regen,
        ),
        // Healing aura upgrades
        SceptreSkill::HHeal => SkillStrings::with_mult(
            "hud-skill-sc_heal_heal_title",
            "hud-skill-sc_heal_heal",
            modifiers.healing_aura.strength,
        ),
        SceptreSkill::HRange => SkillStrings::with_mult(
            "hud-skill-sc_heal_range_title",
            "hud-skill-sc_heal_range",
            modifiers.healing_aura.range,
        ),
        SceptreSkill::HDuration => SkillStrings::with_mult(
            "hud-skill-sc_heal_duration_title",
            "hud-skill-sc_heal_duration",
            modifiers.healing_aura.duration,
        ),
        SceptreSkill::HCost => SkillStrings::with_mult(
            "hud-skill-sc_heal_cost_title",
            "hud-skill-sc_heal_cost",
            modifiers.healing_aura.energy_cost,
        ),
        // Warding aura upgrades
        SceptreSkill::UnlockAura => SkillStrings::plain(
            "hud-skill-sc_wardaura_unlock_title",
            "hud-skill-sc_wardaura_unlock",
        ),
        SceptreSkill::AStrength => SkillStrings::with_mult(
            "hud-skill-sc_wardaura_strength_title",
            "hud-skill-sc_wardaura_strength",
            modifiers.warding_aura.strength,
        ),
        SceptreSkill::ADuration => SkillStrings::with_mult(
            "hud-skill-sc_wardaura_duration_title",
            "hud-skill-sc_wardaura_duration",
            modifiers.warding_aura.duration,
        ),
        SceptreSkill::ARange => SkillStrings::with_mult(
            "hud-skill-sc_wardaura_range_title",
            "hud-skill-sc_wardaura_range",
            modifiers.warding_aura.range,
        ),
        SceptreSkill::ACost => SkillStrings::with_mult(
            "hud-skill-sc_wardaura_cost_title",
            "hud-skill-sc_wardaura_cost",
            modifiers.warding_aura.energy_cost,
        ),
    }
}

fn climb_skill_strings(skill: ClimbSkill) -> SkillStrings<'static> {
    let modifiers = SKILL_MODIFIERS.general_tree.climb;
    match skill {
        ClimbSkill::Cost => SkillStrings::with_mult(
            "hud-skill-climbing_cost_title",
            "hud-skill-climbing_cost",
            modifiers.energy_cost,
        ),
        ClimbSkill::Speed => SkillStrings::with_mult(
            "hud-skill-climbing_speed_title",
            "hud-skill-climbing_speed",
            modifiers.speed,
        ),
    }
}

fn swim_skill_strings(skill: SwimSkill) -> SkillStrings<'static> {
    let modifiers = SKILL_MODIFIERS.general_tree.swim;
    match skill {
        SwimSkill::Speed => SkillStrings::with_mult(
            "hud-skill-swim_speed_title",
            "hud-skill-swim_speed",
            modifiers.speed,
        ),
    }
}

fn mining_skill_strings(skill: MiningSkill) -> SkillStrings<'static> {
    let modifiers = SKILL_MODIFIERS.mining_tree;
    match skill {
        MiningSkill::Speed => SkillStrings::with_mult(
            "hud-skill-pick_strike_speed_title",
            "hud-skill-pick_strike_speed",
            modifiers.speed,
        ),
        MiningSkill::OreGain => SkillStrings::with_const(
            "hud-skill-pick_strike_oregain_title",
            "hud-skill-pick_strike_oregain",
            (modifiers.ore_gain * 100.0).round() as u32,
        ),
        MiningSkill::GemGain => SkillStrings::with_const(
            "hud-skill-pick_strike_gemgain_title",
            "hud-skill-pick_strike_gemgain",
            (modifiers.gem_gain * 100.0).round() as u32,
        ),
    }
}

/// Helper object used returned by `skill_strings` as source for
/// later internationalization and formatting.
enum SkillStrings<'a> {
    Plain {
        title: &'a str,
        desc: &'a str,
    },
    WithConst {
        title: &'a str,
        desc: &'a str,
        constant: u32,
    },
    WithMult {
        title: &'a str,
        desc: &'a str,
        multiplier: f32,
    },
    Empty,
}

impl<'a> SkillStrings<'a> {
    fn plain(title: &'a str, desc: &'a str) -> Self { Self::Plain { title, desc } }

    fn with_const(title: &'a str, desc: &'a str, constant: u32) -> Self {
        Self::WithConst {
            title,
            desc,
            constant,
        }
    }

    fn with_mult(title: &'a str, desc: &'a str, multiplier: f32) -> Self {
        Self::WithMult {
            title,
            desc,
            multiplier,
        }
    }

    fn localize<'loc>(
        &self,
        i18n: &'loc Localization,
        skill_set: &SkillSet,
        skill: Skill,
    ) -> (Cow<'loc, str>, Cow<'loc, str>) {
        match self {
            Self::Plain { title, desc } => {
                let title = i18n.get_msg(title);

                let args = i18n::fluent_args! {
                    "SP" => sp(i18n, skill_set, skill),
                };
                let desc = i18n.get_msg_ctx(desc, &args);

                (title, desc)
            },
            Self::WithConst {
                title,
                desc,
                constant,
            } => {
                let title = i18n.get_msg(title);
                let args = i18n::fluent_args! {
                    "boost" => constant,
                    "SP" => sp(i18n, skill_set, skill),
                };
                let desc = i18n.get_msg_ctx(desc, &args);

                (title, desc)
            },
            Self::WithMult {
                title,
                desc,
                multiplier,
            } => {
                let percentage = hud::multiplier_to_percentage(*multiplier).abs();

                let title = i18n.get_msg(title);

                let args = i18n::fluent_args! {
                    "boost" => format!("{percentage:.0}"),
                    "SP" => sp(i18n, skill_set, skill),
                };
                let desc = i18n.get_msg_ctx(desc, &args);

                (title, desc)
            },
            Self::Empty => (Cow::Borrowed(""), Cow::Borrowed("")),
        }
    }
}

/// The number of variants of the [`CharacterStat`] enum.
const STAT_COUNT: usize = 15;

#[derive(EnumIter)]
enum CharacterStat {
    Name,
    BattleMode,
    Waypoint,
    Hitpoints,
    Energy,
    Poise,
    CombatRating,
    Protection,
    StunResistance,
    PrecisionPower,
    EnergyReward,
    Stealth,
    WeaponPower,
    WeaponSpeed,
    WeaponEffectPower,
}

impl CharacterStat {
    fn localized_str<'a>(&self, i18n: &'a Localization) -> Cow<'a, str> {
        use CharacterStat::*;

        match self {
            Name => i18n.get_msg("character_window-character_name"),
            BattleMode => i18n.get_msg("hud-battle-mode"),
            Waypoint => i18n.get_msg("hud-waypoint"),
            Hitpoints => i18n.get_msg("hud-bag-health"),
            Energy => i18n.get_msg("hud-bag-energy"),
            CombatRating => i18n.get_msg("hud-bag-combat_rating"),
            Protection => i18n.get_msg("hud-bag-protection"),
            StunResistance => i18n.get_msg("hud-bag-stun_res"),
            Poise => i18n.get_msg("common-stats-poise_res"),
            PrecisionPower => i18n.get_msg("common-stats-precision_power"),
            EnergyReward => i18n.get_msg("common-stats-energy_reward"),
            Stealth => i18n.get_msg("common-stats-stealth"),
            WeaponPower => i18n.get_msg("common-stats-power"),
            WeaponSpeed => i18n.get_msg("common-stats-speed"),
            WeaponEffectPower => i18n.get_msg("common-stats-effect-power"),
        }
    }
}

fn sp<'loc>(i18n: &'loc Localization, skill_set: &SkillSet, skill: Skill) -> Cow<'loc, str> {
    let current_level = skill_set.skill_level(skill);
    if matches!(current_level, Ok(level) if level == skill.max_level()) {
        Cow::Borrowed("")
    } else {
        i18n.get_msg_ctx("hud-skill-req_sp", &i18n::fluent_args! {
            "number" => skill_set.skill_cost(skill),
        })
    }
}
