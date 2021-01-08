use crate::persistence::character_loader::CharacterLoader;
use common::comp::{inventory::loadout_builder::LoadoutBuilder, Body, Inventory, Item, Stats};
use specs::{Entity, ReadExpect};

pub fn create_character(
    entity: Entity,
    player_uuid: String,
    character_alias: String,
    character_tool: Option<String>,
    body: Body,
    character_loader: &ReadExpect<'_, CharacterLoader>,
) {
    let stats = Stats::new(character_alias.to_string(), body);

    let loadout = LoadoutBuilder::new()
        .defaults()
        .active_item(Some(Item::new_from_asset_expect(
            character_tool.as_deref().unwrap(),
        )))
        .build();

    let mut inventory = Inventory::new_with_loadout(loadout);

    // Default items for new characters
    inventory.push(Item::new_from_asset_expect(
        "common.items.consumable.potion_minor",
    ));
    inventory.push(Item::new_from_asset_expect("common.items.food.cheese"));

    let waypoint = None;

    character_loader.create_character(
        entity,
        player_uuid,
        character_alias,
        (body, stats, inventory, waypoint),
    );
}
