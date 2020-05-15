extern crate diesel;

use super::{establish_connection, models::StatsUpdate, schema};
use crate::comp;
use crossbeam::channel;
use diesel::prelude::*;

fn update(character_id: i32, stats: &StatsUpdate, connection: &SqliteConnection) {
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
}

fn batch_update(updates: impl Iterator<Item = (i32, StatsUpdate)>, db_dir: &str) {
    let connection = establish_connection(db_dir);

    updates
        .for_each(|(character_id, stats_update)| update(character_id, &stats_update, &connection));
}

pub struct Updater {
    update_tx: Option<channel::Sender<Vec<(i32, StatsUpdate)>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}
impl Updater {
    pub fn new(db_dir: String) -> Self {
        let (update_tx, update_rx) = channel::unbounded::<Vec<(i32, StatsUpdate)>>();
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

    pub fn batch_update<'a>(&self, updates: impl Iterator<Item = (i32, &'a comp::Stats)>) {
        let updates = updates
            .map(|(id, stats)| (id, StatsUpdate::from(stats)))
            .collect();

        if let Err(err) = self.update_tx.as_ref().unwrap().send(updates) {
            log::error!("Could not send stats updates: {:?}", err);
        }
    }

    pub fn update(&self, character_id: i32, stats: &comp::Stats) {
        self.batch_update(std::iter::once((character_id, stats)));
    }
}

impl Drop for Updater {
    fn drop(&mut self) {
        drop(self.update_tx.take());
        if let Err(err) = self.handle.take().unwrap().join() {
            log::error!("Error from joining stats update thread: {:?}", err);
        }
    }
}
