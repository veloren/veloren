use crate::ui::img_ids::{BlankGraphic, ImageGraphic, VoxelGraphic, VoxelPixArtGraphic};

// TODO: Combine with image_ids, see macro definition
rotation_image_ids! {
    pub struct ImgsRot {
        <VoxelGraphic>

        // Tooltip Test
        tt_side: "voxygen/element/frames/tt_test_edge",
        tt_corner: "voxygen/element/frames/tt_test_corner_tr",

//////////////////////////////////////////////////////////////////////////////////////////////////////

        <VoxelPixArtGraphic>

        // Minimap
        indicator_mmap_small: "voxygen.element.buttons.indicator_mmap_small",
    }
}

image_ids! {
    pub struct Imgs {
        <VoxelGraphic>

        // Bag
        bag_contents: "voxygen.element.frames.bag",
        inv_grid: "voxygen.element.frames.inv_grid",
        inv_slot: "voxygen.element.buttons.inv_slot",
        inv_slot_sel: "voxygen.element.buttons.inv_slot_sel",
        grid_inv: "voxygen.element.buttons.grid_inv",
        bag_top: "voxygen.element.bag.top",
        bag_mid: "voxygen.element.bag.mid",
        bag_bot: "voxygen.element.bag.bot",

        // Skillbar
        xp_bar_mid: "voxygen.element.skillbar.xp_bar_mid",
        xp_bar_left: "voxygen.element.skillbar.xp_bar_left",
        xp_bar_right: "voxygen.element.skillbar.xp_bar_right",
        skillbar_slot: "voxygen.element.skillbar.skillbar_slot",
        skillbar_slot_act: "voxygen.element.skillbar.skillbar_slot_active",
        skillbar_slot_l: "voxygen.element.skillbar.skillbar_slot_l",
        skillbar_slot_r: "voxygen.element.skillbar.skillbar_slot_r",
        skillbar_slot_l_act: "voxygen.element.skillbar.skillbar_slot_l_active",
        skillbar_slot_r_act: "voxygen.element.skillbar.skillbar_slot_r_active",
        skillbar_slot_bg: "voxygen.element.skillbar.skillbar_slot_bg",
        skillbar_slot_big: "voxygen.element.skillbar.skillbar_slot_big",
        skillbar_slot_big_act: "voxygen.element.skillbar.skillbar_slot_big_active",
        skillbar_slot_big_bg: "voxygen.element.skillbar.skillbar_slot_big_bg",
        healthbar_bg: "voxygen.element.skillbar.healthbar_bg",
        energybar_bg: "voxygen.element.skillbar.energybar_bg",
        bar_content: "voxygen.element.skillbar.bar_content",
        level_up: "voxygen.element.misc_bg.level_up",
        level_down:"voxygen.element.misc_bg.level_down",
        stamina_0:"voxygen.element.skillbar.stamina_wheel-empty",
        stamina_1:"voxygen.element.skillbar.stamina_wheel-0",
        stamina_2:"voxygen.element.skillbar.stamina_wheel-1",
        stamina_3:"voxygen.element.skillbar.stamina_wheel-2",
        stamina_4:"voxygen.element.skillbar.stamina_wheel-3",
        stamina_5:"voxygen.element.skillbar.stamina_wheel-4",
        stamina_6:"voxygen.element.skillbar.stamina_wheel-5",
        stamina_7:"voxygen.element.skillbar.stamina_wheel-6",
        stamina_8:"voxygen.element.skillbar.stamina_wheel-7",

        // Window Parts
        window_3: "voxygen.element.frames.window_3",
        tab_bg: "voxygen.element.frames.tab_bg",
        tab_small_open: "voxygen.element.frames.tab_small_open",
        tab_small_closed: "voxygen.element.frames.tab_small_closed",

        // MiniMap
        mmap_frame: "voxygen.element.frames.mmap",
        mmap_frame_closed: "voxygen.element.frames.mmap_closed",

        // Missing: Buff Frame Animation .gif ?! we could do animation in ui.maintain, or in shader?
        window_frame: "voxygen.element.frames.window2",

        // Social Window
        social_button: "voxygen.element.buttons.social_tab",
        social_button_pressed: "voxygen.element.buttons.social_tab_pressed",
        social_button_hover: "voxygen.element.buttons.social_tab_hover",
        social_button_press: "voxygen.element.buttons.social_tab_press",
        social_frame: "voxygen.element.frames.social_frame",


        // Settings Window
        settings_frame_r: "voxygen.element.frames.settings_r",
        settings_frame_l: "voxygen.element.frames.settings_l",
        settings_button: "voxygen.element.buttons.settings_button",
        settings_button_pressed: "voxygen.element.buttons.settings_button_pressed",
        settings_button_hover: "voxygen.element.buttons.settings_button_hover",
        settings_button_press: "voxygen.element.buttons.settings_button_press",
        slider: "voxygen.element.slider.track",
        slider_indicator: "voxygen.element.slider.indicator",
        esc_frame: "voxygen.element.frames.esc_menu",

        // Map Window
        map_frame_l: "voxygen.element.frames.map_l",
        map_frame_r: "voxygen.element.frames.map_r",
        map_frame_bl: "voxygen.element.frames.map_bl",
        map_frame_br: "voxygen.element.frames.map_br",
        pos_indicator: "voxygen.element.buttons.qlog",

        // Chat-Arrows
        chat_arrow: "voxygen.element.buttons.arrow_down",
        chat_arrow_mo: "voxygen.element.buttons.arrow_down_hover",
        chat_arrow_press: "voxygen.element.buttons.arrow_down_press",

        // Crosshair
        crosshair_inner: "voxygen.element.misc_bg.crosshair_inner",



////////////////////////////////////////////////////////////////////////
        <VoxelPixArtGraphic>

        // Skill Icons
        twohsword_m1: "voxygen.element.icons.2hsword_m1",
        twohsword_m2: "voxygen.element.icons.2hsword_m2",
        twohhammer_m1: "voxygen.element.icons.2hhammer_m1",
        twohhammer_m2: "voxygen.element.icons.2hhammer_m2",
        twohaxe_m1: "voxygen.element.icons.2haxe_m1",
        twohaxe_m2: "voxygen.element.icons.2haxe_m2",
        bow_m1: "voxygen.element.icons.bow_m1",
        bow_m2: "voxygen.element.icons.bow_m2",
        staff_m1: "voxygen.element.icons.staff_m1",
        staff_m2: "voxygen.element.icons.staff_m2",
        flyingrod_m1: "voxygen.element.icons.debug_wand_m1",
        flyingrod_m2: "voxygen.element.icons.debug_wand_m2",
        charge: "voxygen.element.icons.skill_charge_3",


        // Icons
        flower: "voxygen.element.icons.item_flower",
        grass: "voxygen.element.icons.item_grass",
        apple: "voxygen.element.icons.item_apple",
        mushroom: "voxygen.element.icons.item_mushroom",
        skull: "voxygen.element.icons.skull",
        skull_2: "voxygen.element.icons.skull_2",

        // Map
        indicator_mmap: "voxygen.element.buttons.indicator_mmap",
        indicator_mmap_2: "voxygen.element.buttons.indicator_mmap_2",
        indicator_mmap_3: "voxygen.element.buttons.indicator_mmap_3",

        // Crosshair

        crosshair_outer_round: "voxygen.element.misc_bg.crosshair_outer_1",
        crosshair_outer_round_edges: "voxygen.element.misc_bg.crosshair_outer_2",
        crosshair_outer_edges: "voxygen.element.misc_bg.crosshair_outer_3",

        crosshair_bg: "voxygen.element.misc_bg.crosshair_bg",
        crosshair_bg_hover: "voxygen.element.misc_bg.crosshair_bg_hover",
        crosshair_bg_press: "voxygen.element.misc_bg.crosshair_bg_press",
        crosshair_bg_pressed: "voxygen.element.misc_bg.crosshair_bg_pressed",

        // Checkboxes and Radio buttons

        check: "voxygen.element.buttons.radio.inactive",
        check_mo: "voxygen.element.buttons.radio.inactive_hover",
        check_press: "voxygen.element.buttons.radio.press",
        check_checked: "voxygen.element.buttons.radio.active",
        check_checked_mo: "voxygen.element.buttons.radio.hover",
        checkbox: "voxygen.element.buttons.checkbox.inactive",
        checkbox_mo: "voxygen.element.buttons.checkbox.inactive_hover",
        checkbox_press: "voxygen.element.buttons.checkbox.press",
        checkbox_checked: "voxygen.element.buttons.checkbox.active",
        checkbox_checked_mo: "voxygen.element.buttons.checkbox.hover",

        // Buttons
        mmap_closed: "voxygen.element.buttons.button_mmap_closed",
        mmap_closed_hover: "voxygen.element.buttons.button_mmap_closed_hover",
        mmap_closed_press: "voxygen.element.buttons.button_mmap_closed_press",
        mmap_open: "voxygen.element.buttons.button_mmap_open",
        mmap_open_hover: "voxygen.element.buttons.button_mmap_open_hover",
        mmap_open_press: "voxygen.element.buttons.button_mmap_open_press",
        mmap_plus: "voxygen.element.buttons.min_plus.mmap_button-plus",
        mmap_plus_hover: "voxygen.element.buttons.min_plus.mmap_button-plus_hover",
        mmap_plus_press: "voxygen.element.buttons.min_plus.mmap_button-plus_press",
        mmap_minus: "voxygen.element.buttons.min_plus.mmap_button-min",
        mmap_minus_hover: "voxygen.element.buttons.min_plus.mmap_button-min_hover",
        mmap_minus_press: "voxygen.element.buttons.min_plus.mmap_button-min_press",

        // Grid
        grid: "voxygen.element.buttons.grid",
        grid_hover: "voxygen.element.buttons.grid",
        grid_press: "voxygen.element.buttons.grid",

        settings: "voxygen.element.buttons.settings",
        settings_hover: "voxygen.element.buttons.settings_hover",
        settings_press: "voxygen.element.buttons.settings_press",

        social: "voxygen.element.buttons.social",
        social_hover: "voxygen.element.buttons.social_hover",
        social_press: "voxygen.element.buttons.social_press",

        map_button: "voxygen.element.buttons.map",
        map_hover: "voxygen.element.buttons.map_hover",
        map_press: "voxygen.element.buttons.map_press",

        spellbook_button: "voxygen.element.buttons.spellbook",
        spellbook_hover: "voxygen.element.buttons.spellbook_hover",
        spellbook_press: "voxygen.element.buttons.spellbook_press",

        character_button: "voxygen.element.buttons.character",
        character_hover: "voxygen.element.buttons.character_hover",
        character_press: "voxygen.element.buttons.character_press",

        qlog_button: "voxygen.element.buttons.qlog",
        qlog_hover: "voxygen.element.buttons.qlog_hover",
        qlog_press: "voxygen.element.buttons.qlog_press",

        // Charwindow
        xp_charwindow: "voxygen.element.frames.xp_charwindow",
        divider: "voxygen.element.frames.divider_charwindow",
        head_bg: "voxygen.element.icons.head",
        shoulders_bg: "voxygen.element.icons.shoulders",
        hands_bg: "voxygen.element.icons.hands",
        belt_bg: "voxygen.element.icons.belt",
        legs_bg: "voxygen.element.icons.legs",
        feet_bg: "voxygen.element.icons.feet",
        ring_r_bg: "voxygen.element.icons.ring",
        ring_l_bg: "voxygen.element.icons.ring",
        tabard_bg: "voxygen.element.icons.tabard",
        chest_bg: "voxygen.element.icons.chest",
        back_bg: "voxygen.element.icons.back",
        gem_bg: "voxygen.element.icons.gem",
        necklace_bg: "voxygen.element.icons.necklace",
        mainhand_bg: "voxygen.element.icons.mainhand",
        offhand_bg: "voxygen.element.icons.offhand",

        // Close button
        close_button: "voxygen.element.buttons.x",
        close_button_hover: "voxygen.element.buttons.x_hover",
        close_button_press: "voxygen.element.buttons.x_press",

        // Esc-Menu
        fireplace: "voxygen.element.misc_bg.fireplace",
        button: "voxygen.element.buttons.button",
        button_hover: "voxygen.element.buttons.button_hover",
        button_press: "voxygen.element.buttons.button_press",

        // Items
        potion_red: "voxygen.voxel.object.potion_red",
        potion_green: "voxygen.voxel.object.potion_green",
        potion_blue: "voxygen.voxel.object.potion_blue",
        key: "voxygen.voxel.object.key",
        key_gold: "voxygen.voxel.object.key_gold",





//////////////////////////////////////////////////////////////////////////////////////////////////////

        <ImageGraphic>

        not_found:"voxygen.element.not_found",

        help:"voxygen.element.help",

        charwindow_gradient:"voxygen.element.misc_bg.charwindow",

        death_bg: "voxygen.background.death",
        hurt_bg: "voxygen.background.hurt",

        // Enemy Healthbar
        enemy_health: "voxygen.element.frames.enemybar",
        // Enemy Bar Content:
        enemy_bar: "voxygen.element.skillbar.enemy_bar_content",
        // Spell Book Window
        spellbook_icon: "voxygen.element.icons.spellbook",
        // Bag
        bag: "voxygen.element.buttons.bag.closed",
        bag_hover: "voxygen.element.buttons.bag.closed_hover",
        bag_press: "voxygen.element.buttons.bag.closed_press",
        bag_open: "voxygen.element.buttons.bag.open",
        bag_open_hover: "voxygen.element.buttons.bag.open_hover",
        bag_open_press: "voxygen.element.buttons.bag.open_press",

        map_icon: "voxygen.element.icons.map",

        grid_button: "voxygen.element.buttons.border",
        grid_button_hover: "voxygen.element.buttons.border_mo",
        grid_button_press: "voxygen.element.buttons.border_press",
        grid_button_open: "voxygen.element.buttons.border_pressed",

        // Char Window
        progress_frame: "voxygen.element.frames.progress_bar",
        progress: "voxygen.element.misc_bg.progress",

        // Quest-Log Window
        questlog_icon: "voxygen.element.icons.questlog",


        // Social Window
        social_icon: "voxygen.element.icons.social",

        <BlankGraphic>
        nothing: (),
    }
}
