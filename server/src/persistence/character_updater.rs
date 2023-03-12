use crate::comp;
use common::character::CharacterId;

use crate::persistence::{
    character_loader::{
        CharacterScreenResponse, CharacterScreenResponseKind, CharacterUpdaterMessage,
    },
    error::PersistenceError,
    establish_connection, ConnectionMode, DatabaseSettings, EditableComponents,
    PersistedComponents, VelorenConnection,
};
use crossbeam_channel::TryIter;
use rusqlite::DropBehavior;
use specs::Entity;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};
use tracing::{debug, error, info, trace, warn};

pub type CharacterUpdateData = (
    CharacterId,
    comp::SkillSet,
    comp::Inventory,
    Vec<PetPersistenceData>,
    Option<comp::Waypoint>,
    comp::ability::ActiveAbilities,
    Option<comp::MapMarker>,
);

pub type PetPersistenceData = (comp::Pet, comp::Body, comp::Stats);

#[allow(clippy::large_enum_variant)]
enum CharacterUpdaterAction {
    BatchUpdate {
        batch_id: u64,
        updates: Vec<DatabaseActionKind>,
    },
    CreateCharacter {
        entity: Entity,
        player_uuid: String,
        character_alias: String,
        persisted_components: PersistedComponents,
    },
    EditCharacter {
        entity: Entity,
        player_uuid: String,
        character_id: CharacterId,
        character_alias: String,
        editable_components: EditableComponents,
    },
    DisconnectedSuccess,
}

#[derive(Clone)]
enum DatabaseAction {
    New(DatabaseActionKind),
    Submitted { batch_id: u64 },
}

impl DatabaseAction {
    fn take_new(&mut self, batch_id: u64) -> Option<DatabaseActionKind> {
        match core::mem::replace(self, Self::Submitted { batch_id }) {
            Self::New(action) => Some(action),
            submitted @ Self::Submitted { .. } => {
                *self = submitted; // restore old batch_id
                None
            },
        }
    }
}

#[derive(Clone)]
enum DatabaseActionKind {
    UpdateCharacter(Box<CharacterUpdateData>),
    DeleteCharacter {
        requesting_player_uuid: String,
        character_id: CharacterId,
    },
}

/// A unidirectional messaging resource for saving characters in a
/// background thread.
///
/// This is used to make updates to a character and their persisted components,
/// such as inventory, loadout, etc...
pub struct CharacterUpdater {
    update_tx: Option<crossbeam_channel::Sender<CharacterUpdaterAction>>,
    response_rx: crossbeam_channel::Receiver<CharacterUpdaterMessage>,
    handle: Option<std::thread::JoinHandle<()>>,
    /// Pending actions to be performed during the next persistence batch, such
    /// as updates for recently logged out players and character deletions
    pending_database_actions: HashMap<CharacterId, DatabaseAction>,
    /// Will disconnect all characters (without persistence) on the next tick if
    /// set to true
    disconnect_all_clients_requested: Arc<AtomicBool>,
    last_pending_database_event_id: u64,
}

