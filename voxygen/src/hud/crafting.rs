use super::{
    get_quality_col,
    img_ids::{Imgs, ImgsRot},
    item_imgs::{animate_by_pulse, ItemImgs},
    slots::{CraftSlot, CraftSlotInfo, SlotManager},
    util, HudInfo, Show, TEXT_COLOR, TEXT_DULL_RED_COLOR, TEXT_GRAY_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::ui::{
    fonts::Fonts,
    slot::{ContentSize, SlotMaker},
    ImageFrame, ItemTooltip, ItemTooltipManager, ItemTooltipable, Tooltip, TooltipManager,
    Tooltipable,
};
use client::{self, Client};
use common::{
    assets::AssetExt,
    comp::inventory::{
        item::{
            item_key::ItemKey,
            modular::{self, ModularComponent},
            tool::{AbilityMap, ToolKind},
            Item, ItemBase, ItemDef, ItemDesc, ItemI18n, ItemKind, ItemTag, MaterialStatManifest,
            Quality, TagExampleInfo,
        },
        slot::{InvSlotId, Slot},
        Inventory,
    },
    mounting::VolumePos,
    recipe::{ComponentKey, Recipe, RecipeInput},
    terrain::SpriteKind,
};
use conrod_core::{
    color, image,
    position::Dimension,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text, TextEdit},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use hashbrown::HashMap;
use i18n::Localization;
use std::{borrow::Cow, collections::BTreeMap, sync::Arc};
use strum::{EnumIter, IntoEnumIterator};
use tracing::{error, warn};
use vek::*;

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
        btn_craft_all,
        recipe_list_btns[],
        recipe_list_labels[],
        recipe_list_quality_indicators[],
        recipe_list_materials_indicators[],
        recipe_img_frame[],
        recipe_img[],
        ingredients[],
        ingredient_frame[],
        ingredient_btn[],
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
        dismantle_title,
        dismantle_img,
        dismantle_txt,
        repair_buttons[],
        craft_slots[],
        modular_art,
        modular_desc_txt,
        modular_wep_empty_bg,
        modular_wep_ing_1_bg,
        modular_wep_ing_2_bg,
    }
}

pub enum Event {
    CraftRecipe {
        recipe_name: String,
        amount: u32,
    },
    CraftModularWeapon {
        primary_slot: InvSlotId,
        secondary_slot: InvSlotId,
    },
    CraftModularWeaponComponent {
        toolkind: ToolKind,
        material: InvSlotId,
        modifier: Option<InvSlotId>,
    },
    ChangeCraftingTab(CraftingTab),
    Close,
    Focus(widget::Id),
    SearchRecipe(Option<String>),
    ClearRecipeInputs,
    RepairItem {
        slot: Slot,
    },
}

pub struct CraftingShow {
    pub crafting_tab: CraftingTab,
    pub crafting_search_key: Option<String>,
    pub craft_sprite: Option<(VolumePos, SpriteKind)>,
    pub salvage: bool,
    pub initialize_repair: bool,
    // TODO: Maybe try to do something that doesn't need to allocate?
    pub recipe_inputs: HashMap<u32, Slot>,
}

impl Default for CraftingShow {
    fn default() -> Self {
        Self {
            crafting_tab: CraftingTab::All,
            crafting_search_key: None,
            craft_sprite: None,
            salvage: false,
            initialize_repair: false,
            recipe_inputs: HashMap::new(),
        }
    }
}

#[derive(WidgetCommon)]
pub struct Crafting<'a> {
    client: &'a Client,
    info: &'a HudInfo,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    item_i18n: &'a ItemI18n,
    pulse: f32,
    rot_imgs: &'a ImgsRot,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    slot_manager: &'a mut SlotManager,
    item_imgs: &'a ItemImgs,
    inventory: &'a Inventory,
    msm: &'a MaterialStatManifest,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    tooltip_manager: &'a mut TooltipManager,
    show: &'a mut Show,
}

impl<'a> Crafting<'a> {
    pub fn new(
        client: &'a Client,
        info: &'a HudInfo,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        item_i18n: &'a ItemI18n,
        pulse: f32,
        rot_imgs: &'a ImgsRot,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut SlotManager,
        item_imgs: &'a ItemImgs,
        inventory: &'a Inventory,
        msm: &'a MaterialStatManifest,
        tooltip_manager: &'a mut TooltipManager,
        show: &'a mut Show,
    ) -> Self {
        Self {
            client,
            info,
            imgs,
            fonts,
            localized_strings,
            item_i18n,
            pulse,
            rot_imgs,
            item_tooltip_manager,
            slot_manager,
            tooltip_manager,
            item_imgs,
            inventory,
            msm,
            show,
            common: widget::CommonBuilder::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, PartialEq, Eq)]
pub enum CraftingTab {
    All,
    Tool,
    Armor,
    Weapon,
    ProcessedMaterial,
    Food,
    Potion,
    Bag,
    Utility,
    Glider,
    Dismantle,
}

impl CraftingTab {
    fn name_key(self) -> &'static str {
        match self {
            CraftingTab::All => "hud-crafting-tabs-all",
            CraftingTab::Armor => "hud-crafting-tabs-armor",
            CraftingTab::Food => "hud-crafting-tabs-food",
            CraftingTab::Glider => "hud-crafting-tabs-glider",
            CraftingTab::Potion => "hud-crafting-tabs-potion",
            CraftingTab::Tool => "hud-crafting-tabs-tool",
            CraftingTab::Utility => "hud-crafting-tabs-utility",
            CraftingTab::Weapon => "hud-crafting-tabs-weapon",
            CraftingTab::Bag => "hud-crafting-tabs-bag",
            CraftingTab::ProcessedMaterial => "hud-crafting-tabs-processed_material",
            CraftingTab::Dismantle => "hud-crafting-tabs-dismantle",
        }
    }

    fn img_id(self, imgs: &Imgs) -> image::Id {
        match self {
            CraftingTab::All => imgs.icon_globe,
            CraftingTab::Armor => imgs.icon_armor,
            CraftingTab::Food => imgs.icon_food,
            CraftingTab::Glider => imgs.icon_glider,
            CraftingTab::Potion => imgs.icon_potion,
            CraftingTab::Tool => imgs.icon_tools,
            CraftingTab::Utility => imgs.icon_utility,
            CraftingTab::Weapon => imgs.icon_weapon,
            CraftingTab::Bag => imgs.icon_bag,
            CraftingTab::ProcessedMaterial => imgs.icon_processed_material,
            // These tabs are never shown, so using not found is fine
            CraftingTab::Dismantle => imgs.not_found,
        }
    }

    fn satisfies(self, recipe: &Recipe) -> bool {
        let (item, _count) = &recipe.output;
        match self {
            CraftingTab::All | CraftingTab::Dismantle => true,
            CraftingTab::Food => item.tags().contains(&ItemTag::Food),
            CraftingTab::Armor => match &*item.kind() {
                ItemKind::Armor(_) => !item.tags().contains(&ItemTag::Bag),
                _ => false,
            },
            CraftingTab::Glider => matches!(&*item.kind(), ItemKind::Glider),
            CraftingTab::Potion => item.tags().contains(&ItemTag::Potion),
            CraftingTab::ProcessedMaterial => item
                .tags()
                .iter()
                .any(|tag| matches!(tag, &ItemTag::MaterialKind(_) | &ItemTag::BaseMaterial)),
            CraftingTab::Bag => item.tags().contains(&ItemTag::Bag),
            CraftingTab::Tool => item.tags().contains(&ItemTag::CraftingTool),
            CraftingTab::Utility => item.tags().contains(&ItemTag::Utility),
            CraftingTab::Weapon => match &*item.kind() {
                ItemKind::Tool(_) => !item.tags().contains(&ItemTag::CraftingTool),
                ItemKind::ModularComponent(
                    ModularComponent::ToolPrimaryComponent { .. }
                    | ModularComponent::ToolSecondaryComponent { .. },
                ) => true,
                _ => false,
            },
        }
    }

