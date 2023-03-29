use crate::persistence::{character_updater::CharacterUpdater, PersistedComponents};
use common::{
    character::CharacterId,
    comp::{inventory::loadout_builder::LoadoutBuilder, Body, Inventory, Item, SkillSet, Stats},
};
use specs::{Entity, WriteExpect};

const VALID_STARTER_ITEMS: &[[Option<&str>; 2]] = &[
    [None, None], // Not used with an unmodified client but should still be allowed (zesterer)
    [Some("common.items.weapons.hammer.starter_hammer"), None],
    [Some("common.items.weapons.bow.starter"), None],
    [Some("common.items.weapons.axe.starter_axe"), None],
    [Some("common.items.weapons.staff.starter_staff"), None],
    [Some("common.items.weapons.sword.starter"), None],
    [
        Some("common.items.weapons.sword_1h.starter"),
        Some("common.items.weapons.sword_1h.starter"),
    ],
];

#[derive(Debug)]
pub enum CreationError {
    InvalidWeapon,
    InvalidBody,
}

pub fn create_character(
    entity: Entity,
    player_uuid: String,
    character_alias: String,
    character_mainhand: Option<String>,
    character_offhand: Option<String>,
    body: Body,
    character_updater: &mut WriteExpect<'_, CharacterUpdater>,
) -> Result<(), CreationError> {
    // quick fix whitelist validation for now; eventually replace the
    // `Option<String>` with an index into a server-provided list of starter
    // items, and replace `comp::body::Body` with `comp::body::humanoid::Body`
    // throughout the messages involved
    if !matches!(body, Body::Humanoid(_)) {
        return Err(CreationError::InvalidBody);
    }
    if !VALID_STARTER_ITEMS.contains(&[character_mainhand.as_deref(), character_offhand.as_deref()])
    {
        return Err(CreationError::InvalidWeapon);
    };
    // The client sends None if a weapon hand is empty
    let loadout = LoadoutBuilder::empty()
        .defaults()
        .active_mainhand(character_mainhand.map(|x| Item::new_from_asset_expect(&x)))
        .active_offhand(character_offhand.map(|x| Item::new_from_asset_expect(&x)))
        .build();
    let mut inventory = Inventory::with_loadout_humanoid(loadout);
    let stats = Stats::new(character_alias.to_string(), body);
    let skill_set = SkillSet::default();
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
    let map_marker = None;

    character_updater.create_character(entity, player_uuid, character_alias, PersistedComponents {
        body,
        stats,
        skill_set,
        inventory,
        waypoint,
        pets: Vec::new(),
        active_abilities: Default::default(),
        map_marker,
    });
    Ok(())
}

pub fn edit_character(
    entity: Entity,
    player_uuid: String,
    id: CharacterId,
    character_alias: String,
    body: Body,
    character_updater: &mut WriteExpect<'_, CharacterUpdater>,
) -> Result<(), CreationError> {
    if !matches!(body, Body::Humanoid(_)) {
        return Err(CreationError::InvalidBody);
    }

    character_updater.edit_character(entity, player_uuid, id, character_alias, (body,));
    Ok(())
}

// Error handling
impl core::fmt::Display for CreationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CreationError::InvalidWeapon => write!(
                f,
                "Invalid weapon.\nServer and client might be partially incompatible."
            ),
            CreationError::InvalidBody => write!(
                f,
                "Invalid Body.\nServer and client might be partially incompatible"
            ),
        }
    }
}