impl CharacterUpdater {
    pub fn new(settings: Arc<RwLock<DatabaseSettings>>) -> rusqlite::Result<Self> {
        let (update_tx, update_rx) = crossbeam_channel::unbounded::<CharacterUpdaterAction>();
        let (response_tx, response_rx) = crossbeam_channel::unbounded::<CharacterUpdaterMessage>();

        let disconnect_all_clients_requested = Arc::new(AtomicBool::new(false));
        let disconnect_all_clients_requested_clone = Arc::clone(&disconnect_all_clients_requested);

        let builder = std::thread::Builder::new().name("persistence_updater".into());
        let handle = builder
            .spawn(move || {
                // Unwrap here is safe as there is no code that can panic when the write lock is
                // taken that could cause the RwLock to become poisoned.
                let mut conn =
                    establish_connection(&settings.read().unwrap(), ConnectionMode::ReadWrite);
                while let Ok(action) = update_rx.recv() {
                    match action {
                        CharacterUpdaterAction::BatchUpdate { batch_id, updates } => {
                            if disconnect_all_clients_requested_clone.load(Ordering::Relaxed) {
                                debug!(
                                    "Skipping persistence due to pending disconnection of all \
                                     clients"
                                );
                                continue;
                            }
                            conn.update_log_mode(&settings);

                            if let Err(e) = execute_batch_update(updates.into_iter(), &mut conn) {
                                error!(
                                    ?e,
                                    "Error during character batch update, disconnecting all \
                                     clients to avoid loss of data integrity."
                                );
                                disconnect_all_clients_requested_clone
                                    .store(true, Ordering::Relaxed);
                            };

                            if let Err(e) = response_tx
                                .send(CharacterUpdaterMessage::DatabaseBatchCompletion(batch_id))
                            {
                                error!(?e, "Could not send DatabaseBatchCompletion message");
                            } else {
                                debug!(
                                    "Submitted DatabaseBatchCompletion - Batch ID: {}",
                                    batch_id
                                );
                            }
                        },
                        CharacterUpdaterAction::CreateCharacter {
                            entity,
                            character_alias,
                            player_uuid,
                            persisted_components,
                        } => {
                            match execute_character_create(
                                entity,
                                character_alias,
                                &player_uuid,
                                persisted_components,
                                &mut conn,
                            ) {
                                Ok(response) => {
                                    if let Err(e) = response_tx.send(response) {
                                        error!(?e, "Could not send character creation response");
                                    } else {
                                        debug!(
                                            "Processed character create for player {}",
                                            player_uuid
                                        );
                                    }
                                },
                                Err(e) => error!(
                                    "Error creating character for player {}, error: {:?}",
                                    player_uuid, e
                                ),
                            }
                        },
                        CharacterUpdaterAction::EditCharacter {
                            entity,
                            character_id,
                            character_alias,
                            player_uuid,
                            editable_components,
                        } => {
                            match execute_character_edit(
                                entity,
                                character_id,
                                character_alias,
                                &player_uuid,
                                editable_components,
                                &mut conn,
                            ) {
                                Ok(response) => {
                                    if let Err(e) = response_tx.send(response) {
                                        error!(?e, "Could not send character edit response");
                                    } else {
                                        debug!(
                                            "Processed character edit for player {}",
                                            player_uuid
                                        );
                                    }
                                },
                                Err(e) => error!(
                                    "Error editing character for player {}, error: {:?}",
                                    player_uuid, e
                                ),
                            }
                        },
                        CharacterUpdaterAction::DisconnectedSuccess => {
                            info!(
                                "CharacterUpdater received DisconnectedSuccess event, resuming \
                                 batch updates"
                            );
                            // Reset the disconnection request as we have had confirmation that all
                            // clients have been disconnected
                            disconnect_all_clients_requested_clone.store(false, Ordering::Relaxed);
                        },
                    }
                }
            })
            .unwrap();

        Ok(Self {
            update_tx: Some(update_tx),
            response_rx,
            handle: Some(handle),
            pending_database_actions: HashMap::new(),
            disconnect_all_clients_requested,
            last_pending_database_event_id: 0,
        })
    }

    /// Adds a character to the list of characters that have recently logged out
    /// and will be persisted in the next batch update.
    pub fn add_pending_logout_update(&mut self, update_data: CharacterUpdateData) {
        if self
            .disconnect_all_clients_requested
            .load(Ordering::Relaxed)
        {
            warn!(
                "Ignoring request to add pending logout update for character ID {} as there is a \
                 disconnection of all clients in progress",
                update_data.0
            );
            return;
        }

        if self.pending_database_actions.contains_key(&update_data.0) {
            warn!(
                "Ignoring request to add pending logout update for character ID {} as there is \
                 already a pending delete for this character",
                update_data.0
            );
            return;
        }

        self.pending_database_actions.insert(
            update_data.0, // CharacterId
            DatabaseAction::New(DatabaseActionKind::UpdateCharacter(Box::new(update_data))),
        );
    }

    pub fn has_pending_database_action(&self, character_id: CharacterId) -> bool {
        self.pending_database_actions.get(&character_id).is_some()
    }

    pub fn process_batch_completion(&mut self, completed_batch_id: u64) {
        self.pending_database_actions.drain_filter(|_, event| {
            matches!(event, DatabaseAction::Submitted {
                    batch_id,
            } if completed_batch_id == *batch_id)
        });
        debug!(
            "Processed database batch completion - Batch ID: {}",
            completed_batch_id
        )
    }

    /// Returns a value indicating whether there is a pending request to
    /// disconnect all clients due to a batch update transaction failure
    pub fn disconnect_all_clients_requested(&self) -> bool {
        self.disconnect_all_clients_requested
            .load(Ordering::Relaxed)
    }

    pub fn create_character(
        &mut self,
        entity: Entity,
        requesting_player_uuid: String,
        alias: String,
        persisted_components: PersistedComponents,
    ) {
        if let Err(e) =
            self.update_tx
                .as_ref()
                .unwrap()
                .send(CharacterUpdaterAction::CreateCharacter {
                    entity,
                    player_uuid: requesting_player_uuid,
                    character_alias: alias,
                    persisted_components,
                })
        {
            error!(?e, "Could not send character creation request");
        }
    }

    pub fn edit_character(
        &mut self,
        entity: Entity,
        requesting_player_uuid: String,
        character_id: CharacterId,
        alias: String,
        editable_components: EditableComponents,
    ) {
        if let Err(e) =
            self.update_tx
                .as_ref()
                .unwrap()
                .send(CharacterUpdaterAction::EditCharacter {
                    entity,
                    player_uuid: requesting_player_uuid,
                    character_id,
                    character_alias: alias,
                    editable_components,
                })
        {
            error!(?e, "Could not send character edit request");
        }
    }

    fn next_pending_database_event_id(&mut self) -> u64 {
        self.last_pending_database_event_id += 1;
        self.last_pending_database_event_id
    }

