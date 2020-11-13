//! Database operations related to character data
//!
//! Methods in this module should remain private to the persistence module -
//! database updates and loading are communicated via requests to the
//! [`CharacterLoader`] and [`CharacterUpdater`] while results/responses are
//! polled and handled each server tick.
extern crate diesel;

use super::{error::Error, models::*, schema, VelorenTransaction};
use crate::{
    comp,
    persistence::{
        character::conversions::{
            convert_body_from_database, convert_body_to_database_json,
            convert_character_from_database, convert_inventory_from_database_items,
            convert_items_to_database_items, convert_loadout_from_database_items,
            convert_stats_from_database, convert_stats_to_database,
            convert_waypoint_to_database_json,
        },
        character_loader::{CharacterDataResult, CharacterListResult},
        error::Error::DatabaseError,
        json_models::CharacterPosition,
        PersistedComponents,
    },
};
use common::{
    character::{CharacterId, CharacterItem, MAX_CHARACTERS_PER_PLAYER},
    comp::item::tool::AbilityMap,
    state::Time,
};
use core::ops::Range;
use diesel::{prelude::*, sql_query, sql_types::BigInt};
use std::sync::Arc;
use tracing::{error, trace};

/// Private module for very tightly coupled database conversion methods.  In
/// general, these have many invariants that need to be maintained when they're
/// called--do not assume it's safe to make these public!
mod conversions;

pub(crate) type EntityId = i64;

const CHARACTER_PSEUDO_CONTAINER_DEF_ID: &str = "veloren.core.pseudo_containers.character";
const INVENTORY_PSEUDO_CONTAINER_DEF_ID: &str = "veloren.core.pseudo_containers.inventory";
const LOADOUT_PSEUDO_CONTAINER_DEF_ID: &str = "veloren.core.pseudo_containers.loadout";
const INVENTORY_PSEUDO_CONTAINER_POSITION: &str = "inventory";
const LOADOUT_PSEUDO_CONTAINER_POSITION: &str = "loadout";
const WORLD_PSEUDO_CONTAINER_ID: EntityId = 1;

#[derive(Clone, Copy)]
struct CharacterContainers {
    inventory_container_id: EntityId,
    loadout_container_id: EntityId,
}

/// Load stored data for a character.
///
/// After first logging in, and after a character is selected, we fetch this
/// data for the purpose of inserting their persisted data for the entity.
pub fn load_character_data(
    requesting_player_uuid: String,
    char_id: CharacterId,
    connection: VelorenTransaction,
    map: &AbilityMap,
) -> CharacterDataResult {
    use schema::{body::dsl::*, character::dsl::*, item::dsl::*, stats::dsl::*};

    let character_containers = get_pseudo_containers(connection, char_id)?;

    // TODO: Make inventory and loadout item loading work with recursive items when
    // container items are supported
    let inventory_items = item
        .filter(parent_container_item_id.eq(character_containers.inventory_container_id))
        .load::<Item>(&*connection)?;

    let loadout_items = item
        .filter(parent_container_item_id.eq(character_containers.loadout_container_id))
        .load::<Item>(&*connection)?;

    let (character_data, stats_data) = character
        .filter(
            schema::character::dsl::character_id
                .eq(char_id)
                .and(player_uuid.eq(requesting_player_uuid)),
        )
        .inner_join(stats)
        .first::<(Character, Stats)>(&*connection)?;

    let char_body = body
        .filter(schema::body::dsl::body_id.eq(char_id))
        .first::<Body>(&*connection)?;

    let waypoint = item
        .filter(item_id.eq(char_id))
        .first::<Item>(&*connection)
        .ok()
        .and_then(|it: Item| {
            (serde_json::de::from_str::<CharacterPosition>(it.position.as_str()))
                .ok()
                .map(|charpos| comp::Waypoint::new(charpos.waypoint, Time(0.0)))
        });

    Ok((
        convert_body_from_database(&char_body)?,
        convert_stats_from_database(&stats_data, character_data.alias),
        convert_inventory_from_database_items(&inventory_items)?,
        convert_loadout_from_database_items(&loadout_items, map)?,
        waypoint,
    ))
}

