use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
    TEXT_COLOR, TEXT_DULL_RED_COLOR, TEXT_GRAY_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    i18n::VoxygenLocalization,
    ui::{fonts::ConrodVoxygenFonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
};
use client::{self, Client};
use common::comp::Inventory;
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

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
        title_ing,
        align_ing,
        scrollbar_ing,
        btn_craft,
        recipe_names[],
        recipe_img_frame[],
        recipe_img[],
        ingredients[],
        ingredient_frame[],
        ingredient_img[],
        req_text[],
        ingredients_txt,
        output_img_frame,
        output_img,
    }
}

pub enum Event {
    CraftRecipe(String),
    Close,
}

#[derive(WidgetCommon)]
pub struct Crafting<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    item_imgs: &'a ItemImgs,
    inventory: &'a Inventory,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
#[allow(clippy::too_many_arguments)]
impl<'a> Crafting<'a> {
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        item_imgs: &'a ItemImgs,
        inventory: &'a Inventory,
    ) -> Self {
        Self {
            client,
            imgs,
            fonts,
            localized_strings,
            rot_imgs,
            tooltip_manager,
            item_imgs,
            inventory,
            common: widget::CommonBuilder::default(),
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

        if state.ids.recipe_names.len() < self.client.recipe_book().iter().len() {
            state.update(|state| {
                state.ids.recipe_names.resize(
                    self.client.recipe_book().iter().len(),
                    &mut ui.widget_id_generator(),
                )
            });
        }
        /*if state.ids.recipe_img_frame.len() < self.client.recipe_book().iter().len() {
            state.update(|state| {
                state.ids.recipe_img_frame.resize(
                    self.client.recipe_book().iter().len(),
                    &mut ui.widget_id_generator(),
                )
            });
        }
        if state.ids.recipe_img.len() < self.client.recipe_book().iter().len() {
            state.update(|state| {
                state.ids.recipe_img.resize(
                    self.client.recipe_book().iter().len(),
                    &mut ui.widget_id_generator(),
                )
            });
        }*/

        let ids = &state.ids;

        let mut events = Vec::new();

        // Tooltips
        let item_tooltip = Tooltip::new({
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
        .title_text_color(TEXT_COLOR)
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);

        Image::new(self.imgs.crafting_window)
            .bottom_right_with_margins_on(ui.window, 308.0, 450.0)
            .color(Some(UI_MAIN))
            .w_h(422.0, 460.0)
            .set(ids.window, ui);
        Image::new(self.imgs.crafting_frame)
            .middle_of(ids.window)
            .color(Some(UI_HIGHLIGHT_0))
            .w_h(422.0, 460.0)
            .set(ids.window_frame, ui);
        Image::new(self.imgs.crafting_icon_bordered)
            .w_h(38.0, 38.0)
            .top_left_with_margins_on(state.ids.window_frame, 4.0, 4.0)
            .set(state.ids.icon, ui);
        //  Close Button
        if Button::image(self.imgs.close_button)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(ids.window, 0.0, 0.0)
            .set(ids.close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // Title
        Text::new(&self.localized_strings.get("hud.crafting"))
            .mid_top_with_margin_on(ids.window_frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(TEXT_COLOR)
            .set(ids.title_main, ui);

        // Alignment
        Rectangle::fill_with([136.0, 378.0], color::TRANSPARENT)
            .top_left_with_margins_on(ids.window_frame, 74.0, 5.0)
            .set(ids.align_rec, ui);
        Rectangle::fill_with([274.0, 340.0], color::TRANSPARENT)
            .top_right_with_margins_on(ids.window, 74.0, 5.0)
            .set(ids.align_ing, ui);
        let client = &self.client;
        // First available recipes, then unavailable ones
        let recipe_iter = self
            .client
            .recipe_book()
            .iter()
            .filter(|(name, _)| client.available_recipes().contains(name.as_str()))
            .map(|(name, recipe)| (name, recipe, true))
            .chain(
                client
                    .recipe_book()
                    .iter()
                    .filter(|(name, _)| !client.available_recipes().contains(name.as_str()))
                    .map(|(name, recipe)| (name, recipe, false)),
            );
        match &state.selected_recipe {
            None => {},
            Some(recipe) => {
                let can_perform = client.available_recipes().contains(recipe.as_str());
                // Ingredients Text
                Text::new(&self.localized_strings.get("hud.crafting.ingredients"))
                    .top_left_with_margins_on(state.ids.align_ing, 10.0, 5.0)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(18))
                    .color(TEXT_COLOR)
                    .set(state.ids.ingredients_txt, ui);
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
                    .mid_bottom_with_margin_on(ids.align_ing, -31.0)
                    .set(ids.btn_craft, ui)
                    .was_clicked()
                {
                    events.push(Event::CraftRecipe(recipe.clone()));
                }
                // Result Image BG
                Image::new(self.imgs.inv_slot)
                    .w_h(60.0, 60.0)
                    .top_right_with_margins_on(state.ids.align_ing, 15.0, 10.0)
                    .parent(ids.align_ing)
                    .set(ids.output_img_frame, ui);

                if let Some(recipe) = state
                    .selected_recipe
                    .as_ref()
                    .and_then(|r| self.client.recipe_book().get(r.as_str()))
                {
                    let output_text = format!("x{}", &recipe.output.1.to_string());
                    // Output Image
                    Button::image(
                        self.item_imgs
                            .img_id_or_not_found_img((&recipe.output.0).into()),
                    )
                    .w_h(55.0, 55.0)
                    .middle_of(state.ids.output_img_frame)
                    .label(if recipe.output.1 > 1 {&output_text} else {""}) // Only show output amount for amounts > 1
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(14))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_y(conrod_core::position::Relative::Scalar(-24.0))
                    .label_x(conrod_core::position::Relative::Scalar(24.0))
                    .with_tooltip(
                        self.tooltip_manager,
                        recipe.output.0.name(),
                        recipe.output.0.description(),
                        &item_tooltip,
                    )
                    .set(state.ids.output_img, ui);
                }
            },
        }

        // Recipe list
        for (i, (name, recipe, can_perform)) in recipe_iter.enumerate() {
            let button = Button::image(
                if state
                    .selected_recipe
                    .as_ref()
                    .map(|s| s != name)
                    .unwrap_or(false)
                {
                    self.imgs.nothing
                } else {
                    match state.selected_recipe {
                        None => self.imgs.nothing,
                        Some(_) => self.imgs.selection,
                    }
                },
            );
            // Recipe Button
            let button = if i == 0 {
                button.mid_top_with_margin_on(state.ids.align_rec, 2.0)
            } else {
                button.mid_bottom_with_margin_on(state.ids.recipe_names[i - 1], -25.0)
            };
            if button
                .label(recipe.output.0.name())
                .w_h(130.0, 20.0)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .label_color(can_perform.then_some(TEXT_COLOR).unwrap_or(TEXT_GRAY_COLOR))
                .label_font_size(self.fonts.cyri.scale(12))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .set(state.ids.recipe_names[i], ui)
                .was_clicked()
            {
                if state
                    .selected_recipe
                    .as_ref()
                    .map(|s| s == name)
                    .unwrap_or(false)
                {
                    state.update(|s| s.selected_recipe = None);
                } else {
                    state.update(|s| s.selected_recipe = Some(name.clone()));
                }
            }
            // Image BG
            /*Rectangle::fill_with([10.0, 10.0], color::TRANSPARENT)
                .w_h(20.0, 20.0)
                .mid_left_of(state.ids.recipe_names[i])
                .set(state.ids.recipe_img_frame[i], ui);
            //Item Image
            Image::new(
                self.item_imgs
                    .img_id_or_not_found_img((&recipe.output.0).into()),
            )
            .w_h(18.0, 18.0)
            .color(
                can_perform
                    .then_some(Some(TEXT_COLOR))
                    .unwrap_or(Some(TEXT_GRAY_COLOR)),
            )
            .middle_of(state.ids.recipe_img_frame[i])
            .set(state.ids.recipe_img[i], ui);*/
        }

        //Ingredients
        if let Some(recipe) = state
            .selected_recipe
            .as_ref()
            .and_then(|r| self.client.recipe_book().get(r.as_str()))
        {
            // Title
            Text::new(&recipe.output.0.name().to_string())
                .mid_top_with_margin_on(state.ids.align_ing, -22.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .parent(state.ids.window)
                .set(state.ids.title_ing, ui);
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
            for (i, (item, amount)) in recipe.inputs.iter().enumerate() {
                // Grey color for images and text if their amount is too low to craft the item
                let col = if self.inventory.item_count(item) as f32 / *amount as f32 >= 1.0 {
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
                let frame = Image::new(self.imgs.inv_slot).w_h(25.0, 25.0);
                let frame = if *amount == 0 {
                    frame.down_from(state.ids.req_text[i], 10.0 + frame_offset)
                } else {
                    frame.down_from(frame_pos, 10.0 + frame_offset)
                };
                frame.set(state.ids.ingredient_frame[i], ui);
                //Item Image
                let title = item.name();
                let desc = item.description();
                Button::image(self.item_imgs.img_id_or_not_found_img(item.into()))
                    .w_h(22.0, 22.0)
                    .middle_of(state.ids.ingredient_frame[i])
                    //.image_color(col)
                    .with_tooltip(self.tooltip_manager, title, desc, &item_tooltip)
                    .set(state.ids.ingredient_img[i], ui);
                // Ingredients text and amount
                // Don't show inventory amounts above 999 to avoid the widget clipping
                let over9k = "999+";
                let in_inv: &str = &self.inventory.item_count(item).to_string();
                // Show Ingredients
                // Align "Required" Text below last ingredient
                if *amount == 0 {
                    // Catalysts/Tools
                    Text::new(&self.localized_strings.get("hud.crafting.tool_cata"))
                        .down_from(state.ids.ingredient_frame[i - 1], 20.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(col)
                        .set(state.ids.req_text[i], ui);
                    Text::new(item.name())
                        .right_from(state.ids.ingredient_frame[i], 10.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(col)
                        .set(state.ids.ingredients[i], ui);
                } else {
                    // Ingredients
                    let input = format!(
                        "{}x {} ({})",
                        amount,
                        item.name(),
                        if self.inventory.item_count(item) > 999 {
                            over9k
                        } else {
                            in_inv
                        }
                    );
                    // Ingredient Text
                    Text::new(&input)
                        .right_from(state.ids.ingredient_frame[i], 10.0)
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(col)
                        .set(state.ids.ingredients[i], ui);
                }
            }
        }

        let ids = &state.ids;
        // Scrollbars
        Scrollbar::y_axis(ids.align_rec)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(ids.scrollbar_rec, ui);
        Scrollbar::y_axis(ids.align_ing)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(ids.scrollbar_ing, ui);

        // Title Recipes and Ingredients
        Text::new(&self.localized_strings.get("hud.crafting.recipes"))
            .mid_top_with_margin_on(ids.align_rec, -22.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(TEXT_COLOR)
            .parent(ids.window)
            .set(ids.title_rec, ui);

        events
    }
}