    pub fn queue_character_deletion(
        &mut self,
        requesting_player_uuid: String,
        character_id: CharacterId,
    ) {
        // Insert the delete as a pending database action - if the player has recently
        // logged out this will replace their pending update with a delete which
        // is fine, as the user has actively chosen to delete the character.
        self.pending_database_actions.insert(
            character_id,
            DatabaseAction::New(DatabaseActionKind::DeleteCharacter {
                requesting_player_uuid,
                character_id,
            }),
        );
    }

    /// Updates a collection of characters based on their id and components
    pub fn batch_update(&mut self, updates: impl Iterator<Item = CharacterUpdateData>) {
        let batch_id = self.next_pending_database_event_id();

        // Collect any new updates, ignoring updates from a previous update that are
        // still pending completion
        let existing_pending_actions = self
            .pending_database_actions
            .iter_mut()
            .filter_map(|(_, event)| event.take_new(batch_id));

        // Combine the pending actions with the updates for logged in characters
        let pending_actions = existing_pending_actions
            .into_iter()
            .chain(updates.map(|update| DatabaseActionKind::UpdateCharacter(Box::new(update))))
            .collect::<Vec<DatabaseActionKind>>();

        if !pending_actions.is_empty() {
            debug!(
                "Sending persistence update batch ID {} containing {} updates",
                batch_id,
                pending_actions.len()
            );
            if let Err(e) =
                self.update_tx
                    .as_ref()
                    .unwrap()
                    .send(CharacterUpdaterAction::BatchUpdate {
                        batch_id,
                        updates: pending_actions,
                    })
            {
                error!(?e, "Could not send persistence batch update");
            }
        } else {
            trace!("Skipping persistence batch - no pending updates")
        }
    }

    /// Indicates to the batch update thread that a requested disconnection of
    /// all clients has been processed
    pub fn disconnected_success(&mut self) {
        self.update_tx
            .as_ref()
            .unwrap()
            .send(CharacterUpdaterAction::DisconnectedSuccess)
            .expect(
                "Failed to send DisconnectedSuccess event - not sending this event will prevent \
                 future persistence batches from running",
            );
    }

    /// Returns a non-blocking iterator over CharacterLoaderResponse messages
    pub fn messages(&self) -> TryIter<CharacterUpdaterMessage> { self.response_rx.try_iter() }
}

fn execute_batch_update(
    updates: impl Iterator<Item = DatabaseActionKind>,
    connection: &mut VelorenConnection,
) -> Result<(), PersistenceError> {
    let mut transaction = connection.connection.transaction()?;
    transaction.set_drop_behavior(DropBehavior::Rollback);
    trace!("Transaction started for character batch update");
    updates.into_iter().try_for_each(|event| match event {
        DatabaseActionKind::UpdateCharacter(box (
            character_id,
            stats,
            inventory,
            pets,
            waypoint,
            active_abilities,
            map_marker,
        )) => super::character::update(
            character_id,
            stats,
            inventory,
            pets,
            waypoint,
            active_abilities,
            map_marker,
            &mut transaction,
        ),
        DatabaseActionKind::DeleteCharacter {
            requesting_player_uuid,
            character_id,
        } => super::character::delete_character(
            &requesting_player_uuid,
            character_id,
            &mut transaction,
        ),
    })?;

    transaction.commit()?;

    trace!("Commit for character batch update completed");
    Ok(())
}

fn execute_character_create(
    entity: Entity,
    alias: String,
    requesting_player_uuid: &str,
    persisted_components: PersistedComponents,
    connection: &mut VelorenConnection,
) -> Result<CharacterUpdaterMessage, PersistenceError> {
    let mut transaction = connection.connection.transaction()?;

    let response = CharacterScreenResponse {
        target_entity: entity,
        response_kind: CharacterScreenResponseKind::CharacterCreation(
            super::character::create_character(
                requesting_player_uuid,
                &alias,
                persisted_components,
                &mut transaction,
            ),
        ),
    };

    if !response.is_err() {
        transaction.commit()?;
    };

    Ok(CharacterUpdaterMessage::CharacterScreenResponse(response))
}

fn execute_character_edit(
    entity: Entity,
    character_id: CharacterId,
    alias: String,
    requesting_player_uuid: &str,
    editable_components: EditableComponents,
    connection: &mut VelorenConnection,
) -> Result<CharacterUpdaterMessage, PersistenceError> {
    let mut transaction = connection.connection.transaction()?;

    let response = CharacterScreenResponse {
        target_entity: entity,
        response_kind: CharacterScreenResponseKind::CharacterEdit(
            super::character::edit_character(
                editable_components,
                &mut transaction,
                character_id,
                requesting_player_uuid,
                &alias,
            ),
        ),
    };

    if !response.is_err() {
        transaction.commit()?;
    };

    Ok(CharacterUpdaterMessage::CharacterScreenResponse(response))
}

impl Drop for CharacterUpdater {
    fn drop(&mut self) {
        drop(self.update_tx.take());
        if let Err(e) = self.handle.take().unwrap().join() {
            error!(?e, "Error from joining character update thread");
        }
    }
}