/// Loads a list of characters belonging to the player. This data is a small
/// subset of the character's data, and is used to render the character and
/// their level in the character list.
///
/// In the event that a join fails, for a character (i.e. they lack an entry for
/// stats, body, etc...) the character is skipped, and no entry will be
/// returned.
pub fn load_character_list(
    player_uuid_: &str,
    connection: VelorenTransaction,
    map: &AbilityMap,
) -> CharacterListResult {
    use schema::{body::dsl::*, character::dsl::*, item::dsl::*, stats::dsl::*};

    let result = character
        .filter(player_uuid.eq(player_uuid_))
        .inner_join(stats)
        .order(schema::character::dsl::character_id.desc())
        .load::<(Character, Stats)>(&*connection)?;

    result
        .iter()
        .map(|(character_data, char_stats)| {
            let char = convert_character_from_database(character_data);

            let db_body = body
                .filter(schema::body::dsl::body_id.eq(character_data.character_id))
                .first::<Body>(&*connection)?;

            let char_body = convert_body_from_database(&db_body)?;

            let loadout_container_id = get_pseudo_container_id(
                connection,
                character_data.character_id,
                LOADOUT_PSEUDO_CONTAINER_POSITION,
            )?;

            // TODO: Make work with recursive items if containers are ever supported as part
            // of a loadout
            let loadout_items = item
                .filter(parent_container_item_id.eq(loadout_container_id))
                .load::<Item>(&*connection)?;

            let loadout = convert_loadout_from_database_items(&loadout_items, map)?;

            Ok(CharacterItem {
                character: char,
                body: char_body,
                level: char_stats.level as usize,
                loadout,
            })
        })
        .collect()
}

pub fn create_character(
    uuid: &str,
    character_alias: &str,
    persisted_components: PersistedComponents,
    connection: VelorenTransaction,
    map: &AbilityMap,
) -> CharacterListResult {
    use schema::item::dsl::*;

    check_character_limit(uuid, connection)?;

    use schema::{body, character, stats};

    let (body, stats, inventory, loadout, waypoint) = persisted_components;

    // Fetch new entity IDs for character, inventory and loadout
    let mut new_entity_ids = get_new_entity_ids(connection, |next_id| next_id + 3)?;

    // Create pseudo-container items for character
    let character_id = new_entity_ids.next().unwrap();
    let inventory_container_id = new_entity_ids.next().unwrap();
    let loadout_container_id = new_entity_ids.next().unwrap();
    // by default the character's position is the id in textual form
    let character_position = waypoint
        .and_then(|waypoint| serde_json::to_string(&waypoint.get_pos()).ok())
        .unwrap_or_else(|| character_id.to_string());
    let pseudo_containers = vec![
        Item {
            stack_size: 1,
            item_id: character_id,
            parent_container_item_id: WORLD_PSEUDO_CONTAINER_ID,
            item_definition_id: CHARACTER_PSEUDO_CONTAINER_DEF_ID.to_owned(),
            position: character_position,
        },
        Item {
            stack_size: 1,
            item_id: inventory_container_id,
            parent_container_item_id: character_id,
            item_definition_id: INVENTORY_PSEUDO_CONTAINER_DEF_ID.to_owned(),
            position: INVENTORY_PSEUDO_CONTAINER_POSITION.to_owned(),
        },
        Item {
            stack_size: 1,
            item_id: loadout_container_id,
            parent_container_item_id: character_id,
            item_definition_id: LOADOUT_PSEUDO_CONTAINER_DEF_ID.to_owned(),
            position: LOADOUT_PSEUDO_CONTAINER_POSITION.to_owned(),
        },
    ];
    let pseudo_container_count = diesel::insert_into(item)
        .values(pseudo_containers)
        .execute(&*connection)?;

    if pseudo_container_count != 3 {
        return Err(Error::OtherError(format!(
            "Error inserting initial pseudo containers for character id {} (expected 3, actual {})",
            character_id, pseudo_container_count
        )));
    }

    // Insert stats record
    let db_stats = convert_stats_to_database(character_id, &stats);
    let stats_count = diesel::insert_into(stats::table)
        .values(&db_stats)
        .execute(&*connection)?;

    if stats_count != 1 {
        return Err(Error::OtherError(format!(
            "Error inserting into stats table for char_id {}",
            character_id
        )));
    }

    // Insert body record
    let new_body = Body {
        body_id: character_id,
        body_data: convert_body_to_database_json(&body)?,
        variant: "humanoid".to_string(),
    };

    let body_count = diesel::insert_into(body::table)
        .values(&new_body)
        .execute(&*connection)?;

    if body_count != 1 {
        return Err(Error::OtherError(format!(
            "Error inserting into body table for char_id {}",
            character_id
        )));
    }

    // Insert character record
    let new_character = NewCharacter {
        character_id,
        player_uuid: uuid,
        alias: &character_alias,
    };
    let character_count = diesel::insert_into(character::table)
        .values(&new_character)
        .execute(&*connection)?;

    if character_count != 1 {
        return Err(Error::OtherError(format!(
            "Error inserting into character table for char_id {}",
            character_id
        )));
    }

    // Insert default inventory and loadout item records
    let mut inserts = Vec::new();

    get_new_entity_ids(connection, |mut next_id| {
        let (inserts_, _deletes) = convert_items_to_database_items(
            &loadout,
            loadout_container_id,
            &inventory,
            inventory_container_id,
            &mut next_id,
        );
        inserts = inserts_;
        next_id
    })?;

    let expected_inserted_count = inserts.len();
    let inserted_items = inserts
        .into_iter()
        .map(|item_pair| item_pair.model)
        .collect::<Vec<_>>();
    let inserted_count = diesel::insert_into(item)
        .values(&inserted_items)
        .execute(&*connection)?;

    if expected_inserted_count != inserted_count {
        return Err(Error::OtherError(format!(
            "Expected insertions={}, actual={}, for char_id {}--unsafe to continue transaction.",
            expected_inserted_count, inserted_count, character_id
        )));
    }

    load_character_list(uuid, connection, map)
}

