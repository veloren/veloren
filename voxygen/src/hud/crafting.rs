use super::{
    get_quality_col,
    img_ids::{Imgs, ImgsRot},
    item_imgs::{animate_by_pulse, ItemImgs, ItemKey::Tool},
    Show, TEXT_COLOR, TEXT_DULL_RED_COLOR, TEXT_GRAY_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    i18n::Localization,
    ui::{
        fonts::Fonts, ImageFrame, ItemTooltip, ItemTooltipManager, ItemTooltipable, Tooltip,
        TooltipManager, Tooltipable,
    },
};
use client::{self, Client};
use common::{
    assets::AssetExt,
    comp::{
        item::{
            ItemDef, ItemDesc, ItemKind, ItemTag, MaterialStatManifest, Quality, TagExampleInfo,
        },
        Inventory,
    },
    recipe::RecipeInput,
    terrain::SpriteKind,
};
use conrod_core::{
    color, image,
    position::Dimension,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text, TextEdit},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use std::sync::Arc;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

widget_ids! {
    pub struct Ids {
        window,
        window_frame,
        close,
        icon,
        title_main,
        title_rec,
        align_rec,
        scrollbar_rec,
        btn_open_search,
        btn_close_search,
        input_search,
        input_bg_search,
        input_overlay_search,
        title_ing,
        tags_ing[],
        align_ing,
        scrollbar_ing,
        btn_craft,
        recipe_list_btns[],
        recipe_list_labels[],
        recipe_list_quality_indicators[],
        recipe_img_frame[],
        recipe_img[],
        ingredients[],
        ingredient_frame[],
        ingredient_img[],
        req_text[],
        ingredients_txt,
        req_station_title,
        req_station_img,
        req_station_txt,
        output_img_frame,
        output_img,
        output_amount,
        category_bgs[],
        category_tabs[],
        category_imgs[],
    }
}

pub enum Event {
    CraftRecipe(String),
    ChangeCraftingTab(CraftingTab),
    Close,
    Focus(widget::Id),
    SearchRecipe(Option<String>),
}

#[derive(WidgetCommon)]
pub struct Crafting<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    pulse: f32,
    rot_imgs: &'a ImgsRot,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    item_imgs: &'a ItemImgs,
    inventory: &'a Inventory,
    msm: &'a MaterialStatManifest,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    tooltip_manager: &'a mut TooltipManager,
    show: &'a mut Show,
}
#[allow(clippy::too_many_arguments)]
impl<'a> Crafting<'a> {
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        pulse: f32,
        rot_imgs: &'a ImgsRot,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        item_imgs: &'a ItemImgs,
        inventory: &'a Inventory,
        msm: &'a MaterialStatManifest,
        tooltip_manager: &'a mut TooltipManager,
        show: &'a mut Show,
    ) -> Self {
        Self {
            client,
            imgs,
            fonts,
            localized_strings,
            pulse,
            rot_imgs,
            item_tooltip_manager,
            tooltip_manager,
            item_imgs,
            inventory,
            msm,
            show,
            common: widget::CommonBuilder::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, PartialEq)]
pub enum CraftingTab {
    All,
    Armor,
    Weapon,
    Food,
    Dismantle,
    Potion,
    Bag,
    Tool,
    Utility,
    Glider,
    ProcessedMaterial,
}
impl CraftingTab {
    fn name_key(&self) -> &str {
        match self {
            CraftingTab::All => "hud.crafting.tabs.all",
            CraftingTab::Armor => "hud.crafting.tabs.armor",
            CraftingTab::Dismantle => "hud.crafting.tabs.dismantle",
            CraftingTab::Food => "hud.crafting.tabs.food",
            CraftingTab::Glider => "hud.crafting.tabs.glider",
            CraftingTab::Potion => "hud.crafting.tabs.potion",
            CraftingTab::Tool => "hud.crafting.tabs.tool",
            CraftingTab::Utility => "hud.crafting.tabs.utility",
            CraftingTab::Weapon => "hud.crafting.tabs.weapon",
            CraftingTab::Bag => "hud.crafting.tabs.bag",
            CraftingTab::ProcessedMaterial => "hud.crafting.tabs.processed_material",
        }
    }

