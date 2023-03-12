use crate::persistence::{
    character::{load_character_data, load_character_list},
    error::PersistenceError,
    establish_connection, ConnectionMode, DatabaseSettings, PersistedComponents,
};
use common::{
    character::{CharacterId, CharacterItem},
    event::UpdateCharacterMetadata,
};
use crossbeam_channel::{self, TryIter};
use rusqlite::Connection;
use std::sync::{Arc, RwLock};
use tracing::error;

pub(crate) type CharacterListResult = Result<Vec<CharacterItem>, PersistenceError>;
pub(crate) type CharacterCreationResult =
    Result<(CharacterId, Vec<CharacterItem>), PersistenceError>;
pub(crate) type CharacterEditResult = Result<(CharacterId, Vec<CharacterItem>), PersistenceError>;
pub(crate) type CharacterDataResult =
    Result<(PersistedComponents, UpdateCharacterMetadata), PersistenceError>;
type CharacterLoaderRequest = (specs::Entity, CharacterLoaderRequestKind);

/// Available database operations when modifying a player's character list
enum CharacterLoaderRequestKind {
    LoadCharacterList {
        player_uuid: String,
    },
    LoadCharacterData {
        player_uuid: String,
        character_id: CharacterId,
    },
}

#[derive(Debug)]
pub enum CharacterUpdaterMessage {
    CharacterScreenResponse(CharacterScreenResponse),
    DatabaseBatchCompletion(u64),
}

/// An event emitted from CharacterUpdater in response to a request made from
/// the character selection/editing screen
#[derive(Debug)]
pub struct CharacterScreenResponse {
    pub target_entity: specs::Entity,
    pub response_kind: CharacterScreenResponseKind,
}

impl CharacterScreenResponse {
    pub fn is_err(&self) -> bool {
        matches!(
            &self.response_kind,
            CharacterScreenResponseKind::CharacterData(box Err(_))
                | CharacterScreenResponseKind::CharacterList(Err(_))
                | CharacterScreenResponseKind::CharacterCreation(Err(_))
        )
    }
}

#[derive(Debug)]
pub enum CharacterScreenResponseKind {
    CharacterList(CharacterListResult),
    CharacterData(Box<CharacterDataResult>),
    CharacterCreation(CharacterCreationResult),
    CharacterEdit(CharacterEditResult),
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
    update_rx: crossbeam_channel::Receiver<CharacterUpdaterMessage>,
    update_tx: crossbeam_channel::Sender<CharacterLoaderRequest>,
}

impl CharacterLoader {
    pub fn new(settings: Arc<RwLock<DatabaseSettings>>) -> Result<Self, PersistenceError> {
        let (update_tx, internal_rx) = crossbeam_channel::unbounded::<CharacterLoaderRequest>();
        let (internal_tx, update_rx) = crossbeam_channel::unbounded::<CharacterUpdaterMessage>();

        let builder = std::thread::Builder::new().name("persistence_loader".into());
        builder
            .spawn(move || {
                // Unwrap here is safe as there is no code that can panic when the write lock is
                // taken that could cause the RwLock to become poisoned.
                //
                // This connection -must- remain read-only to avoid lock contention with the
                // CharacterUpdater thread.
                let mut conn =
                    establish_connection(&settings.read().unwrap(), ConnectionMode::ReadOnly);

                for request in internal_rx {
                    conn.update_log_mode(&settings);

                    let response = CharacterLoader::process_request(request, &conn);
                    if let Err(e) = internal_tx.send(response) {
                        error!(?e, "Could not send character loader response");
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
        connection: &Connection,
    ) -> CharacterUpdaterMessage {
        let (entity, kind) = request;
        CharacterUpdaterMessage::CharacterScreenResponse(CharacterScreenResponse {
            target_entity: entity,
            response_kind: match kind {
                CharacterLoaderRequestKind::LoadCharacterList { player_uuid } => {
                    CharacterScreenResponseKind::CharacterList(load_character_list(
                        &player_uuid,
                        connection,
                    ))
                },
                CharacterLoaderRequestKind::LoadCharacterData {
                    player_uuid,
                    character_id,
                } => {
                    let result = load_character_data(player_uuid, character_id, connection);
                    if result.is_err() {
                        error!(
                            ?result,
                            "Error loading character data for character_id: {}", character_id
                        );
                    }
                    CharacterScreenResponseKind::CharacterData(Box::new(result))
                },
            },
        })
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
    pub fn messages(&self) -> TryIter<CharacterUpdaterMessage> { self.update_rx.try_iter() }
}
