extern crate diesel;

use super::{
    error::Error,
    establish_connection,
    models::{
        Body, Character, Inventory, InventoryUpdate, NewCharacter, Stats, StatsJoinData,
        StatsUpdate,
    },
    schema,
};
use crate::comp;
use common::character::{Character as CharacterData, CharacterItem, MAX_CHARACTERS_PER_PLAYER};
use crossbeam::channel;
use diesel::prelude::*;

type CharacterListResult = Result<Vec<CharacterItem>, Error>;

/// Load stored data for a character.
///
/// After first logging in, and after a character is selected, we fetch this
/// data for the purpose of inserting their persisted data for the entity.
pub fn load_character_data(
    character_id: i32,
    db_dir: &str,
) -> Result<(comp::Stats, comp::Inventory), Error> {
    let connection = establish_connection(db_dir);

    let (character_data, body_data, stats_data, maybe_inventory) =
        schema::character::dsl::character
            .filter(schema::character::id.eq(character_id))
            .inner_join(schema::body::table)
            .inner_join(schema::stats::table)
            .left_join(schema::inventory::table)
            .first::<(Character, Body, Stats, Option<Inventory>)>(&connection)?;

    Ok((
        comp::Stats::from(StatsJoinData {
            alias: &character_data.alias,
            body: &comp::Body::from(&body_data),
            stats: &stats_data,
        }),
        maybe_inventory.map_or_else(
            || {
                // If no inventory record was found for the character, create it now
                let row = Inventory::from((character_data.id, comp::Inventory::default()));

                if let Err(error) = diesel::insert_into(schema::inventory::table)
                    .values(&row)
                    .execute(&connection)
                {
                    log::warn!(
                        "Failed to create an inventory record for character {}: {}",
                        &character_data.id,
                        error
                    )
                }

                comp::Inventory::default()
            },
            |inv| comp::Inventory::from(inv),
        ),
    ))
}

/// Loads a list of characters belonging to the player. This data is a small
/// subset of the character's data, and is used to render the character and
/// their level in the character list.
///
/// In the event that a join fails, for a character (i.e. they lack an entry for
/// stats, body, etc...) the character is skipped, and no entry will be
/// returned.
pub fn load_character_list(player_uuid: &str, db_dir: &str) -> CharacterListResult {
    let data: Vec<(Character, Body, Stats)> = schema::character::dsl::character
        .filter(schema::character::player_uuid.eq(player_uuid))
        .order(schema::character::id.desc())
        .inner_join(schema::body::table)
        .inner_join(schema::stats::table)
        .load::<(Character, Body, Stats)>(&establish_connection(db_dir))?;

    Ok(data
        .iter()
        .map(|(character_data, body_data, stats_data)| {
            let character = CharacterData::from(character_data);
            let body = comp::Body::from(body_data);
            let level = stats_data.level as usize;

            CharacterItem {
                character,
                body,
                level,
            }
        })
        .collect())
}

/// Create a new character with provided comp::Character and comp::Body data.
///
/// Note that sqlite does not support returning the inserted data after a
/// successful insert. To workaround, we wrap this in a transaction which
/// inserts, queries for the newly created chaacter id, then uses the character
/// id for insertion of the `body` table entry
pub fn create_character(
    uuid: &str,
    character_alias: String,
    character_tool: Option<String>,
    body: &comp::Body,
    db_dir: &str,
) -> CharacterListResult {
    check_character_limit(uuid, db_dir)?;

    let connection = establish_connection(db_dir);

    connection.transaction::<_, diesel::result::Error, _>(|| {
        use schema::{body, character, character::dsl::*, inventory, stats};

        match body {
            comp::Body::Humanoid(body_data) => {
                let new_character = NewCharacter {
                    player_uuid: uuid,
                    alias: &character_alias,
                    tool: character_tool.as_deref(),
                };

                diesel::insert_into(character::table)
                    .values(&new_character)
                    .execute(&connection)?;

                let inserted_character = character
                    .filter(player_uuid.eq(uuid))
                    .order(id.desc())
                    .first::<Character>(&connection)?;

                let new_body = Body {
                    character_id: inserted_character.id as i32,
                    species: body_data.species as i16,
                    body_type: body_data.body_type as i16,
                    hair_style: body_data.hair_style as i16,
                    beard: body_data.beard as i16,
                    eyes: body_data.eyes as i16,
                    accessory: body_data.accessory as i16,
                    hair_color: body_data.hair_color as i16,
                    skin: body_data.skin as i16,
                    eye_color: body_data.eye_color as i16,
                };

                diesel::insert_into(body::table)
                    .values(&new_body)
                    .execute(&connection)?;

                let default_stats = comp::Stats::new(String::from(new_character.alias), *body);

                // Insert some default stats
                let new_stats = Stats {
                    character_id: inserted_character.id as i32,
                    level: default_stats.level.level() as i32,
                    exp: default_stats.exp.current() as i32,
                    endurance: default_stats.endurance as i32,
                    fitness: default_stats.fitness as i32,
                    willpower: default_stats.willpower as i32,
                };

                diesel::insert_into(stats::table)
                    .values(&new_stats)
                    .execute(&connection)?;

                // Default inventory
                let inventory =
                    Inventory::from((inserted_character.id, comp::Inventory::default()));

                diesel::insert_into(inventory::table)
                    .values(&inventory)
                    .execute(&connection)?;
            },
            _ => log::warn!("Creating non-humanoid characters is not supported."),
        };

        Ok(())
    })?;

    load_character_list(uuid, db_dir)
}