    fn img_id(&self, imgs: &Imgs) -> image::Id {
        match self {
            CraftingTab::All => imgs.icon_globe,
            CraftingTab::Armor => imgs.icon_armor,
            CraftingTab::Dismantle => imgs.icon_dismantle,
            CraftingTab::Food => imgs.icon_food,
            CraftingTab::Glider => imgs.icon_glider,
            CraftingTab::Potion => imgs.icon_potion,
            CraftingTab::Tool => imgs.icon_tools,
            CraftingTab::Utility => imgs.icon_utility,
            CraftingTab::Weapon => imgs.icon_weapon,
            CraftingTab::Bag => imgs.icon_bag,
            CraftingTab::ProcessedMaterial => imgs.icon_processed_material,
        }
    }

    fn satisfies(&self, item: &ItemDef) -> bool {
        match self {
            CraftingTab::All => true,
            CraftingTab::Food => item.tags().contains(&ItemTag::Food),
            CraftingTab::Armor => match item.kind() {
                ItemKind::Armor(_) => !item.tags().contains(&ItemTag::Bag),
                _ => false,
            },
            CraftingTab::Glider => matches!(item.kind(), ItemKind::Glider(_)),
            CraftingTab::Potion => item.tags().contains(&ItemTag::Potion),
            CraftingTab::ProcessedMaterial => {
                item.tags().contains(&ItemTag::MetalIngot)
                    | item.tags().contains(&ItemTag::Textile)
                    | item.tags().contains(&ItemTag::Leather)
                    | item.tags().contains(&ItemTag::BaseMaterial)
            },
            CraftingTab::Bag => item.tags().contains(&ItemTag::Bag),
            CraftingTab::Tool => item.tags().contains(&ItemTag::CraftingTool),
            CraftingTab::Utility => item.tags().contains(&ItemTag::Utility),
            CraftingTab::Weapon => match item.kind() {
                ItemKind::Tool(_) => !item.tags().contains(&ItemTag::CraftingTool),
                _ => false,
            },
            CraftingTab::Dismantle => match item.kind() {
                ItemKind::Ingredient { .. } => !item.tags().contains(&ItemTag::CraftingTool),
                _ => false,
            },
        }
    }
}

pub struct State {
    ids: Ids,
    selected_recipe: Option<String>,
}

