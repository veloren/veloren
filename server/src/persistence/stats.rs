extern crate diesel;

use super::{establish_connection, models::StatsUpdate, schema};
use crate::comp;
use diesel::prelude::*;

pub fn update(
    character_id: i32,
    stats: &comp::Stats,
    conn: Option<&SqliteConnection>,
    db_dir: &str,
) {
    log::warn!("stats persisting...");

    if let Err(error) =
        diesel::update(schema::stats::table.filter(schema::stats::character_id.eq(character_id)))
            .set(&StatsUpdate::from(stats))
            .execute(conn.unwrap_or(&establish_connection(db_dir)))
    {
        log::warn!(
            "Failed to update stats for character: {:?}: {:?}",
            character_id,
            error
        )
    }
}

pub fn batch_update<'a>(updates: impl Iterator<Item = (i32, &'a comp::Stats)>, db_dir: &str) {
    let connection = &establish_connection(db_dir);

    updates.for_each(|(character_id, stats)| update(character_id, stats, Some(connection), db_dir));
}
