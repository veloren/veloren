use crate::ui::img_ids::{BlankGraphic, ImageGraphic, VoxelGraphic, VoxelMs9Graphic};

image_ids! {
    pub struct Imgs {
        <VoxelGraphic>

        // Bag
        bag_contents: "voxygen/element/frames/bag.vox",
        inv_grid: "voxygen/element/frames/inv_grid.vox",
        inv_slot: "voxygen/element/buttons/inv_slot.vox",
        grid_inv: "voxygen/element/buttons/grid_inv.vox",
        bag_top: "voxygen/element/bag/top.vox",
        bag_mid: "voxygen/element/bag/mid.vox",
        bag_bot: "voxygen/element/bag/bot.vox",

        // Window Parts
        window_3: "voxygen/element/frames/window_3.vox",
        tab_bg: "voxygen/element/frames/tab_bg.vox",
        tab_small_open: "voxygen/element/frames/tab_small_open.vox",
        tab_small_closed: "voxygen/element/frames/tab_small_closed.vox",

        // MiniMap
        mmap_frame: "voxygen/element/frames/mmap.vox",
        mmap_frame_closed: "voxygen/element/frames/mmap_closed.vox",

        // Missing: Buff Frame Animation .gif ?! we could do animation in ui.maintain, or in shader?
        window_frame: "voxygen/element/frames/window2.vox",

        // Settings Window
        settings_frame_r: "voxygen/element/frames/settings_r.vox",
        settings_frame_l: "voxygen/element/frames/settings_l.vox",
        settings_button: "voxygen/element/buttons/settings_button.vox",
        settings_button_pressed: "voxygen/element/buttons/settings_button_pressed.vox",
        settings_button_hover: "voxygen/element/buttons/settings_button_hover.vox",
        settings_button_press: "voxygen/element/buttons/settings_button_press.vox",
        check: "voxygen/element/buttons/check/no.vox",
        check_mo: "voxygen/element/buttons/check/no_mo.vox",
        check_press: "voxygen/element/buttons/check/press.vox",
        check_checked: "voxygen/element/buttons/check/yes.vox",
        check_checked_mo: "voxygen/element/buttons/check/yes_mo.vox",
        slider: "voxygen/element/slider/track.vox",
        slider_indicator: "voxygen/element/slider/indicator.vox",
        esc_frame: "voxygen/element/frames/esc_menu.vox",

        // Map Window
        map_frame_l: "voxygen/element/frames/map_l.vox",
        map_frame_r: "voxygen/element/frames/map_r.vox",
        map_frame_bl: "voxygen/element/frames/map_bl.vox",
        map_frame_br: "voxygen/element/frames/map_br.vox",

        // Chat-Arrows
        chat_arrow: "voxygen/element/buttons/arrow_down.vox",
        chat_arrow_mo: "voxygen/element/buttons/arrow_down_hover.vox",
        chat_arrow_press: "voxygen/element/buttons/arrow_down_press.vox",

        // Crosshair
        crosshair_inner: "voxygen/element/misc_bg/crosshair_inner.vox",

////////////////////////////////////////////////////////////////////////
        <VoxelMs9Graphic>

        crosshair_outer_round: "voxygen/element/misc_bg/crosshair_outer_1.vox",
        crosshair_outer_round_edges: "voxygen/element/misc_bg/crosshair_outer_2.vox",
        crosshair_outer_edges: "voxygen/element/misc_bg/crosshair_outer_3.vox",

        crosshair_bg: "voxygen/element/misc_bg/crosshair_bg.vox",
        crosshair_bg_hover: "voxygen/element/misc_bg/crosshair_bg_hover.vox",
        crosshair_bg_press: "voxygen/element/misc_bg/crosshair_bg_press.vox",
        crosshair_bg_pressed: "voxygen/element/misc_bg/crosshair_bg_pressed.vox",

        // Buttons
        mmap_closed: "voxygen/element/buttons/button_mmap_closed.vox",
        mmap_closed_hover: "voxygen/element/buttons/button_mmap_closed_hover.vox",
        mmap_closed_press: "voxygen/element/buttons/button_mmap_closed_press.vox",
        mmap_open: "voxygen/element/buttons/button_mmap_open.vox",
        mmap_open_hover: "voxygen/element/buttons/button_mmap_open_hover.vox",
        mmap_open_press: "voxygen/element/buttons/button_mmap_open_press.vox",

        // Grid
        grid: "voxygen/element/buttons/grid.vox",
        grid_hover: "voxygen/element/buttons/grid.vox",
        grid_press: "voxygen/element/buttons/grid.vox",

        settings: "voxygen/element/buttons/settings.vox",
        settings_hover: "voxygen/element/buttons/settings_hover.vox",
        settings_press: "voxygen/element/buttons/settings_press.vox",

        social_button: "voxygen/element/buttons/social.vox",
        social_hover: "voxygen/element/buttons/social_hover.vox",
        social_press: "voxygen/element/buttons/social_press.vox",

        map_button: "voxygen/element/buttons/map.vox",
        map_hover: "voxygen/element/buttons/map_hover.vox",
        map_press: "voxygen/element/buttons/map_press.vox",

        spellbook_button: "voxygen/element/buttons/spellbook.vox",
        spellbook_hover: "voxygen/element/buttons/spellbook_hover.vox",
        spellbook_press: "voxygen/element/buttons/spellbook_press.vox",

        character_button: "voxygen/element/buttons/character.vox",
        character_hover: "voxygen/element/buttons/character_hover.vox",
        character_press: "voxygen/element/buttons/character_press.vox",

        qlog_button: "voxygen/element/buttons/qlog.vox",
        qlog_hover: "voxygen/element/buttons/qlog_hover.vox",
        qlog_press: "voxygen/element/buttons/qlog_press.vox",

        // Charwindow
        xp_charwindow: "voxygen/element/frames/xp_charwindow.vox",
        divider: "voxygen/element/frames/divider_charwindow.vox",
        head_bg: "voxygen/element/icons/head.vox",
        shoulders_bg: "voxygen/element/icons/shoulders.vox",
        hands_bg: "voxygen/element/icons/hands.vox",
        belt_bg: "voxygen/element/icons/belt.vox",
        legs_bg: "voxygen/element/icons/legs.vox",
        feet_bg: "voxygen/element/icons/feet.vox",
        ring_r_bg: "voxygen/element/icons/ring.vox",
        ring_l_bg: "voxygen/element/icons/ring.vox",
        tabard_bg: "voxygen/element/icons/tabard.vox",
        chest_bg: "voxygen/element/icons/chest.vox",
        back_bg: "voxygen/element/icons/back.vox",
        gem_bg: "voxygen/element/icons/gem.vox",
        necklace_bg: "voxygen/element/icons/necklace.vox",
        mainhand_bg: "voxygen/element/icons/mainhand.vox",
        offhand_bg: "voxygen/element/icons/offhand.vox",

        // Close button
        close_button: "voxygen/element/buttons/x.vox",
        close_button_hover: "voxygen/element/buttons/x_hover.vox",
        close_button_press: "voxygen/element/buttons/x_press.vox",

        // Esc-Menu
        fireplace: "voxygen/element/misc_bg/fireplace.vox",
        button: "voxygen/element/buttons/button.vox",
        button_hover: "voxygen/element/buttons/button_hover.vox",
        button_press: "voxygen/element/buttons/button_press.vox",

        // Items
        potion_red: "voxygen/voxel/object/potion_red.vox",
        potion_green: "voxygen/voxel/object/potion_green.vox",
        potion_blue: "voxygen/voxel/object/potion_blue.vox",
        key: "voxygen/voxel/object/key.vox",
        key_gold: "voxygen/voxel/object/key_gold.vox",
//////////////////////////////////////////////////////////////////////////////////////////////////////
        <ImageGraphic>

        charwindow_gradient:"voxygen/element/misc_bg/charwindow.png",

        // Spell Book Window
        spellbook_icon: "voxygen/element/icons/spellbook.png",
        // Bag
        bag: "voxygen/element/buttons/bag/closed.png",
        bag_hover: "voxygen/element/buttons/bag/closed_hover.png",
        bag_press: "voxygen/element/buttons/bag/closed_press.png",
        bag_open: "voxygen/element/buttons/bag/open.png",
        bag_open_hover: "voxygen/element/buttons/bag/open_hover.png",
        bag_open_press: "voxygen/element/buttons/bag/open_press.png",

        map_icon: "voxygen/element/icons/map.png",

        grid_button: "voxygen/element/buttons/border.png",
        grid_button_hover: "voxygen/element/buttons/border_mo.png",
        grid_button_press: "voxygen/element/buttons/border_press.png",
        grid_button_open: "voxygen/element/buttons/border_pressed.png",

        // Skillbar Module
        sb_grid: "voxygen/element/skill_bar/sbar_grid.png",
        sb_grid_bg: "voxygen/element/skill_bar/sbar_grid_bg.png",
        l_click: "voxygen/element/skill_bar/l.png",
        r_click: "voxygen/element/skill_bar/r.png",
        mana_bar: "voxygen/element/skill_bar/mana_bar.png",
        health_bar: "voxygen/element/skill_bar/health_bar.png",
        xp_bar: "voxygen/element/skill_bar/xp_bar.png",

        esc_bg: "voxygen/element/frames/menu.png",

        window_frame_2: "voxygen/element/frames/window_2.png",

        // Char Window
        charwindow: "voxygen/element/misc_bg/charwindow.png",
        charwindow_icon: "voxygen/element/icons/charwindow.png",
        charwindow_tab_bg: "voxygen/element/frames/tab.png",
        charwindow_tab: "voxygen/element/buttons/tab.png",
        charwindow_expbar: "voxygen/element/misc_bg/small_bg.png",
        progress_frame: "voxygen/element/frames/progress_bar.png",
        progress: "voxygen/element/misc_bg/progress.png",

        // Quest-Log Window
        questlog_icon: "voxygen/element/icons/questlog.png",

        // Window BG
        window_bg: "voxygen/element/misc_bg/window_bg.png",

        // Social Window
        social_icon: "voxygen/element/icons/social.png",

        <BlankGraphic>
        blank: (),
    }
}