impl<'a> Widget for Crafting<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            selected_recipe: None,
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

        // Tooltips
        let item_tooltip = ItemTooltip::new(
            {
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
            },
            self.client,
            self.imgs,
            self.item_imgs,
            self.pulse,
            self.msm,
            self.localized_strings,
        )
        .title_font_size(self.fonts.cyri.scale(20))
        .parent(ui.window)
        .desc_font_size(self.fonts.cyri.scale(12))
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);
        // Tab tooltips
        let tabs_tooltip = Tooltip::new({
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

        // Frame and window
        Image::new(self.imgs.crafting_window)
            .bottom_right_with_margins_on(ui.window, 308.0, 450.0)
            .color(Some(UI_MAIN))
            .w_h(456.0, 460.0)
            .set(state.ids.window, ui);
        // Window
        Image::new(self.imgs.crafting_frame)
            .middle_of(state.ids.window)
            .color(Some(UI_HIGHLIGHT_0))
            .wh_of(state.ids.window)
            .set(state.ids.window_frame, ui);

        // Crafting Icon
        Image::new(self.imgs.crafting_icon_bordered)
            .w_h(38.0, 38.0)
            .top_left_with_margins_on(state.ids.window_frame, 4.0, 4.0)
            .set(state.ids.icon, ui);

        // Close Button
        if Button::image(self.imgs.close_button)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.window, 0.0, 0.0)
            .set(state.ids.close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // Title
        Text::new(&self.localized_strings.get("hud.crafting"))
            .mid_top_with_margin_on(state.ids.window_frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(TEXT_COLOR)
            .set(state.ids.title_main, ui);

        // Alignment
        Rectangle::fill_with([170.0, 378.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.window_frame, 74.0, 5.0)
            .scroll_kids_vertically()
            .set(state.ids.align_rec, ui);
        Rectangle::fill_with([274.0, 340.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.window, 74.0, 5.0)
            .scroll_kids_vertically()
            .set(state.ids.align_ing, ui);

        // Category Tabs
        if state.ids.category_bgs.len() < CraftingTab::iter().enumerate().len() {
            state.update(|s| {
                s.ids.category_bgs.resize(
                    CraftingTab::iter().enumerate().len(),
                    &mut ui.widget_id_generator(),
                )
            })
        };
        if state.ids.category_tabs.len() < CraftingTab::iter().enumerate().len() {
            state.update(|s| {
                s.ids.category_tabs.resize(
                    CraftingTab::iter().enumerate().len(),
                    &mut ui.widget_id_generator(),
                )
            })
        };
        if state.ids.category_imgs.len() < CraftingTab::iter().enumerate().len() {
            state.update(|s| {
                s.ids.category_imgs.resize(
                    CraftingTab::iter().enumerate().len(),
                    &mut ui.widget_id_generator(),
                )
            })
        };
        let sel_crafting_tab = &self.show.crafting_tab;
        for (i, crafting_tab) in CraftingTab::iter().enumerate() {
            let tab_img = crafting_tab.img_id(self.imgs);
            // Button Background
            let mut bg = Image::new(self.imgs.pixel)
                .w_h(40.0, 30.0)
                .color(Some(UI_MAIN));
            if i == 0 {
                bg = bg.top_left_with_margins_on(state.ids.window_frame, 50.0, -40.0)
            } else {
                bg = bg.down_from(state.ids.category_bgs[i - 1], 0.0)
            };
            bg.set(state.ids.category_bgs[i], ui);
            // Category Button
            if Button::image(if crafting_tab == *sel_crafting_tab {
                self.imgs.wpn_icon_border_pressed
            } else {
                self.imgs.wpn_icon_border
            })
            .wh_of(state.ids.category_bgs[i])
            .middle_of(state.ids.category_bgs[i])
            .hover_image(if crafting_tab == *sel_crafting_tab {
                self.imgs.wpn_icon_border_pressed
            } else {
                self.imgs.wpn_icon_border_mo
            })
            .press_image(if crafting_tab == *sel_crafting_tab {
                self.imgs.wpn_icon_border_pressed
            } else {
                self.imgs.wpn_icon_border_press
            })
            .with_tooltip(
                self.tooltip_manager,
                &self.localized_strings.get(crafting_tab.name_key()),
                "",
                &tabs_tooltip,
                TEXT_COLOR,
            )
            .set(state.ids.category_tabs[i], ui)
            .was_clicked()
            {
                events.push(Event::ChangeCraftingTab(crafting_tab))
            };
            // Tab images
            Image::new(tab_img)
                .middle_of(state.ids.category_tabs[i])
                .w_h(20.0, 20.0)
                .graphics_for(state.ids.category_tabs[i])
                .set(state.ids.category_imgs[i], ui);
        }

        // First available recipes, then unavailable ones, each alphabetically
        // In the triples, "name" is the recipe book key, and "recipe.output.0.name()"
        // is the display name (as stored in the item descriptors)
        let mut ordered_recipes: Vec<_> = self
            .client
            .recipe_book()
            .iter()
            .filter(|(_, recipe)| {
                let output_name = recipe.output.0.name.to_lowercase();
                if let Some(key) = &self.show.crafting_search_key {
                    key.as_str()
                        .to_lowercase()
                        .split_whitespace()
                        .all(|substring| output_name.contains(substring))
                } else {
                    true
                }
            })
            .map(|(name, recipe)| {
                let is_craftable =
                    self.client
                        .available_recipes()
                        .get(name.as_str())
                        .map_or(false, |cs| {
                            cs.map_or(true, |cs| {
                                Some(cs) == self.show.craft_sprite.map(|(_, s)| s)
                            })
                        });
                (name, recipe, is_craftable)
            })
            .collect();
        ordered_recipes.sort_by_key(|(_, recipe, state)| {
            (!state, recipe.output.0.quality(), recipe.output.0.name())
        });

        // Recipe list
        if state.ids.recipe_list_btns.len() < self.client.recipe_book().iter().len() {
            state.update(|state| {
                state.ids.recipe_list_btns.resize(
                    self.client.recipe_book().iter().len(),
                    &mut ui.widget_id_generator(),
                )
            });
        }
        if state.ids.recipe_list_labels.len() < self.client.recipe_book().iter().len() {
            state.update(|state| {
                state.ids.recipe_list_labels.resize(
                    self.client.recipe_book().iter().len(),
                    &mut ui.widget_id_generator(),
                )
            });
        }
        if state.ids.recipe_list_quality_indicators.len() < self.client.recipe_book().iter().len() {
            state.update(|state| {
                state.ids.recipe_list_quality_indicators.resize(
                    self.client.recipe_book().iter().len(),
                    &mut ui.widget_id_generator(),
                )
            });
        }
        for (i, (name, recipe, is_craftable)) in ordered_recipes
            .into_iter()
            .filter(|(_, recipe, _)| self.show.crafting_tab.satisfies(recipe.output.0.as_ref()))
            .enumerate()
        {
            let button = Button::image(if state.selected_recipe.as_ref() == Some(name) {
                self.imgs.selection
            } else {
                self.imgs.nothing
            })
            .and(|button| {
                if i == 0 {
                    button.top_left_with_margins_on(state.ids.align_rec, 2.0, 7.0)
                } else {
                    button.down_from(state.ids.recipe_list_btns[i - 1], 5.0)
                }
            })
            .w(157.0)
            .hover_image(self.imgs.selection_hover)
            .press_image(self.imgs.selection_press);

            let text = Text::new(recipe.output.0.name())
                .color(if is_craftable {
                    TEXT_COLOR
                } else {
                    TEXT_GRAY_COLOR
                })
                .font_size(self.fonts.cyri.scale(12))
                .font_id(self.fonts.cyri.conrod_id)
                .w(149.0)
                .mid_top_with_margin_on(state.ids.recipe_list_btns[i], 3.0)
                .graphics_for(state.ids.recipe_list_btns[i])
                .center_justify();

            let text_height = match text.get_y_dimension(ui) {
                Dimension::Absolute(y) => y,
                _ => 0.0,
            };

            if button
                .h((text_height + 7.0).max(20.0))
                .set(state.ids.recipe_list_btns[i], ui)
                .was_clicked()
            {
                if state.selected_recipe.as_ref() == Some(name) {
                    state.update(|s| s.selected_recipe = None);
                } else {
                    state.update(|s| s.selected_recipe = Some(name.clone()));
                }
            }
            // set the text here so that the correct position of the button is retrieved
            text.set(state.ids.recipe_list_labels[i], ui);

            // Sidebar color
            let color::Hsla(h, s, l, _) = get_quality_col(recipe.output.0.as_ref()).to_hsl();
            let val_multiplier = if is_craftable { 0.7 } else { 0.5 };
            // Apply conversion to hsv, multiply v by the desired amount, then revert to
            // hsl. Conversion formulae: https://en.wikipedia.org/wiki/HSL_and_HSV#Interconversion
            // Note that division by 0 is not possible since none of the colours are black
            // or white
            let quality_col = color::hsl(
                h,
                s * val_multiplier * f32::min(l, 1.0 - l)
                    / f32::min(l * val_multiplier, 1.0 - l * val_multiplier),
                l * val_multiplier,
            );

            Button::image(self.imgs.quality_indicator)
                .image_color(quality_col)
                .h_of(state.ids.recipe_list_btns[i])
                .w(4.0)
                .left_from(state.ids.recipe_list_btns[i], 1.0)
                .graphics_for(state.ids.recipe_list_btns[i])
                .set(state.ids.recipe_list_quality_indicators[i], ui);
        }

        // Selected Recipe
        if let Some((recipe_name, recipe)) = state
            .selected_recipe
            .as_ref()
            .and_then(|rn| self.client.recipe_book().get(rn.as_str()).map(|r| (rn, r)))
        {
            // Title
            Text::new(&recipe.output.0.name())
                .mid_top_with_margin_on(state.ids.align_ing, -22.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .parent(state.ids.window)
                .set(state.ids.title_ing, ui);
            let can_perform = self
                .client
                .available_recipes()
                .get(recipe_name.as_str())
                .map_or(false, |cs| {
                    cs.map_or(true, |cs| {
                        Some(cs) == self.show.craft_sprite.map(|(_, s)| s)
                    })
                });

            // Craft button
            if Button::image(self.imgs.button)
                .w_h(105.0, 25.0)
                .hover_image(
                    can_perform
                        .then_some(self.imgs.button_hover)
                        .unwrap_or(self.imgs.button),
                )
                .press_image(
                    can_perform
                        .then_some(self.imgs.button_press)
                        .unwrap_or(self.imgs.button),
                )
                .label(&self.localized_strings.get("hud.crafting.craft"))
                .label_y(conrod_core::position::Relative::Scalar(1.0))
                .label_color(can_perform.then_some(TEXT_COLOR).unwrap_or(TEXT_GRAY_COLOR))
                .label_font_size(self.fonts.cyri.scale(12))
                .label_font_id(self.fonts.cyri.conrod_id)
                .image_color(can_perform.then_some(TEXT_COLOR).unwrap_or(TEXT_GRAY_COLOR))
                .mid_bottom_with_margin_on(state.ids.align_ing, -31.0)
                .parent(state.ids.window_frame)
                .set(state.ids.btn_craft, ui)
                .was_clicked()
            {
                events.push(Event::CraftRecipe(recipe_name.clone()));
            }

            // Output Image Frame
            let quality_col_img = match recipe.output.0.quality {
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
                .w_h(60.0, 60.0)
                .top_right_with_margins_on(state.ids.align_ing, 15.0, 10.0)
                .parent(state.ids.align_ing)
                .set(state.ids.output_img_frame, ui);

            let output_text = format!("x{}", &recipe.output.1.to_string());
            // Output Image
            Button::image(animate_by_pulse(
                &self
                    .item_imgs
                    .img_ids_or_not_found_img((&*recipe.output.0).into()),
                self.pulse,
            ))
            .w_h(55.0, 55.0)
            .label(&output_text)
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(14))
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(conrod_core::position::Relative::Scalar(-24.0))
            .label_x(conrod_core::position::Relative::Scalar(24.0))
            .middle_of(state.ids.output_img_frame)
            .with_item_tooltip(
                self.item_tooltip_manager,
                &*recipe.output.0,
                &None,
                &item_tooltip,
            )
            .set(state.ids.output_img, ui);

            // Tags
            if state.ids.tags_ing.len() < CraftingTab::iter().len() {
                state.update(|state| {
                    state
                        .ids
                        .tags_ing
                        .resize(CraftingTab::iter().len(), &mut ui.widget_id_generator())
                });
            }
            for (row, chunk) in CraftingTab::iter()
                .filter(|crafting_tab| match crafting_tab {
                    CraftingTab::All => false,
                    _ => crafting_tab.satisfies(recipe.output.0.as_ref()),
                })
                .filter(|crafting_tab| crafting_tab != &self.show.crafting_tab)
                .collect::<Vec<_>>()
                .chunks(3)
                .enumerate()
            {
                for (col, crafting_tab) in chunk.iter().rev().enumerate() {
                    let i = 3 * row + col;
                    let icon = Image::new(crafting_tab.img_id(self.imgs))
                        .w_h(20.0, 20.0)
                        .parent(state.ids.window);
                    let icon = if col == 0 {
                        icon.bottom_right_with_margins_on(
                            state.ids.output_img_frame,
                            -24.0 - 24.0 * (row as f64),
                            4.0,
                        )
                    } else {
                        icon.left_from(state.ids.tags_ing[i - 1], 4.0)
                    };
                    icon.with_tooltip(
                        self.tooltip_manager,
                        &self.localized_strings.get(crafting_tab.name_key()),
                        "",
                        &tabs_tooltip,
                        TEXT_COLOR,
                    )
                    .set(state.ids.tags_ing[i], ui);
                }
            }
            // Crafting Station Info
            if recipe.craft_sprite.is_some() {
                Text::new(
                    &self
                        .localized_strings
                        .get("hud.crafting.req_crafting_station"),
                )
                .top_left_with_margins_on(state.ids.align_ing, 10.0, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(18))
                .color(TEXT_COLOR)
                .set(state.ids.req_station_title, ui);
                let station_img = match recipe.craft_sprite {
                    Some(SpriteKind::Anvil) => "Anvil",
                    Some(SpriteKind::Cauldron) => "Cauldron",
                    Some(SpriteKind::CookingPot) => "CookingPot",
                    Some(SpriteKind::CraftingBench) => "CraftingBench",
                    Some(SpriteKind::Forge) => "Forge",
                    Some(SpriteKind::Loom) => "Loom",
                    Some(SpriteKind::SpinningWheel) => "SpinningWheel",
                    Some(SpriteKind::TanningRack) => "TanningRack",
                    None => "CraftsmanHammer",
                    _ => "CraftsmanHammer",
                };
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(Tool(station_img.to_string())),
                    self.pulse,
                ))
                .w_h(25.0, 25.0)
                .down_from(state.ids.req_station_title, 10.0)
                .parent(state.ids.align_ing)
                .set(state.ids.req_station_img, ui);

                let station_name = match recipe.craft_sprite {
                    Some(SpriteKind::Anvil) => "hud.crafting.anvil",
                    Some(SpriteKind::Cauldron) => "hud.crafting.cauldron",
                    Some(SpriteKind::CookingPot) => "hud.crafting.cooking_pot",
                    Some(SpriteKind::CraftingBench) => "hud.crafting.crafting_bench",
                    Some(SpriteKind::Forge) => "hud.crafting.forge",
                    Some(SpriteKind::Loom) => "hud.crafting.loom",
                    Some(SpriteKind::SpinningWheel) => "hud.crafting.spinning_wheel",
                    Some(SpriteKind::TanningRack) => "hud.crafting.tanning_rack",
                    _ => "",
                };
                Text::new(&self.localized_strings.get(station_name))
                    .right_from(state.ids.req_station_img, 10.0)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(14))
                    .color(
                        if self.show.craft_sprite.map(|(_, s)| s) == recipe.craft_sprite {
                            TEXT_COLOR
                        } else {
                            TEXT_DULL_RED_COLOR
                        },
                    )
                    .set(state.ids.req_station_txt, ui);
            }
            // Ingredients Text
            let mut ing_txt = Text::new(&self.localized_strings.get("hud.crafting.ingredients"))
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(18))
                .color(TEXT_COLOR);
            if recipe.craft_sprite.is_some() {
                ing_txt = ing_txt.down_from(state.ids.req_station_img, 10.0);
            } else {
                ing_txt = ing_txt.top_left_with_margins_on(state.ids.align_ing, 10.0, 5.0);
            };
            ing_txt.set(state.ids.ingredients_txt, ui);

            // Ingredient images with tooltip
            if state.ids.ingredient_frame.len() < recipe.inputs().len() {
                state.update(|state| {
                    state
                        .ids
                        .ingredient_frame
                        .resize(recipe.inputs().len(), &mut ui.widget_id_generator())
                });
            };
            if state.ids.ingredients.len() < recipe.inputs().len() {
                state.update(|state| {
                    state
                        .ids
                        .ingredients
                        .resize(recipe.inputs().len(), &mut ui.widget_id_generator())
                });
            };
            if state.ids.ingredient_img.len() < recipe.inputs().len() {
                state.update(|state| {
                    state
                        .ids
                        .ingredient_img
                        .resize(recipe.inputs().len(), &mut ui.widget_id_generator())
                });
            };
            if state.ids.req_text.len() < recipe.inputs().len() {
                state.update(|state| {
                    state
                        .ids
                        .req_text
                        .resize(recipe.inputs().len(), &mut ui.widget_id_generator())
                });
            };

            // Widget generation for every ingredient
            for (i, (recipe_input, amount)) in recipe.inputs.iter().enumerate() {
                let item_def = match recipe_input {
                    RecipeInput::Item(item_def) => Arc::clone(item_def),
                    RecipeInput::Tag(tag) => Arc::<ItemDef>::load_expect_cloned(
                        &self
                            .inventory
                            .slots()
                            .filter_map(|slot| {
                                slot.as_ref().and_then(|item| {
                                    if item.matches_recipe_input(recipe_input) {
                                        Some(item.item_definition_id().to_string())
                                    } else {
                                        None
                                    }
                                })
                            })
                            .next()
                            .unwrap_or_else(|| tag.exemplar_identifier().to_string()),
                    ),
                };

                // Grey color for images and text if their amount is too low to craft the item
                let item_count_in_inventory = self.inventory.item_count(&*item_def);
                let col = if item_count_in_inventory >= u64::from(*amount.max(&1)) {
                    TEXT_COLOR
                } else {
                    TEXT_DULL_RED_COLOR
                };
                // Slot BG
                let frame_pos = if i == 0 {
                    state.ids.ingredients_txt
                } else {
                    state.ids.ingredient_frame[i - 1]
                };
                // add a larger offset for the the first ingredient and the "Required Text for
                // Catalysts/Tools"
                let frame_offset = if i == 0 {
                    10.0
                } else if *amount == 0 {
                    5.0
                } else {
                    0.0
                };
                let quality_col_img = match &item_def.quality {
                    Quality::Low => self.imgs.inv_slot_grey,
                    Quality::Common => self.imgs.inv_slot,
                    Quality::Moderate => self.imgs.inv_slot_green,
                    Quality::High => self.imgs.inv_slot_blue,
                    Quality::Epic => self.imgs.inv_slot_purple,
                    Quality::Legendary => self.imgs.inv_slot_gold,
                    Quality::Artifact => self.imgs.inv_slot_orange,
                    _ => self.imgs.inv_slot_red,
                };
                let frame = Image::new(quality_col_img).w_h(25.0, 25.0);
                let frame = if *amount == 0 {
                    frame.down_from(state.ids.req_text[i], 10.0 + frame_offset)
                } else {
                    frame.down_from(frame_pos, 10.0 + frame_offset)
                };
                frame.set(state.ids.ingredient_frame[i], ui);
                //Item Image
                Button::image(animate_by_pulse(
                    &self.item_imgs.img_ids_or_not_found_img((&*item_def).into()),
                    self.pulse,
                ))
                .w_h(22.0, 22.0)
                .middle_of(state.ids.ingredient_frame[i])
                .with_item_tooltip(self.item_tooltip_manager, &*item_def, &None, &item_tooltip)
                .set(state.ids.ingredient_img[i], ui);
                // Ingredients text and amount
                // Don't show inventory amounts above 999 to avoid the widget clipping
                let over9k = "99+";
                let in_inv: &str = &item_count_in_inventory.to_string();
                // Show Ingredients
                // Align "Required" Text below last ingredient
                if *amount == 0 {
                    // Catalysts/Tools
                    Text::new(&self.localized_strings.get("hud.crafting.tool_cata"))
                        .down_from(state.ids.ingredient_frame[i - 1], 20.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(TEXT_COLOR)
                        .set(state.ids.req_text[i], ui);
                    Text::new(&item_def.name())
                        .right_from(state.ids.ingredient_frame[i], 10.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(col)
                        .set(state.ids.ingredients[i], ui);
                } else {
                    // Ingredients
                    let name = match recipe_input {
                        RecipeInput::Item(_) => item_def.name().to_string(),
                        RecipeInput::Tag(tag) => format!("Any {}", tag.name()),
                    };
                    let input = format!(
                        "{}x {} ({})",
                        amount,
                        name,
                        if item_count_in_inventory > 99 {
                            over9k
                        } else {
                            in_inv
                        }
                    );
                    // Ingredient Text
                    Text::new(&input)
                        .right_from(state.ids.ingredient_frame[i], 10.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(12))
                        .color(col)
                        .set(state.ids.ingredients[i], ui);
                }
            }
        }

        // Search / Title Recipes
        if let Some(key) = &self.show.crafting_search_key {
            if Button::image(self.imgs.close_btn)
                .top_left_with_margins_on(state.ids.align_rec, -20.0, 5.0)
                .w_h(14.0, 14.0)
                .hover_image(self.imgs.close_btn_hover)
                .press_image(self.imgs.close_btn_press)
                .parent(state.ids.window)
                .set(state.ids.btn_close_search, ui)
                .was_clicked()
            {
                events.push(Event::SearchRecipe(None));
            }
            Rectangle::fill([148.0, 20.0])
                .top_left_with_margins_on(state.ids.btn_close_search, -2.0, 16.0)
                .hsla(0.0, 0.0, 0.0, 0.7)
                .depth(1.0)
                .parent(state.ids.window)
                .set(state.ids.input_bg_search, ui);
            if let Some(string) = TextEdit::new(key.as_str())
                .top_left_with_margins_on(state.ids.btn_close_search, -2.0, 18.0)
                .w_h(124.0, 20.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .parent(state.ids.window)
                .set(state.ids.input_search, ui)
            {
                events.push(Event::SearchRecipe(Some(string)));
            }
        } else {
            Text::new(&self.localized_strings.get("hud.crafting.recipes"))
                .mid_top_with_margin_on(state.ids.align_rec, -22.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .parent(state.ids.window)
                .set(state.ids.title_rec, ui);
            Rectangle::fill_with([148.0, 20.0], color::TRANSPARENT)
                .top_left_with_margins_on(state.ids.window, 52.0, 26.0)
                .graphics_for(state.ids.btn_open_search)
                .set(state.ids.input_overlay_search, ui);
            if Button::image(self.imgs.search_btn)
                .top_left_with_margins_on(state.ids.align_rec, -21.0, 5.0)
                .w_h(16.0, 16.0)
                .hover_image(self.imgs.search_btn_hover)
                .press_image(self.imgs.search_btn_press)
                .parent(state.ids.window)
                .set(state.ids.btn_open_search, ui)
                .was_clicked()
            {
                events.push(Event::SearchRecipe(Some(String::new())));
                events.push(Event::Focus(state.ids.input_search));
            }
        }
        // Scrollbars
        Scrollbar::y_axis(state.ids.align_rec)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.scrollbar_rec, ui);
        Scrollbar::y_axis(state.ids.align_ing)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.scrollbar_ing, ui);

        events
    }
}
