extern crate diesel;

use super::establish_connection;
use crate::comp;
use diesel::prelude::*;

/// Update DB rows for stats given a Vec of (character_id, Stats) tuples
pub fn update(data: Vec<(i32, &comp::Stats)>) {
    match establish_connection().execute(&build_query(data)) {
        Err(diesel_error) => log::warn!("Error updating stats: {:?}", diesel_error),
        _ => {},
    }
}

/// Takes a Vec of (character_id, Stats) tuples and builds an SQL UPDATE query
/// Since there is apprently no sensible way to update > 1 row using diesel, we
/// just construct the raw SQL
fn build_query(data: Vec<(i32, &comp::Stats)>) -> String {
    data.iter()
        .map(|(character_id, stats)| {
            String::from(format!(
                "UPDATE stats SET level = {}, exp = {}, endurance = {}, fitness = {}, willpower = \
                 {} WHERE character_id = {};",
                stats.level.level() as i32,
                stats.exp.current() as i32,
                stats.endurance as i32,
                stats.fitness as i32,
                stats.willpower as i32,
                *character_id as i32
            ))
        })
        .collect::<Vec<String>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_query_for_multiple_characters() {
        let mut stats_one = comp::Stats::new(
            String::from("One"),
            comp::Body::Humanoid(comp::humanoid::Body::random()),
        );

        stats_one.endurance = 1;
        stats_one.fitness = 1;
        stats_one.willpower = 1;

        let mut stats_two = comp::Stats::new(
            String::from("Two"),
            comp::Body::Humanoid(comp::humanoid::Body::random()),
        );

        stats_two.endurance = 2;
        stats_two.fitness = 2;
        stats_two.willpower = 2;

        let mut stats_three = comp::Stats::new(
            String::from("Three"),
            comp::Body::Humanoid(comp::humanoid::Body::random()),
        );

        stats_three.endurance = 3;
        stats_three.fitness = 3;
        stats_three.willpower = 3;

        let data = vec![
            (1_i32, &stats_one),
            (2_i32, &stats_two),
            (3_i32, &stats_three),
        ];

        assert_eq!(
            build_query(data),
            "UPDATE stats SET level = 1, exp = 0, endurance = 1, fitness = 1, willpower = 1 WHERE \
             character_id = 1; UPDATE stats SET level = 1, exp = 0, endurance = 2, fitness = 2, \
             willpower = 2 WHERE character_id = 2; UPDATE stats SET level = 1, exp = 0, endurance \
             = 3, fitness = 3, willpower = 3 WHERE character_id = 3;"
        );
    }
}
