use crate::comp;
use common::{character::CharacterId, comp::item::ItemId};

use crate::persistence::{establish_connection, VelorenConnection};
use crossbeam::channel;
use std::{path::Path, sync::Arc};
use tracing::{error, trace};

pub type CharacterUpdateData = (
    comp::Stats,
    comp::Inventory,
    comp::Loadout,
    Option<comp::Waypoint>,
);

/// A unidirectional messaging resource for saving characters in a
/// background thread.
///
/// This is used to make updates to a character and their persisted components,
/// such as inventory, loadout, etc...
pub struct CharacterUpdater {
    update_tx: Option<channel::Sender<Vec<(CharacterId, CharacterUpdateData)>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl CharacterUpdater {
    pub fn new(db_dir: &Path) -> diesel::QueryResult<Self> {
        let (update_tx, update_rx) =
            channel::unbounded::<Vec<(CharacterId, CharacterUpdateData)>>();

        let mut conn = establish_connection(db_dir)?;

        let handle = std::thread::spawn(move || {
            while let Ok(updates) = update_rx.recv() {
                trace!("Persistence batch update starting");
                execute_batch_update(updates, &mut conn);
                trace!("Persistence batch update finished");
            }
        });

        Ok(Self {
            update_tx: Some(update_tx),
            handle: Some(handle),
        })
    }

    /// Updates a collection of characters based on their id and components
    pub fn batch_update<'a>(
        &self,
        updates: impl Iterator<
            Item = (
                CharacterId,
                &'a comp::Stats,
                &'a comp::Inventory,
                &'a comp::Loadout,
                Option<&'a comp::Waypoint>,
            ),
        >,
    ) {
        let updates = updates
            .map(|(character_id, stats, inventory, loadout, waypoint)| {
                (
                    character_id,
                    (
                        stats.clone(),
                        inventory.clone(),
                        loadout.clone(),
                        waypoint.cloned(),
                    ),
                )
            })
            .collect::<Vec<_>>();

        if let Err(e) = self.update_tx.as_ref().unwrap().send(updates) {
            error!(?e, "Could not send stats updates");
        }
    }

    /// Updates a single character based on their id and components
    pub fn update(
        &self,
        character_id: CharacterId,
        stats: &comp::Stats,
        inventory: &comp::Inventory,
        loadout: &comp::Loadout,
        waypoint: Option<&comp::Waypoint>,
    ) {
        self.batch_update(std::iter::once((
            character_id,
            stats,
            inventory,
            loadout,
            waypoint,
        )));
    }
}

fn execute_batch_update(
    updates: Vec<(CharacterId, CharacterUpdateData)>,
    connection: &mut VelorenConnection,
) {
    let mut inserted_items = Vec::<Arc<ItemId>>::new();

    if let Err(e) = connection.transaction::<_, super::error::Error, _>(|txn| {
        for (character_id, (stats, inventory, loadout, waypoint)) in updates {
            inserted_items.append(&mut super::character::update(
                character_id,
                stats,
                inventory,
                loadout,
                waypoint,
                txn,
            )?);
        }

        Ok(())
    }) {
        error!(?e, "Error during character batch update transaction");
    }

    // NOTE: On success, updating thee atomics is already taken care of
    // internally.
}

impl Drop for CharacterUpdater {
    fn drop(&mut self) {
        drop(self.update_tx.take());
        if let Err(e) = self.handle.take().unwrap().join() {
            error!(?e, "Error from joining character update thread");
        }
    }
}
