//! Database operations related to character data
//!
//! Methods in this module should remain private to the persistence module -
//! database updates and loading are communicated via requests to the
//! [`CharacterLoader`] and [`CharacterUpdater`] while results/responses are
//! polled and handled each server tick.
extern crate rusqlite;

use super::{error::PersistenceError, models::*};
use crate::{
    comp::{self, Inventory},
    persistence::{
        character::conversions::{
            convert_active_abilities_from_database, convert_active_abilities_to_database,
            convert_body_from_database, convert_body_to_database_json,
            convert_character_from_database, convert_inventory_from_database_items,
            convert_items_to_database_items, convert_loadout_from_database_items,
            convert_skill_groups_to_database, convert_skill_set_from_database,
            convert_stats_from_database, convert_waypoint_from_database_json,
            convert_waypoint_to_database_json,
        },
        character_loader::{CharacterCreationResult, CharacterDataResult, CharacterListResult},
        character_updater::PetPersistenceData,
        error::PersistenceError::DatabaseError,
        EditableComponents, PersistedComponents,
    },
};
use common::{
    character::{CharacterId, CharacterItem, MAX_CHARACTERS_PER_PLAYER},
    event::UpdateCharacterMetadata,
};
use core::ops::Range;
use rusqlite::{types::Value, Connection, ToSql, Transaction};
use std::{num::NonZeroU64, rc::Rc};
use tracing::{debug, error, trace, warn};

/// Private module for very tightly coupled database conversion methods.  In
/// general, these have many invariants that need to be maintained when they're
/// called--do not assume it's safe to make these public!
mod conversions;

pub(crate) type EntityId = i64;

pub(crate) use conversions::convert_waypoint_from_database_json as parse_waypoint;

const CHARACTER_PSEUDO_CONTAINER_DEF_ID: &str = "veloren.core.pseudo_containers.character";
const INVENTORY_PSEUDO_CONTAINER_DEF_ID: &str = "veloren.core.pseudo_containers.inventory";
const LOADOUT_PSEUDO_CONTAINER_DEF_ID: &str = "veloren.core.pseudo_containers.loadout";
const OVERFLOW_ITEMS_PSEUDO_CONTAINER_DEF_ID: &str =
    "veloren.core.pseudo_containers.overflow_items";
const INVENTORY_PSEUDO_CONTAINER_POSITION: &str = "inventory";
const LOADOUT_PSEUDO_CONTAINER_POSITION: &str = "loadout";
const OVERFLOW_ITEMS_PSEUDO_CONTAINER_POSITION: &str = "overflow_items";
const WORLD_PSEUDO_CONTAINER_ID: EntityId = 1;

#[derive(Clone, Copy)]
struct CharacterContainers {
    inventory_container_id: EntityId,
    loadout_container_id: EntityId,
    overflow_items_container_id: EntityId,
}

/// Load the inventory/loadout
///
/// Loading is done recursively to ensure that each is topologically sorted in
/// the sense required by convert_inventory_from_database_items.
///
/// For items with components, the parent item must sorted so that its
/// components are after the parent item.
pub fn load_items(connection: &Connection, root: i64) -> Result<Vec<Item>, PersistenceError> {
    let mut stmt = connection.prepare_cached(
        "
        WITH RECURSIVE
        items_tree (
            item_id,
            parent_container_item_id,
            item_definition_id,
            stack_size,
            position,
            properties
        ) AS (
            SELECT  item_id,
                    parent_container_item_id,
                    item_definition_id,
                    stack_size,
                    position,
                    properties
            FROM item
            WHERE parent_container_item_id = ?1
            UNION ALL
            SELECT  item.item_id,
                    item.parent_container_item_id,
                    item.item_definition_id,
                    item.stack_size,
                    item.position,
                    item.properties
            FROM item, items_tree
            WHERE item.parent_container_item_id = items_tree.item_id
        )
        SELECT  *
        FROM    items_tree",
    )?;

    let items = stmt
        .query_map([root], |row| {
            Ok(Item {
                item_id: row.get(0)?,
                parent_container_item_id: row.get(1)?,
                item_definition_id: row.get(2)?,
                stack_size: row.get(3)?,
                position: row.get(4)?,
                properties: row.get(5)?,
            })
        })?
        .filter_map(Result::ok)
        .collect::<Vec<Item>>();

    Ok(items)
}

