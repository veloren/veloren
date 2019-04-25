use super::Asset;

use crate::ui::{Ui, Graphic};

use common::figure::Segment;

use dot_vox::DotVoxData;
use image::DynamicImage;
use conrod_core::image::Id as ImgId;

pub trait UiId where Self: std::marker::Sized {
    fn to_ui_asset(self, ui: &mut Ui) -> ImgId;
}

impl UiId for DynamicImage {
    fn to_ui_asset(self, ui: &mut Ui) -> ImgId {
        ui.new_graphic(Graphic::Image(self))
    }
}

impl UiId for DotVoxData {
    fn to_ui_asset(self, ui: &mut Ui) -> ImgId {
        ui.new_graphic(Graphic::Voxel(Segment::from(self)))
    }
}

/// This macro will automatically load all specified assets, get the corresponding ImgIds and
/// create a struct with all of them
///
/// Example usage:
/// ```
/// image_ids! {
///     struct<DotVoxData> Voxs {
///         button1: "filename1.vox",
///         button2: "filename2.vox",
///     }
///     struct<DynamicImage> Voxs {
///         background: "background.png",
///     }
/// }
/// ```
macro_rules! image_ids {
    ($(pub struct<$T:ty> $Ids:ident { $( $name:ident: $file:expr ), *$(,)? } )*) => {
        $(
            pub struct $Ids {
                $( $name: ImgId, )*
            }

            impl $Ids {
                pub fn load(ui: &mut Ui) -> Result<Self, std::io::Error> {
                    Ok(Self {
                        $( $name: Id::to_ui_asset(<$T>::load($file)?, ui), )*
                    })
                }
            }
        )*
    };
}

image_ids! {
    pub struct<DotVoxData> Voxs {
        // Bag
        bag_contents: "element/frames/bag.vox",
        inv_grid: "element/frames/inv_grid.vox",
        inv_slot: "element/buttons/inv_slot.vox",

        // Buttons
        settings: "element/buttons/settings.vox",
        settings_hover: "element/buttons/settings_hover.vox",
        settings_press: "element/buttons/settings_press.vox",

        social_button: "element/buttons/social.vox",
        social_hover: "element/buttons/social_hover.vox",
        social_press: "element/buttons/social_press.vox",

        map_button: "element/buttons/map.vox",
        map_hover: "element/buttons/map_hover.vox",
        map_press: "element/buttons/map_press.vox",

        spellbook_button: "element/buttons/spellbook.vox",
        spellbook_hover: "element/buttons/spellbook_hover.vox",
        spellbook_press: "element/buttons/spellbook_press.vox",

        character_button: "element/buttons/character.vox",
        character_hover: "element/buttons/character_hover.vox",
        character_press: "element/buttons/character_press.vox",

        qlog_button: "element/buttons/qlog.vox",
        qlog_hover: "element/buttons/qlog_hover.vox",
        qlog_press: "element/buttons/qlog_press.vox",

        close_button: "element/buttons/x.vox",
        close_button_hover: "element/buttons/x_hover.vox",
        close_button_press: "element/buttons/x_press.vox",

        //  Esc menu
        fireplace: "element/misc_bg/fireplace.vox",
        button_dark: "element/buttons/button_dark.vox",

        // Minimap
        mmap_frame: "element/frames/mmap.vox",
        window_frame: "element/frames/window2.vox",
        map_frame_l: "element/frames/map_l.vox",
        map_frame_r: "element/frames/map_r.vox",
    }

    pub struct<DynamicImage> Imgs {
        // Bag
        bag: "element/buttons/bag/closed.png",
        bag_hover: "element/buttons/bag/closed_hover.png",
        bag_press: "element/buttons/bag/closed_press.png",
        bag_open: "element/buttons/bag/open.png",
        bag_open_hover: "element/buttons/bag/open_hover.png",
        bag_open_press: "element/buttons/bag/open_press.png",

        // Buttons
        mmap_button: "element/buttons/border.png",
        mmap_button_hover: "element/buttons/border_mo.png",
        mmap_button_press: "element/buttons/border_press.png",
        mmap_button_open: "element/buttons/border_pressed.png",

        // Esc-Menu
        esc_bg: "element/frames/menu.png",
        button_dark_hover: "element/buttons/button_dark_hover.png",
        button_dark_press: "element/buttons/button_dark_press.png",

        // MiniMap
        mmap_frame_bg: "element/misc_bg/mmap_bg.png",

        // Skillbar Module
        sb_grid: "element/skill_bar/sbar_grid.png",
        sb_grid_bg: "element/skill_bar/sbar_grid_bg.png",
        l_click: "element/skill_bar/l.png",
        r_click: "element/skill_bar/r.png",
        mana_bar: "element/skill_bar/mana_bar.png",
        health_bar: "element/skill_bar/health_bar.png",
        xp_bar: "element/skill_bar/xp_bar.png",

        // Missing: Buff Frame Animation (.gif ?!) (we could do animation in ui.maintain(), or in shader?)
        window_frame_2: "element/frames/window_2.png",

        // Settings Window
        settings_bg: "element/frames/settings.png",
        settings_icon: "element/icons/settings.png",
        settings_button_mo: "element/buttons/blue_mo.png",
        check: "element/buttons/check/no.png",
        check_mo: "element/buttons/check/no_mo.png",
        check_press: "element/buttons/check/press.png",
        check_checked: "element/buttons/check/yes.png",
        check_checked_mo: "element/buttons/check/yes_mo.png",
        slider: "element/slider/track.png",
        slider_indicator: "element/slider/indicator.png",
        //button_blank:  ui.new_graphic(ui::Graphic::Blank),
        button_blue_mo: "element/buttons/blue_mo.png",
        button_blue_press: "element/buttons/blue_press.png",

        // Window BG
        window_bg: "element/misc_bg/window_bg.png",

        // Social Window
        social_bg: "element/misc_bg/small_bg.png",
        social_icon: "element/icons/social.png",

        // Map Window
        map_bg: "element/misc_bg/small_bg.png",
        map_icon: "element/icons/map.png",

        // Spell Book Window
        spellbook_bg: "element/misc_bg/small_bg.png",
        spellbook_icon: "element/icons/spellbook.png",

        // Char Window
        charwindow: "element/misc_bg/charwindow.png",
        charwindow_icon: "element/icons/charwindow.png",
        charwindow_tab_bg: "element/frames/tab.png",
        charwindow_tab: "element/buttons/tab.png",
        charwindow_expbar: "element/misc_bg/small_bg.png",
        progress_frame: "element/frames/progress_bar.png",
        progress: "element/misc_bg/progress.png",

        // Quest-Log Window
        questlog_bg: "element/misc_bg/small_bg.png",
        questlog_icon: "element/icons/questlog.png",

        // Chat-Arrows
        chat_arrow: "element/buttons/arrow/chat_arrow.png",
        chat_arrow_mo: "element/buttons/arrow/chat_arrow_mo.png",
        chat_arrow_press: "element/buttons/arrow/chat_arrow_press.png",
    }
}
