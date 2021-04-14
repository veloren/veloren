use crate::persistence::{
    character::{create_character, delete_character, load_character_data, load_character_list},
    error::PersistenceError,
    establish_connection, DatabaseSettings, PersistedComponents,
};
use common::character::{CharacterId, CharacterItem};
use crossbeam_channel::{self, TryIter};
use rusqlite::Transaction;
use std::sync::{Arc, RwLock};
use tracing::{error, trace};

pub(crate) type CharacterListResult = Result<Vec<CharacterItem>, PersistenceError>;
pub(crate) type CharacterCreationResult =
    Result<(CharacterId, Vec<CharacterItem>), PersistenceError>;
pub(crate) type CharacterDataResult = Result<PersistedComponents, PersistenceError>;
type CharacterLoaderRequest = (specs::Entity, CharacterLoaderRequestKind);

/// Available database operations when modifying a player's character list
enum CharacterLoaderRequestKind {
    CreateCharacter {
        player_uuid: String,
        character_alias: String,
        persisted_components: PersistedComponents,
    },
    DeleteCharacter {
        player_uuid: String,
        character_id: CharacterId,
    },
    LoadCharacterList {
        player_uuid: String,
    },
    LoadCharacterData {
        player_uuid: String,
        character_id: CharacterId,
    },
}

/// Wrapper for results for character actions. Can be a list of
/// characters, or component data belonging to an individual character
#[derive(Debug)]
pub enum CharacterLoaderResponseKind {
    CharacterList(CharacterListResult),
    CharacterData(Box<CharacterDataResult>),
    CharacterCreation(CharacterCreationResult),
}

/// Common message format dispatched in response to an update request
#[derive(Debug)]
pub struct CharacterLoaderResponse {
    pub entity: specs::Entity,
    pub result: CharacterLoaderResponseKind,
}

impl CharacterLoaderResponse {
    pub fn is_err(&self) -> bool {
        matches!(
            &self.result,
            CharacterLoaderResponseKind::CharacterData(box Err(_))
                | CharacterLoaderResponseKind::CharacterList(Err(_))
                | CharacterLoaderResponseKind::CharacterCreation(Err(_))
        )
    }
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
    update_rx: crossbeam_channel::Receiver<CharacterLoaderResponse>,
    update_tx: crossbeam_channel::Sender<CharacterLoaderRequest>,
}

impl CharacterLoader {
    pub fn new(settings: Arc<RwLock<DatabaseSettings>>) -> Result<Self, PersistenceError> {
        let (update_tx, internal_rx) = crossbeam_channel::unbounded::<CharacterLoaderRequest>();
        let (internal_tx, update_rx) = crossbeam_channel::unbounded::<CharacterLoaderResponse>();

        let builder = std::thread::Builder::new().name("persistence_loader".into());
        builder
            .spawn(move || {
                // Unwrap here is safe as there is no code that can panic when the write lock is
                // taken that could cause the RwLock to become poisoned.
                let mut conn = establish_connection(&*settings.read().unwrap());

                for request in internal_rx {
                    conn.update_log_mode(&settings);

                    match conn.connection.transaction() {
                        Ok(mut transaction) => {
                            let response =
                                CharacterLoader::process_request(request, &mut transaction);
                            if !response.is_err() {
                                match transaction.commit() {
                                    Ok(()) => {
                                        trace!("Commit for character loader completed");
                                    },
                                    Err(e) => error!(
                                        "Failed to commit transaction for character loader, \
                                         error: {:?}",
                                        e
                                    ),
                                };
                            };

                            if let Err(e) = internal_tx.send(response) {
                                error!(?e, "Could not send character loader response");
                            }
                        },
                        Err(e) => {
                            error!(
                                "Failed to start transaction for character loader, error: {:?}",
                                e
                            )
                        },
                    }
                }
            })
            .unwrap();

        Ok(Self {
            update_rx,
            update_tx,
        })
    }

    // TODO: Refactor the way that we send errors to the client to not require a
    // specific Result type per CharacterLoaderResponseKind, and remove
    // CharacterLoaderResponse::is_err()
    fn process_request(
        request: CharacterLoaderRequest,
        mut transaction: &mut Transaction,
    ) -> CharacterLoaderResponse {
        let (entity, kind) = request;
        CharacterLoaderResponse {
            entity,
            result: match kind {
                CharacterLoaderRequestKind::CreateCharacter {
                    player_uuid,
                    character_alias,
                    persisted_components,
                } => CharacterLoaderResponseKind::CharacterCreation(create_character(
                    &player_uuid,
                    &character_alias,
                    persisted_components,
                    &mut transaction,
                )),
                CharacterLoaderRequestKind::DeleteCharacter {
                    player_uuid,
                    character_id,
                } => CharacterLoaderResponseKind::CharacterList(delete_character(
                    &player_uuid,
                    character_id,
                    &mut transaction,
                )),
                CharacterLoaderRequestKind::LoadCharacterList { player_uuid } => {
                    CharacterLoaderResponseKind::CharacterList(load_character_list(
                        &player_uuid,
                        &mut transaction,
                    ))
                },
                CharacterLoaderRequestKind::LoadCharacterData {
                    player_uuid,
                    character_id,
                } => {
                    let result = load_character_data(player_uuid, character_id, &mut transaction);
                    if result.is_err() {
                        error!(
                            ?result,
                            "Error loading character data for character_id: {}", character_id
                        );
                    }
                    CharacterLoaderResponseKind::CharacterData(Box::new(result))
                },
            },
        }
    }

    /// Create a new character belonging to the player identified by
    /// `player_uuid`
    pub fn create_character(
        &self,
        entity: specs::Entity,
        player_uuid: String,
        character_alias: String,
        persisted_components: PersistedComponents,
    ) {
        if let Err(e) = self
            .update_tx
            .send((entity, CharacterLoaderRequestKind::CreateCharacter {
                player_uuid,
                character_alias,
                persisted_components,
            }))
        {
            error!(?e, "Could not send character creation request");
        }
    }

    /// Delete a character by `id` and `player_uuid`
    pub fn delete_character(
        &self,
        entity: specs::Entity,
        player_uuid: String,
        character_id: CharacterId,
    ) {
        if let Err(e) = self
            .update_tx
            .send((entity, CharacterLoaderRequestKind::DeleteCharacter {
                player_uuid,
                character_id,
            }))
        {
            error!(?e, "Could not send character deletion request");
        }
    }

    /// Loads a list of characters belonging to the player identified by
    /// `player_uuid`
    pub fn load_character_list(&self, entity: specs::Entity, player_uuid: String) {
        if let Err(e) = self
            .update_tx
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
        character_id: CharacterId,
    ) {
        if let Err(e) =
            self.update_tx
                .send((entity, CharacterLoaderRequestKind::LoadCharacterData {
                    player_uuid,
                    character_id,
                }))
        {
            error!(?e, "Could not send character data load request");
        }
    }

    /// Returns a non-blocking iterator over CharacterLoaderResponse messages
    pub fn messages(&self) -> TryIter<CharacterLoaderResponse> { self.update_rx.try_iter() }
}