/// Load stored data for a character.
///
/// After first logging in, and after a character is selected, we fetch this
/// data for the purpose of inserting their persisted data for the entity.
pub fn load_character_data(
    requesting_player_uuid: String,
    char_id: CharacterId,
    connection: &Connection,
) -> CharacterDataResult {
    let character_containers = get_pseudo_containers(connection, char_id)?;
    let inventory_items = load_items(connection, character_containers.inventory_container_id)?;
    let loadout_items = load_items(connection, character_containers.loadout_container_id)?;
    let overflow_items_items =
        load_items(connection, character_containers.overflow_items_container_id)?;

    let mut stmt = connection.prepare_cached(
        "
        SELECT  c.character_id,
                c.alias,
                c.waypoint,
                b.variant,
                b.body_data
        FROM    character c
        JOIN    body b ON (c.character_id = b.body_id)
        WHERE   c.player_uuid = ?1
        AND     c.character_id = ?2",
    )?;

    let (body_data, character_data) = stmt.query_row(
        [requesting_player_uuid.clone(), char_id.0.to_string()],
        |row| {
            let character_data = Character {
                character_id: row.get(0)?,
                player_uuid: requesting_player_uuid,
                alias: row.get(1)?,
                waypoint: row.get(2)?,
            };

            let body_data = Body {
                body_id: row.get(0)?,
                variant: row.get(3)?,
                body_data: row.get(4)?,
            };

            Ok((body_data, character_data))
        },
    )?;

    let (char_waypoint, char_map_marker) = match character_data
        .waypoint
        .as_ref()
        .map(|x| convert_waypoint_from_database_json(x))
    {
        Some(Ok(w)) => w,
        Some(Err(e)) => {
            warn!(
                "Error reading waypoint from database for character ID
    {}, error: {}",
                char_id.0, e
            );
            (None, None)
        },
        None => (None, None),
    };

    let mut stmt = connection.prepare_cached(
        "
        SELECT  skill_group_kind,
                earned_exp,
                spent_exp,
                skills,
                hash_val
        FROM    skill_group
        WHERE   entity_id = ?1",
    )?;

    let skill_group_data = stmt
        .query_map([char_id.0], |row| {
            Ok(SkillGroup {
                entity_id: char_id.0,
                skill_group_kind: row.get(0)?,
                earned_exp: row.get(1)?,
                spent_exp: row.get(2)?,
                skills: row.get(3)?,
                hash_val: row.get(4)?,
            })
        })?
        .filter_map(Result::ok)
        .collect::<Vec<SkillGroup>>();

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        SELECT  p.pet_id,
                p.name,
                b.variant,
                b.body_data
        FROM    pet p
        JOIN    body b ON (p.pet_id = b.body_id)
        WHERE   p.character_id = ?1",
    )?;

    let db_pets = stmt
        .query_map([char_id.0], |row| {
            Ok(Pet {
                database_id: row.get(0)?,
                name: row.get(1)?,
                body_variant: row.get(2)?,
                body_data: row.get(3)?,
            })
        })?
        .filter_map(Result::ok)
        .collect::<Vec<Pet>>();

    // Re-construct the pet components for the player's pets, including
    // de-serializing the pets' bodies and creating their Pet and Stats
    // components
    let pets = db_pets
        .iter()
        .filter_map(|db_pet| {
            if let Ok(pet_body) =
                convert_body_from_database(&db_pet.body_variant, &db_pet.body_data)
            {
                let pet = comp::Pet::new_from_database(
                    NonZeroU64::new(db_pet.database_id as u64).unwrap(),
                );
                let pet_stats = comp::Stats::new(db_pet.name.to_owned(), pet_body);
                Some((pet, pet_body, pet_stats))
            } else {
                warn!(
                    "Failed to deserialize pet_id: {} for character_id {}",
                    db_pet.database_id, char_id.0
                );
                None
            }
        })
        .collect::<Vec<(comp::Pet, comp::Body, comp::Stats)>>();

    let mut stmt = connection.prepare_cached(
        "
            SELECT  ability_sets
            FROM    ability_set
            WHERE   entity_id = ?1",
    )?;

    let ability_set_data = stmt.query_row([char_id.0], |row| {
        Ok(AbilitySets {
            entity_id: char_id.0,
            ability_sets: row.get(0)?,
        })
    })?;

    let (skill_set, skill_set_persistence_load_error) =
        convert_skill_set_from_database(&skill_group_data);
    let body = convert_body_from_database(&body_data.variant, &body_data.body_data)?;
    Ok((
        PersistedComponents {
            body,
            stats: convert_stats_from_database(character_data.alias, body),
            skill_set,
            inventory: convert_inventory_from_database_items(
                character_containers.inventory_container_id,
                &inventory_items,
                character_containers.loadout_container_id,
                &loadout_items,
                character_containers.overflow_items_container_id,
                &overflow_items_items,
            )?,
            waypoint: char_waypoint,
            pets,
            active_abilities: convert_active_abilities_from_database(&ability_set_data),
            map_marker: char_map_marker,
        },
        UpdateCharacterMetadata {
            skill_set_persistence_load_error,
        },
    ))
}

