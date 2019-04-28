image_ids! {
    pub struct Imgs {
        <dot_vox::DotVoxData>
        // Bag
        bag_contents: "/voxygen/element/frames/bag.vox",
        inv_grid: "/voxygen/element/frames/inv_grid.vox",
        inv_slot: "/voxygen/element/buttons/inv_slot.vox",

        // Buttons
        settings: "/voxygen/element/buttons/settings.vox",
        settings_hover: "/voxygen/element/buttons/settings_hover.vox",
        settings_press: "/voxygen/element/buttons/settings_press.vox",

        social_button: "/voxygen/element/buttons/social.vox",
        social_hover: "/voxygen/element/buttons/social_hover.vox",
        social_press: "/voxygen/element/buttons/social_press.vox",

        map_button: "/voxygen/element/buttons/map.vox",
        map_hover: "/voxygen/element/buttons/map_hover.vox",
        map_press: "/voxygen/element/buttons/map_press.vox",

        spellbook_button: "/voxygen/element/buttons/spellbook.vox",
        spellbook_hover: "/voxygen/element/buttons/spellbook_hover.vox",
        spellbook_press: "/voxygen/element/buttons/spellbook_press.vox",

        character_button: "/voxygen/element/buttons/character.vox",
        character_hover: "/voxygen/element/buttons/character_hover.vox",
        character_press: "/voxygen/element/buttons/character_press.vox",

        qlog_button: "/voxygen/element/buttons/qlog.vox",
        qlog_hover: "/voxygen/element/buttons/qlog_hover.vox",
        qlog_press: "/voxygen/element/buttons/qlog_press.vox",

        close_button: "/voxygen/element/buttons/x.vox",
        close_button_hover: "/voxygen/element/buttons/x_hover.vox",
        close_button_press: "/voxygen/element/buttons/x_press.vox",

        //  Esc menu
        fireplace: "/voxygen/element/misc_bg/fireplace.vox",
        button_dark: "/voxygen/element/buttons/button_dark.vox",

        // Minimap
        mmap_frame: "/voxygen/element/frames/mmap.vox",
        window_frame: "/voxygen/element/frames/window2.vox",
        map_frame_l: "/voxygen/element/frames/map_l.vox",
        map_frame_r: "/voxygen/element/frames/map_r.vox",

        <image::DynamicImage>
        // Bag
        bag: "/voxygen/element/buttons/bag/closed.png",
        bag_hover: "/voxygen/element/buttons/bag/closed_hover.png",
        bag_press: "/voxygen/element/buttons/bag/closed_press.png",
        bag_open: "/voxygen/element/buttons/bag/open.png",
        bag_open_hover: "/voxygen/element/buttons/bag/open_hover.png",
        bag_open_press: "/voxygen/element/buttons/bag/open_press.png",

        // Buttons
        mmap_button: "/voxygen/element/buttons/border.png",
        mmap_button_hover: "/voxygen/element/buttons/border_mo.png",
        mmap_button_press: "/voxygen/element/buttons/border_press.png",
        mmap_button_open: "/voxygen/element/buttons/border_pressed.png",

        // Esc-Menu
        esc_bg: "/voxygen/element/frames/menu.png",
        button_dark_hover: "/voxygen/element/buttons/button_dark_hover.png",
        button_dark_press: "/voxygen/element/buttons/button_dark_press.png",

        // MiniMap
        mmap_frame_bg: "/voxygen/element/misc_bg/mmap_bg.png",

        // Skillbar Module
        sb_grid: "/voxygen/element/skill_bar/sbar_grid.png",
        sb_grid_bg: "/voxygen/element/skill_bar/sbar_grid_bg.png",
        l_click: "/voxygen/element/skill_bar/l.png",
        r_click: "/voxygen/element/skill_bar/r.png",
        mana_bar: "/voxygen/element/skill_bar/mana_bar.png",
        health_bar: "/voxygen/element/skill_bar/health_bar.png",
        xp_bar: "/voxygen/element/skill_bar/xp_bar.png",

        // Missing: Buff Frame Animation (.gif ?!) (we could do animation in ui.maintain(), or in shader?)
        window_frame_2: "/voxygen/element/frames/window_2.png",

        // Settings Window
        settings_bg: "/voxygen/element/frames/settings.png",
        settings_icon: "/voxygen/element/icons/settings.png",
        settings_button_mo: "/voxygen/element/buttons/blue_mo.png",
        check: "/voxygen/element/buttons/check/no.png",
        check_mo: "/voxygen/element/buttons/check/no_mo.png",
        check_press: "/voxygen/element/buttons/check/press.png",
        check_checked: "/voxygen/element/buttons/check/yes.png",
        check_checked_mo: "/voxygen/element/buttons/check/yes_mo.png",
        slider: "/voxygen/element/slider/track.png",
        slider_indicator: "/voxygen/element/slider/indicator.png",
        //button_blank:  ui.new_graphic(ui::Graphic::Blank),
        button_blue_mo: "/voxygen/element/buttons/blue_mo.png",
        button_blue_press: "/voxygen/element/buttons/blue_press.png",

        // Window BG
        window_bg: "/voxygen/element/misc_bg/window_bg.png",

        // Social Window
        social_bg: "/voxygen/element/misc_bg/small_bg.png",
        social_icon: "/voxygen/element/icons/social.png",

        // Map Window
        map_bg: "/voxygen/element/misc_bg/small_bg.png",
        map_icon: "/voxygen/element/icons/map.png",

        // Spell Book Window
        spellbook_bg: "/voxygen/element/misc_bg/small_bg.png",
        spellbook_icon: "/voxygen/element/icons/spellbook.png",

        // Char Window
        charwindow: "/voxygen/element/misc_bg/charwindow.png",
        charwindow_icon: "/voxygen/element/icons/charwindow.png",
        charwindow_tab_bg: "/voxygen/element/frames/tab.png",
        charwindow_tab: "/voxygen/element/buttons/tab.png",
        charwindow_expbar: "/voxygen/element/misc_bg/small_bg.png",
        progress_frame: "/voxygen/element/frames/progress_bar.png",
        progress: "/voxygen/element/misc_bg/progress.png",

        // Quest-Log Window
        questlog_bg: "/voxygen/element/misc_bg/small_bg.png",
        questlog_icon: "/voxygen/element/icons/questlog.png",

        // Chat-Arrows
        chat_arrow: "/voxygen/element/buttons/arrow/chat_arrow.png",
        chat_arrow_mo: "/voxygen/element/buttons/arrow/chat_arrow_mo.png",
        chat_arrow_press: "/voxygen/element/buttons/arrow/chat_arrow_press.png",
    }
}