/// Delete a character. Returns the updated character list.
pub fn delete_character(uuid: &str, character_id: i32, db_dir: &str) -> CharacterListResult {
    use schema::character::dsl::*;

    diesel::delete(
        character
            .filter(id.eq(character_id))
            .filter(player_uuid.eq(uuid)),
    )
    .execute(&establish_connection(db_dir))?;

    load_character_list(uuid, db_dir)
}

fn check_character_limit(uuid: &str, db_dir: &str) -> Result<(), Error> {
    use diesel::dsl::count_star;
    use schema::character::dsl::*;

    let character_count = character
        .select(count_star())
        .filter(player_uuid.eq(uuid))
        .load::<i64>(&establish_connection(db_dir))?;

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

pub type CharacterUpdateData = (StatsUpdate, InventoryUpdate);

pub struct CharacterUpdater {
    update_tx: Option<channel::Sender<Vec<(i32, CharacterUpdateData)>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl CharacterUpdater {
    pub fn new(db_dir: String) -> Self {
        let (update_tx, update_rx) = channel::unbounded::<Vec<(i32, CharacterUpdateData)>>();
        let handle = std::thread::spawn(move || {
            while let Ok(updates) = update_rx.recv() {
                batch_update(updates.into_iter(), &db_dir);
            }
        });

        Self {
            update_tx: Some(update_tx),
            handle: Some(handle),
        }
    }

    pub fn batch_update<'a>(
        &self,
        updates: impl Iterator<Item = (i32, &'a comp::Stats, &'a comp::Inventory)>,
    ) {
        let updates = updates
            .map(|(id, stats, inventory)| {
                (
                    id,
                    (StatsUpdate::from(stats), InventoryUpdate::from(inventory)),
                )
            })
            .collect();

        if let Err(err) = self.update_tx.as_ref().unwrap().send(updates) {
            log::error!("Could not send stats updates: {:?}", err);
        }
    }

    pub fn update(&self, character_id: i32, stats: &comp::Stats, inventory: &comp::Inventory) {
        self.batch_update(std::iter::once((character_id, stats, inventory)));
    }
}

fn batch_update(updates: impl Iterator<Item = (i32, CharacterUpdateData)>, db_dir: &str) {
    let connection = establish_connection(db_dir);

    if let Err(err) = connection.transaction::<_, diesel::result::Error, _>(|| {
        updates.for_each(|(character_id, (stats_update, inventory_update))| {
            update(character_id, &stats_update, &inventory_update, &connection)
        });

        Ok(())
    }) {
        log::error!("Error during stats batch update transaction: {:?}", err);
    }
}

fn update(
    character_id: i32,
    stats: &StatsUpdate,
    inventory: &InventoryUpdate,
    connection: &SqliteConnection,
) {
    if let Err(error) =
        diesel::update(schema::stats::table.filter(schema::stats::character_id.eq(character_id)))
            .set(stats)
            .execute(connection)
    {
        log::warn!(
            "Failed to update stats for character: {:?}: {:?}",
            character_id,
            error
        )
    }

    if let Err(error) = diesel::update(
        schema::inventory::table.filter(schema::inventory::character_id.eq(character_id)),
    )
    .set(inventory)
    .execute(connection)
    {
        log::warn!(
            "Failed to update inventory for character: {:?}: {:?}",
            character_id,
            error
        )
    }
}

impl Drop for CharacterUpdater {
    fn drop(&mut self) {
        drop(self.update_tx.take());
        if let Err(err) = self.handle.take().unwrap().join() {
            log::error!("Error from joining character update thread: {:?}", err);
        }
    }
}