/// Loads a list of characters belonging to the player. This data is a small
/// subset of the character's data, and is used to render the character and
/// their level in the character list.
///
/// In the event that a join fails, for a character (i.e. they lack an entry for
/// stats, body, etc...) the character is skipped, and no entry will be
/// returned.
pub fn load_character_list(player_uuid_: &str, connection: &Connection) -> CharacterListResult {
    let mut stmt = connection.prepare_cached(
        "
            SELECT  character_id,
                    alias,
                    waypoint
            FROM    character
            WHERE   player_uuid = ?1
            ORDER BY character_id",
    )?;

    let characters = stmt
        .query_map([player_uuid_], |row| {
            Ok(Character {
                character_id: row.get(0)?,
                alias: row.get(1)?,
                player_uuid: player_uuid_.to_owned(),
                waypoint: row.get(2)?,
            })
        })?
        .map(|x| x.unwrap())
        .collect::<Vec<Character>>();
    drop(stmt);

    characters
        .iter()
        .map(|character_data| {
            let char = convert_character_from_database(character_data);

            let mut stmt = connection.prepare_cached(
                "
                SELECT  body_id,
                        variant,
                        body_data
                FROM    body
                WHERE   body_id = ?1",
            )?;
            let db_body = stmt.query_row([char.id.map(|c| c.0)], |row| {
                Ok(Body {
                    body_id: row.get(0)?,
                    variant: row.get(1)?,
                    body_data: row.get(2)?,
                })
            })?;
            drop(stmt);

            let char_body = convert_body_from_database(&db_body.variant, &db_body.body_data)?;

            let loadout_container_id = get_pseudo_container_id(
                connection,
                CharacterId(character_data.character_id),
                LOADOUT_PSEUDO_CONTAINER_POSITION,
            )?;

            let loadout_items = load_items(connection, loadout_container_id)?;

            let loadout =
                convert_loadout_from_database_items(loadout_container_id, &loadout_items)?;

            Ok(CharacterItem {
                character: char,
                body: char_body,
                inventory: Inventory::with_loadout(loadout, char_body),
                location: character_data.waypoint.as_ref().cloned(),
            })
        })
        .collect()
}

