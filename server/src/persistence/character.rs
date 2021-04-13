//! Database operations related to character data
//!
//! Methods in this module should remain private to the persistence module -
//! database updates and loading are communicated via requests to the
//! [`CharacterLoader`] and [`CharacterUpdater`] while results/responses are
//! polled and handled each server tick.
extern crate rusqlite;

use super::{error::PersistenceError, models::*};
use crate::{
    comp,
    comp::{item::MaterialStatManifest, Inventory},
    persistence::{
        character::conversions::{
            convert_body_from_database, convert_body_to_database_json,
            convert_character_from_database, convert_inventory_from_database_items,
            convert_items_to_database_items, convert_loadout_from_database_items,
            convert_skill_groups_to_database, convert_skills_to_database,
            convert_stats_from_database, convert_waypoint_from_database_json,
            convert_waypoint_to_database_json,
        },
        character_loader::{CharacterCreationResult, CharacterDataResult, CharacterListResult},
        error::PersistenceError::DatabaseError,
        PersistedComponents,
    },
};
use common::character::{CharacterId, CharacterItem, MAX_CHARACTERS_PER_PLAYER};
use core::ops::Range;
use rusqlite::{types::Value, ToSql, Transaction, NO_PARAMS};
use std::{collections::VecDeque, rc::Rc};
use tracing::{error, trace, warn};

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

/// BFS the inventory/loadout to ensure that each is topologically sorted in the
/// sense required by convert_inventory_from_database_items to support recursive
/// items
pub fn load_items_bfs(
    connection: &mut Transaction,
    root: i64,
) -> Result<Vec<Item>, PersistenceError> {
    let mut items = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_front(root);

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        SELECT  item_id,
                parent_container_item_id,
                item_definition_id,
                stack_size,
                position
        FROM    item
        WHERE   parent_container_item_id = ?1")?;

    while let Some(id) = queue.pop_front() {
        let frontier = stmt
            .query_map(&[id], |row| {
                Ok(Item {
                    item_id: row.get(0)?,
                    parent_container_item_id: row.get(1)?,
                    item_definition_id: row.get(2)?,
                    stack_size: row.get(3)?,
                    position: row.get(4)?,
                })
            })?
            .filter_map(Result::ok)
            .collect::<Vec<Item>>();

        for i in frontier.iter() {
            queue.push_back(i.item_id);
        }
        items.extend(frontier);
    }
    Ok(items)
}

