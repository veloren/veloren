//! Database operations related to character data
//!
//! Methods in this module should remain private - database updates and loading
//! are communicated via requests to the [`CharacterLoader`] and
//! [`CharacterUpdater`] while results/responses are polled and handled each
//! server tick.

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
use crate::{comp, persistence::models::SkillSetData};
use common::{
    character::{Character as CharacterData, CharacterItem, MAX_CHARACTERS_PER_PLAYER},
    LoadoutBuilder,
};
use crossbeam::{channel, channel::TryIter};
use diesel::prelude::*;
use tracing::{error, warn};

type CharacterLoaderRequest = (specs::Entity, CharacterLoaderRequestKind);

/// Available database operations when modifying a player's characetr list
enum CharacterLoaderRequestKind {
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
    LoadCharacterData {
        player_uuid: String,
        character_id: i32,
    },
}

/// A tuple of the components that are persisted to the DB for each character
pub type PersistedComponents = (comp::Body, comp::Stats, comp::Inventory, comp::Loadout);

type CharacterListResult = Result<Vec<CharacterItem>, Error>;
type CharacterDataResult = Result<PersistedComponents, Error>;

/// Wrapper for results for character actions. Can be a list of
/// characters, or component data belonging to an individual character
#[derive(Debug)]
pub enum CharacterLoaderResponseType {
    CharacterList(CharacterListResult),
    CharacterData(Box<CharacterDataResult>),
}

/// Common message format dispatched in response to an update request
#[derive(Debug)]
pub struct CharacterLoaderResponse {
    pub entity: specs::Entity,
    pub result: CharacterLoaderResponseType,
}