pub fn create_character(
    uuid: &str,
    character_alias: &str,
    persisted_components: PersistedComponents,
    transaction: &mut Transaction,
) -> CharacterCreationResult {
    check_character_limit(uuid, transaction)?;

    let PersistedComponents {
        body,
        stats: _,
        skill_set,
        inventory,
        waypoint,
        pets: _,
        active_abilities,
        map_marker,
    } = persisted_components;

    // Fetch new entity IDs for character, inventory, loadout, and overflow items
    let mut new_entity_ids = get_new_entity_ids(transaction, |next_id| next_id + 4)?;

    // Create pseudo-container items for character
    let character_id = new_entity_ids.next().unwrap();
    let inventory_container_id = new_entity_ids.next().unwrap();
    let loadout_container_id = new_entity_ids.next().unwrap();
    let overflow_items_container_id = new_entity_ids.next().unwrap();

    let pseudo_containers = vec![
        Item {
            stack_size: 1,
            item_id: character_id,
            parent_container_item_id: WORLD_PSEUDO_CONTAINER_ID,
            item_definition_id: CHARACTER_PSEUDO_CONTAINER_DEF_ID.to_owned(),
            position: character_id.to_string(),
            properties: String::new(),
        },
        Item {
            stack_size: 1,
            item_id: inventory_container_id,
            parent_container_item_id: character_id,
            item_definition_id: INVENTORY_PSEUDO_CONTAINER_DEF_ID.to_owned(),
            position: INVENTORY_PSEUDO_CONTAINER_POSITION.to_owned(),
            properties: String::new(),
        },
        Item {
            stack_size: 1,
            item_id: loadout_container_id,
            parent_container_item_id: character_id,
            item_definition_id: LOADOUT_PSEUDO_CONTAINER_DEF_ID.to_owned(),
            position: LOADOUT_PSEUDO_CONTAINER_POSITION.to_owned(),
            properties: String::new(),
        },
        Item {
            stack_size: 1,
            item_id: loadout_container_id,
            parent_container_item_id: character_id,
            item_definition_id: OVERFLOW_ITEMS_PSEUDO_CONTAINER_DEF_ID.to_owned(),
            position: OVERFLOW_ITEMS_PSEUDO_CONTAINER_POSITION.to_owned(),
            properties: String::new(),
        },
    ];

    let mut stmt = transaction.prepare_cached(
        "
        INSERT INTO item (item_id,
                          parent_container_item_id,
                          item_definition_id,
                          stack_size,
                          position,
                          properties)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;

    for pseudo_container in pseudo_containers {
        stmt.execute([
            &pseudo_container.item_id as &dyn ToSql,
            &pseudo_container.parent_container_item_id,
            &pseudo_container.item_definition_id,
            &pseudo_container.stack_size,
            &pseudo_container.position,
            &pseudo_container.properties,
        ])?;
    }
    drop(stmt);

    let mut stmt = transaction.prepare_cached(
        "
        INSERT INTO body (body_id,
                          variant,
                          body_data)
        VALUES (?1, ?2, ?3)",
    )?;

    let (body_variant, body_json) = convert_body_to_database_json(&body)?;
    stmt.execute([
        &character_id as &dyn ToSql,
        &body_variant.to_string(),
        &body_json,
    ])?;
    drop(stmt);

    let mut stmt = transaction.prepare_cached(
        "
        INSERT INTO character (character_id,
                               player_uuid,
                               alias,
                               waypoint)
        VALUES (?1, ?2, ?3, ?4)",
    )?;

    stmt.execute([
        &character_id as &dyn ToSql,
        &uuid,
        &character_alias,
        &convert_waypoint_to_database_json(waypoint, map_marker),
    ])?;
    drop(stmt);

    let db_skill_groups =
        convert_skill_groups_to_database(CharacterId(character_id), skill_set.skill_groups());

    let mut stmt = transaction.prepare_cached(
        "
        INSERT INTO skill_group (entity_id,
                                 skill_group_kind,
                                 earned_exp,
                                 spent_exp,
                                 skills,
                                 hash_val)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;

    for skill_group in db_skill_groups {
        stmt.execute([
            &character_id as &dyn ToSql,
            &skill_group.skill_group_kind,
            &skill_group.earned_exp,
            &skill_group.spent_exp,
            &skill_group.skills,
            &skill_group.hash_val,
        ])?;
    }
    drop(stmt);

    let ability_sets =
        convert_active_abilities_to_database(CharacterId(character_id), &active_abilities);

    let mut stmt = transaction.prepare_cached(
        "
        INSERT INTO ability_set (entity_id,
                                 ability_sets)
        VALUES (?1, ?2)",
    )?;

    stmt.execute([
        &character_id as &dyn ToSql,
        &ability_sets.ability_sets as &dyn ToSql,
    ])?;
    drop(stmt);

    // Insert default inventory and loadout item records
    let mut inserts = Vec::new();

    get_new_entity_ids(transaction, |mut next_id| {
        let inserts_ = convert_items_to_database_items(
            loadout_container_id,
            &inventory,
            inventory_container_id,
            overflow_items_container_id,
            &mut next_id,
        );
        inserts = inserts_;
        next_id
    })?;

    let mut stmt = transaction.prepare_cached(
        "
        INSERT INTO item (item_id,
                          parent_container_item_id,
                          item_definition_id,
                          stack_size,
                          position,
                          properties)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;

    for item in inserts {
        stmt.execute([
            &item.model.item_id as &dyn ToSql,
            &item.model.parent_container_item_id,
            &item.model.item_definition_id,
            &item.model.stack_size,
            &item.model.position,
            &item.model.properties,
        ])?;
    }
    drop(stmt);

    load_character_list(uuid, transaction).map(|list| (CharacterId(character_id), list))
}

pub fn edit_character(
    editable_components: EditableComponents,
    transaction: &mut Transaction,
    character_id: CharacterId,
    uuid: &str,
    character_alias: &str,
) -> CharacterCreationResult {
    let (body,) = editable_components;
    let mut char_list = load_character_list(uuid, transaction);

    if let Ok(char_list) = &mut char_list {
        if let Some(char) = char_list
            .iter_mut()
            .find(|c| c.character.id == Some(character_id))
        {
            if let (comp::Body::Humanoid(new), comp::Body::Humanoid(old)) = (body, char.body) {
                if new.species != old.species || new.body_type != old.body_type {
                    warn!(
                        "Character edit rejected due to failed validation - Character ID: {} \
                         Alias: {}",
                        character_id.0, character_alias
                    );
                    return Err(PersistenceError::CharacterDataError);
                } else {
                    char.body = body;
                }
            }
        }
    }

    let mut stmt = transaction
        .prepare_cached("UPDATE body SET variant = ?1, body_data = ?2 WHERE body_id = ?3")?;

    let (body_variant, body_data) = convert_body_to_database_json(&body)?;
    stmt.execute([
        &body_variant.to_string(),
        &body_data,
        &character_id.0 as &dyn ToSql,
    ])?;
    drop(stmt);

    let mut stmt =
        transaction.prepare_cached("UPDATE character SET alias = ?1 WHERE character_id = ?2")?;

    stmt.execute([&character_alias, &character_id.0 as &dyn ToSql])?;
    drop(stmt);

    char_list.map(|list| (character_id, list))
}

/// Permanently deletes a character
pub fn delete_character(
    requesting_player_uuid: &str,
    char_id: CharacterId,
    transaction: &mut Transaction,
) -> Result<(), PersistenceError> {
    debug!(?requesting_player_uuid, ?char_id, "Deleting character");

    let mut stmt = transaction.prepare_cached(
        "
        SELECT  COUNT(1)
        FROM    character
        WHERE   character_id = ?1
        AND     player_uuid = ?2",
    )?;

    let result = stmt.query_row([&char_id.0 as &dyn ToSql, &requesting_player_uuid], |row| {
        let y: i64 = row.get(0)?;
        Ok(y)
    })?;
    drop(stmt);

    if result != 1 {
        // The character does not exist, or does not belong to the requesting player so
        // silently drop the request.
        return Ok(());
    }

    // Delete skill groups
    let mut stmt = transaction.prepare_cached(
        "
        DELETE
        FROM    skill_group
        WHERE   entity_id = ?1",
    )?;

    stmt.execute([&char_id.0])?;
    drop(stmt);

    let pet_ids = get_pet_ids(char_id, transaction)?
        .iter()
        .map(|x| Value::from(*x))
        .collect::<Vec<Value>>();
    if !pet_ids.is_empty() {
        delete_pets(transaction, char_id, Rc::new(pet_ids))?;
    }

    // Delete ability sets
    let mut stmt = transaction.prepare_cached(
        "
        DELETE
        FROM    ability_set
        WHERE   entity_id = ?1",
    )?;

    stmt.execute([&char_id.0])?;
    drop(stmt);

    // Delete character
    let mut stmt = transaction.prepare_cached(
        "
        DELETE
        FROM    character
        WHERE   character_id = ?1",
    )?;

    stmt.execute([&char_id.0])?;
    drop(stmt);

    // Delete body
    let mut stmt = transaction.prepare_cached(
        "
        DELETE
        FROM    body
        WHERE   body_id = ?1",
    )?;

    stmt.execute([&char_id.0])?;
    drop(stmt);

    // Delete all items, recursively walking all containers starting from the
    // "character" pseudo-container that is the root for all items owned by
    // a character.
    let mut stmt = transaction.prepare_cached(
        "
        WITH RECURSIVE
        parents AS (
            SELECT  item_id
            FROM    item
            WHERE   item.item_id = ?1 -- Item with character id is the character pseudo-container
            UNION ALL
            SELECT  item.item_id
            FROM    item,
                    parents
            WHERE   item.parent_container_item_id = parents.item_id
        )
        DELETE
        FROM    item
        WHERE   EXISTS (SELECT 1 FROM parents WHERE parents.item_id = item.item_id)",
    )?;

    let deleted_item_count = stmt.execute([&char_id.0])?;
    drop(stmt);

    if deleted_item_count < 3 {
        return Err(PersistenceError::OtherError(format!(
            "Error deleting from item table for char_id {} (expected at least 3 deletions, found \
             {})",
            char_id.0, deleted_item_count
        )));
    }

    Ok(())
}

/// Before creating a character, we ensure that the limit on the number of
/// characters has not been exceeded
pub fn check_character_limit(
    uuid: &str,
    transaction: &mut Transaction,
) -> Result<(), PersistenceError> {
    let mut stmt = transaction.prepare_cached(
        "
        SELECT  COUNT(1)
        FROM    character
        WHERE   player_uuid = ?1",
    )?;

    #[allow(clippy::needless_question_mark)]
    let character_count: i64 = stmt.query_row([&uuid], |row| Ok(row.get(0)?))?;
    drop(stmt);

    if character_count < MAX_CHARACTERS_PER_PLAYER as i64 {
        Ok(())
    } else {
        Err(PersistenceError::CharacterLimitReached)
    }
}

/// NOTE: This relies heavily on serializability to work correctly.
///
/// The count function takes the starting entity id, and returns the desired
/// count of new entity IDs.
///
/// These are then inserted into the entities table.
fn get_new_entity_ids(
    transaction: &mut Transaction,
    mut max: impl FnMut(i64) -> i64,
) -> Result<Range<EntityId>, PersistenceError> {
    // The sqlite_sequence table is used here to avoid reusing entity IDs for
    // deleted entities. This table always contains the highest used ID for
    // each AUTOINCREMENT column in a SQLite database.
    let mut stmt = transaction.prepare_cached(
        "
        SELECT  seq + 1 AS entity_id
        FROM    sqlite_sequence
        WHERE   name = 'entity'",
    )?;

    #[allow(clippy::needless_question_mark)]
    let next_entity_id = stmt.query_row([], |row| Ok(row.get(0)?))?;
    let max_entity_id = max(next_entity_id);

    // Create a new range of IDs and insert them into the entity table
    let new_ids: Range<EntityId> = next_entity_id..max_entity_id;

    let mut stmt = transaction.prepare_cached("INSERT INTO entity (entity_id) VALUES (?1)")?;

    // SQLite has no bulk insert
    for i in new_ids.clone() {
        stmt.execute([i])?;
    }

    trace!(
        "Created {} new persistence entity_ids: {}",
        new_ids.end - new_ids.start,
        new_ids
            .clone()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    );
    Ok(new_ids)
}

/// Fetches the pseudo_container IDs for a character
fn get_pseudo_containers(
    connection: &Connection,
    character_id: CharacterId,
) -> Result<CharacterContainers, PersistenceError> {
    let character_containers = CharacterContainers {
        loadout_container_id: get_pseudo_container_id(
            connection,
            character_id,
            LOADOUT_PSEUDO_CONTAINER_POSITION,
        )?,
        inventory_container_id: get_pseudo_container_id(
            connection,
            character_id,
            INVENTORY_PSEUDO_CONTAINER_POSITION,
        )?,
        overflow_items_container_id: get_pseudo_container_id(
            connection,
            character_id,
            OVERFLOW_ITEMS_PSEUDO_CONTAINER_POSITION,
        )?,
    };

    Ok(character_containers)
}

fn get_pseudo_container_id(
    connection: &Connection,
    character_id: CharacterId,
    pseudo_container_position: &str,
) -> Result<EntityId, PersistenceError> {
    let mut stmt = connection.prepare_cached(
        "
        SELECT  item_id
        FROM    item
        WHERE   parent_container_item_id = ?1
        AND     position = ?2",
    )?;

    #[allow(clippy::needless_question_mark)]
    let res = stmt.query_row(
        [
            character_id.0.to_string(),
            pseudo_container_position.to_string(),
        ],
        |row| Ok(row.get(0)?),
    );

    match res {
        Ok(id) => Ok(id),
        Err(e) => {
            error!(
                ?e,
                ?character_id,
                ?pseudo_container_position,
                "Failed to retrieve pseudo container ID"
            );
            Err(DatabaseError(e))
        },
    }
}

/// Stores new pets in the database, and removes pets from the database that the
/// player no longer has. Currently there are no actual updates to pet data
/// since we don't store any updatable data about pets in the database.
fn update_pets(
    char_id: CharacterId,
    pets: Vec<PetPersistenceData>,
    transaction: &mut Transaction,
) -> Result<(), PersistenceError> {
    debug!("Updating {} pets for character {}", pets.len(), char_id.0);

    let db_pets = get_pet_ids(char_id, transaction)?;
    if !db_pets.is_empty() {
        let dead_pet_ids = Rc::new(
            db_pets
                .iter()
                .filter(|pet_id| {
                    !pets.iter().any(|(pet, _, _)| {
                        pet.get_database_id()
                            .load()
                            .map_or(false, |x| x.get() == **pet_id as u64)
                    })
                })
                .map(|x| Value::from(*x))
                .collect::<Vec<Value>>(),
        );

        if !dead_pet_ids.is_empty() {
            delete_pets(transaction, char_id, dead_pet_ids)?;
        }
    }

    for (pet, body, stats) in pets
        .iter()
        .filter(|(pet, _, _)| pet.get_database_id().load().is_none())
    {
        let pet_entity_id = get_new_entity_ids(transaction, |next_id| next_id + 1)?.start;

        let (body_variant, body_json) = convert_body_to_database_json(body)?;

        #[rustfmt::skip]
        let mut stmt = transaction.prepare_cached("
            INSERT
            INTO    body (
                    body_id,
                    variant,
                    body_data)
            VALUES  (?1, ?2, ?3)"
        )?;

        stmt.execute([
            &pet_entity_id as &dyn ToSql,
            &body_variant.to_string(),
            &body_json,
        ])?;

        #[rustfmt::skip]
        let mut stmt = transaction.prepare_cached("
            INSERT
            INTO    pet (
                    pet_id,
                    character_id,
                    name)
            VALUES  (?1, ?2, ?3)",
        )?;

        stmt.execute([&pet_entity_id as &dyn ToSql, &char_id.0, &stats.name])?;
        drop(stmt);

        pet.get_database_id()
            .store(NonZeroU64::new(pet_entity_id as u64));
    }

    Ok(())
}

fn get_pet_ids(
    char_id: CharacterId,
    transaction: &mut Transaction,
) -> Result<Vec<i64>, PersistenceError> {
    #[rustfmt::skip]
        let mut stmt = transaction.prepare_cached("
        SELECT  pet_id
        FROM    pet
        WHERE   character_id = ?1
    ")?;

    #[allow(clippy::needless_question_mark)]
    let db_pets = stmt
        .query_map([&char_id.0], |row| Ok(row.get(0)?))?
        .map(|x| x.unwrap())
        .collect::<Vec<i64>>();
    drop(stmt);
    Ok(db_pets)
}

fn delete_pets(
    transaction: &mut Transaction,
    char_id: CharacterId,
    pet_ids: Rc<Vec<Value>>,
) -> Result<(), PersistenceError> {
    #[rustfmt::skip]
    let mut stmt = transaction.prepare_cached("
            DELETE
            FROM    pet
            WHERE   pet_id IN rarray(?1)"
    )?;

    let delete_count = stmt.execute([&pet_ids])?;
    drop(stmt);
    debug!(
        "Deleted {} pets for character id {}",
        delete_count, char_id.0
    );

    #[rustfmt::skip]
    let mut stmt = transaction.prepare_cached("
            DELETE
            FROM    body
            WHERE   body_id IN rarray(?1)"
    )?;

    let delete_count = stmt.execute([&pet_ids])?;
    debug!(
        "Deleted {} pet bodies for character id {}",
        delete_count, char_id.0
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    char_id: CharacterId,
    char_skill_set: comp::SkillSet,
    inventory: Inventory,
    pets: Vec<PetPersistenceData>,
    char_waypoint: Option<comp::Waypoint>,
    active_abilities: comp::ability::ActiveAbilities,
    map_marker: Option<comp::MapMarker>,
    transaction: &mut Transaction,
) -> Result<(), PersistenceError> {
    // Run pet persistence
    update_pets(char_id, pets, transaction)?;

    let pseudo_containers = get_pseudo_containers(transaction, char_id)?;
    let mut upserts = Vec::new();
    // First, get all the entity IDs for any new items, and identify which
    // slots to upsert and which ones to delete.
    get_new_entity_ids(transaction, |mut next_id| {
        let upserts_ = convert_items_to_database_items(
            pseudo_containers.loadout_container_id,
            &inventory,
            pseudo_containers.inventory_container_id,
            pseudo_containers.overflow_items_container_id,
            &mut next_id,
        );
        upserts = upserts_;
        next_id
    })?;

    // Next, delete any slots we aren't upserting.
    trace!("Deleting items for character_id {}", char_id.0);
    let mut existing_item_ids: Vec<_> = vec![
        Value::from(pseudo_containers.inventory_container_id),
        Value::from(pseudo_containers.loadout_container_id),
        Value::from(pseudo_containers.overflow_items_container_id),
    ];
    for it in load_items(transaction, pseudo_containers.inventory_container_id)? {
        existing_item_ids.push(Value::from(it.item_id));
    }
    for it in load_items(transaction, pseudo_containers.loadout_container_id)? {
        existing_item_ids.push(Value::from(it.item_id));
    }
    for it in load_items(transaction, pseudo_containers.overflow_items_container_id)? {
        existing_item_ids.push(Value::from(it.item_id));
    }

    let non_upserted_items = upserts
        .iter()
        .map(|item_pair| Value::from(item_pair.model.item_id))
        .collect::<Vec<Value>>();

    let mut stmt = transaction.prepare_cached(
        "
        DELETE
        FROM    item
        WHERE   parent_container_item_id
        IN      rarray(?1)
        AND     item_id NOT IN rarray(?2)",
    )?;
    let delete_count = stmt.execute([Rc::new(existing_item_ids), Rc::new(non_upserted_items)])?;
    trace!("Deleted {} items", delete_count);

    // Upsert items
    let expected_upsert_count = upserts.len();
    if expected_upsert_count > 0 {
        let (upserted_items, _): (Vec<_>, Vec<_>) = upserts
            .into_iter()
            .map(|model_pair| {
                debug_assert_eq!(
                    model_pair.model.item_id,
                    model_pair.comp.load().unwrap().get() as i64
                );
                (model_pair.model, model_pair.comp)
            })
            .unzip();
        trace!(
            "Upserting items {:?} for character_id {}",
            upserted_items,
            char_id.0
        );

        // When moving inventory items around, foreign key constraints on
        // `parent_container_item_id` can be temporarily violated by one
        // upsert, but restored by another upsert. Deferred constraints
        // allow SQLite to check this when committing the transaction.
        // The `defer_foreign_keys` pragma treats the foreign key
        // constraints as deferred for the next transaction (it turns itself
        // off at the commit boundary). https://sqlite.org/foreignkeys.html#fk_deferred
        transaction.pragma_update(None, "defer_foreign_keys", "ON")?;

        let mut stmt = transaction.prepare_cached(
            "
            REPLACE
            INTO    item (item_id,
                          parent_container_item_id,
                          item_definition_id,
                          stack_size,
                          position,
                          properties)
            VALUES  (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;

        for item in upserted_items.iter() {
            stmt.execute([
                &item.item_id as &dyn ToSql,
                &item.parent_container_item_id,
                &item.item_definition_id,
                &item.stack_size,
                &item.position,
                &item.properties,
            ])?;
        }
    }

    let db_skill_groups = convert_skill_groups_to_database(char_id, char_skill_set.skill_groups());

    let mut stmt = transaction.prepare_cached(
        "
        REPLACE
        INTO    skill_group (entity_id,
                             skill_group_kind,
                             earned_exp,
                             spent_exp,
                             skills,
                             hash_val)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;

    for skill_group in db_skill_groups {
        stmt.execute([
            &skill_group.entity_id as &dyn ToSql,
            &skill_group.skill_group_kind,
            &skill_group.earned_exp,
            &skill_group.spent_exp,
            &skill_group.skills,
            &skill_group.hash_val,
        ])?;
    }

    let db_waypoint = convert_waypoint_to_database_json(char_waypoint, map_marker);

    let mut stmt = transaction.prepare_cached(
        "
        UPDATE  character
        SET     waypoint = ?1
        WHERE   character_id = ?2
    ",
    )?;

    let waypoint_count = stmt.execute([&db_waypoint as &dyn ToSql, &char_id.0])?;

    if waypoint_count != 1 {
        return Err(PersistenceError::OtherError(format!(
            "Error updating character table for char_id {}",
            char_id.0
        )));
    }

    let ability_sets = convert_active_abilities_to_database(char_id, &active_abilities);

    let mut stmt = transaction.prepare_cached(
        "
        UPDATE  ability_set
        SET     ability_sets = ?1
        WHERE   entity_id = ?2
    ",
    )?;

    let ability_sets_count = stmt.execute([
        &ability_sets.ability_sets as &dyn ToSql,
        &char_id.0 as &dyn ToSql,
    ])?;

    if ability_sets_count != 1 {
        return Err(PersistenceError::OtherError(format!(
            "Error updating ability_set table for char_id {}",
            char_id.0,
        )));
    }

    Ok(())
}
