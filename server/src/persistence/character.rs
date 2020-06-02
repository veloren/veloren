//! Database operations related to characters

extern crate diesel;

use super::{
    error::Error,
    establish_connection,
    models::{
        Body, Character, Inventory, InventoryUpdate, Loadout, LoadoutUpdate, NewCharacter,
        NewLoadout, Stats, StatsJoinData, StatsUpdate,
    },
    schema,
};
use crate::comp;
use common::{
    character::{Character as CharacterData, CharacterItem, MAX_CHARACTERS_PER_PLAYER},
    LoadoutBuilder,
};
use crossbeam::{channel, channel::TryIter};
use diesel::prelude::*;
use tracing::{error, warn};

/// Available database operations when modifying a player's characetr list
enum CharacterListRequestKind {
    CreateCharacter {
        player_uuid: String,
        character_alias: String,
        character_tool: Option<String>,
        body: comp::Body,
    },
    DeleteCharacter {
        player_uuid: String,
        character_id: i32,
    },
    LoadCharacterList {
        player_uuid: String,
    },
}

/// Common format dispatched in response to an update request
#[derive(Debug)]
pub struct CharacterListResponse {
    pub entity: specs::Entity,
    pub result: CharacterListResult,
}

/// A bi-directional messaging resource for making modifications to a player's
/// character list in a background thread.
///
/// This is used exclusively during character selection, and handles loading the
/// player's character list, deleting characters, and creating new characters.
/// Operations not related to the character list (such as saving a character's
/// inventory, stats, etc..) are performed by the [`CharacterUpdater`]
pub struct CharacterListUpdater {
    update_rx: Option<channel::Receiver<CharacterListResponse>>,
    update_tx: Option<channel::Sender<CharacterListRequest>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

type CharacterListRequest = (specs::Entity, CharacterListRequestKind);

impl CharacterListUpdater {
    pub fn new(db_dir: String) -> Self {
        let (update_tx, internal_rx) = channel::unbounded::<CharacterListRequest>();
        let (internal_tx, update_rx) = channel::unbounded::<CharacterListResponse>();

        let handle = std::thread::spawn(move || {
            while let Ok(request) = internal_rx.recv() {
                let (entity, kind) = request;

                if let Err(err) = internal_tx.send(CharacterListResponse {
                    entity,
                    result: match kind {
                        CharacterListRequestKind::CreateCharacter {
                            player_uuid,
                            character_alias,
                            character_tool,
                            body,
                        } => create_character(
                            &player_uuid,
                            character_alias,
                            character_tool,
                            &body,
                            &db_dir,
                        ),
                        CharacterListRequestKind::DeleteCharacter {
                            player_uuid,
                            character_id,
                        } => delete_character(&player_uuid, character_id, &db_dir),
                        CharacterListRequestKind::LoadCharacterList { player_uuid } => {
                            load_character_list(&player_uuid, &db_dir)
                        },
                    },
                }) {
                    log::error!("Could not send persistence request: {:?}", err);
                }
            }
        });

        Self {
            update_tx: Some(update_tx),
            update_rx: Some(update_rx),
            handle: Some(handle),
        }
    }

    /// Create a new character belonging to the player identified by
    /// `player_uuid`.
    pub fn create_character(
        &self,
        entity: specs::Entity,
        player_uuid: String,
        character_alias: String,
        character_tool: Option<String>,
        body: comp::Body,
    ) {
        if let Err(err) = self.update_tx.as_ref().unwrap().send((
            entity,
            CharacterListRequestKind::CreateCharacter {
                player_uuid,
                character_alias,
                character_tool,
                body,
            },
        )) {
            log::error!("Could not send character creation request: {:?}", err);
        }
    }

    /// Delete a character by `id` and `player_uuid`.
    pub fn delete_character(&self, entity: specs::Entity, player_uuid: String, character_id: i32) {
        if let Err(err) = self.update_tx.as_ref().unwrap().send((
            entity,
            CharacterListRequestKind::DeleteCharacter {
                player_uuid,
                character_id,
            },
        )) {
            log::error!("Could not send character deletion request: {:?}", err);
        }
    }