/// A bi-directional messaging resource for making requests to modify or load
/// character data in a background thread.
///
/// This is used on the character selection screen, and after character
/// selection when loading the components associated with a character.
///
/// Requests messages are sent in the form of
/// [`CharacterLoaderRequestKind`] and are dispatched at the character select
/// screen.
///
/// Responses are polled on each server tick in the format
/// [`CharacterLoaderResponse`]
pub struct CharacterLoader {
    update_rx: Option<channel::Receiver<CharacterLoaderResponse>>,
    update_tx: Option<channel::Sender<CharacterLoaderRequest>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl CharacterLoader {
    pub fn new(db_dir: String) -> Self {
        let (update_tx, internal_rx) = channel::unbounded::<CharacterLoaderRequest>();
        let (internal_tx, update_rx) = channel::unbounded::<CharacterLoaderResponse>();

        let handle = std::thread::spawn(move || {
            while let Ok(request) = internal_rx.recv() {
                let (entity, kind) = request;

                if let Err(e) = internal_tx.send(CharacterLoaderResponse {
                    entity,
                    result: match kind {
                        CharacterLoaderRequestKind::CreateCharacter {
                            player_uuid,
                            character_alias,
                            character_tool,
                            body,
                        } => CharacterLoaderResponseType::CharacterList(create_character(
                            &player_uuid,
                            &character_alias,
                            character_tool,
                            &body,
                            &db_dir,
                        )),
                        CharacterLoaderRequestKind::DeleteCharacter {
                            player_uuid,
                            character_id,
                        } => CharacterLoaderResponseType::CharacterList(delete_character(
                            &player_uuid,
                            character_id,
                            &db_dir,
                        )),
                        CharacterLoaderRequestKind::LoadCharacterList { player_uuid } => {
                            CharacterLoaderResponseType::CharacterList(load_character_list(
                                &player_uuid,
                                &db_dir,
                            ))
                        },
                        CharacterLoaderRequestKind::LoadCharacterData {
                            player_uuid,
                            character_id,
                        } => CharacterLoaderResponseType::CharacterData(Box::new(
                            load_character_data(&player_uuid, character_id, &db_dir),
                        )),
                    },
                }) {
                    error!(?e, "Could not send send persistence request");
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
    /// `player_uuid`
    pub fn create_character(
        &self,
        entity: specs::Entity,
        player_uuid: String,
        character_alias: String,
        character_tool: Option<String>,
        body: comp::Body,
    ) {
        if let Err(e) = self.update_tx.as_ref().unwrap().send((
            entity,
            CharacterLoaderRequestKind::CreateCharacter {
                player_uuid,
                character_alias,
                character_tool,
                body,
            },
        )) {
            error!(?e, "Could not send character creation request");
        }
    }

    /// Delete a character by `id` and `player_uuid`
    pub fn delete_character(&self, entity: specs::Entity, player_uuid: String, character_id: i32) {
        if let Err(e) = self.update_tx.as_ref().unwrap().send((
            entity,
            CharacterLoaderRequestKind::DeleteCharacter {
                player_uuid,
                character_id,
            },
        )) {
            error!(?e, "Could not send character deletion request");
        }
    }

    /// Loads a list of characters belonging to the player identified by
    /// `player_uuid`
    pub fn load_character_list(&self, entity: specs::Entity, player_uuid: String) {
        if let Err(e) = self
            .update_tx
            .as_ref()
            .unwrap()
            .send((entity, CharacterLoaderRequestKind::LoadCharacterList {
                player_uuid,
            }))
        {
            error!(?e, "Could not send character list load request");
        }
    }

    /// Loads components associated with a character
    pub fn load_character_data(
        &self,
        entity: specs::Entity,
        player_uuid: String,
        character_id: i32,
    ) {
        if let Err(e) = self.update_tx.as_ref().unwrap().send((
            entity,
            CharacterLoaderRequestKind::LoadCharacterData {
                player_uuid,
                character_id,
            },
        )) {
            error!(?e, "Could not send character data load request");
        }
    }

    /// Returns a non-blocking iterator over CharacterLoaderResponse messages
    pub fn messages(&self) -> TryIter<CharacterLoaderResponse> {
        self.update_rx.as_ref().unwrap().try_iter()
    }
}

impl Drop for CharacterLoader {
    fn drop(&mut self) {
        drop(self.update_tx.take());
        if let Err(e) = self.handle.take().unwrap().join() {
            error!(?e, "Error from joining character loader thread");
        }
    }
}

/// Load stored data for a character.
///
/// After first logging in, and after a character is selected, we fetch this
/// data for the purpose of inserting their persisted data for the entity.
fn load_character_data(player_uuid: &str, character_id: i32, db_dir: &str) -> CharacterDataResult {
    let connection = establish_connection(db_dir);

    let result = schema::character::dsl::character
        .filter(schema::character::id.eq(character_id))
        .filter(schema::character::player_uuid.eq(player_uuid))
        .inner_join(schema::body::table)
        .inner_join(schema::stats::table)
        .inner_join(schema::inventory::table)
        .inner_join(schema::loadout::table)
        .first::<(Character, Body, Stats, Inventory, Loadout)>(&connection);

    match result {
        Ok((character_data, body_data, stats_data, inventory, loadout)) => Ok((
            comp::Body::from(&body_data),
            comp::Stats::from(StatsJoinData {
                alias: &character_data.alias,
                body: &comp::Body::from(&body_data),
                stats: &stats_data,
            }),
            comp::Inventory::from(inventory),
            comp::Loadout::from(&loadout),
        )),
        Err(e) => {
            error!(
                ?e,
                ?character_id,
                "Failed to load character data for character"
            );
            Err(Error::CharacterDataError)
        },
    }
}

/// Loads a list of characters belonging to the player. This data is a small
/// subset of the character's data, and is used to render the character and
/// their level in the character list.
///
/// In the event that a join fails, for a character (i.e. they lack an entry for
/// stats, body, etc...) the character is skipped, and no entry will be
/// returned.
fn load_character_list(player_uuid: &str, db_dir: &str) -> CharacterListResult {
    let result = schema::character::dsl::character
        .filter(schema::character::player_uuid.eq(player_uuid))
        .order(schema::character::id.desc())
        .inner_join(schema::body::table)
        .inner_join(schema::stats::table)
        .inner_join(schema::loadout::table)
        .load::<(Character, Body, Stats, Loadout)>(&establish_connection(db_dir));

    match result {
        Ok(data) => Ok(data
            .iter()
            .map(|(character_data, body_data, stats_data, loadout)| {
                let character = CharacterData::from(character_data);
                let body = comp::Body::from(body_data);
                let level = stats_data.level as usize;
                let loadout = comp::Loadout::from(loadout);

                CharacterItem {
                    character,
                    body,
                    level,
                    loadout,
                }
            })
            .collect()),
        Err(e) => {
            error!(?e, ?player_uuid, "Failed to load character list for player");
            Err(Error::CharacterDataError)
        },
    }
}

/// Create a new character with provided comp::Character and comp::Body data.
///
/// Note that sqlite does not support returning the inserted data after a
/// successful insert. To workaround, we wrap this in a transaction which
/// inserts, queries for the newly created chaacter id, then uses the character
/// id for subsequent insertions
fn create_character(
    uuid: &str,
    character_alias: &str,
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
                    skills: SkillSetData(default_stats.skill_set),
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
    // Update Stats
    if let Err(e) =
        diesel::update(schema::stats::table.filter(schema::stats::character_id.eq(character_id)))
            .set(stats)
            .execute(connection)
    {
        error!(?e, ?character_id, "Failed to update stats for character",)
    }

    // Update Inventory
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

    // Update Loadout
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