    // Tells UI whether tab is an adhoc tab that should only sometimes be present
    // depending on what station is accessed
    fn is_adhoc(self) -> bool { matches!(self, CraftingTab::Dismantle) }
}

pub struct State {
    ids: Ids,
    selected_recipe: Option<String>,
}

enum SearchFilter {
    None,
    Input,
    Nonexistent,
}

impl SearchFilter {
    fn parse_from_str(string: &str) -> Self {
        match string {
            "input" => Self::Input,
            _ => Self::Nonexistent,
        }
    }
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

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Crafting::update");
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

        // Handle any initialization
        // TODO: Replace with struct instead of making assorted booleans once there is
        // more than 1 field.
        if self.show.crafting_fields.initialize_repair {
            state.update(|s| {
                s.selected_recipe = Some(String::from("veloren.core.pseudo_recipe.repair"))
            });
        }
        self.show.crafting_fields.initialize_repair = false;

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
            self.info,
            self.imgs,
            self.item_imgs,
            self.pulse,
            self.msm,
            self.localized_strings,
            self.item_i18n,
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
            .w_h(470.0, 460.0)
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
        Text::new(&self.localized_strings.get_msg("hud-crafting"))
            .mid_top_with_margin_on(state.ids.window_frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(TEXT_COLOR)
            .set(state.ids.title_main, ui);

        // Alignment
        Rectangle::fill_with([184.0, 378.0], color::TRANSPARENT)
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
        let sel_crafting_tab = &self.show.crafting_fields.crafting_tab;
        for (i, crafting_tab) in CraftingTab::iter()
            .filter(|tab| !tab.is_adhoc())
            .enumerate()
        {
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
                &self.localized_strings.get_msg(crafting_tab.name_key()),
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

        // TODO: Consider UX for filtering searches, maybe a checkbox or a dropdown if
        // more filters gets added
        let mut _lower_case_search = String::new();
        let (search_filter, search_keys) = {
            if let Some(key) = &self.show.crafting_fields.crafting_search_key {
                _lower_case_search = key.as_str().to_lowercase();
                _lower_case_search
                    .split_once(':')
                    .map(|(filter, key)| {
                        (
                            SearchFilter::parse_from_str(filter),
                            key.split_whitespace().collect(),
                        )
                    })
                    .unwrap_or((
                        SearchFilter::None,
                        _lower_case_search.split_whitespace().collect(),
                    ))
            } else {
                (SearchFilter::None, vec![])
            }
        };

        let make_pseudo_recipe = |craft_sprite| Recipe {
            output: (
                Arc::<ItemDef>::load_expect_cloned("common.items.weapons.empty.empty"),
                0,
            ),
            inputs: Vec::new(),
            craft_sprite: Some(craft_sprite),
        };

        let weapon_recipe = make_pseudo_recipe(SpriteKind::CraftingBench);
        let metal_comp_recipe = make_pseudo_recipe(SpriteKind::Anvil);
        let wood_comp_recipe = make_pseudo_recipe(SpriteKind::CraftingBench);
        let repair_recipe = make_pseudo_recipe(SpriteKind::RepairBench);

        // TODO: localize
        let pseudo_entries = {
            // A BTreeMap is used over a HashMap as when a HashMap is used, the UI shuffles
            // the positions of these every tick, so a BTreeMap is necessary to keep it
            // ordered.
            let mut pseudo_entries = BTreeMap::new();
            pseudo_entries.insert(
                String::from("veloren.core.pseudo_recipe.modular_weapon"),
                (&weapon_recipe, "Modular Weapon", CraftingTab::Weapon),
            );
            pseudo_entries.insert(
                String::from("veloren.core.pseudo_recipe.modular_weapon_component.sword"),
                (&metal_comp_recipe, "Sword Blade", CraftingTab::Weapon),
            );
            pseudo_entries.insert(
                String::from("veloren.core.pseudo_recipe.modular_weapon_component.axe"),
                (&metal_comp_recipe, "Axe Head", CraftingTab::Weapon),
            );
            pseudo_entries.insert(
                String::from("veloren.core.pseudo_recipe.modular_weapon_component.hammer"),
                (&metal_comp_recipe, "Hammer Head", CraftingTab::Weapon),
            );
            pseudo_entries.insert(
                String::from("veloren.core.pseudo_recipe.modular_weapon_component.bow"),
                (&wood_comp_recipe, "Bow Limbs", CraftingTab::Weapon),
            );
            pseudo_entries.insert(
                String::from("veloren.core.pseudo_recipe.modular_weapon_component.staff"),
                (&wood_comp_recipe, "Staff Shaft", CraftingTab::Weapon),
            );
            pseudo_entries.insert(
                String::from("veloren.core.pseudo_recipe.modular_weapon_component.sceptre"),
                (&wood_comp_recipe, "Sceptre Shaft", CraftingTab::Weapon),
            );
            pseudo_entries.insert(
                String::from("veloren.core.pseudo_recipe.repair"),
                (&repair_recipe, "Repair Equipment", CraftingTab::All),
            );
            pseudo_entries
        };

        // First available recipes, then ones with available materials,
        // then unavailable ones, each sorted by quality and then alphabetically
        // In the tuple, "name" is the recipe book key, and "recipe.output.0.name()"
        // is the display name (as stored in the item descriptors)
        let mut ordered_recipes: Vec<_> = self
            .client
            .recipe_book()
            .iter()
            .filter(|(_, recipe)| match search_filter {
                SearchFilter::None => {
                    #[allow(deprecated)]
                    let output_name = recipe.output.0.name().to_lowercase();
                    search_keys
                        .iter()
                        .all(|&substring| output_name.contains(substring))
                },
                SearchFilter::Input => recipe.inputs().any(|(input, _, _)| {
                    let search = |input_name: &str| {
                        let input_name = input_name.to_lowercase();
                        search_keys
                            .iter()
                            .all(|&substring| input_name.contains(substring))
                    };

                    match input {
                        #[allow(deprecated)]
                        RecipeInput::Item(def) => search(&def.name()),
                        RecipeInput::Tag(tag) => search(tag.name()),
                        RecipeInput::TagSameItem(tag) => search(tag.name()),
                        #[allow(deprecated)]
                        RecipeInput::ListSameItem(defs) => {
                            defs.iter().any(|def| search(&def.name()))
                        },
                    }
                }),
                SearchFilter::Nonexistent => false,
            })
            .map(|(name, recipe)| {
                let has_materials = self.client.available_recipes().get(name.as_str()).is_some();
                let is_craftable =
                    self.client
                        .available_recipes()
                        .get(name.as_str())
                        .map_or(false, |cs| {
                            cs.map_or(true, |cs| {
                                Some(cs) == self.show.crafting_fields.craft_sprite.map(|(_, s)| s)
                            })
                        });
                (name, recipe, is_craftable, has_materials)
            })
            .chain(
                pseudo_entries
                    .iter()
                    // Filter by selected tab
                    .filter(|(_, (_, _, tab))| *sel_crafting_tab == CraftingTab::All || sel_crafting_tab == tab)
                    // Filter by search filter
                    .filter(|(_, (_, output_name, _))| {
                        match search_filter {
                            SearchFilter::None => {
                                let output_name = output_name.to_lowercase();
                                search_keys
                                    .iter()
                                    .all(|&substring| output_name.contains(substring))
                            },
                            // TODO: Get input filtering to work here, probably requires
                            // checking component recipe book?
                            SearchFilter::Input => false,
                            SearchFilter::Nonexistent => false,
                        }
                    })
                    .map(|(recipe_name, (recipe, _, _))| {
                        (
                            recipe_name,
                            *recipe,
                            self.show.crafting_fields.craft_sprite.map(|(_, s)| s)
                                == recipe.craft_sprite,
                            true,
                        )
                    }),
            )
            .collect();
        ordered_recipes.sort_by_key(|(_, recipe, is_craftable, has_materials)| {
            (
                !is_craftable,
                !has_materials,
                recipe.output.0.quality(),
                #[allow(deprecated)]
                recipe.output.0.name(),
            )
        });

        // Recipe list
        let recipe_list_length = self.client.recipe_book().iter().len() + pseudo_entries.len();
        if state.ids.recipe_list_btns.len() < recipe_list_length {
            state.update(|state| {
                state
                    .ids
                    .recipe_list_btns
                    .resize(recipe_list_length, &mut ui.widget_id_generator())
            });
        }
        if state.ids.recipe_list_labels.len() < recipe_list_length {
            state.update(|state| {
                state
                    .ids
                    .recipe_list_labels
                    .resize(recipe_list_length, &mut ui.widget_id_generator())
            });
        }
        if state.ids.recipe_list_quality_indicators.len() < recipe_list_length {
            state.update(|state| {
                state
                    .ids
                    .recipe_list_quality_indicators
                    .resize(recipe_list_length, &mut ui.widget_id_generator())
            });
        }
        if state.ids.recipe_list_materials_indicators.len() < recipe_list_length {
            state.update(|state| {
                state
                    .ids
                    .recipe_list_materials_indicators
                    .resize(recipe_list_length, &mut ui.widget_id_generator())
            });
        }
        for (i, (name, recipe, is_craftable, has_materials)) in ordered_recipes
            .into_iter()
            .filter(|(_, recipe, _, _)| self.show.crafting_fields.crafting_tab.satisfies(recipe))
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
            .w(171.0)
            .hover_image(self.imgs.selection_hover)
            .press_image(self.imgs.selection_press)
            .image_color(color::rgba(1.0, 0.82, 0.27, 1.0));

            let title;
            let recipe_name =
                if let Some((_recipe, pseudo_name, _filter_tab)) = pseudo_entries.get(name) {
                    *pseudo_name
                } else {
                    (title, _) = util::item_text(
                        recipe.output.0.as_ref(),
                        self.localized_strings,
                        self.item_i18n,
                    );
                    &title
                };

            let text = Text::new(recipe_name)
                .color(if is_craftable {
                    TEXT_COLOR
                } else {
                    TEXT_GRAY_COLOR
                })
                .font_size(self.fonts.cyri.scale(12))
                .font_id(self.fonts.cyri.conrod_id)
                .w(163.0)
                .mid_top_with_margin_on(state.ids.recipe_list_btns[i], 3.0)
                .graphics_for(state.ids.recipe_list_btns[i])
                .center_justify();

            let text_height = match text.get_y_dimension(ui) {
                Dimension::Absolute(y) => y,
                _ => 0.0,
            };
            let button_height = (text_height + 7.0).max(20.0);

            if button
                .h(button_height)
                .set(state.ids.recipe_list_btns[i], ui)
                .was_clicked()
            {
                if state.selected_recipe.as_ref() == Some(name) {
                    state.update(|s| s.selected_recipe = None);
                } else {
                    if self.show.crafting_fields.crafting_tab.is_adhoc() {
                        // If current tab is an adhoc tab, and recipe is selected, change to general
                        // tab
                        events.push(Event::ChangeCraftingTab(CraftingTab::All));
                    }
                    state.update(|s| s.selected_recipe = Some(name.clone()));
                }
                events.push(Event::ClearRecipeInputs);
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
                .w_h(4.0, button_height)
                .left_from(state.ids.recipe_list_btns[i], 1.0)
                .graphics_for(state.ids.recipe_list_btns[i])
                .set(state.ids.recipe_list_quality_indicators[i], ui);

            // Sidebar crafting tool icon
            if has_materials && !is_craftable {
                let station_img = match recipe.craft_sprite {
                    Some(SpriteKind::Anvil) => Some("Anvil"),
                    Some(SpriteKind::Cauldron) => Some("Cauldron"),
                    Some(SpriteKind::CookingPot) => Some("CookingPot"),
                    Some(SpriteKind::CraftingBench) => Some("CraftingBench"),
                    Some(SpriteKind::Forge) => Some("Forge"),
                    Some(SpriteKind::Loom) => Some("Loom"),
                    Some(SpriteKind::SpinningWheel) => Some("SpinningWheel"),
                    Some(SpriteKind::TanningRack) => Some("TanningRack"),
                    Some(SpriteKind::DismantlingBench) => Some("DismantlingBench"),
                    _ => None,
                };

                if let Some(station_img_str) = station_img {
                    Button::image(animate_by_pulse(
                        &self
                            .item_imgs
                            .img_ids_or_not_found_img(ItemKey::Simple(station_img_str.to_string())),
                        self.pulse,
                    ))
                    .image_color(color::LIGHT_RED)
                    .w_h(button_height - 8.0, button_height - 8.0)
                    .top_left_with_margins_on(state.ids.recipe_list_btns[i], 4.0, 4.0)
                    .graphics_for(state.ids.recipe_list_btns[i])
                    .set(state.ids.recipe_list_materials_indicators[i], ui);
                }
            }
        }

        // Deselect recipe if current tab is an adhoc tab, elsewhere if recipe selected
        // while in an adhoc tab, tab is changed to general
        if self.show.crafting_fields.crafting_tab.is_adhoc() {
            state.update(|s| s.selected_recipe = None);
        }

        // Selected Recipe
        if let Some((recipe_name, recipe)) = match state.selected_recipe.as_deref() {
            Some(selected_recipe) => {
                if let Some((modular_recipe, _pseudo_name, _filter_tab)) =
                    pseudo_entries.get(selected_recipe)
                {
                    Some((selected_recipe, *modular_recipe))
                } else {
                    self.client
                        .recipe_book()
                        .get(selected_recipe)
                        .map(|r| (selected_recipe, r))
                }
            },
            None => None,
        } {
            let recipe_name = String::from(recipe_name);

            let title;
            let title = if let Some((_recipe, pseudo_name, _filter_tab)) =
                pseudo_entries.get(&recipe_name)
            {
                *pseudo_name
            } else {
                (title, _) = util::item_text(
                    recipe.output.0.as_ref(),
                    self.localized_strings,
                    self.item_i18n,
                );
                &title
            };

            // Title
            Text::new(title)
                .mid_top_with_margin_on(state.ids.align_ing, -22.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .parent(state.ids.window)
                .set(state.ids.title_ing, ui);

            #[derive(Clone, Copy, Debug)]
            enum RecipeKind {
                ModularWeapon,
                Component(ToolKind),
                Simple,
                Repair,
            }

            let recipe_kind = match recipe_name.as_str() {
                "veloren.core.pseudo_recipe.modular_weapon" => RecipeKind::ModularWeapon,
                "veloren.core.pseudo_recipe.modular_weapon_component.sword" => {
                    RecipeKind::Component(ToolKind::Sword)
                },
                "veloren.core.pseudo_recipe.modular_weapon_component.axe" => {
                    RecipeKind::Component(ToolKind::Axe)
                },
                "veloren.core.pseudo_recipe.modular_weapon_component.hammer" => {
                    RecipeKind::Component(ToolKind::Hammer)
                },
                "veloren.core.pseudo_recipe.modular_weapon_component.bow" => {
                    RecipeKind::Component(ToolKind::Bow)
                },
                "veloren.core.pseudo_recipe.modular_weapon_component.staff" => {
                    RecipeKind::Component(ToolKind::Staff)
                },
                "veloren.core.pseudo_recipe.modular_weapon_component.sceptre" => {
                    RecipeKind::Component(ToolKind::Sceptre)
                },
                "veloren.core.pseudo_recipe.repair" => RecipeKind::Repair,
                _ => RecipeKind::Simple,
            };

            let mut slot_maker = SlotMaker {
                empty_slot: self.imgs.inv_slot,
                filled_slot: self.imgs.inv_slot,
                selected_slot: self.imgs.inv_slot_sel,
                background_color: Some(UI_MAIN),
                content_size: ContentSize {
                    width_height_ratio: 1.0,
                    max_fraction: 0.75,
                },
                selected_content_scale: 1.067,
                amount_font: self.fonts.cyri.conrod_id,
                amount_margins: Vec2::new(-4.0, 0.0),
                amount_font_size: self.fonts.cyri.scale(12),
                amount_text_color: TEXT_COLOR,
                content_source: self.inventory,
                image_source: self.item_imgs,
                slot_manager: Some(self.slot_manager),
                pulse: self.pulse,
            };

            // Output slot, tags, and modular input slots
            let (craft_slot_1, craft_slot_2, can_perform) = match recipe_kind {
                RecipeKind::ModularWeapon | RecipeKind::Component(_) => {
                    if state.ids.craft_slots.len() < 2 {
                        state.update(|s| {
                            s.ids.craft_slots.resize(2, &mut ui.widget_id_generator());
                        });
                    }
                    // Modular Weapon Crafting BG-Art
                    Image::new(self.imgs.crafting_modular_art)
                        .mid_top_with_margin_on(state.ids.align_ing, 55.0)
                        .w_h(168.0, 250.0)
                        .set(state.ids.modular_art, ui);

                    let primary_slot = CraftSlot {
                        index: 0,
                        slot: self.show.crafting_fields.recipe_inputs.get(&0).copied(),
                        requirement: match recipe_kind {
                            RecipeKind::ModularWeapon => |item, _, _| {
                                matches!(
                                    &*item.kind(),
                                    ItemKind::ModularComponent(
                                        ModularComponent::ToolPrimaryComponent { .. }
                                    )
                                )
                            },
                            RecipeKind::Component(_) => |item, comp_recipes, info| {
                                if let Some(CraftSlotInfo::Tool(toolkind)) = info {
                                    comp_recipes
                                        .iter()
                                        .filter(|(key, _)| key.toolkind == toolkind)
                                        .any(|(key, _)| {
                                            Some(key.material.as_str())
                                                == item.item_definition_id().itemdef_id()
                                        })
                                } else {
                                    false
                                }
                            },
                            RecipeKind::Simple | RecipeKind::Repair => |_, _, _| unreachable!(),
                        },
                        info: match recipe_kind {
                            RecipeKind::Component(toolkind) => Some(CraftSlotInfo::Tool(toolkind)),
                            RecipeKind::ModularWeapon | RecipeKind::Simple | RecipeKind::Repair => {
                                None
                            },
                        },
                    };

                    let primary_slot_widget = slot_maker
                        .fabricate(primary_slot, [40.0; 2])
                        .top_left_with_margins_on(state.ids.modular_art, 4.0, 4.0)
                        .parent(state.ids.align_ing);

                    if let Some(item) = primary_slot.item(self.inventory) {
                        primary_slot_widget
                            .with_item_tooltip(
                                self.item_tooltip_manager,
                                core::iter::once(item as &dyn ItemDesc),
                                &None,
                                &item_tooltip,
                            )
                            .set(state.ids.craft_slots[0], ui);
                    } else {
                        let (tooltip_title, tooltip_desc) = match recipe_kind {
                            RecipeKind::ModularWeapon => (
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_weap_prim_slot_title"),
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_weap_prim_slot_desc"),
                            ),
                            RecipeKind::Component(
                                ToolKind::Sword | ToolKind::Axe | ToolKind::Hammer,
                            ) => (
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_comp_metal_prim_slot_title"),
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_comp_metal_prim_slot_desc"),
                            ),
                            RecipeKind::Component(
                                ToolKind::Bow | ToolKind::Staff | ToolKind::Sceptre,
                            ) => (
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_comp_wood_prim_slot_title"),
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_comp_wood_prim_slot_desc"),
                            ),
                            RecipeKind::Component(_) | RecipeKind::Simple | RecipeKind::Repair => {
                                (Cow::Borrowed(""), Cow::Borrowed(""))
                            },
                        };
                        primary_slot_widget
                            .with_tooltip(
                                self.tooltip_manager,
                                &tooltip_title,
                                &tooltip_desc,
                                &tabs_tooltip,
                                TEXT_COLOR,
                            )
                            .set(state.ids.craft_slots[0], ui);
                    }

                    let secondary_slot = CraftSlot {
                        index: 1,
                        slot: self.show.crafting_fields.recipe_inputs.get(&1).copied(),
                        requirement: match recipe_kind {
                            RecipeKind::ModularWeapon => |item, _, _| {
                                matches!(
                                    &*item.kind(),
                                    ItemKind::ModularComponent(
                                        ModularComponent::ToolSecondaryComponent { .. }
                                    )
                                )
                            },
                            RecipeKind::Component(_) => |item, comp_recipes, info| {
                                if let Some(CraftSlotInfo::Tool(toolkind)) = info {
                                    comp_recipes
                                        .iter()
                                        .filter(|(key, _)| key.toolkind == toolkind)
                                        .any(|(key, _)| {
                                            key.modifier.as_deref()
                                                == item.item_definition_id().itemdef_id()
                                        })
                                } else {
                                    false
                                }
                            },
                            RecipeKind::Simple | RecipeKind::Repair => |_, _, _| unreachable!(),
                        },
                        info: match recipe_kind {
                            RecipeKind::Component(toolkind) => Some(CraftSlotInfo::Tool(toolkind)),
                            RecipeKind::ModularWeapon | RecipeKind::Simple | RecipeKind::Repair => {
                                None
                            },
                        },
                    };

                    let secondary_slot_widget = slot_maker
                        .fabricate(secondary_slot, [40.0; 2])
                        .top_right_with_margins_on(state.ids.modular_art, 4.0, 4.0)
                        .parent(state.ids.align_ing);

                    if let Some(item) = secondary_slot.item(self.inventory) {
                        secondary_slot_widget
                            .with_item_tooltip(
                                self.item_tooltip_manager,
                                core::iter::once(item as &dyn ItemDesc),
                                &None,
                                &item_tooltip,
                            )
                            .set(state.ids.craft_slots[1], ui);
                    } else {
                        let (tooltip_title, tooltip_desc) = match recipe_kind {
                            RecipeKind::ModularWeapon => (
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_weap_sec_slot_title"),
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_weap_sec_slot_desc"),
                            ),
                            RecipeKind::Component(_) => (
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_comp_sec_slot_title"),
                                self.localized_strings
                                    .get_msg("hud-crafting-mod_comp_sec_slot_desc"),
                            ),
                            RecipeKind::Simple | RecipeKind::Repair => {
                                (Cow::Borrowed(""), Cow::Borrowed(""))
                            },
                        };
                        secondary_slot_widget
                            .with_tooltip(
                                self.tooltip_manager,
                                &tooltip_title,
                                &tooltip_desc,
                                &tabs_tooltip,
                                TEXT_COLOR,
                            )
                            .set(state.ids.craft_slots[1], ui);
                    }

                    let prim_item_placed = primary_slot.slot.is_some();
                    let sec_item_placed = secondary_slot.slot.is_some();

                    let prim_icon = match recipe_kind {
                        RecipeKind::ModularWeapon => self.imgs.icon_primary_comp,
                        RecipeKind::Component(ToolKind::Sword) => self.imgs.icon_ingot,
                        RecipeKind::Component(ToolKind::Axe) => self.imgs.icon_ingot,
                        RecipeKind::Component(ToolKind::Hammer) => self.imgs.icon_ingot,
                        RecipeKind::Component(ToolKind::Bow) => self.imgs.icon_log,
                        RecipeKind::Component(ToolKind::Staff) => self.imgs.icon_log,
                        RecipeKind::Component(ToolKind::Sceptre) => self.imgs.icon_log,
                        _ => self.imgs.not_found,
                    };

                    let sec_icon = match recipe_kind {
                        RecipeKind::ModularWeapon => self.imgs.icon_secondary_comp,
                        RecipeKind::Component(_) => self.imgs.icon_claw,
                        _ => self.imgs.not_found,
                    };

                    // Output Image
                    Image::new(self.imgs.inv_slot)
                        .w_h(80.0, 80.0)
                        .mid_bottom_with_margin_on(state.ids.align_ing, 50.0)
                        .parent(state.ids.align_ing)
                        .set(state.ids.output_img_frame, ui);
                    let bg_col = Color::Rgba(1.0, 1.0, 1.0, 0.4);
                    if !prim_item_placed {
                        Image::new(prim_icon)
                            .middle_of(state.ids.craft_slots[0])
                            .color(Some(bg_col))
                            .w_h(34.0, 34.0)
                            .graphics_for(state.ids.craft_slots[0])
                            .set(state.ids.modular_wep_ing_1_bg, ui);
                    }
                    if !sec_item_placed {
                        Image::new(sec_icon)
                            .middle_of(state.ids.craft_slots[1])
                            .color(Some(bg_col))
                            .w_h(50.0, 50.0)
                            .graphics_for(state.ids.craft_slots[1])
                            .set(state.ids.modular_wep_ing_2_bg, ui);
                    }

                    let ability_map = &AbilityMap::load().read();
                    let msm = &MaterialStatManifest::load().read();

                    let output_item = match recipe_kind {
                        RecipeKind::ModularWeapon => {
                            if let Some((primary_comp, toolkind, hand_restriction)) =
                                primary_slot.item(self.inventory).and_then(|item| {
                                    if let ItemKind::ModularComponent(
                                        ModularComponent::ToolPrimaryComponent {
                                            toolkind,
                                            hand_restriction,
                                            ..
                                        },
                                    ) = &*item.kind()
                                    {
                                        Some((item, *toolkind, *hand_restriction))
                                    } else {
                                        None
                                    }
                                })
                            {
                                secondary_slot
                                    .item(self.inventory)
                                    .filter(|item| {
                                        matches!(
                                            &*item.kind(),
                                            ItemKind::ModularComponent(
                                                ModularComponent::ToolSecondaryComponent { toolkind: toolkind_b, hand_restriction: hand_restriction_b, .. }
                                            ) if toolkind == *toolkind_b && modular::compatible_handedness(hand_restriction, *hand_restriction_b)
                                        )
                                    })
                                    .map(|secondary_comp| {
                                        Item::new_from_item_base(
                                            ItemBase::Modular(modular::ModularBase::Tool),
                                            vec![
                                                primary_comp.duplicate(ability_map, msm),
                                                secondary_comp.duplicate(ability_map, msm),
                                            ],
                                            ability_map,
                                            msm,
                                        )
                                    })
                            } else {
                                None
                            }
                        },
                        RecipeKind::Component(toolkind) => {
                            if let Some(material) =
                                primary_slot.item(self.inventory).and_then(|item| {
                                    item.item_definition_id().itemdef_id().map(String::from)
                                })
                            {
                                let component_key = ComponentKey {
                                    toolkind,
                                    material,
                                    modifier: secondary_slot.item(self.inventory).and_then(
                                        |item| {
                                            item.item_definition_id().itemdef_id().map(String::from)
                                        },
                                    ),
                                };
                                self.client.component_recipe_book().get(&component_key).map(
                                    |component_recipe| {
                                        component_recipe.item_output(ability_map, msm)
                                    },
                                )
                            } else {
                                None
                            }
                        },
                        RecipeKind::Simple | RecipeKind::Repair => None,
                    };

                    if let Some(output_item) = output_item {
                        let (name, _) =
                            util::item_text(&output_item, self.localized_strings, self.item_i18n);
                        Button::image(animate_by_pulse(
                            &self
                                .item_imgs
                                .img_ids_or_not_found_img(ItemKey::from(&output_item)),
                            self.pulse,
                        ))
                        .w_h(55.0, 55.0)
                        .label(&name)
                        .label_color(TEXT_COLOR)
                        .label_font_size(self.fonts.cyri.scale(14))
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .label_y(conrod_core::position::Relative::Scalar(-64.0))
                        .label_x(conrod_core::position::Relative::Scalar(0.0))
                        .middle_of(state.ids.output_img_frame)
                        .with_item_tooltip(
                            self.item_tooltip_manager,
                            core::iter::once(&output_item as &dyn ItemDesc),
                            &None,
                            &item_tooltip,
                        )
                        .set(state.ids.output_img, ui);
                        (
                            primary_slot.slot,
                            secondary_slot.slot,
                            self.show.crafting_fields.craft_sprite.map(|(_, s)| s)
                                == recipe.craft_sprite,
                        )
                    } else {
                        Text::new(&self.localized_strings.get_msg("hud-crafting-modular_desc"))
                            .mid_top_with_margin_on(state.ids.modular_art, -18.0)
                            .font_id(self.fonts.cyri.conrod_id)
                            .font_size(self.fonts.cyri.scale(13))
                            .color(TEXT_COLOR)
                            .set(state.ids.title_main, ui);
                        Image::new(self.imgs.icon_mod_weap)
                            .middle_of(state.ids.output_img_frame)
                            .color(Some(bg_col))
                            .w_h(70.0, 70.0)
                            .graphics_for(state.ids.output_img)
                            .set(state.ids.modular_wep_empty_bg, ui);
                        (primary_slot.slot, secondary_slot.slot, false)
                    }
                },
                RecipeKind::Simple => {
                    // Output Image Frame
                    let quality_col_img = match recipe.output.0.quality() {
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
                        core::iter::once(&*recipe.output.0 as &dyn ItemDesc),
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
                            _ => crafting_tab.satisfies(recipe),
                        })
                        .filter(|crafting_tab| {
                            crafting_tab != &self.show.crafting_fields.crafting_tab
                        })
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
                                &self.localized_strings.get_msg(crafting_tab.name_key()),
                                "",
                                &tabs_tooltip,
                                TEXT_COLOR,
                            )
                            .set(state.ids.tags_ing[i], ui);
                        }
                    }
                    (
                        None,
                        None,
                        self.client
                            .available_recipes()
                            .get(&recipe_name)
                            .map_or(false, |cs| {
                                cs.map_or(true, |cs| {
                                    Some(cs)
                                        == self.show.crafting_fields.craft_sprite.map(|(_, s)| s)
                                })
                            }),
                    )
                },
                RecipeKind::Repair => {
                    if state.ids.craft_slots.len() < 1 {
                        state.update(|s| {
                            s.ids.craft_slots.resize(1, &mut ui.widget_id_generator());
                        });
                    }
                    if state.ids.repair_buttons.len() < 2 {
                        state.update(|s| {
                            s.ids
                                .repair_buttons
                                .resize(2, &mut ui.widget_id_generator());
                        });
                    }

                    // Slot for item to be repaired
                    let repair_slot = CraftSlot {
                        index: 0,
                        slot: self.show.crafting_fields.recipe_inputs.get(&0).copied(),
                        requirement: |item, _, _| item.durability_lost().map_or(false, |d| d > 0),
                        info: None,
                    };

                    let repair_slot_widget = slot_maker
                        .fabricate(repair_slot, [40.0; 2])
                        .top_left_with_margins_on(state.ids.align_ing, 20.0, 40.0)
                        .parent(state.ids.align_ing);

                    if let Some(item) = repair_slot.item(self.inventory) {
                        repair_slot_widget
                            .with_item_tooltip(
                                self.item_tooltip_manager,
                                core::iter::once(item as &dyn ItemDesc),
                                &None,
                                &item_tooltip,
                            )
                            .set(state.ids.craft_slots[0], ui);
                    } else {
                        repair_slot_widget
                            .with_tooltip(
                                self.tooltip_manager,
                                &self
                                    .localized_strings
                                    .get_msg("hud-crafting-repair_slot_title"),
                                &self
                                    .localized_strings
                                    .get_msg("hud-crafting-repair_slot_desc"),
                                &tabs_tooltip,
                                TEXT_COLOR,
                            )
                            .set(state.ids.craft_slots[0], ui);
                    }

                    let can_repair = |item: &Item| {
                        // Check that item needs to be repaired, and that inventory has sufficient
                        // materials to repair
                        item.durability_lost().map_or(false, |d| d > 0)
                            && self.client.repair_recipe_book().repair_recipe(item).map_or(
                                false,
                                |recipe| {
                                    recipe
                                        .inventory_contains_ingredients(item, self.inventory)
                                        .is_ok()
                                },
                            )
                    };

                    // Repair equipped button
                    if Button::image(self.imgs.button)
                        .w_h(105.0, 25.0)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .label(
                            &self
                                .localized_strings
                                .get_msg("hud-crafting-repair_equipped"),
                        )
                        .label_y(conrod_core::position::Relative::Scalar(1.0))
                        .label_color(TEXT_COLOR)
                        .label_font_size(self.fonts.cyri.scale(12))
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .image_color(TEXT_COLOR)
                        .top_right_with_margins_on(state.ids.align_ing, 20.0, 20.0)
                        .set(state.ids.repair_buttons[0], ui)
                        .was_clicked()
                    {
                        self.inventory
                            .equipped_items_with_slot()
                            .filter(|(_, item)| can_repair(item))
                            .for_each(|(slot, _)| {
                                events.push(Event::RepairItem {
                                    slot: Slot::Equip(slot),
                                });
                            })
                    }

                    // Repair all button
                    if Button::image(self.imgs.button)
                        .w_h(105.0, 25.0)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .label(&self.localized_strings.get_msg("hud-crafting-repair_all"))
                        .label_y(conrod_core::position::Relative::Scalar(1.0))
                        .label_color(TEXT_COLOR)
                        .label_font_size(self.fonts.cyri.scale(12))
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .image_color(TEXT_COLOR)
                        .mid_bottom_with_margin_on(state.ids.repair_buttons[0], -45.0)
                        .set(state.ids.repair_buttons[1], ui)
                        .was_clicked()
                    {
                        self.inventory
                            .equipped_items_with_slot()
                            .filter(|(_, item)| can_repair(item))
                            .for_each(|(slot, _)| {
                                events.push(Event::RepairItem {
                                    slot: Slot::Equip(slot),
                                });
                            });
                        self.inventory
                            .slots_with_id()
                            .filter(|(_, item)| item.as_ref().map_or(false, can_repair))
                            .for_each(|(slot, _)| {
                                events.push(Event::RepairItem {
                                    slot: Slot::Inventory(slot),
                                });
                            });
                    }

                    let can_perform = repair_slot.item(self.inventory).map_or(false, can_repair);

                    (repair_slot.slot, None, can_perform)
                },
            };

