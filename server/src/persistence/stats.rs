extern crate diesel;

use super::{establish_connection, models::StatsUpdate, schema};
use diesel::prelude::*;

pub fn update(
    character_id: i32,
    level: Option<i32>,
    exp: Option<i32>,
    endurance: Option<i32>,
    fitness: Option<i32>,
    willpower: Option<i32>,
) {
    use schema::stats;

    match diesel::update(stats::table)
        .set(&StatsUpdate {
            level,
            exp,
            endurance,
            fitness,
            willpower,
        })
        .execute(&establish_connection())
    {
        Err(error) => log::warn!(
            "Failed to update stats for player with character_id: {:?}: {:?}",
            character_id,
            error
        ),
        _ => {},
    };
}
