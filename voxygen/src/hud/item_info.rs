use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::{animate_by_pulse, ItemImgs, ItemKey::Tool as ToolKey},
    util,
};

use crate::{
    hud::get_quality_col,
    i18n::Localization,
    ui::{fonts::Fonts, ImageFrame, Ingameable, Tooltip, TooltipManager, Tooltipable},
};
use client::Client;
use common::{
    combat::{combat_rating, Damage},
    comp::item::{
        armor::{Armor, ArmorKind, Protection},
        tool::{Hands, StatKind, Stats, Tool, ToolKind},
        Item, ItemDesc, ItemKind, MaterialStatManifest, Quality,
    },
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    pub struct Ids {
        title,
        subtitle,
        desc,
        stat1,
        stat2,
        stat3,
        diff1,
        diff2,
        diff3,
        item_frame,
        item_render,
        background,
    }
}

#[derive(WidgetCommon)]
pub struct ItemInfo<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    pulse: f32,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    //rot_imgs: &'a ImgsRot,
    //tooltip_manager: &'a mut TooltipManager,
    localized_strings: &'a Localization,
    item: &'a Item,
    msm: &'a MaterialStatManifest,
}

impl<'a> ItemInfo<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        pulse: f32,
        //rot_imgs: &'a ImgsRot,
        //tooltip_manager: &'a mut TooltipManager,
        localized_strings: &'a Localization,
        item: &'a Item,
        msm: &'a MaterialStatManifest,
    ) -> Self {
        Self {
            client,
            imgs,
            item_imgs,
            fonts,
            pulse,
            common: widget::CommonBuilder::default(),
            //rot_imgs,
            //tooltip_manager,
            localized_strings,
            item,
            msm,
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Ingameable for ItemInfo<'a> {
    fn prim_count(&self) -> usize {
        // Number of conrod primitives contained in the overitem display.
        // TODO maybe this could be done automatically?
        // - 2 Text for name
        // - 0 or 2 Rectangle and Text for button
        4 + match self.item.kind() {
            ItemKind::Tool(_) => 3,
            ItemKind::Armor(_) => 2,
            _ => 0,
        }
    }
}

pub enum Event {
    //Show(bool),
}

impl<'a> Widget for ItemInfo<'a> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let item = self.item;
        let _i18n = &self.localized_strings;

        let inventories = self.client.inventories();
        let inventory = match inventories.get(self.client.entity()) {
            Some(l) => l,
            None => return None,
        };

        let equip_slot = inventory.equipped_items_of_kind(self.item.kind().clone());

        let (title, desc) = (item.name().to_string(), item.description().to_string());

        let quality = get_quality_col(self.item);

        let subtitle = util::kind_text(item.kind());

        let text_color = conrod_core::color::WHITE;

        let art_size = [64.0, 64.0];

        /*// Apply transparency
        let color = style.color(ui.theme()).alpha(self.transparency);

        // Background image frame
        self.image_frame
            .wh(rect.dim())
            .xy(rect.xy())
            .graphics_for(id)
            .parent(id)
            .color(color)
            .set(state.ids.image_frame, ui);*/

        widget::Rectangle::fill([310.0, 310.0])
            .color(Color::Rgba(0.0, 0.0, 0.0, 0.98))
            .depth(1.0)
            .set(state.ids.background, ui);

        // Icon BG
        let quality_col_img = match &item.quality() {
            Quality::Low => self.imgs.inv_slot_grey,
            Quality::Common => self.imgs.inv_slot,
            Quality::Moderate => self.imgs.inv_slot_green,
            Quality::High => self.imgs.inv_slot_blue,
            Quality::Epic => self.imgs.inv_slot_purple,
            Quality::Legendary => self.imgs.inv_slot_gold,
            Quality::Artifact => self.imgs.inv_slot_orange,
            _ => self.imgs.inv_slot_red,
        };
        Image::new(quality_col_img)
            .w_h(art_size[0] + 10.0, art_size[1] + 10.0)
            .top_left_with_margin_on(state.ids.background, 10.0)
            .set(state.ids.item_frame, ui);

        // Icon
        Image::new(animate_by_pulse(
            &self.item_imgs.img_ids_or_not_found_img(item.into()),
            self.pulse,
        ))
        .color(Some(conrod_core::color::WHITE))
        .wh(art_size)
        .middle_of(state.ids.item_frame)
        .set(state.ids.item_render, ui);

        // Title
        Text::new(&title)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(25))
            .y_align_to(state.ids.item_frame, conrod_core::position::Align::End)
            .right_from(state.ids.item_frame, 10.0)
            .color(quality)
            .depth(2.0)
            .set(state.ids.title, ui);

        // Subtitle
        Text::new(&subtitle)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(15))
            .color(conrod_core::color::GREY)
            .depth(3.0)
            .set(state.ids.subtitle, ui);

        // Stats
        match item.kind() {
            ItemKind::Tool(tool) => {
                let stat1 = tool.base_power(self.msm, item.components()) * 10.0;
                let stat2 = tool.base_speed(self.msm, item.components()) * 10.0;
                let stat3 = tool.base_poise_strength(self.msm, item.components()) * 10.0;

                Text::new(&format!("Power : {}", stat1.to_string()))
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(15))
                    .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                    .color(text_color)
                    .depth(3.0)
                    .set(state.ids.stat1, ui);
                Text::new(&format!("Speed : {}", stat2.to_string()))
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(15))
                    .color(text_color)
                    .depth(3.0)
                    .set(state.ids.stat2, ui);
                Text::new(&format!("Poise : {}", stat3.to_string()))
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(15))
                    .color(text_color)
                    .depth(3.0)
                    .set(state.ids.stat3, ui);
                if let Some(equipped_item) = equip_slot.cloned().next() {
                    if let ItemKind::Tool(equipped_tool) = equipped_item.kind() {
                        let tool_stats = tool
                            .stats
                            .resolve_stats(self.msm, item.components())
                            .clamp_speed();
                        let equipped_tool_stats = equipped_tool
                            .stats
                            .resolve_stats(self.msm, equipped_item.components())
                            .clamp_speed();
                        let diff = tool_stats - equipped_tool_stats;
                        let diff1 = util::comparison(tool_stats.power, equipped_tool_stats.power);
                        let diff2 = util::comparison(tool_stats.speed, equipped_tool_stats.speed);
                        let diff3 = util::comparison(
                            tool_stats.poise_strength,
                            equipped_tool_stats.poise_strength,
                        );

                        Text::new(&format!("{} {:.1}", &diff1.0, &diff.power * 10.0))
                            .font_id(self.fonts.cyri.conrod_id)
                            .font_size(self.fonts.cyri.scale(15))
                            .color(diff1.1)
                            .align_middle_y_of(state.ids.stat1)
                            .right_from(state.ids.stat1, 10.0)
                            .depth(3.0)
                            .set(state.ids.diff1, ui);
                        Text::new(&format!("{} {:.1}", &diff2.0, &diff.speed * 10.0))
                            .font_id(self.fonts.cyri.conrod_id)
                            .font_size(self.fonts.cyri.scale(15))
                            .color(diff2.1)
                            .align_middle_y_of(state.ids.stat2)
                            .right_from(state.ids.stat2, 10.0)
                            .depth(3.0)
                            .set(state.ids.diff2, ui);
                        Text::new(&format!("{} {:.1}", &diff3.0, &diff.poise_strength * 10.0))
                            .font_id(self.fonts.cyri.conrod_id)
                            .font_size(self.fonts.cyri.scale(15))
                            .color(diff3.1)
                            .align_middle_y_of(state.ids.stat3)
                            .right_from(state.ids.stat3, 10.0)
                            .depth(3.0)
                            .set(state.ids.diff3, ui);
                    }
                }
            },
            ItemKind::Armor(armor) => {
                let stat1 = armor.get_protection();
                let stat2 = armor.get_poise_resilience();

                Text::new(&format!("Armour : {}", util::protec2string(stat1)))
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(15))
                    .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
                    .color(text_color)
                    .depth(3.0)
                    .set(state.ids.stat1, ui);
                Text::new(&format!("Poise res : {}", util::protec2string(stat2)))
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(15))
                    .color(text_color)
                    .depth(3.0)
                    .set(state.ids.stat2, ui);

                if let Some(equipped_item) = equip_slot.cloned().next() {
                    if let ItemKind::Armor(equipped_armor) = equipped_item.kind() {
                        let diff = armor.stats - equipped_armor.stats;
                        let diff1 = util::comparison(
                            &armor.stats.protection,
                            &equipped_armor.stats.protection,
                        );
                        let diff2 = util::comparison(
                            &armor.stats.poise_resilience,
                            &equipped_armor.stats.poise_resilience,
                        );

                        Text::new(&format!(
                            "{} {}",
                            &diff1.0,
                            util::protec2string(diff.protection)
                        ))
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(15))
                        .color(diff1.1)
                        .align_middle_y_of(state.ids.stat1)
                        .right_from(state.ids.stat1, 10.0)
                        .depth(3.0)
                        .set(state.ids.diff1, ui);
                        Text::new(&format!(
                            "{} {}",
                            &diff2.0,
                            util::protec2string(diff.protection)
                        ))
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(15))
                        .color(diff2.1)
                        .align_middle_y_of(state.ids.stat2)
                        .right_from(state.ids.stat2, 10.0)
                        .depth(3.0)
                        .set(state.ids.diff2, ui);
                    }
                }
            },
            _ => (),
        }

        Text::new(&desc)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(15))
            .x_align_to(state.ids.item_frame, conrod_core::position::Align::Start)
            .color(conrod_core::color::GREY)
            .depth(3.0)
            .w(300.0)
            .set(state.ids.desc, ui);

        /*let test = widget::Text::new(&desc).w(300.0).get_h(ui);
        dbg!(test);*/

        /*// Items
        let stats_count: usize = match item.kind() {
            ItemKind::Armor(armor) => 2,
            ItemKind::Tool(tool) => 4,
            _ => 0,
        };
        let gen = &mut ui.widget_id_generator();
        state.update(|state| state.ids.stats.resize(item_count, gen));
        state.update(|state| state.ids.stats_icons.resize(item_count, gen));

        // Create Stats Widgets
        let stats_vec = state
            .ids
            .stats
            .iter()
            .copied()
            .zip(state.ids.stats_icons.iter().copied())
            .zip(stats)
            .collect::<Vec<_>>();*/

        None
    }
}