            // Craft button
            if Button::image(self.imgs.button)
                .w_h(105.0, 25.0)
                .hover_image(if can_perform {
                    self.imgs.button_hover
                } else {
                    self.imgs.button
                })
                .press_image(if can_perform {
                    self.imgs.button_press
                } else {
                    self.imgs.button
                })
                .label(&match recipe_kind {
                    RecipeKind::Repair => self.localized_strings.get_msg("hud-crafting-repair"),
                    _ => self.localized_strings.get_msg("hud-crafting-craft"),
                })
                .label_y(conrod_core::position::Relative::Scalar(1.0))
                .label_color(if can_perform {
                    TEXT_COLOR
                } else {
                    TEXT_GRAY_COLOR
                })
                .label_font_size(self.fonts.cyri.scale(12))
                .label_font_id(self.fonts.cyri.conrod_id)
                .image_color(if can_perform {
                    TEXT_COLOR
                } else {
                    TEXT_GRAY_COLOR
                })
                .bottom_left_with_margins_on(state.ids.align_ing, -31.0, 15.0)
                .parent(state.ids.window_frame)
                .set(state.ids.btn_craft, ui)
                .was_clicked()
                && can_perform
            {
                match recipe_kind {
                    RecipeKind::ModularWeapon => {
                        if let (
                            Some(Slot::Inventory(primary_slot)),
                            Some(Slot::Inventory(secondary_slot)),
                        ) = (craft_slot_1, craft_slot_2)
                        {
                            events.push(Event::CraftModularWeapon {
                                primary_slot,
                                secondary_slot,
                            });
                        }
                    },
                    RecipeKind::Component(toolkind) => {
                        if let Some(Slot::Inventory(primary_slot)) = craft_slot_1 {
                            events.push(Event::CraftModularWeaponComponent {
                                toolkind,
                                material: primary_slot,
                                modifier: craft_slot_2.and_then(|slot| match slot {
                                    Slot::Inventory(slot) => Some(slot),
                                    Slot::Equip(_) => None,
                                    Slot::Overflow(_) => None,
                                }),
                            });
                        }
                    },
                    RecipeKind::Simple => events.push(Event::CraftRecipe {
                        recipe_name,
                        amount: 1,
                    }),
                    RecipeKind::Repair => {
                        if let Some(slot) = craft_slot_1 {
                            events.push(Event::RepairItem { slot });
                        }
                    },
                }
            }