/// Delete a character. Returns the updated character list.
pub fn delete_character(
    requesting_player_uuid: &str,
    char_id: CharacterId,
    connection: VelorenTransaction,
    map: &AbilityMap,
) -> CharacterListResult {
    use schema::{body::dsl::*, character::dsl::*, stats::dsl::*};

    // Load the character to delete - ensures that the requesting player
    // owns the character
    let _character_data = character
        .filter(
            schema::character::dsl::character_id
                .eq(char_id)
                .and(player_uuid.eq(requesting_player_uuid)),
        )
        .first::<Character>(&*connection)?;

    // Delete character
    let character_count = diesel::delete(
        character
            .filter(schema::character::dsl::character_id.eq(char_id))
            .filter(player_uuid.eq(requesting_player_uuid)),
    )
    .execute(&*connection)?;

    if character_count != 1 {
        return Err(Error::OtherError(format!(
            "Error deleting from character table for char_id {}",
            char_id
        )));
    }

    // Delete stats
    let stats_count = diesel::delete(stats.filter(schema::stats::dsl::stats_id.eq(char_id)))
        .execute(&*connection)?;

    if stats_count != 1 {
        return Err(Error::OtherError(format!(
            "Error deleting from stats table for char_id {}",
            char_id
        )));
    }
    // Delete body
    let body_count = diesel::delete(body.filter(schema::body::dsl::body_id.eq(char_id)))
        .execute(&*connection)?;

    if body_count != 1 {
        return Err(Error::OtherError(format!(
            "Error deleting from body table for char_id {}",
            char_id
        )));
    }

    // Delete all items, recursively walking all containers starting from the
    // "character" pseudo-container that is the root for all items owned by
    // a character.
    let item_count = diesel::sql_query(format!(
        "
    WITH RECURSIVE
    parents AS (
        SELECT  item_id
        FROM    item
        WHERE   item.item_id = {} -- Item with character id is the character pseudo-container
        UNION ALL
        SELECT  item.item_id
        FROM    item,
                parents
        WHERE   item.parent_container_item_id = parents.item_id
    )
    DELETE
    FROM    item
    WHERE EXISTS (SELECT 1 FROM parents WHERE parents.item_id = item.item_id)",
        char_id
    ))
    .execute(&*connection)?;

    if item_count < 3 {
        return Err(Error::OtherError(format!(
            "Error deleting from item table for char_id {} (expected at least 3 deletions, found \
             {})",
            char_id, item_count
        )));
    }

    load_character_list(requesting_player_uuid, connection, map)
}

/// Before creating a character, we ensure that the limit on the number of
/// characters has not been exceeded
pub fn check_character_limit(uuid: &str, connection: VelorenTransaction) -> Result<(), Error> {
    use diesel::dsl::count_star;
    use schema::character::dsl::*;

    let character_count = character
        .select(count_star())
        .filter(player_uuid.eq(uuid))
        .load::<i64>(&*connection)?;

    match character_count.first() {
        Some(count) => {
            if count < &(MAX_CHARACTERS_PER_PLAYER as i64) {
                Ok(())
            } else {
                Err(Error::CharacterLimitReached)
            }
        },
        _ => Ok(()),
    }
}

