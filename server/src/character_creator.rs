use crate::persistence::character_updater::CharacterUpdater;
use common::comp::{
    inventory::loadout_builder::LoadoutBuilder, Body, Inventory, Item, SkillSet, Stats,
};
use specs::{Entity, WriteExpect};

const VALID_STARTER_ITEMS: [&str; 6] = [
    "common.items.weapons.hammer.starter_hammer",
    "common.items.weapons.bow.starter",
    "common.items.weapons.axe.starter_axe",
    "common.items.weapons.staff.starter_staff",
    "common.items.weapons.sword.starter",
    "common.items.weapons.sceptre.starter_sceptre",
];

pub fn create_character(
    entity: Entity,
    player_uuid: String,
    character_alias: String,
    character_tool: Option<String>,
    body: Body,
    character_updater: &mut WriteExpect<'_, CharacterUpdater>,
) {
    // quick fix whitelist validation for now; eventually replace the
    // `Option<String>` with an index into a server-provided list of starter
    // items, and replace `comp::body::Body` with `comp::body::humanoid::Body`
    // throughout the messages involved
    let tool_id = match character_tool {
        Some(tool_id) if VALID_STARTER_ITEMS.contains(&&*tool_id) => tool_id,
        _ => return,
    };
    if !matches!(body, Body::Humanoid(_)) {
        return;
    }

    let stats = Stats::new(character_alias.to_string());
    let skill_set = SkillSet::default();

    let loadout = LoadoutBuilder::new()
        .defaults()
        .active_item(Some(Item::new_from_asset_expect(&tool_id)))
        .build();

    let mut inventory = Inventory::new_with_loadout(loadout);

    // Default items for new characters
    inventory
        .push(Item::new_from_asset_expect(
            "common.items.consumable.potion_minor",
        ))
        .expect("Inventory has at least 2 slots left!");
    inventory
        .push(Item::new_from_asset_expect("common.items.food.cheese"))
        .expect("Inventory has at least 1 slot left!");

    let waypoint = None;

    character_updater.create_character(
        entity,
        player_uuid,
        character_alias,
        (body, stats, skill_set, inventory, waypoint),
    );
}