            // Craft All button
            let can_perform_all = can_perform && matches!(recipe_kind, RecipeKind::Simple);
            if Button::image(self.imgs.button)
                .w_h(105.0, 25.0)
                .hover_image(if can_perform {
                    self.imgs.button_hover
                } else {
                    self.imgs.button
                })
                .press_image(if can_perform {
                    self.imgs.button_press
                } else {
                    self.imgs.button
                })
                .label(&self.localized_strings.get_msg("hud-crafting-craft_all"))
                .label_y(conrod_core::position::Relative::Scalar(1.0))
                .label_color(if can_perform_all {
                    TEXT_COLOR
                } else {
                    TEXT_GRAY_COLOR
                })
                .label_font_size(self.fonts.cyri.scale(12))
                .label_font_id(self.fonts.cyri.conrod_id)
                .image_color(if can_perform_all {
                    TEXT_COLOR
                } else {
                    TEXT_GRAY_COLOR
                })
                .bottom_right_with_margins_on(state.ids.align_ing, -31.0, 15.0)
                .parent(state.ids.window_frame)
                .set(state.ids.btn_craft_all, ui)
                .was_clicked()
                && can_perform_all
            {
                if let (RecipeKind::Simple, Some(selected_recipe)) =
                    (recipe_kind, &state.selected_recipe)
                {
                    let amount = recipe.max_from_ingredients(self.inventory);
                    if amount > 0 {
                        events.push(Event::CraftRecipe {
                            recipe_name: selected_recipe.to_string(),
                            amount,
                        });
                    }
                } else {
                    error!("State shows no selected recipe when trying to craft multiple.");
                }
            };