/// NOTE: This relies heavily on serializability to work correctly.
///
/// The count function takes the starting entity id, and returns the desired
/// count of new entity IDs.
///
/// These are then inserted into the entities table.
fn get_new_entity_ids(
    conn: VelorenTransaction,
    mut max: impl FnMut(i64) -> i64,
) -> Result<Range<EntityId>, Error> {
    use super::schema::entity::dsl::*;

    #[derive(QueryableByName)]
    struct NextEntityId {
        #[sql_type = "BigInt"]
        entity_id: i64,
    }

    // The sqlite_sequence table is used here to avoid reusing entity IDs for
    // deleted entities. This table always contains the highest used ID for each
    // AUTOINCREMENT column in a SQLite database.
    let next_entity_id = sql_query(
        "
        SELECT  seq + 1 AS entity_id
        FROM    sqlite_sequence
        WHERE name = 'entity'",
    )
    .load::<NextEntityId>(&*conn)?
    .pop()
    .ok_or_else(|| Error::OtherError("No rows returned for sqlite_sequence query ".to_string()))?
    .entity_id;

    let max_entity_id = max(next_entity_id);

    // Create a new range of IDs and insert them into the entity table
    let new_ids: Range<EntityId> = next_entity_id..max_entity_id;

    let new_entities: Vec<Entity> = new_ids.clone().map(|x| Entity { entity_id: x }).collect();

    let actual_count = diesel::insert_into(entity)
        .values(&new_entities)
        .execute(&*conn)?;

    if actual_count != new_entities.len() {
        return Err(Error::OtherError(format!(
            "Error updating entity table: expected to add the range {:?}) to entities, but actual \
             insertions={}",
            new_ids, actual_count
        )));
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
    connection: VelorenTransaction,
    character_id: CharacterId,
) -> Result<CharacterContainers, Error> {
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
    };

    Ok(character_containers)
}

fn get_pseudo_container_id(
    connection: VelorenTransaction,
    character_id: CharacterId,
    pseudo_container_position: &str,
) -> Result<EntityId, Error> {
    use super::schema::item::dsl::*;
    match item
        .select(item_id)
        .filter(
            parent_container_item_id
                .eq(character_id)
                .and(position.eq(pseudo_container_position)),
        )
        .first::<EntityId>(&*connection)
    {
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

pub fn update(
    char_id: CharacterId,
    char_stats: comp::Stats,
    inventory: comp::Inventory,
    loadout: comp::Loadout,
    waypoint: Option<comp::Waypoint>,
    connection: VelorenTransaction,
) -> Result<Vec<Arc<common::comp::item::ItemId>>, Error> {
    use super::schema::{item::dsl::*, stats::dsl::*};

    let pseudo_containers = get_pseudo_containers(connection, char_id)?;

    let mut upserts = Vec::new();

    // First, get all the entity IDs for any new items, and identify which slots to
    // upsert and which ones to delete.
    get_new_entity_ids(connection, |mut next_id| {
        let (upserts_, _deletes) = convert_items_to_database_items(
            &loadout,
            pseudo_containers.loadout_container_id,
            &inventory,
            pseudo_containers.inventory_container_id,
            &mut next_id,
        );
        upserts = upserts_;
        next_id
    })?;

    if let Some(waypoint) = waypoint {
        match convert_waypoint_to_database_json(&waypoint) {
            Ok(character_position) => {
                diesel::update(item.filter(item_id.eq(char_id)))
                    .set(position.eq(character_position))
                    .execute(&*connection)?;
            },
            Err(err) => {
                return Err(Error::ConversionError(format!(
                    "Error encoding waypoint: {:?}",
                    err
                )));
            },
        }
    }

    // Next, delete any slots we aren't upserting.
    trace!("Deleting items for character_id {}", char_id);
    let existing_items = parent_container_item_id
        .eq(pseudo_containers.inventory_container_id)
        .or(parent_container_item_id.eq(pseudo_containers.loadout_container_id));
    let non_upserted_items = item_id.ne_all(
        upserts
            .iter()
            .map(|item_pair| item_pair.model.item_id)
            .collect::<Vec<_>>(),
    );

    let delete_count = diesel::delete(item.filter(existing_items.and(non_upserted_items)))
        .execute(&*connection)?;
    trace!("Deleted {} items", delete_count);

    // Upsert items
    let expected_upsert_count = upserts.len();
    let mut upserted_comps = Vec::new();
    if expected_upsert_count > 0 {
        let (upserted_items, upserted_comps_): (Vec<_>, Vec<_>) = upserts
            .into_iter()
            .map(|model_pair| (model_pair.model, model_pair.comp))
            .unzip();
        upserted_comps = upserted_comps_;
        trace!(
            "Upserting items {:?} for character_id {}",
            upserted_items,
            char_id
        );

        let upsert_count = diesel::replace_into(item)
            .values(&upserted_items)
            .execute(&*connection)?;
        if upsert_count != expected_upsert_count {
            return Err(Error::OtherError(format!(
                "Expected upsertions={}, actual={}, for char_id {}--unsafe to continue \
                 transaction.",
                expected_upsert_count, upsert_count, char_id
            )));
        }
    }

    let db_stats = convert_stats_to_database(char_id, &char_stats);
    let stats_count = diesel::update(stats.filter(stats_id.eq(char_id)))
        .set(db_stats)
        .execute(&*connection)?;

    if stats_count != 1 {
        return Err(Error::OtherError(format!(
            "Error updating stats table for char_id {}",
            char_id
        )));
    }

    Ok(upserted_comps)
}
