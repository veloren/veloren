use crate::ui::img_ids::{BlankGraphic, ImageGraphic, VoxelPixArtGraphic};

// TODO: Combine with image_ids, see macro definition
rotation_image_ids! {
    pub struct ImgsRot {
        <VoxelGraphic>

        <ImageGraphic>
        indicator_mmap_small: "voxygen.element.ui.minimap.icons.indicator_mmap",
         // Tooltip Test
         tt_side: "voxygen.element.ui.generic.frames.tt_test_edge",
         tt_corner: "voxygen.element.ui.generic.frames.tt_test_corner_tr",
//////////////////////////////////////////////////////////////////////////////////////////////////////

        <VoxelPixArtGraphic>
    }
}

image_ids! {
    pub struct Imgs {
        <VoxelGraphic>
        // Missing: Buff Frame Animation .gif ?! we could do animation in ui.maintain, or in shader?
////////////////////////////////////////////////////////////////////////
        <VoxelPixArtGraphic>
        // Icons
        flower: "voxygen.element.items.item_flower",
        grass: "voxygen.element.items.item_grass",

        // Items
        potion_red: "voxygen.voxel.object.potion_red",
        potion_green: "voxygen.voxel.object.potion_green",
        potion_blue: "voxygen.voxel.object.potion_blue",
        key: "voxygen.voxel.object.key",
        key_gold: "voxygen.voxel.object.key_gold",


//////////////////////////////////////////////////////////////////////////////////////////////////////

        <ImageGraphic>

        // Checkboxes and Radio buttons
        check: "voxygen.element.ui.generic.buttons.radio.inactive",
        check_mo: "voxygen.element.ui.generic.buttons.radio.inactive_hover",
        check_press: "voxygen.element.ui.generic.buttons.radio.press",
        check_checked: "voxygen.element.ui.generic.buttons.radio.active",
        check_checked_mo: "voxygen.element.ui.generic.buttons.radio.hover",
        checkbox: "voxygen.element.ui.generic.buttons.checkbox.inactive",
        checkbox_mo: "voxygen.element.ui.generic.buttons.checkbox.inactive_hover",
        checkbox_press: "voxygen.element.ui.generic.buttons.checkbox.press",
        checkbox_checked: "voxygen.element.ui.generic.buttons.checkbox.active",
        checkbox_checked_mo: "voxygen.element.ui.generic.buttons.checkbox.hover",

        // Selection Frame
        selection: "voxygen.element.ui.generic.frames.selection",
        selection_hover: "voxygen.element.ui.generic.frames.selection_hover",
        selection_press: "voxygen.element.ui.generic.frames.selection_press",

        // Prompt Dialog
        prompt_top: "voxygen.element.ui.generic.frames.prompt_dialog_top",
        prompt_mid: "voxygen.element.ui.generic.frames.prompt_dialog_mid",
        prompt_bot: "voxygen.element.ui.generic.frames.prompt_dialog_bot",
        key_button: "voxygen.element.ui.generic.buttons.key_button",
        key_button_press: "voxygen.element.ui.generic.buttons.key_button_press",

        // Diary Window
        diary_bg: "voxygen.element.ui.diary.diary_bg",
        diary_frame: "voxygen.element.ui.diary.diary_frame",
        diary_exp_bg: "voxygen.element.ui.diary.diary_exp_bg",
        diary_exp_frame: "voxygen.element.ui.diary.diary_exp_frame",
        pixel: "voxygen.element.ui.diary.pixel",

        // Skill Trees
        slot_skills: "voxygen.element.ui.diary.buttons.slot_skilltree",
        swords_crossed: "voxygen.element.weapons.swords_crossed",
        sceptre: "voxygen.element.weapons.sceptre",
        sword: "voxygen.element.weapons.sword",
        axe: "voxygen.element.weapons.axe",
        hammer: "voxygen.element.weapons.hammer",
        bow: "voxygen.element.weapons.bow",
        staff: "voxygen.element.weapons.staff",
        pickaxe: "voxygen.element.weapons.pickaxe",
        lock: "voxygen.element.ui.diary.buttons.lock",
        wpn_icon_border_skills: "voxygen.element.ui.diary.buttons.border_skills",
        wpn_icon_border: "voxygen.element.ui.generic.buttons.border",
        wpn_icon_border_mo: "voxygen.element.ui.generic.buttons.border_mo",
        wpn_icon_border_press: "voxygen.element.ui.generic.buttons.border_press",
        wpn_icon_border_pressed: "voxygen.element.ui.generic.buttons.border_pressed",

        // Social Window
        social_frame_on: "voxygen.element.ui.social.social_frame",
        social_bg_on: "voxygen.element.ui.social.social_bg",

        // Crafting Window
        crafting_window: "voxygen.element.ui.crafting.crafting",
        crafting_frame: "voxygen.element.ui.crafting.crafting_frame",
        crafting_icon_bordered: "voxygen.element.ui.generic.buttons.anvil",
        crafting_icon: "voxygen.element.ui.generic.buttons.anvil",
        crafting_icon_hover: "voxygen.element.ui.generic.buttons.anvil_hover",
        crafting_icon_press: "voxygen.element.ui.generic.buttons.anvil_press",
        quality_indicator: "voxygen.element.ui.crafting.quality_indicator",
        icon_armor: "voxygen.element.ui.crafting.icons.armors",
        icon_tools: "voxygen.element.ui.crafting.icons.crafting_tools",
        icon_dismantle: "voxygen.element.ui.crafting.icons.dismantle",
        icon_food: "voxygen.element.ui.crafting.icons.foods",
        icon_glider: "voxygen.element.ui.crafting.icons.gliders",
        icon_globe: "voxygen.element.ui.crafting.icons.globe",
        icon_potion: "voxygen.element.ui.crafting.icons.potions",
        icon_utility: "voxygen.element.ui.crafting.icons.utilities",
        icon_weapon: "voxygen.element.ui.crafting.icons.weapons",
        icon_bag: "voxygen.element.items.item_bag_leather_large",
        icon_processed_material: "voxygen.element.ui.crafting.icons.processed_material",

        // Group Window
        member_frame: "voxygen.element.ui.groups.group_member_frame",
        member_bg: "voxygen.element.ui.groups.group_member_bg",

        // Chat-Arrows
        chat_arrow: "voxygen.element.ui.generic.buttons.arrow_down",
        chat_arrow_mo: "voxygen.element.ui.generic.buttons.arrow_down_hover",
        chat_arrow_press: "voxygen.element.ui.generic.buttons.arrow_down_press",

        // Settings Window
        settings_button: "voxygen.element.ui.settings.buttons.settings_button",
        settings_button_pressed: "voxygen.element.ui.settings.buttons.settings_button_pressed",
        settings_button_hover: "voxygen.element.ui.settings.buttons.settings_button_hover",
        settings_button_press: "voxygen.element.ui.settings.buttons.settings_button_press",

        settings_plus: "voxygen.element.ui.settings.buttons.settings_button_plus",
        settings_plus_hover: "voxygen.element.ui.settings.buttons.settings_button_plus_hover",
        settings_plus_press: "voxygen.element.ui.settings.buttons.settings_button_plus_press",

        chat_tab_settings_bg: "voxygen.element.ui.settings.chat_tab_settings_bg",
        chat_tab_settings_frame: "voxygen.element.ui.settings.chat_tab_settings_frame",

        quest_bg: "voxygen.element.ui.quests.temp_quest_bg",

        // Slider
        slider: "voxygen.element.ui.generic.slider.track",
        slider_indicator: "voxygen.element.ui.generic.slider.indicator",
        slider_indicator_small: "voxygen.element.ui.generic.slider.indicator_round",

        // Buttons
        settings: "voxygen.element.ui.generic.buttons.settings",
        settings_hover: "voxygen.element.ui.generic.buttons.settings_hover",
        settings_press: "voxygen.element.ui.generic.buttons.settings_press",

        social: "voxygen.element.ui.generic.buttons.social",
        social_hover: "voxygen.element.ui.generic.buttons.social_hover",
        social_press: "voxygen.element.ui.generic.buttons.social_press",

        map_button: "voxygen.element.ui.generic.buttons.map",
        map_hover: "voxygen.element.ui.generic.buttons.map_hover",
        map_press: "voxygen.element.ui.generic.buttons.map_press",

        spellbook_button: "voxygen.element.ui.generic.buttons.spellbook",
        spellbook_hover: "voxygen.element.ui.generic.buttons.spellbook_hover",
        spellbook_press: "voxygen.element.ui.generic.buttons.spellbook_press",

        group_icon: "voxygen.element.ui.generic.buttons.group",
        group_icon_hover: "voxygen.element.ui.generic.buttons.group_hover",
        group_icon_press: "voxygen.element.ui.generic.buttons.group_press",

        sp_indicator_arrow: "voxygen.element.ui.generic.buttons.arrow_down_gold",

        // Skill Icons
        twohsword_m1: "voxygen.element.skills.2hsword_m1",
        twohsword_m2: "voxygen.element.skills.2hsword_m2",
        onehdagger_m1: "voxygen.element.weapons.daggers",
        onehdagger_m2: "voxygen.element.skills.skill_slice_2",
        onehshield_m1: "voxygen.element.weapons.swordshield",
        onehshield_m2: "voxygen.element.weapons.swordshield",
        twohhammer_m1: "voxygen.element.skills.2hhammer_m1",
        twohaxe_m1: "voxygen.element.skills.2haxe_m1",
        bow_m1: "voxygen.element.skills.bow_m1",
        bow_m2: "voxygen.element.skills.bow_m2",
        staff_melee: "voxygen.element.skills.staff_m1",
        fireball: "voxygen.element.skills.staff_m2",
        flyingrod_m1: "voxygen.element.skills.debug_wand_m1",
        flyingrod_m2: "voxygen.element.skills.debug_wand_m2",
        sword_pierce: "voxygen.element.skills.skill_sword_pierce",
        hammergolf: "voxygen.element.skills.skill_hammergolf",
        axespin: "voxygen.element.skills.skill_axespin",
        fire_aoe: "voxygen.element.skills.skill_fire_aoe",
        flamethrower: "voxygen.element.skills.skill_flamethrower",

        // Skilltree Icons
        health_plus_skill: "voxygen.element.skills.skilltree.health_plus",
        stamina_plus_skill: "voxygen.element.skills.skilltree.stamina_plus",
        unlock_axe_skill: "voxygen.element.skills.skilltree.unlock_axe",
        unlock_bow_skill: "voxygen.element.skills.skilltree.unlock_bow",
        unlock_hammer_skill: "voxygen.element.skills.skilltree.unlock_hammer",
        unlock_sceptre_skill: "voxygen.element.skills.skilltree.unlock_sceptre",
        unlock_staff_skill0: "voxygen.element.skills.skilltree.unlock_staff-0",
        unlock_sword_skill: "voxygen.element.skills.skilltree.unlock_sword",
        skill_dodge_skill: "voxygen.element.skills.skilltree.skill_dodge",
        skill_climbing_skill: "voxygen.element.skills.skilltree.skill_climbing",
        skill_swim_skill: "voxygen.element.skills.skilltree.skill_swim",

        buff_amount_skill: "voxygen.element.skills.skilltree.buff_amount",
        buff_combo_skill: "voxygen.element.skills.skilltree.buff_combo",
        buff_cost_skill: "voxygen.element.skills.skilltree.buff_cost",
        buff_damage_skill: "voxygen.element.skills.skilltree.buff_damage",
        buff_distance_skill: "voxygen.element.skills.skilltree.buff_distance",
        buff_energy_drain_skill: "voxygen.element.skills.skilltree.buff_energy_drain",
        buff_energy_regen_skill: "voxygen.element.skills.skilltree.buff_energy_regen",
        buff_explosion_skill: "voxygen.element.skills.skilltree.buff_explosion",
        buff_heal_skill: "voxygen.element.skills.skilltree.buff_heal",
        buff_helicopter_skill: "voxygen.element.skills.skilltree.buff_helicopter",
        buff_infinite_skill: "voxygen.element.skills.skilltree.buff_infinite",
        buff_knockback_skill: "voxygen.element.skills.skilltree.buff_knockback",
        buff_lifesteal_skill: "voxygen.element.skills.skilltree.buff_lifesteal",
        buff_projectile_speed_skill: "voxygen.element.skills.skilltree.buff_projectile_speed",
        buff_radius_skill: "voxygen.element.skills.skilltree.buff_radius",
        buff_speed_skill: "voxygen.element.skills.skilltree.buff_speed",
        buff_duration_skill: "voxygen.element.skills.skilltree.buff_duration",

        debuff_amount_skill: "voxygen.element.skills.skilltree.debuff_amount",
        debuff_combo_skill: "voxygen.element.skills.skilltree.debuff_combo",
        debuff_cost_skill: "voxygen.element.skills.skilltree.debuff_cost",
        debuff_damage_skill: "voxygen.element.skills.skilltree.debuff_damage",
        debuff_distance_skill: "voxygen.element.skills.skilltree.debuff_distance",
        debuff_energy_drain_skill: "voxygen.element.skills.skilltree.debuff_energy_drain",
        debuff_energy_regen_skill: "voxygen.element.skills.skilltree.debuff_energy_regen",
        debuff_explosion_skill: "voxygen.element.skills.skilltree.debuff_explosion",
        debuff_heal_skill: "voxygen.element.skills.skilltree.debuff_heal",
        debuff_helicopter_skill: "voxygen.element.skills.skilltree.debuff_helicopter",
        debuff_infinite_skill: "voxygen.element.skills.skilltree.debuff_infinite",
        debuff_knockback_skill: "voxygen.element.skills.skilltree.debuff_knockback",
        debuff_lifesteal_skill: "voxygen.element.skills.skilltree.debuff_lifesteal",
        debuff_projectile_speed_skill: "voxygen.element.skills.skilltree.debuff_projectile_speed",
        debuff_radius_skill: "voxygen.element.skills.skilltree.debuff_radius",
        debuff_speed_skill: "voxygen.element.skills.skilltree.debuff_speed",
        debuff_duration_skill: "voxygen.element.skills.skilltree.debuff_duration",

        heal_amount_skill: "voxygen.element.skills.skilltree.heal_amount",
        heal_combo_skill: "voxygen.element.skills.skilltree.heal_combo",
        heal_cost_skill: "voxygen.element.skills.skilltree.heal_cost",
        heal_damage_skill: "voxygen.element.skills.skilltree.heal_damage",
        heal_distance_skill: "voxygen.element.skills.skilltree.heal_distance",
        heal_energy_drain_skill: "voxygen.element.skills.skilltree.heal_energy_drain",
        heal_energy_regen_skill: "voxygen.element.skills.skilltree.heal_energy_regen",
        heal_explosion_skill: "voxygen.element.skills.skilltree.heal_explosion",
        heal_heal_skill: "voxygen.element.skills.skilltree.heal_heal",
        heal_helicopter_skill: "voxygen.element.skills.skilltree.heal_helicopter",
        heal_infinite_skill: "voxygen.element.skills.skilltree.heal_infinite",
        heal_knockback_skill: "voxygen.element.skills.skilltree.heal_knockback",
        heal_lifesteal_skill: "voxygen.element.skills.skilltree.heal_lifesteal",
        heal_projectile_speed_skill: "voxygen.element.skills.skilltree.heal_projectile_speed",
        heal_radius_skill: "voxygen.element.skills.skilltree.heal_radius",
        heal_speed_skill: "voxygen.element.skills.skilltree.heal_speed",
        heal_duration_skill: "voxygen.element.skills.skilltree.heal_duration",

        magic_amount_skill: "voxygen.element.skills.skilltree.magic_amount",
        magic_combo_skill: "voxygen.element.skills.skilltree.magic_combo",
        magic_cost_skill: "voxygen.element.skills.skilltree.magic_cost",
        magic_damage_skill: "voxygen.element.skills.skilltree.magic_damage",
        magic_distance_skill: "voxygen.element.skills.skilltree.magic_distance",
        magic_energy_drain_skill: "voxygen.element.skills.skilltree.magic_energy_drain",
        magic_energy_regen_skill: "voxygen.element.skills.skilltree.magic_energy_regen",
        magic_explosion_skill: "voxygen.element.skills.skilltree.magic_explosion",
        magic_heal_skill: "voxygen.element.skills.skilltree.magic_heal",
        magic_helicopter_skill: "voxygen.element.skills.skilltree.magic_helicopter",
        magic_infinite_skill: "voxygen.element.skills.skilltree.magic_infinite",
        magic_knockback_skill: "voxygen.element.skills.skilltree.magic_knockback",
        magic_lifesteal_skill: "voxygen.element.skills.skilltree.magic_lifesteal",
        magic_projectile_speed_skill: "voxygen.element.skills.skilltree.magic_projectile_speed",
        magic_radius_skill: "voxygen.element.skills.skilltree.magic_radius",
        magic_speed_skill: "voxygen.element.skills.skilltree.magic_speed",
        magic_duration_skill: "voxygen.element.skills.skilltree.magic_duration",

        physical_amount_skill: "voxygen.element.skills.skilltree.physical_amount",
        physical_combo_skill: "voxygen.element.skills.skilltree.physical_combo",
        physical_cost_skill: "voxygen.element.skills.skilltree.physical_cost",
        physical_damage_skill: "voxygen.element.skills.skilltree.physical_damage",
        physical_distance_skill: "voxygen.element.skills.skilltree.physical_distance",
        physical_energy_drain_skill: "voxygen.element.skills.skilltree.physical_energy_drain",
        physical_energy_regen_skill: "voxygen.element.skills.skilltree.physical_energy_regen",
        physical_explosion_skill: "voxygen.element.skills.skilltree.physical_explosion",
        physical_heal_skill: "voxygen.element.skills.skilltree.physical_heal",
        physical_helicopter_skill: "voxygen.element.skills.skilltree.physical_helicopter",
        physical_infinite_skill: "voxygen.element.skills.skilltree.physical_infinite",
        physical_knockback_skill: "voxygen.element.skills.skilltree.physical_knockback",
        physical_lifesteal_skill: "voxygen.element.skills.skilltree.physical_lifesteal",
        physical_projectile_speed_skill: "voxygen.element.skills.skilltree.physical_projectile_speed",
        physical_radius_skill: "voxygen.element.skills.skilltree.physical_radius",
        physical_speed_skill: "voxygen.element.skills.skilltree.physical_speed",
        physical_duration_skill: "voxygen.element.skills.skilltree.physical_duration",

        utility_amount_skill: "voxygen.element.skills.skilltree.utility_amount",
        utility_combo_skill: "voxygen.element.skills.skilltree.utility_combo",
        utility_cost_skill: "voxygen.element.skills.skilltree.utility_cost",
        utility_damage_skill: "voxygen.element.skills.skilltree.utility_damage",
        utility_distance_skill: "voxygen.element.skills.skilltree.utility_distance",
        utility_energy_drain_skill: "voxygen.element.skills.skilltree.utility_energy_drain",
        utility_energy_regen_skill: "voxygen.element.skills.skilltree.utility_energy_regen",
        utility_explosion_skill: "voxygen.element.skills.skilltree.utility_explosion",
        utility_heal_skill: "voxygen.element.skills.skilltree.utility_heal",
        utility_helicopter_skill: "voxygen.element.skills.skilltree.utility_helicopter",
        utility_infinite_skill: "voxygen.element.skills.skilltree.utility_infinite",
        utility_knockback_skill: "voxygen.element.skills.skilltree.utility_knockback",
        utility_lifesteal_skill: "voxygen.element.skills.skilltree.utility_lifesteal",
        utility_projectile_speed_skill: "voxygen.element.skills.skilltree.utility_projectile_speed",
        utility_radius_skill: "voxygen.element.skills.skilltree.utility_radius",
        utility_speed_skill: "voxygen.element.skills.skilltree.utility_speed",
        utility_duration_skill: "voxygen.element.skills.skilltree.utility_duration",

        pickaxe_speed_skill: "voxygen.element.skills.pickaxe_speed",
        pickaxe_oregain_skill: "voxygen.element.skills.pickaxe_oregain",
        pickaxe_gemgain_skill: "voxygen.element.skills.pickaxe_gemgain",

        // Skillbar
        level_up: "voxygen.element.ui.skillbar.level_up",
        bar_content: "voxygen.element.ui.skillbar.bar_content",
        skillbar_bg: "voxygen.element.ui.skillbar.bg",
        skillbar_frame: "voxygen.element.ui.skillbar.frame",
        health_bg: "voxygen.element.ui.skillbar.health_bg",
        health_frame: "voxygen.element.ui.skillbar.health_frame",
        decayed_bg: "voxygen.element.ui.skillbar.decayed_bg",
        stamina_bg: "voxygen.element.ui.skillbar.stamina_bg",
        stamina_frame: "voxygen.element.ui.skillbar.stamina_frame",
        m1_ico: "voxygen.element.ui.generic.icons.m1",
        m2_ico: "voxygen.element.ui.generic.icons.m2",
        m_scroll_ico: "voxygen.element.ui.generic.icons.m_scroll",
        m_move_ico: "voxygen.element.ui.generic.icons.m_move",
        m_click_ico: "voxygen.element.ui.generic.icons.m_click",
        skillbar_slot: "voxygen.element.ui.skillbar.slot",

        // Other Icons/Art
        skull: "voxygen.element.ui.generic.icons.skull",
        skull_2: "voxygen.element.ui.generic.icons.skull_2",
        fireplace: "voxygen.element.ui.generic.fireplace",

        // Crosshair
        crosshair_inner: "voxygen.element.ui.settings.icons.crosshair_inner",

        crosshair_outer_round: "voxygen.element.ui.settings.icons.crosshair_outer_1",
        crosshair_outer_round_edges: "voxygen.element.ui.settings.icons.crosshair_outer_2",
        crosshair_outer_edges: "voxygen.element.ui.settings.icons.crosshair_outer_3",

        crosshair_bg: "voxygen.element.ui.settings.icons.crosshair_bg",
        crosshair_bg_hover: "voxygen.element.ui.settings.icons.crosshair_bg_hover",
        crosshair_bg_press: "voxygen.element.ui.settings.icons.crosshair_bg_press",
        crosshair_bg_pressed: "voxygen.element.ui.settings.icons.crosshair_bg_pressed",

        // Map
        map_topo: "voxygen.element.ui.map.buttons.topographic",
        map_bg: "voxygen.element.ui.map.map_bg",
        map_frame: "voxygen.element.ui.map.map_frame",
        map_frame_art: "voxygen.element.ui.map.map_frame_art",
        indicator_mmap: "voxygen.element.ui.minimap.icons.indicator_mmap",
        indicator_map_overlay: "voxygen.element.ui.minimap.icons.indicator_mmap_small",
        indicator_group: "voxygen.element.ui.map.buttons.group_indicator",
        indicator_group_up: "voxygen.element.ui.map.buttons.group_indicator_arrow_up",
        indicator_group_down: "voxygen.element.ui.map.buttons.group_indicator_arrow_down",
        location_marker: "voxygen.element.ui.map.buttons.location_marker",
        map_mode_overlay: "voxygen.element.ui.map.buttons.map_modes",

        // MiniMap
        mmap_frame: "voxygen.element.ui.minimap.mmap",
        mmap_frame_2: "voxygen.element.ui.minimap.mmap_frame",
        mmap_frame_closed: "voxygen.element.ui.minimap.mmap_closed",
        mmap_closed: "voxygen.element.ui.minimap.buttons.button_mmap_closed",
        mmap_closed_hover: "voxygen.element.ui.minimap.buttons.button_mmap_closed_hover",
        mmap_closed_press: "voxygen.element.ui.minimap.buttons.button_mmap_closed_press",
        mmap_open: "voxygen.element.ui.minimap.buttons.button_mmap_open",
        mmap_open_hover: "voxygen.element.ui.minimap.buttons.button_mmap_open_hover",
        mmap_open_press: "voxygen.element.ui.minimap.buttons.button_mmap_open_press",
        mmap_plus: "voxygen.element.ui.minimap.buttons.mmap_button-plus",
        mmap_plus_hover: "voxygen.element.ui.minimap.buttons.mmap_button-plus_hover",
        mmap_plus_press: "voxygen.element.ui.minimap.buttons.mmap_button-plus_press",
        mmap_minus: "voxygen.element.ui.minimap.buttons.mmap_button-min",
        mmap_minus_hover: "voxygen.element.ui.minimap.buttons.mmap_button-min_hover",
        mmap_minus_press: "voxygen.element.ui.minimap.buttons.mmap_button-min_press",
        mmap_north: "voxygen.element.ui.minimap.buttons.mmap_button-north",
        mmap_north_hover: "voxygen.element.ui.minimap.buttons.mmap_button-north_hover",
        mmap_north_press: "voxygen.element.ui.minimap.buttons.mmap_button-north_press",
        mmap_north_press_hover: "voxygen.element.ui.minimap.buttons.mmap_button-north_press_hover",
        map_dif_1: "voxygen.element.ui.map.icons.dif_1",
        map_dif_2: "voxygen.element.ui.map.icons.dif_2",
        map_dif_3: "voxygen.element.ui.map.icons.dif_3",
        map_dif_4: "voxygen.element.ui.map.icons.dif_4",
        map_dif_5: "voxygen.element.ui.map.icons.dif_5",
        map_dif_6: "voxygen.element.ui.map.icons.dif_6",
        mmap_site_town: "voxygen.element.ui.map.buttons.town",
        mmap_site_town_hover: "voxygen.element.ui.map.buttons.town_hover",
        mmap_site_town_bg: "voxygen.element.ui.map.buttons.town_bg",
        mmap_site_dungeon: "voxygen.element.ui.map.buttons.dungeon",
        mmap_site_dungeon_hover: "voxygen.element.ui.map.buttons.dungeon_hover",
        mmap_site_dungeon_bg: "voxygen.element.ui.map.buttons.dungeon_bg",
        mmap_site_castle: "voxygen.element.ui.map.buttons.castle",
        mmap_site_castle_hover: "voxygen.element.ui.map.buttons.castle_hover",
        mmap_site_castle_bg: "voxygen.element.ui.map.buttons.castle_bg",
        mmap_site_cave_bg: "voxygen.element.ui.map.buttons.cave_bg",
        mmap_site_cave_hover: "voxygen.element.ui.map.buttons.cave_hover",
        mmap_site_cave: "voxygen.element.ui.map.buttons.cave",
        mmap_site_excl: "voxygen.element.ui.map.buttons.excl",
        mmap_site_tree: "voxygen.element.ui.map.buttons.tree",
        mmap_site_tree_hover: "voxygen.element.ui.map.buttons.tree_hover",
        mmap_poi_peak: "voxygen.element.ui.map.buttons.peak",
        mmap_poi_peak_hover: "voxygen.element.ui.map.buttons.peak_hover",

        // Window Parts
        window_3: "voxygen.element.ui.generic.frames.window_3",
        esc_frame: "voxygen.element.ui.generic.frames.esc_menu",
        // Settings
        settings_bg: "voxygen.element.ui.settings.settings_bg",
        settings_frame: "voxygen.element.ui.settings.settings_frame",

        // Close-Button
        close_btn: "voxygen.element.ui.generic.buttons.close_btn",
        close_btn_hover: "voxygen.element.ui.generic.buttons.close_btn_hover",
        close_btn_press: "voxygen.element.ui.generic.buttons.close_btn_press",
        close_button: "voxygen.element.ui.generic.buttons.close_btn",
        close_button_hover: "voxygen.element.ui.generic.buttons.close_btn_hover",
        close_button_press: "voxygen.element.ui.generic.buttons.close_btn_press",

        // Search-button
        search_btn: "voxygen.element.ui.generic.buttons.search_btn",
        search_btn_hover: "voxygen.element.ui.generic.buttons.search_btn_hover",
        search_btn_press: "voxygen.element.ui.generic.buttons.search_btn_press",
        // Inventory
        collapse_btn: "voxygen.element.ui.bag.buttons.inv_collapse",
        collapse_btn_hover: "voxygen.element.ui.bag.buttons.inv_collapse_hover",
        collapse_btn_press: "voxygen.element.ui.bag.buttons.inv_collapse_press",
        expand_btn: "voxygen.element.ui.bag.buttons.inv_expand",
        expand_btn_hover: "voxygen.element.ui.bag.buttons.inv_expand_hover",
        expand_btn_press: "voxygen.element.ui.bag.buttons.inv_expand_press",
        inv_sort_btn: "voxygen.element.ui.bag.buttons.inv_sort",
        inv_sort_btn_hover: "voxygen.element.ui.bag.buttons.inv_sort_hover",
        inv_sort_btn_press: "voxygen.element.ui.bag.buttons.inv_sort_press",
        swap_equipped_weapons_btn: "voxygen.element.ui.bag.buttons.swap_equipped_weapons",
        swap_equipped_weapons_btn_hover: "voxygen.element.ui.bag.buttons.swap_equipped_weapons_hover",
        swap_equipped_weapons_btn_press: "voxygen.element.ui.bag.buttons.swap_equipped_weapons_press",
        coin_ico: "voxygen.element.items.coin",
        cheese_ico: "voxygen.element.items.item_cheese",
        inv_bg_armor: "voxygen.element.ui.bag.inv_bg_0",
        inv_bg_stats: "voxygen.element.ui.bag.inv_bg_1",
        inv_frame: "voxygen.element.ui.bag.inv_frame",
        inv_frame_bag: "voxygen.element.ui.bag.inv_frame_bag",
        inv_bg_bag: "voxygen.element.ui.bag.inv_bg_bag",
        char_art: "voxygen.element.ui.bag.icons.character",
        inv_slot: "voxygen.element.ui.bag.buttons.inv_slot",
        inv_slot_grey: "voxygen.element.ui.bag.buttons.inv_slot_grey",
        inv_slot_green: "voxygen.element.ui.bag.buttons.inv_slot_green",
        inv_slot_blue: "voxygen.element.ui.bag.buttons.inv_slot_blue",
        inv_slot_purple: "voxygen.element.ui.bag.buttons.inv_slot_purple",
        inv_slot_gold: "voxygen.element.ui.bag.buttons.inv_slot_gold",
        inv_slot_orange: "voxygen.element.ui.bag.buttons.inv_slot_orange",
        inv_slot_red: "voxygen.element.ui.bag.buttons.inv_slot_red",
        inv_slot_sel: "voxygen.element.ui.bag.buttons.inv_slot_sel",
        scrollbar_bg: "voxygen.element.ui.generic.slider.scrollbar",
        scrollbar_bg_big: "voxygen.element.ui.generic.slider.scrollbar_1",
        inv_tab_active: "voxygen.element.ui.bag.buttons.inv_tab_active",
        inv_tab_inactive: "voxygen.element.ui.bag.buttons.inv_tab_inactive",
        inv_tab_inactive_hover: "voxygen.element.ui.bag.buttons.inv_tab_inactive",
        inv_tab_inactive_press: "voxygen.element.ui.bag.buttons.inv_tab_inactive",
        armor_slot: "voxygen.element.ui.generic.buttons.armor_slot",
        armor_slot_sel: "voxygen.element.ui.generic.buttons.armor_slot_selected",
        armor_slot_empty: "voxygen.element.ui.generic.buttons.armor_slot_empty",
        head_bg: "voxygen.element.ui.bag.backgrounds.head",
        shoulders_bg: "voxygen.element.ui.bag.backgrounds.shoulders",
        hands_bg: "voxygen.element.ui.bag.backgrounds.hands",
        belt_bg: "voxygen.element.ui.bag.backgrounds.belt",
        legs_bg: "voxygen.element.ui.bag.backgrounds.legs",
        feet_bg: "voxygen.element.ui.bag.backgrounds.feet",
        ring_bg: "voxygen.element.ui.bag.backgrounds.ring",
        tabard_bg: "voxygen.element.ui.bag.backgrounds.tabard",
        glider_bg: "voxygen.element.ui.bag.backgrounds.glider",
        chest_bg: "voxygen.element.ui.bag.backgrounds.chest",
        back_bg: "voxygen.element.ui.bag.backgrounds.back",
        lantern_bg: "voxygen.element.ui.bag.backgrounds.lantern",
        necklace_bg: "voxygen.element.ui.bag.backgrounds.necklace",
        mainhand_bg: "voxygen.element.ui.bag.backgrounds.mainhand",
        bag_bg: "voxygen.element.ui.bag.backgrounds.bag",
        offhand_bg: "voxygen.element.ui.bag.backgrounds.offhand",
        stamina_ico: "voxygen.element.ui.bag.icons.stamina",
        health_ico: "voxygen.element.ui.bag.icons.health",
        protection_ico: "voxygen.element.ui.bag.icons.protection",
        stun_res_ico: "voxygen.element.ui.bag.icons.stun_res",
        combat_rating_ico: "voxygen.element.ui.bag.icons.combat_rating",
        combat_rating_ico_shadow: "voxygen.element.ui.bag.icons.combat_rating_shadow",

        not_found: "voxygen.element.not_found",

        death_bg: "voxygen.background.death",
        hurt_bg: "voxygen.background.hurt",

        banner_top: "voxygen.element.ui.generic.frames.banner_top",

        // Icons
        snake_arrow_0: "voxygen.element.skills.snake",
        skill_sceptre_lifesteal: "voxygen.element.skills.lifesteal",
        sword_whirlwind: "voxygen.element.skills.sword_whirlwind",
        skill_sceptre_heal: "voxygen.element.skills.heal_0",
        hammerleap: "voxygen.element.skills.skill_hammerleap",
        skill_axe_leap_slash: "voxygen.element.skills.skill_axe_leap_slash",
        skill_bow_jump_burst: "voxygen.element.skills.skill_bow_jump_burst",
        skill_sceptre_aura: "voxygen.element.skills.sceptre_protection",
        missing_icon: "voxygen.element.missing_icon_grey",

        // Buttons
        button: "voxygen.element.ui.generic.buttons.button",
        button_hover: "voxygen.element.ui.generic.buttons.button_hover",
        button_press: "voxygen.element.ui.generic.buttons.button_press",

        // Enemy Healthbar
        enemy_health: "voxygen.element.ui.generic.frames.enemybar",
        enemy_health_bg: "voxygen.element.ui.generic.frames.enemybar_bg",
        health_bar_group: "voxygen.element.ui.generic.frames.enemybar_1",
        health_bar_group_bg: "voxygen.element.ui.generic.frames.enemybar_bg_1",
        // Enemy Bar Content:
        enemy_bar: "voxygen.element.ui.skillbar.enemy_bar_content",

        // Bag
        bag: "voxygen.element.ui.generic.buttons.bag.closed",
        bag_hover: "voxygen.element.ui.generic.buttons.bag.closed_hover",
        bag_press: "voxygen.element.ui.generic.buttons.bag.closed_press",
        bag_open: "voxygen.element.ui.generic.buttons.bag.open",
        bag_open_hover: "voxygen.element.ui.generic.buttons.bag.open_hover",
        bag_open_press: "voxygen.element.ui.generic.buttons.bag.open_press",

        map_icon: "voxygen.element.ui.generic.buttons.map",

        grid_button: "voxygen.element.ui.generic.buttons.border",
        grid_button_hover: "voxygen.element.ui.generic.buttons.border_mo",
        grid_button_press: "voxygen.element.ui.generic.buttons.border_press",
        grid_button_open: "voxygen.element.ui.generic.buttons.border_pressed",

        // Char Window
        progress_frame: "voxygen.element.ui.invite.progress_bar",
        progress: "voxygen.element.ui.invite.progress",

        // Speech bubbles
        speech_bubble_top_left: "voxygen.element.ui.generic.frames.bubble.top_left",
        speech_bubble_top: "voxygen.element.ui.generic.frames.bubble.top",
        speech_bubble_top_right: "voxygen.element.ui.generic.frames.bubble.top_right",
        speech_bubble_left: "voxygen.element.ui.generic.frames.bubble.left",
        speech_bubble_mid: "voxygen.element.ui.generic.frames.bubble.mid",
        speech_bubble_right: "voxygen.element.ui.generic.frames.bubble.right",
        speech_bubble_bottom_left: "voxygen.element.ui.generic.frames.bubble.bottom_left",
        speech_bubble_bottom: "voxygen.element.ui.generic.frames.bubble.bottom",
        speech_bubble_bottom_right: "voxygen.element.ui.generic.frames.bubble.bottom_right",
        speech_bubble_tail: "voxygen.element.ui.generic.frames.bubble.tail",
        speech_bubble_icon_frame: "voxygen.element.ui.generic.frames.bubble_dark.icon_frame",

        dark_bubble_top_left: "voxygen.element.ui.generic.frames.bubble_dark.top_left",
        dark_bubble_top: "voxygen.element.ui.generic.frames.bubble_dark.top",
        dark_bubble_top_right: "voxygen.element.ui.generic.frames.bubble_dark.top_right",
        dark_bubble_left: "voxygen.element.ui.generic.frames.bubble_dark.left",
        dark_bubble_mid: "voxygen.element.ui.generic.frames.bubble_dark.mid",
        dark_bubble_right: "voxygen.element.ui.generic.frames.bubble_dark.right",
        dark_bubble_bottom_left: "voxygen.element.ui.generic.frames.bubble_dark.bottom_left",
        dark_bubble_bottom: "voxygen.element.ui.generic.frames.bubble_dark.bottom",
        dark_bubble_bottom_right: "voxygen.element.ui.generic.frames.bubble_dark.bottom_right",
        dark_bubble_tail: "voxygen.element.ui.generic.frames.bubble_dark.tail",
        dark_bubble_icon_frame: "voxygen.element.ui.generic.frames.bubble_dark.icon_frame",


        // Chat icons
        chat_faction_small: "voxygen.element.ui.chat.icons.faction_small",
        chat_group_small: "voxygen.element.ui.chat.icons.group_small",
        chat_kill_small: "voxygen.element.ui.chat.icons.kill_small",
        chat_region_small: "voxygen.element.ui.chat.icons.region_small",
        chat_say_small: "voxygen.element.ui.chat.icons.say_small",
        chat_tell_small: "voxygen.element.ui.chat.icons.tell_small",
        chat_world_small: "voxygen.element.ui.chat.icons.world_small",
        chat_command_error_small: "voxygen.element.ui.chat.icons.command_error_small",
        chat_command_info_small: "voxygen.element.ui.chat.icons.command_info_small",
        chat_online_small: "voxygen.element.ui.chat.icons.online_small",
        chat_offline_small: "voxygen.element.ui.chat.icons.offline_small",

        chat_faction: "voxygen.element.ui.chat.icons.faction",
        chat_group: "voxygen.element.ui.chat.icons.group",
        chat_region: "voxygen.element.ui.chat.icons.region",
        chat_say: "voxygen.element.ui.chat.icons.say",
        chat_tell: "voxygen.element.ui.chat.icons.tell",
        chat_world: "voxygen.element.ui.chat.icons.world",

        // Buffs
        buff_plus_0: "voxygen.element.de_buffs.buff_plus_0",
        buff_saturation_0: "voxygen.element.de_buffs.buff_saturation_0",
        buff_potion_0: "voxygen.element.de_buffs.buff_potion_0",
        buff_campfire_heal_0: "voxygen.element.de_buffs.buff_campfire_heal_0",
        buff_energyplus_0: "voxygen.element.de_buffs.buff_energyplus_0",
        buff_healthplus_0: "voxygen.element.de_buffs.buff_healthplus_0",
        buff_invincibility_0: "voxygen.element.de_buffs.buff_invincibility_0",
        buff_dmg_red_0: "voxygen.element.de_buffs.buff_damage_reduce_0",
        buff_frenzy_0: "voxygen.element.de_buffs.buff_frenzy_0",

        // Debuffs
        debuff_skull_0: "voxygen.element.de_buffs.debuff_skull_0",
        debuff_bleed_0: "voxygen.element.de_buffs.debuff_bleed_0",
        debuff_burning_0: "voxygen.element.de_buffs.debuff_burning_0",
        debuff_crippled_0: "voxygen.element.de_buffs.debuff_cripple_0",
        debuff_frozen_0: "voxygen.element.de_buffs.debuff_frozen_0",
        debuff_wet_0: "voxygen.element.de_buffs.debuff_wet_0",

        // Animation Frames
        // Buff Frame
        buff_0: "voxygen.element.animation.buff_frame.1",
        buff_1: "voxygen.element.animation.buff_frame.2",
        buff_2: "voxygen.element.animation.buff_frame.3",
        buff_3: "voxygen.element.animation.buff_frame.4",
        buff_4: "voxygen.element.animation.buff_frame.5",
        buff_5: "voxygen.element.animation.buff_frame.6",
        buff_6: "voxygen.element.animation.buff_frame.7",
        buff_7: "voxygen.element.animation.buff_frame.8",

        <BlankGraphic>
        nothing: (),
    }
}