            // Crafting Station Info
            if recipe.craft_sprite.is_some() {
                Text::new(
                    &self
                        .localized_strings
                        .get_msg("hud-crafting-req_crafting_station"),
                )
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(18))
                .color(TEXT_COLOR)
                .and(|t| match recipe_kind {
                    RecipeKind::Simple => {
                        t.top_left_with_margins_on(state.ids.align_ing, 10.0, 5.0)
                    },
                    RecipeKind::ModularWeapon | RecipeKind::Component(_) => {
                        t.top_left_with_margins_on(state.ids.align_ing, 325.0, 5.0)
                    },
                    RecipeKind::Repair => {
                        t.top_left_with_margins_on(state.ids.align_ing, 80.0, 5.0)
                    },
                })
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
                    Some(SpriteKind::DismantlingBench) => "DismantlingBench",
                    Some(SpriteKind::RepairBench) => "RepairBench",
                    None => "CraftsmanHammer",
                    _ => "CraftsmanHammer",
                };
                Image::new(animate_by_pulse(
                    &self
                        .item_imgs
                        .img_ids_or_not_found_img(ItemKey::Simple(station_img.to_string())),
                    self.pulse,
                ))
                .w_h(25.0, 25.0)
                .down_from(state.ids.req_station_title, 10.0)
                .parent(state.ids.align_ing)
                .set(state.ids.req_station_img, ui);

                let station_name = match recipe.craft_sprite {
                    Some(SpriteKind::Anvil) => "hud-crafting-anvil",
                    Some(SpriteKind::Cauldron) => "hud-crafting-cauldron",
                    Some(SpriteKind::CookingPot) => "hud-crafting-cooking_pot",
                    Some(SpriteKind::CraftingBench) => "hud-crafting-crafting_bench",
                    Some(SpriteKind::Forge) => "hud-crafting-forge",
                    Some(SpriteKind::Loom) => "hud-crafting-loom",
                    Some(SpriteKind::SpinningWheel) => "hud-crafting-spinning_wheel",
                    Some(SpriteKind::TanningRack) => "hud-crafting-tanning_rack",
                    Some(SpriteKind::DismantlingBench) => "hud-crafting-salvaging_station",
                    Some(SpriteKind::RepairBench) => "hud-crafting-repair_bench",
                    _ => "",
                };
                Text::new(&self.localized_strings.get_msg(station_name))
                    .right_from(state.ids.req_station_img, 10.0)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(14))
                    .color(
                        if self.show.crafting_fields.craft_sprite.map(|(_, s)| s)
                            == recipe.craft_sprite
                        {
                            TEXT_COLOR
                        } else {
                            TEXT_DULL_RED_COLOR
                        },
                    )
                    .set(state.ids.req_station_txt, ui);
            }
            // Ingredients Text
            // Hack from Sharp to account for iterators not having the same type
            let (mut iter_a, mut iter_b, mut iter_c, mut iter_d);
            let ingredients = match recipe_kind {
                RecipeKind::Simple => {
                    iter_a = recipe
                        .inputs
                        .iter()
                        .map(|(recipe, amount, _)| (recipe, *amount));
                    &mut iter_a as &mut dyn ExactSizeIterator<Item = (&RecipeInput, u32)>
                },
                RecipeKind::ModularWeapon => {
                    iter_b = core::iter::empty();
                    &mut iter_b
                },
                RecipeKind::Component(toolkind) => {
                    if let Some(material) = craft_slot_1
                        .and_then(|slot| match slot {
                            Slot::Inventory(slot) => self.inventory.get(slot),
                            Slot::Equip(_) => None,
                            Slot::Overflow(_) => None,
                        })
                        .and_then(|item| item.item_definition_id().itemdef_id().map(String::from))
                    {
                        let component_key = ComponentKey {
                            toolkind,
                            material,
                            modifier: craft_slot_2
                                .and_then(|slot| match slot {
                                    Slot::Inventory(slot) => self.inventory.get(slot),
                                    Slot::Equip(_) => None,
                                    Slot::Overflow(_) => None,
                                })
                                .and_then(|item| {
                                    item.item_definition_id().itemdef_id().map(String::from)
                                }),
                        };
                        if let Some(comp_recipe) =
                            self.client.component_recipe_book().get(&component_key)
                        {
                            iter_c = comp_recipe.inputs();
                            &mut iter_c as &mut dyn ExactSizeIterator<Item = _>
                        } else {
                            iter_b = core::iter::empty();
                            &mut iter_b
                        }
                    } else {
                        iter_b = core::iter::empty();
                        &mut iter_b
                    }
                },
                RecipeKind::Repair => {
                    if let Some(item) = match craft_slot_1 {
                        Some(Slot::Inventory(slot)) => self.inventory.get(slot),
                        Some(Slot::Equip(slot)) => self.inventory.equipped(slot),
                        Some(Slot::Overflow(_)) => None,
                        None => None,
                    } {
                        if let Some(recipe) = self.client.repair_recipe_book().repair_recipe(item) {
                            iter_d = recipe.inputs(item).collect::<Vec<_>>().into_iter();
                            &mut iter_d as &mut dyn ExactSizeIterator<Item = _>
                        } else {
                            iter_b = core::iter::empty();
                            &mut iter_b
                        }
                    } else {
                        iter_b = core::iter::empty();
                        &mut iter_b
                    }
                },
            };

            let num_ingredients = ingredients.len();
            if num_ingredients > 0 {
                Text::new(&self.localized_strings.get_msg("hud-crafting-ingredients"))
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(18))
                    .color(TEXT_COLOR)
                    .and(|t| {
                        if recipe.craft_sprite.is_some() {
                            t.down_from(state.ids.req_station_img, 10.0)
                        } else {
                            t.top_left_with_margins_on(state.ids.align_ing, 10.0, 5.0)
                        }
                    })
                    .set(state.ids.ingredients_txt, ui);

                // Ingredient images with tooltip
                if state.ids.ingredient_frame.len() < num_ingredients {
                    state.update(|state| {
                        state
                            .ids
                            .ingredient_frame
                            .resize(num_ingredients, &mut ui.widget_id_generator())
                    });
                };
                if state.ids.ingredients.len() < num_ingredients {
                    state.update(|state| {
                        state
                            .ids
                            .ingredients
                            .resize(num_ingredients, &mut ui.widget_id_generator())
                    });
                };
                if state.ids.ingredient_btn.len() < num_ingredients {
                    state.update(|state| {
                        state
                            .ids
                            .ingredient_btn
                            .resize(num_ingredients, &mut ui.widget_id_generator())
                    });
                };
                if state.ids.ingredient_img.len() < num_ingredients {
                    state.update(|state| {
                        state
                            .ids
                            .ingredient_img
                            .resize(num_ingredients, &mut ui.widget_id_generator())
                    });
                };
                if state.ids.req_text.len() < num_ingredients {
                    state.update(|state| {
                        state
                            .ids
                            .req_text
                            .resize(num_ingredients, &mut ui.widget_id_generator())
                    });
                };

                // Widget generation for every ingredient
                for (i, (recipe_input, amount)) in ingredients.enumerate() {
                    let item_def = match recipe_input {
                        RecipeInput::Item(item_def) => Some(Arc::clone(item_def)),
                        RecipeInput::Tag(tag) | RecipeInput::TagSameItem(tag) => self
                            .inventory
                            .slots()
                            .find_map(|slot| {
                                slot.as_ref().and_then(|item| {
                                    if item.matches_recipe_input(recipe_input, amount) {
                                        item.item_definition_id()
                                            .itemdef_id()
                                            .map(Arc::<ItemDef>::load_expect_cloned)
                                    } else {
                                        None
                                    }
                                })
                            })
                            .or_else(|| {
                                tag.exemplar_identifier()
                                    .map(Arc::<ItemDef>::load_expect_cloned)
                            }),
                        RecipeInput::ListSameItem(item_defs) => self
                            .inventory
                            .slots()
                            .find_map(|slot| {
                                slot.as_ref().and_then(|item| {
                                    if item.matches_recipe_input(recipe_input, amount) {
                                        item.item_definition_id()
                                            .itemdef_id()
                                            .map(Arc::<ItemDef>::load_expect_cloned)
                                    } else {
                                        None
                                    }
                                })
                            })
                            .or_else(|| {
                                item_defs.first().and_then(|i| {
                                    i.item_definition_id()
                                        .itemdef_id()
                                        .map(Arc::<ItemDef>::load_expect_cloned)
                                })
                            }),
                    };

                    let item_def = if let Some(item_def) = item_def {
                        item_def
                    } else {
                        warn!(
                            "Failed to create example item def for recipe input {:?}",
                            recipe_input
                        );
                        continue;
                    };

                    // Grey color for images and text if their amount is too low to craft the
                    // item
                    let item_count_in_inventory = self.inventory.item_count(&item_def);
                    let col = if item_count_in_inventory >= u64::from(amount.max(1)) {
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
                    // add a larger offset for the the first ingredient and the "Required Text
                    // for Catalysts/Tools"
                    let frame_offset = if i == 0 {
                        10.0
                    } else if amount == 0 {
                        5.0
                    } else {
                        0.0
                    };
                    let quality_col_img = match &item_def.quality() {
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
                    let frame = if amount == 0 {
                        frame.down_from(state.ids.req_text[i], 10.0 + frame_offset)
                    } else {
                        frame.down_from(frame_pos, 10.0 + frame_offset)
                    };
                    frame.set(state.ids.ingredient_frame[i], ui);
                    // Item button for auto search
                    if Button::image(self.imgs.wpn_icon_border)
                        .w_h(22.0, 22.0)
                        .middle_of(state.ids.ingredient_frame[i])
                        .hover_image(self.imgs.wpn_icon_border_mo)
                        .with_item_tooltip(
                            self.item_tooltip_manager,
                            core::iter::once(&*item_def as &dyn ItemDesc),
                            &None,
                            &item_tooltip,
                        )
                        .set(state.ids.ingredient_btn[i], ui)
                        .was_clicked()
                    {
                        events.push(Event::ChangeCraftingTab(CraftingTab::All));
                        #[allow(deprecated)]
                        events.push(Event::SearchRecipe(Some(item_def.name().to_string())));
                    }
                    // Item image
                    Image::new(animate_by_pulse(
                        &self.item_imgs.img_ids_or_not_found_img((&*item_def).into()),
                        self.pulse,
                    ))
                    .middle_of(state.ids.ingredient_btn[i])
                    .w_h(20.0, 20.0)
                    .graphics_for(state.ids.ingredient_btn[i])
                    .with_item_tooltip(
                        self.item_tooltip_manager,
                        core::iter::once(&*item_def as &dyn ItemDesc),
                        &None,
                        &item_tooltip,
                    )
                    .set(state.ids.ingredient_img[i], ui);

                    // Ingredients text and amount
                    // Don't show inventory amounts above 999 to avoid the widget clipping
                    let over9k = "99+";
                    let in_inv: &str = &item_count_in_inventory.to_string();
                    // Show Ingredients
                    // Align "Required" Text below last ingredient
                    if amount == 0 {
                        // Catalysts/Tools
                        let ref_widget = if i == 0 {
                            state.ids.ingredients_txt
                        } else {
                            state.ids.ingredient_frame[i - 1]
                        };
                        Text::new(&self.localized_strings.get_msg("hud-crafting-tool_cata"))
                            .down_from(ref_widget, 20.0)
                            .font_id(self.fonts.cyri.conrod_id)
                            .font_size(self.fonts.cyri.scale(14))
                            .color(TEXT_COLOR)
                            .set(state.ids.req_text[i], ui);

                        let (name, _) = util::item_text(
                            item_def.as_ref(),
                            self.localized_strings,
                            self.item_i18n,
                        );
                        Text::new(&name)
                            .right_from(state.ids.ingredient_frame[i], 10.0)
                            .font_id(self.fonts.cyri.conrod_id)
                            .font_size(self.fonts.cyri.scale(14))
                            .color(col)
                            .set(state.ids.ingredients[i], ui);
                    } else {
                        // Ingredients
                        let name = match recipe_input {
                            RecipeInput::Item(_) => {
                                let (name, _) = util::item_text(
                                    item_def.as_ref(),
                                    self.localized_strings,
                                    self.item_i18n,
                                );

                                name
                            },
                            RecipeInput::Tag(tag) | RecipeInput::TagSameItem(tag) => {
                                // TODO: Localize!
                                format!("Any {} item", tag.name())
                            },
                            RecipeInput::ListSameItem(item_defs) => {
                                // TODO: Localize!
                                format!(
                                    "Any of {}",
                                    item_defs
                                        .iter()
                                        .map(|def| {
                                            let (name, _) = util::item_text(
                                                def.as_ref(),
                                                self.localized_strings,
                                                self.item_i18n,
                                            );

                                            name
                                        })
                                        .collect::<String>()
                                )
                            },
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
                            .wrap_by_word()
                            .w(150.0)
                            .set(state.ids.ingredients[i], ui);
                    }
                }
            }
        } else if *sel_crafting_tab == CraftingTab::Dismantle {
            // Title
            Text::new(
                &self
                    .localized_strings
                    .get_msg("hud-crafting-dismantle_title"),
            )
            .mid_top_with_margin_on(state.ids.align_ing, 0.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(24))
            .color(TEXT_COLOR)
            .parent(state.ids.window)
            .set(state.ids.dismantle_title, ui);

            // Bench Icon
            let size = 140.0;
            Image::new(animate_by_pulse(
                &self
                    .item_imgs
                    .img_ids_or_not_found_img(ItemKey::Simple("DismantlingBench".to_string())),
                self.pulse,
            ))
            .wh([size; 2])
            .mid_top_with_margin_on(state.ids.align_ing, 50.0)
            .parent(state.ids.align_ing)
            .set(state.ids.dismantle_img, ui);

            // Explanation

            Text::new(
                &self
                    .localized_strings
                    .get_msg("hud-crafting-dismantle_explanation"),
            )
            .mid_bottom_with_margin_on(state.ids.dismantle_img, -60.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(TEXT_COLOR)
            .parent(state.ids.window)
            .set(state.ids.dismantle_txt, ui);
        }

        // Search / Title Recipes
        if let Some(key) = &self.show.crafting_fields.crafting_search_key {
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
            Rectangle::fill([162.0, 20.0])
                .top_left_with_margins_on(state.ids.btn_close_search, -2.0, 16.0)
                .hsla(0.0, 0.0, 0.0, 0.7)
                .depth(1.0)
                .parent(state.ids.window)
                .set(state.ids.input_bg_search, ui);
            if let Some(string) = TextEdit::new(key.as_str())
                .top_left_with_margins_on(state.ids.btn_close_search, -2.0, 18.0)
                .w_h(138.0, 20.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .parent(state.ids.window)
                .set(state.ids.input_search, ui)
            {
                events.push(Event::SearchRecipe(Some(string)));
            }
        } else {
            Text::new(&self.localized_strings.get_msg("hud-crafting-recipes"))
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