    /// Loads a list of characters belonging to the player identified by
    /// `player_uuid`
    pub fn load_character_list(&self, entity: specs::Entity, player_uuid: String) {
        if let Err(err) = self
            .update_tx
            .as_ref()
            .unwrap()
            .send((entity, CharacterListRequestKind::LoadCharacterList {
                player_uuid,
            }))
        {
            log::error!("Could not send character list load request: {:?}", err);
        }
    }

    /// Returns a non-blocking iterator over CharacterListResponse messages
    pub fn messages(&self) -> TryIter<CharacterListResponse> {
        self.update_rx.as_ref().unwrap().try_iter()
    }
}

impl Drop for CharacterListUpdater {
    fn drop(&mut self) {
        drop(self.update_tx.take());
        if let Err(err) = self.handle.take().unwrap().join() {
            log::error!("Error from joining character update thread: {:?}", err);
        }
    }
}

type CharacterListResult = Result<Vec<CharacterItem>, Error>;

/// Load stored data for a character.
///
/// After first logging in, and after a character is selected, we fetch this
/// data for the purpose of inserting their persisted data for the entity.

pub fn load_character_data(
    character_id: i32,
    db_dir: &str,
) -> Result<(comp::Stats, comp::Inventory, comp::Loadout), Error> {
    let connection = establish_connection(db_dir);

    let (character_data, body_data, stats_data, maybe_inventory, maybe_loadout) =
        schema::character::dsl::character
            .filter(schema::character::id.eq(character_id))
            .inner_join(schema::body::table)
            .inner_join(schema::stats::table)
            .left_join(schema::inventory::table)
            .left_join(schema::loadout::table)
            .first::<(Character, Body, Stats, Option<Inventory>, Option<Loadout>)>(&connection)?;

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
                    warn!(
                        "Failed to create an inventory record for character {}: {}",
                        &character_data.id, error
                    )
                }

                comp::Inventory::default()
            },
            comp::Inventory::from,
        ),
        maybe_loadout.map_or_else(
            || {
                // Create if no record was found
                let default_loadout = LoadoutBuilder::new()
                    .defaults()
                    .active_item(LoadoutBuilder::default_item_config_from_str(
                        character_data.tool.as_deref(),
                    ))
                    .build();

                let row = NewLoadout::from((character_data.id, &default_loadout));

                if let Err(e) = diesel::insert_into(schema::loadout::table)
                    .values(&row)
                    .execute(&connection)
                {
                    let char_id = character_data.id;
                    warn!(
                        ?e,
                        ?char_id,
                        "Failed to create an loadout record for character",
                    )
                }

                default_loadout
            },
            |data| comp::Loadout::from(&data),
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
fn load_character_list(player_uuid: &str, db_dir: &str) -> CharacterListResult {
    let data = schema::character::dsl::character
        .filter(schema::character::player_uuid.eq(player_uuid))
        .order(schema::character::id.desc())
        .inner_join(schema::body::table)
        .inner_join(schema::stats::table)
        .left_join(schema::loadout::table)
        .load::<(Character, Body, Stats, Option<Loadout>)>(&establish_connection(db_dir))?;

    Ok(data
        .iter()
        .map(|(character_data, body_data, stats_data, maybe_loadout)| {
            let character = CharacterData::from(character_data);
            let body = comp::Body::from(body_data);
            let level = stats_data.level as usize;
            let loadout = maybe_loadout.as_ref().map_or_else(
                || {
                    LoadoutBuilder::new()
                        .defaults()
                        .active_item(LoadoutBuilder::default_item_config_from_str(
                            character.tool.as_deref(),
                        ))
                        .build()
                },
                comp::Loadout::from,
            );

            CharacterItem {
                character,
                body,
                level,
                loadout,
            }
        })
        .collect())
}

/// Create a new character with provided comp::Character and comp::Body data.
///
/// Note that sqlite does not support returning the inserted data after a
/// successful insert. To workaround, we wrap this in a transaction which
/// inserts, queries for the newly created chaacter id, then uses the character
/// id for subsequent insertions
fn create_character(
    uuid: &str,
    character_alias: String,
    character_tool: Option<String>,
    body: &comp::Body,
    db_dir: &str,
) -> CharacterListResult {
    check_character_limit(uuid, db_dir)?;

    let connection = establish_connection(db_dir);

    connection.transaction::<_, diesel::result::Error, _>(|| {
        use schema::{body, character, character::dsl::*, inventory, loadout, stats};

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

                // Insert a loadout with defaults and the chosen active weapon
                let loadout = LoadoutBuilder::new()
                    .defaults()
                    .active_item(LoadoutBuilder::default_item_config_from_str(
                        character_tool.as_deref(),
                    ))
                    .build();

                let new_loadout = NewLoadout::from((inserted_character.id, &loadout));

                diesel::insert_into(loadout::table)
                    .values(&new_loadout)
                    .execute(&connection)?;
            },
            _ => warn!("Creating non-humanoid characters is not supported."),
        };

        Ok(())
    })?;

    load_character_list(uuid, db_dir)
}

