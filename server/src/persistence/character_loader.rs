use crate::persistence::{
    character::{create_character, delete_character, load_character_data, load_character_list},
    error::Error,
    establish_connection, PersistedComponents,
};
use common::character::{CharacterId, CharacterItem};
use crossbeam::{channel, channel::TryIter};
use tracing::error;

pub(crate) type CharacterListResult = Result<Vec<CharacterItem>, Error>;
pub(crate) type CharacterDataResult = Result<PersistedComponents, Error>;
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
    pub fn new(db_dir: String) -> diesel::QueryResult<Self> {
        let (update_tx, internal_rx) = channel::unbounded::<CharacterLoaderRequest>();
        let (internal_tx, update_rx) = channel::unbounded::<CharacterLoaderResponse>();

        let mut conn = establish_connection(&db_dir)?;

        let handle = std::thread::spawn(move || {
            while let Ok(request) = internal_rx.recv() {
                let (entity, kind) = request;

                if let Err(e) = internal_tx.send(CharacterLoaderResponse {
                    entity,
                    result: match kind {
                        CharacterLoaderRequestKind::CreateCharacter {
                            player_uuid,
                            character_alias,
                            persisted_components,
                        } => CharacterLoaderResponseType::CharacterList(conn.transaction(|txn| {
                            create_character(
                                &player_uuid,
                                &character_alias,
                                persisted_components,
                                txn,
                            )
                        })),
                        CharacterLoaderRequestKind::DeleteCharacter {
                            player_uuid,
                            character_id,
                        } => {
                            CharacterLoaderResponseType::CharacterList(conn.transaction(|txn| {
                                delete_character(&player_uuid, character_id, txn)
                            }))
                        },
                        CharacterLoaderRequestKind::LoadCharacterList { player_uuid } => {
                            CharacterLoaderResponseType::CharacterList(
                                conn.transaction(|txn| load_character_list(&player_uuid, txn)),
                            )
                        },
                        CharacterLoaderRequestKind::LoadCharacterData {
                            player_uuid,
                            character_id,
                        } => {
                            CharacterLoaderResponseType::CharacterData(Box::new(conn.transaction(
                                |txn| load_character_data(player_uuid, character_id, txn),
                            )))
                        },
                    },
                }) {
                    error!(?e, "Could not send send persistence request");
                }
            }
        });

        Ok(Self {
            update_tx: Some(update_tx),
            update_rx: Some(update_rx),
            handle: Some(handle),
        })
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
        if let Err(e) = self.update_tx.as_ref().unwrap().send((
            entity,
            CharacterLoaderRequestKind::CreateCharacter {
                player_uuid,
                character_alias,
                persisted_components,
            },
        )) {
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
        character_id: CharacterId,
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
