extern crate diesel;

use super::{establish_connection, models::StatsUpdate, schema};
use crate::comp;
use diesel::prelude::*;

pub fn update<'a>(updates: impl Iterator<Item = (i32, &'a comp::Stats)>) {
    use schema::stats;

    let connection = establish_connection();

    updates.for_each(|(character_id, stats)| {
        if let Err(error) =
            diesel::update(stats::table.filter(schema::stats::character_id.eq(character_id)))
                .set(&StatsUpdate::from(stats))
                .execute(&connection)
        {
            log::warn!(
                "Failed to update stats for character: {:?}: {:?}",
                character_id,
                error
            )
        }
    });
}