/// Load stored data for a character.
///
/// After first logging in, and after a character is selected, we fetch this
/// data for the purpose of inserting their persisted data for the entity.
pub fn load_character_data(
    requesting_player_uuid: String,
    char_id: CharacterId,
    connection: &mut Transaction,
    msm: &MaterialStatManifest,
) -> CharacterDataResult {
    let character_containers = get_pseudo_containers(connection, char_id)?;
    let inventory_items = load_items_bfs(connection, character_containers.inventory_container_id)?;
    let loadout_items = load_items_bfs(connection, character_containers.loadout_container_id)?;

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
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
        &[requesting_player_uuid.clone(), char_id.to_string()],
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

    let char_waypoint = character_data.waypoint.as_ref().and_then(|x| {
        match convert_waypoint_from_database_json(&x) {
            Ok(w) => Some(w),
            Err(e) => {
                warn!(
                    "Error reading waypoint from database for character ID
    {}, error: {}",
                    char_id, e
                );
                None
            },
        }
    });

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        SELECT  skill,
                level
        FROM    skill
        WHERE   entity_id = ?1",
    )?;

    let skill_data = stmt
        .query_map(&[char_id], |row| {
            Ok(Skill {
                entity_id: char_id,
                skill: row.get(0)?,
                level: row.get(1)?,
            })
        })?
        .filter_map(Result::ok)
        .collect::<Vec<Skill>>();

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        SELECT  skill_group_kind,
                exp,
                available_sp,
                earned_sp
        FROM    skill_group
        WHERE   entity_id = ?1",
    )?;

    let skill_group_data = stmt
        .query_map(&[char_id], |row| {
            Ok(SkillGroup {
                entity_id: char_id,
                skill_group_kind: row.get(0)?,
                exp: row.get(1)?,
                available_sp: row.get(2)?,
                earned_sp: row.get(3)?,
            })
        })?
        .filter_map(Result::ok)
        .collect::<Vec<SkillGroup>>();

    Ok((
        convert_body_from_database(&body_data)?,
        convert_stats_from_database(character_data.alias, &skill_data, &skill_group_data),
        convert_inventory_from_database_items(
            character_containers.inventory_container_id,
            &inventory_items,
            character_containers.loadout_container_id,
            &loadout_items,
            msm,
        )?,
        char_waypoint,
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
    connection: &mut Transaction,
    msm: &MaterialStatManifest,
) -> CharacterListResult {
    let characters;
    {
        #[rustfmt::skip]
        let mut stmt = connection
            .prepare_cached("
                SELECT  character_id,
                        alias 
                FROM    character 
                WHERE   player_uuid = ?1
                ORDER BY character_id")?;

        characters = stmt
            .query_map(&[player_uuid_], |row| {
                Ok(Character {
                    character_id: row.get(0)?,
                    alias: row.get(1)?,
                    player_uuid: player_uuid_.to_owned(),
                    waypoint: None, // Not used for character select
                })
            })?
            .map(|x| x.unwrap())
            .collect::<Vec<Character>>();
    }
    characters
        .iter()
        .map(|character_data| {
            let char = convert_character_from_database(&character_data);

            let db_body;

            {
                #[rustfmt::skip]
                let mut stmt = connection
                    .prepare_cached("\
                        SELECT  body_id,\
                                variant,\
                                body_data \
                        FROM    body \
                        WHERE   body_id = ?1")?;
                db_body = stmt.query_row(&[char.id], |row| {
                    Ok(Body {
                        body_id: row.get(0)?,
                        variant: row.get(1)?,
                        body_data: row.get(2)?,
                    })
                })?;
            }

            let char_body = convert_body_from_database(&db_body)?;

            let loadout_container_id = get_pseudo_container_id(
                connection,
                character_data.character_id,
                LOADOUT_PSEUDO_CONTAINER_POSITION,
            )?;

            let loadout_items = load_items_bfs(connection, loadout_container_id)?;

            let loadout =
                convert_loadout_from_database_items(loadout_container_id, &loadout_items, msm)?;

            Ok(CharacterItem {
                character: char,
                body: char_body,
                inventory: Inventory::new_with_loadout(loadout),
            })
        })
        .collect()
}

pub fn create_character(
    uuid: &str,
    character_alias: &str,
    persisted_components: PersistedComponents,
    connection: &mut Transaction,
    msm: &MaterialStatManifest,
) -> CharacterCreationResult {
    check_character_limit(uuid, connection)?;

    let (body, stats, inventory, waypoint) = persisted_components;

    // Fetch new entity IDs for character, inventory and loadout
    let mut new_entity_ids = get_new_entity_ids(connection, |next_id| next_id + 3)?;

    // Create pseudo-container items for character
    let character_id = new_entity_ids.next().unwrap();
    let inventory_container_id = new_entity_ids.next().unwrap();
    let loadout_container_id = new_entity_ids.next().unwrap();

    let pseudo_containers = vec![
        Item {
            stack_size: 1,
            item_id: character_id,
            parent_container_item_id: WORLD_PSEUDO_CONTAINER_ID,
            item_definition_id: CHARACTER_PSEUDO_CONTAINER_DEF_ID.to_owned(),
            position: character_id.to_string(),
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

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        INSERT INTO item (item_id,
                          parent_container_item_id,
                          item_definition_id,
                          stack_size,
                          position)
        VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    for pseudo_container in pseudo_containers {
        stmt.execute(&[
            &pseudo_container.item_id as &dyn ToSql,
            &pseudo_container.parent_container_item_id,
            &pseudo_container.item_definition_id,
            &pseudo_container.stack_size,
            &pseudo_container.position,
        ])?;
    }

    drop(stmt);

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        INSERT INTO body (body_id,
                          variant,
                          body_data)
        VALUES (?1, ?2, ?3)")?;

    stmt.execute(&[
        &character_id as &dyn ToSql,
        &"humanoid".to_string(),
        &convert_body_to_database_json(&body)?,
    ])?;
    drop(stmt);

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        INSERT INTO character (character_id,
                               player_uuid,
                               alias,
                               waypoint)
        VALUES (?1, ?2, ?3, ?4)")?;

    stmt.execute(&[
        &character_id as &dyn ToSql,
        &uuid,
        &character_alias,
        &convert_waypoint_to_database_json(waypoint),
    ])?;
    drop(stmt);

    let skill_set = stats.skill_set;

    let db_skill_groups = convert_skill_groups_to_database(character_id, skill_set.skill_groups);

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        INSERT INTO skill_group (entity_id,
                                 skill_group_kind,
                                 exp,
                                 available_sp,
                                 earned_sp)
        VALUES (?1, ?2, ?3, ?4, ?5)")?;

    for skill_group in db_skill_groups {
        stmt.execute(&[
            &character_id as &dyn ToSql,
            &skill_group.skill_group_kind,
            &skill_group.exp,
            &skill_group.available_sp,
            &skill_group.earned_sp,
        ])?;
    }
    drop(stmt);

    // Insert default inventory and loadout item records
    let mut inserts = Vec::new();

    get_new_entity_ids(connection, |mut next_id| {
        let inserts_ = convert_items_to_database_items(
            loadout_container_id,
            &inventory,
            inventory_container_id,
            &mut next_id,
        );
        inserts = inserts_;
        next_id
    })?;

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        INSERT INTO item (item_id,
                          parent_container_item_id,
                          item_definition_id,
                          stack_size,
                          position)
        VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    for item in inserts {
        stmt.execute(&[
            &item.model.item_id as &dyn ToSql,
            &item.model.parent_container_item_id,
            &item.model.item_definition_id,
            &item.model.stack_size,
            &item.model.position,
        ])?;
    }
    drop(stmt);

    load_character_list(uuid, connection, msm).map(|list| (character_id, list))
}

/// Delete a character. Returns the updated character list.
pub fn delete_character(
    requesting_player_uuid: &str,
    char_id: CharacterId,
    connection: &mut Transaction,
    msm: &MaterialStatManifest,
) -> CharacterListResult {
    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        SELECT  COUNT(1)
        FROM    character
        WHERE   character_id = ?1
        AND     player_uuid = ?2")?;

    let result = stmt.query_row(&[&char_id as &dyn ToSql, &requesting_player_uuid], |row| {
        let y: i64 = row.get(0)?;
        Ok(y)
    })?;
    drop(stmt);

    if result != 1 {
        return Err(PersistenceError::OtherError(
            "Requested character to delete does not belong to the requesting player".to_string(),
        ));
    }
    // Delete skills
    let mut stmt = connection.prepare_cached(
        "
        DELETE
        FROM    skill
        WHERE   entity_id = ?1",
    )?;

    stmt.execute(&[&char_id])?;
    drop(stmt);

    // Delete skill groups
    let mut stmt = connection.prepare_cached(
        "
        DELETE
        FROM    skill_group
        WHERE   entity_id = ?1",
    )?;

    stmt.execute(&[&char_id])?;
    drop(stmt);

    // Delete character
    let mut stmt = connection.prepare_cached(
        "
        DELETE
        FROM    character
        WHERE   character_id = ?1",
    )?;

    stmt.execute(&[&char_id])?;
    drop(stmt);

    // Delete body
    let mut stmt = connection.prepare_cached(
        "
        DELETE
        FROM    body
        WHERE   body_id = ?1",
    )?;

    stmt.execute(&[&char_id])?;
    drop(stmt);

    // Delete all items, recursively walking all containers starting from the
    // "character" pseudo-container that is the root for all items owned by
    // a character.
    let mut stmt = connection.prepare_cached(
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

    let deleted_item_count = stmt.execute(&[&char_id])?;
    drop(stmt);

    if deleted_item_count < 3 {
        return Err(PersistenceError::OtherError(format!(
            "Error deleting from item table for char_id {} (expected at least 3 deletions, found \
             {})",
            char_id, deleted_item_count
        )));
    }

    load_character_list(requesting_player_uuid, connection, msm)
}

/// Before creating a character, we ensure that the limit on the number of
/// characters has not been exceeded
pub fn check_character_limit(
    uuid: &str,
    connection: &mut Transaction,
) -> Result<(), PersistenceError> {
    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        SELECT  COUNT(1)
        FROM    character
        WHERE   player_uuid = ?1")?;

    #[allow(clippy::needless_question_mark)]
    let character_count: i64 = stmt.query_row(&[&uuid], |row| Ok(row.get(0)?))?;
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
    conn: &mut Transaction,
    mut max: impl FnMut(i64) -> i64,
) -> Result<Range<EntityId>, PersistenceError> {
    // The sqlite_sequence table is used here to avoid reusing entity IDs for
    // deleted entities. This table always contains the highest used ID for
    // each AUTOINCREMENT column in a SQLite database.
    #[rustfmt::skip]
    let mut stmt = conn.prepare_cached(
        "
        SELECT  seq + 1 AS entity_id
        FROM    sqlite_sequence
        WHERE   name = 'entity'",
    )?;

    #[allow(clippy::needless_question_mark)]
    let next_entity_id = stmt.query_row(NO_PARAMS, |row| Ok(row.get(0)?))?;
    let max_entity_id = max(next_entity_id);

    // Create a new range of IDs and insert them into the entity table
    let new_ids: Range<EntityId> = next_entity_id..max_entity_id;

    let mut stmt = conn.prepare_cached("INSERT INTO entity (entity_id) VALUES (?1)")?;

    // TODO: bulk insert? rarray doesn't seem to work in VALUES clause
    for i in new_ids.clone() {
        stmt.execute(&[i])?;
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
    connection: &mut Transaction,
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
    };

    Ok(character_containers)
}

fn get_pseudo_container_id(
    connection: &mut Transaction,
    character_id: CharacterId,
    pseudo_container_position: &str,
) -> Result<EntityId, PersistenceError> {
    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("\
        SELECT  item_id
        FROM    item
        WHERE   parent_container_item_id = ?1
        AND     position = ?2",
    )?;

    #[allow(clippy::needless_question_mark)]
    let res = stmt.query_row(
        &[
            character_id.to_string(),
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

pub fn update(
    char_id: CharacterId,
    char_stats: comp::Stats,
    inventory: comp::Inventory,
    char_waypoint: Option<comp::Waypoint>,
    connection: &mut Transaction,
) -> Result<(), PersistenceError> {
    let pseudo_containers = get_pseudo_containers(connection, char_id)?;

    let mut upserts = Vec::new();

    // First, get all the entity IDs for any new items, and identify which
    // slots to upsert and which ones to delete.
    get_new_entity_ids(connection, |mut next_id| {
        let upserts_ = convert_items_to_database_items(
            pseudo_containers.loadout_container_id,
            &inventory,
            pseudo_containers.inventory_container_id,
            &mut next_id,
        );
        upserts = upserts_;
        next_id
    })?;

    // Next, delete any slots we aren't upserting.
    trace!("Deleting items for character_id {}", char_id);
    let mut existing_item_ids: Vec<_> = vec![
        Value::from(pseudo_containers.inventory_container_id),
        Value::from(pseudo_containers.loadout_container_id),
    ];
    for it in load_items_bfs(connection, pseudo_containers.inventory_container_id)? {
        existing_item_ids.push(Value::from(it.item_id));
    }
    for it in load_items_bfs(connection, pseudo_containers.loadout_container_id)? {
        existing_item_ids.push(Value::from(it.item_id));
    }

    let non_upserted_items = upserts
        .iter()
        .map(|item_pair| Value::from(item_pair.model.item_id))
        .collect::<Vec<Value>>();

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        DELETE
        FROM    item
        WHERE   parent_container_item_id
        IN      rarray(?1)
        AND     item_id NOT IN rarray(?2)")?;
    let delete_count = stmt.execute(&[Rc::new(existing_item_ids), Rc::new(non_upserted_items)])?;
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
            char_id
        );

        // When moving inventory items around, foreign key constraints on
        // `parent_container_item_id` can be temporarily violated by one
        // upsert, but restored by another upsert. Deferred constraints
        // allow SQLite to check this when committing the transaction.
        // The `defer_foreign_keys` pragma treats the foreign key
        // constraints as deferred for the next transaction (it turns itself
        // off at the commit boundary). https://sqlite.org/foreignkeys.html#fk_deferred
        connection.pragma_update(None, "defer_foreign_keys", &"ON".to_string())?;

        #[rustfmt::skip]
        let mut stmt =  connection.prepare_cached("
            REPLACE
            INTO    item (item_id,
                          parent_container_item_id,
                          item_definition_id,
                          stack_size,
                          position)
            VALUES  (?1, ?2, ?3, ?4, ?5)")?;

        for item in upserted_items.iter() {
            stmt.execute(&[
                &item.item_id as &dyn ToSql,
                &item.parent_container_item_id,
                &item.item_definition_id,
                &item.stack_size,
                &item.position,
            ])?;
        }
    }

    let char_skill_set = char_stats.skill_set;

    let db_skill_groups = convert_skill_groups_to_database(char_id, char_skill_set.skill_groups);

    #[rustfmt::skip]
    let mut stmt =  connection.prepare_cached("
        REPLACE
        INTO    skill_group (entity_id,
                             skill_group_kind,
                             exp,
                             available_sp,
                             earned_sp)
        VALUES (?1, ?2, ?3, ?4, ?5)")?;

    for skill_group in db_skill_groups {
        stmt.execute(&[
            &skill_group.entity_id as &dyn ToSql,
            &skill_group.skill_group_kind,
            &skill_group.exp,
            &skill_group.available_sp,
            &skill_group.earned_sp,
        ])?;
    }

    let db_skills = convert_skills_to_database(char_id, char_skill_set.skills);

    let known_skills = Rc::new(
        db_skills
            .iter()
            .map(|x| Value::from(x.skill.clone()))
            .collect::<Vec<Value>>(),
    );

    #[rustfmt::skip]
    let mut stmt = connection.prepare_cached("
        DELETE
        FROM    skill
        WHERE   entity_id = ?1
        AND     skill NOT IN rarray(?2)")?;

    let delete_count = stmt.execute(&[&char_id as &dyn ToSql, &known_skills])?;
    trace!("Deleted {} skills", delete_count);

    #[rustfmt::skip]
    let mut stmt =  connection.prepare_cached("
        REPLACE 
        INTO    skill (entity_id,
                       skill,
                       level)
        VALUES (?1, ?2, ?3)")?;

    for skill in db_skills {
        stmt.execute(&[&skill.entity_id as &dyn ToSql, &skill.skill, &skill.level])?;
    }

    let db_waypoint = convert_waypoint_to_database_json(char_waypoint);

    #[rustfmt::skip]
    let mut stmt =  connection.prepare_cached("
        UPDATE  character
        SET     waypoint = ?1
        WHERE   character_id = ?2
    ")?;

    let waypoint_count = stmt.execute(&[&db_waypoint as &dyn ToSql, &char_id])?;

    if waypoint_count != 1 {
        return Err(PersistenceError::OtherError(format!(
            "Error updating character table for char_id {}",
            char_id
        )));
    }

    Ok(())
}