/// Delete a character. Returns the updated character list.
fn delete_character(uuid: &str, character_id: i32, db_dir: &str) -> CharacterListResult {
    use schema::character::dsl::*;

    diesel::delete(
        character
            .filter(id.eq(character_id))
            .filter(player_uuid.eq(uuid)),
    )
    .execute(&establish_connection(db_dir))?;

    load_character_list(uuid, db_dir)
}

/// Before creating a character, we ensure that the limit on the number of
/// characters has not been exceeded
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

type CharacterUpdateData = (StatsUpdate, InventoryUpdate, LoadoutUpdate);

/// A unidirectional messaging resource for saving characters in a
/// background thread.
///
/// This is used to make updates to a character and their persisted components,
/// such as inventory, loadout, etc...
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

    /// Updates a collection of characters based on their id and components
    pub fn batch_update<'a>(
        &self,
        updates: impl Iterator<Item = (i32, &'a comp::Stats, &'a comp::Inventory, &'a comp::Loadout)>,
    ) {
        let updates = updates
            .map(|(id, stats, inventory, loadout)| {
                (
                    id,
                    (
                        StatsUpdate::from(stats),
                        InventoryUpdate::from(inventory),
                        LoadoutUpdate::from((id, loadout)),
                    ),
                )
            })
            .collect();

        if let Err(e) = self.update_tx.as_ref().unwrap().send(updates) {
            error!(?e, "Could not send stats updates");
        }
    }

    /// Updates a single character based on their id and components
    pub fn update(
        &self,
        character_id: i32,
        stats: &comp::Stats,
        inventory: &comp::Inventory,
        loadout: &comp::Loadout,
    ) {
        self.batch_update(std::iter::once((character_id, stats, inventory, loadout)));
    }
}

fn batch_update(updates: impl Iterator<Item = (i32, CharacterUpdateData)>, db_dir: &str) {
    let connection = establish_connection(db_dir);

    if let Err(e) = connection.transaction::<_, diesel::result::Error, _>(|| {
        updates.for_each(
            |(character_id, (stats_update, inventory_update, loadout_update))| {
                update(
                    character_id,
                    &stats_update,
                    &inventory_update,
                    &loadout_update,
                    &connection,
                )
            },
        );

        Ok(())
    }) {
        error!(?e, "Error during stats batch update transaction");
    }
}

fn update(
    character_id: i32,
    stats: &StatsUpdate,
    inventory: &InventoryUpdate,
    loadout: &LoadoutUpdate,
    connection: &SqliteConnection,
) {
    if let Err(e) =
        diesel::update(schema::stats::table.filter(schema::stats::character_id.eq(character_id)))
            .set(stats)
            .execute(connection)
    {
        warn!(?e, ?character_id, "Failed to update stats for character",)
    }

    if let Err(e) = diesel::update(
        schema::inventory::table.filter(schema::inventory::character_id.eq(character_id)),
    )
    .set(inventory)
    .execute(connection)
    {
        warn!(
            ?e,
            ?character_id,
            "Failed to update inventory for character",
        )
    }

    if let Err(e) = diesel::update(
        schema::loadout::table.filter(schema::loadout::character_id.eq(character_id)),
    )
    .set(loadout)
    .execute(connection)
    {
        warn!(?e, ?character_id, "Failed to update loadout for character",)
    }
}

impl Drop for CharacterUpdater {
    fn drop(&mut self) {
        drop(self.update_tx.take());
        if let Err(e) = self.handle.take().unwrap().join() {
            error!(?e, "Error from joining character update thread");
        }
    }
}
