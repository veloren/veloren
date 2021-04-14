use crate::comp;
use common::character::CharacterId;

use crate::persistence::{
    character_loader::{CharacterLoaderResponse, CharacterLoaderResponseKind},
    error::PersistenceError,
    establish_connection, DatabaseSettings, VelorenConnection,
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

pub type CharacterUpdateData = (comp::SkillSet, comp::Inventory, Option<comp::Waypoint>);

pub enum CharacterUpdaterEvent {
    BatchUpdate(Vec<(CharacterId, CharacterUpdateData)>),
    DeleteCharacter {
        entity: Entity,
        requesting_player_uuid: String,
        character_id: CharacterId,
    },
    DisconnectedSuccess,
}

/// A unidirectional messaging resource for saving characters in a
/// background thread.
///
/// This is used to make updates to a character and their persisted components,
/// such as inventory, loadout, etc...
pub struct CharacterUpdater {
    update_tx: Option<crossbeam_channel::Sender<CharacterUpdaterEvent>>,
    response_rx: crossbeam_channel::Receiver<CharacterLoaderResponse>,
    handle: Option<std::thread::JoinHandle<()>>,
    pending_logout_updates: HashMap<CharacterId, CharacterUpdateData>,
    /// Will disconnect all characters (without persistence) on the next tick if
    /// set to true
    disconnect_all_clients_requested: Arc<AtomicBool>,
}

impl CharacterUpdater {
    pub fn new(settings: Arc<RwLock<DatabaseSettings>>) -> rusqlite::Result<Self> {
        let (update_tx, update_rx) = crossbeam_channel::unbounded::<CharacterUpdaterEvent>();
        let (response_tx, response_rx) = crossbeam_channel::unbounded::<CharacterLoaderResponse>();

        let disconnect_all_clients_requested = Arc::new(AtomicBool::new(false));
        let disconnect_all_clients_requested_clone = Arc::clone(&disconnect_all_clients_requested);

        let builder = std::thread::Builder::new().name("persistence_updater".into());
        let handle = builder
            .spawn(move || {
                // Unwrap here is safe as there is no code that can panic when the write lock is
                // taken that could cause the RwLock to become poisoned.
                let mut conn = establish_connection(&*settings.read().unwrap());
                while let Ok(updates) = update_rx.recv() {
                    match updates {
                        CharacterUpdaterEvent::BatchUpdate(updates) => {
                            if disconnect_all_clients_requested_clone.load(Ordering::Relaxed) {
                                debug!(
                                    "Skipping persistence due to pending disconnection of all \
                                     clients"
                                );
                                continue;
                            }
                            conn.update_log_mode(&settings);
                            if let Err(e) = execute_batch_update(updates, &mut conn) {
                                error!(
                                    "Error during character batch update, disconnecting all \
                                     clients to avoid loss of data integrity. Error: {:?}",
                                    e
                                );
                                disconnect_all_clients_requested_clone
                                    .store(true, Ordering::Relaxed);
                            };
                        },
                        CharacterUpdaterEvent::DeleteCharacter {
                            entity,
                            requesting_player_uuid,
                            character_id,
                        } => {
                            match execute_character_delete(
                                entity,
                                &requesting_player_uuid,
                                character_id,
                                &mut conn,
                            ) {
                                Ok(response) => {
                                    if let Err(e) = response_tx.send(response) {
                                        error!(?e, "Could not send character deletion response");
                                    } else {
                                        debug!(
                                            "Processed character delete for character ID {}",
                                            character_id
                                        );
                                    }
                                },
                                Err(e) => error!(
                                    "Error deleting character ID {}, error: {:?}",
                                    character_id, e
                                ),
                            }
                        },
                        CharacterUpdaterEvent::DisconnectedSuccess => {
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
            pending_logout_updates: HashMap::new(),
            disconnect_all_clients_requested,
        })
    }

    /// Adds a character to the list of characters that have recently logged out
    /// and will be persisted in the next batch update.
    pub fn add_pending_logout_update(
        &mut self,
        character_id: CharacterId,
        update_data: CharacterUpdateData,
    ) {
        if !self
            .disconnect_all_clients_requested
            .load(Ordering::Relaxed)
        {
            self.pending_logout_updates
                .insert(character_id, update_data);
        } else {
            warn!(
                "Ignoring request to add pending logout update for character ID {} as there is a \
                 disconnection of all clients in progress",
                character_id
            );
        }
    }

    /// Returns the character IDs of characters that have recently logged out
    /// and are awaiting persistence in the next batch update.
    pub fn characters_pending_logout(&self) -> impl Iterator<Item = CharacterId> + '_ {
        self.pending_logout_updates.keys().copied()
    }

    /// Returns a value indicating whether there is a pending request to
    /// disconnect all clients due to a batch update transaction failure
    pub fn disconnect_all_clients_requested(&self) -> bool {
        self.disconnect_all_clients_requested
            .load(Ordering::Relaxed)
    }

    pub fn delete_character(
        &mut self,
        entity: Entity,
        requesting_player_uuid: String,
        character_id: CharacterId,
    ) {
        if let Err(e) =
            self.update_tx
                .as_ref()
                .unwrap()
                .send(CharacterUpdaterEvent::DeleteCharacter {
                    entity,
                    requesting_player_uuid,
                    character_id,
                })
        {
            error!(?e, "Could not send character deletion request");
        } else {
            // Once a delete request has been sent to the channel we must remove any pending
            // updates for the character in the event that it has recently logged out.
            // Since the user has actively chosen to delete the character there is no value
            // in the pending update data anyway.
            self.pending_logout_updates.remove(&character_id);
        }
    }

    /// Updates a collection of characters based on their id and components
    pub fn batch_update<'a>(
        &mut self,
        updates: impl Iterator<
            Item = (
                CharacterId,
                &'a comp::SkillSet,
                &'a comp::Inventory,
                Option<&'a comp::Waypoint>,
            ),
        >,
    ) {
        let updates = updates
            .map(|(character_id, skill_set, inventory, waypoint)| {
                (
                    character_id,
                    (skill_set.clone(), inventory.clone(), waypoint.cloned()),
                )
            })
            .chain(self.pending_logout_updates.drain())
            .collect::<Vec<_>>();

        if let Err(e) = self
            .update_tx
            .as_ref()
            .unwrap()
            .send(CharacterUpdaterEvent::BatchUpdate(updates))
        {
            error!(?e, "Could not send stats updates");
        }
    }

    /// Updates a single character based on their id and components
    pub fn update(
        &mut self,
        character_id: CharacterId,
        skill_set: &comp::SkillSet,
        inventory: &comp::Inventory,
        waypoint: Option<&comp::Waypoint>,
    ) {
        self.batch_update(std::iter::once((
            character_id,
            skill_set,
            inventory,
            waypoint,
        )));
    }

    /// Indicates to the batch update thread that a requested disconnection of
    /// all clients has been processed
    pub fn disconnected_success(&mut self) {
        self.update_tx
            .as_ref()
            .unwrap()
            .send(CharacterUpdaterEvent::DisconnectedSuccess)
            .expect(
                "Failed to send DisconnectedSuccess event - not sending this event will prevent \
                 future persistence batches from running",
            );
    }

    /// Returns a non-blocking iterator over CharacterLoaderResponse messages
    pub fn messages(&self) -> TryIter<CharacterLoaderResponse> { self.response_rx.try_iter() }
}

fn execute_batch_update(
    updates: Vec<(CharacterId, CharacterUpdateData)>,
    connection: &mut VelorenConnection,
) -> Result<(), PersistenceError> {
    let mut transaction = connection.connection.transaction()?;
    transaction.set_drop_behavior(DropBehavior::Rollback);
    trace!("Transaction started for character batch update");
    updates
        .into_iter()
        .try_for_each(|(character_id, (stats, inventory, waypoint))| {
            super::character::update(character_id, stats, inventory, waypoint, &mut transaction)
        })?;
    transaction.commit()?;

    trace!("Commit for character batch update completed");
    Ok(())
}

fn execute_character_delete(
    entity: Entity,
    requesting_player_uuid: &str,
    character_id: CharacterId,
    connection: &mut VelorenConnection,
) -> Result<CharacterLoaderResponse, PersistenceError> {
    let mut transaction = connection.connection.transaction()?;

    let response = CharacterLoaderResponse {
        entity,
        result: CharacterLoaderResponseKind::CharacterList(super::character::delete_character(
            requesting_player_uuid,
            character_id,
            &mut transaction,
        )),
    };

    if !response.is_err() {
        transaction.commit()?;
    };

    Ok(response)
}

impl Drop for CharacterUpdater {
    fn drop(&mut self) {
        drop(self.update_tx.take());
        if let Err(e) = self.handle.take().unwrap().join() {
            error!(?e, "Error from joining character update thread");
        }
    }
}
