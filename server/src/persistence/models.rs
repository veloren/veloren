extern crate serde_json;

use super::schema::{body, character, inventory, stats};
use crate::comp;
use common::character::Character as CharacterData;
use diesel::sql_types::Text;
use serde::{Deserialize, Serialize};

/// The required elements to build comp::Stats from database data
pub struct StatsJoinData<'a> {
    pub alias: &'a str,
    pub body: &'a comp::Body,
    pub stats: &'a Stats,
}

/// `Character` represents a playable character belonging to a player
#[derive(Identifiable, Queryable, Debug)]
#[table_name = "character"]
pub struct Character {
    pub id: i32,
    pub player_uuid: String,
    pub alias: String,
    pub tool: Option<String>,
}

#[derive(Insertable)]
#[table_name = "character"]
pub struct NewCharacter<'a> {
    pub player_uuid: &'a str,
    pub alias: &'a str,
    pub tool: Option<&'a str>,
}

impl From<&Character> for CharacterData {
    fn from(character: &Character) -> CharacterData {
        CharacterData {
            id: Some(character.id),
            alias: String::from(&character.alias),
            tool: character.tool.clone(),
        }
    }
}

/// `Body` represents the body variety for a character
#[derive(Associations, Identifiable, Queryable, Debug, Insertable)]
#[belongs_to(Character)]
#[primary_key(character_id)]
#[table_name = "body"]
pub struct Body {
    pub character_id: i32,
    pub species: i16,
    pub body_type: i16,
    pub hair_style: i16,
    pub beard: i16,
    pub eyes: i16,
    pub accessory: i16,
    pub hair_color: i16,
    pub skin: i16,
    pub eye_color: i16,
}

impl From<&Body> for comp::Body {
    fn from(body: &Body) -> comp::Body {
        comp::Body::Humanoid(comp::humanoid::Body {
            species: comp::humanoid::ALL_SPECIES[body.species as usize],
            body_type: comp::humanoid::ALL_BODY_TYPES[body.body_type as usize],
            hair_style: body.hair_style as u8,
            beard: body.beard as u8,
            eyes: body.eyes as u8,
            accessory: body.accessory as u8,
            hair_color: body.hair_color as u8,
            skin: body.skin as u8,
            eye_color: body.eye_color as u8,
        })
    }
}

/// `Stats` represents the stats for a character
#[derive(Associations, AsChangeset, Identifiable, Queryable, Debug, Insertable)]
#[belongs_to(Character)]
#[primary_key(character_id)]
#[table_name = "stats"]
pub struct Stats {
    pub character_id: i32,
    pub level: i32,
    pub exp: i32,
    pub endurance: i32,
    pub fitness: i32,
    pub willpower: i32,
}

impl From<StatsJoinData<'_>> for comp::Stats {
    fn from(data: StatsJoinData) -> comp::Stats {
        let level = data.stats.level as u32;

        let mut base_stats = comp::Stats::new(String::from(data.alias), *data.body);

        base_stats.level.set_level(level);
        base_stats.exp.update_maximum(level);

        base_stats.exp.set_current(data.stats.exp as u32);

        base_stats.update_max_hp();
        base_stats
            .health
            .set_to(base_stats.health.maximum(), comp::HealthSource::Revive);

        base_stats.endurance = data.stats.endurance as u32;
        base_stats.fitness = data.stats.fitness as u32;
        base_stats.willpower = data.stats.willpower as u32;

        base_stats
    }
}

#[derive(AsChangeset, Debug, PartialEq)]
#[primary_key(character_id)]
#[table_name = "stats"]
pub struct StatsUpdate {
    pub level: i32,
    pub exp: i32,
    pub endurance: i32,
    pub fitness: i32,
    pub willpower: i32,
}

impl From<&comp::Stats> for StatsUpdate {
    fn from(stats: &comp::Stats) -> StatsUpdate {
        StatsUpdate {
            level: stats.level.level() as i32,
            exp: stats.exp.current() as i32,
            endurance: stats.endurance as i32,
            fitness: stats.fitness as i32,
            willpower: stats.willpower as i32,
        }
    }
}

#[derive(Associations, AsChangeset, Identifiable, Queryable, Debug, Insertable)]
#[belongs_to(Character)]
#[primary_key(character_id)]
#[table_name = "inventory"]
pub struct Inventory {
    character_id: i32,
    items: InventoryData,
}

impl From<(i32, comp::Inventory)> for Inventory {
    fn from(data: (i32, comp::Inventory)) -> Inventory {
        let (character_id, inventory) = data;

        Inventory {
            character_id,
            items: InventoryData(inventory),
        }
    }
}

impl From<Inventory> for comp::Inventory {
    fn from(inventory: Inventory) -> comp::Inventory { inventory.items.0 }
}

#[derive(AsChangeset, Debug, PartialEq)]
#[primary_key(character_id)]
#[table_name = "inventory"]
pub struct InventoryUpdate {
    pub items: InventoryData,
}

impl From<&comp::Inventory> for InventoryUpdate {
    fn from(inventory: &comp::Inventory) -> InventoryUpdate {
        InventoryUpdate {
            items: InventoryData(inventory.clone()),
        }
    }
}

/// Type handling for a character's inventory, which is stored as JSON strings
#[derive(SqlType, AsExpression, Debug, Deserialize, Serialize, FromSqlRow, PartialEq)]
#[sql_type = "Text"]
pub struct InventoryData(comp::Inventory);

impl<DB> diesel::deserialize::FromSql<Text, DB> for InventoryData
where
    DB: diesel::backend::Backend,
    String: diesel::deserialize::FromSql<Text, DB>,
{
    fn from_sql(
        bytes: Option<&<DB as diesel::backend::Backend>::RawValue>,
    ) -> diesel::deserialize::Result<Self> {
        let t = String::from_sql(bytes)?;

        match serde_json::from_str(&t) {
            Ok(data) => Ok(Self(data)),
            Err(error) => {
                log::warn!("Failed to deserialise inventory data: {}", error);

                Ok(Self(comp::Inventory::default()))
            },
        }
    }
}

impl<DB> diesel::serialize::ToSql<Text, DB> for InventoryData
where
    DB: diesel::backend::Backend,
{
    fn to_sql<W: std::io::Write>(
        &self,
        out: &mut diesel::serialize::Output<W, DB>,
    ) -> diesel::serialize::Result {
        let s = serde_json::to_string(&self.0)?;
        <String as diesel::serialize::ToSql<Text, DB>>::to_sql(&s, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comp;

    #[test]
    fn stats_update_from_stats() {
        let mut stats = comp::Stats::new(
            String::from("Test"),
            comp::Body::Humanoid(comp::humanoid::Body::random()),
        );

        stats.level.set_level(2);
        stats.exp.set_current(20);

        stats.endurance = 2;
        stats.fitness = 3;
        stats.willpower = 4;

        assert_eq!(StatsUpdate::from(&stats), StatsUpdate {
            level: 2,
            exp: 20,
            endurance: 2,
            fitness: 3,
            willpower: 4,
        })
    }

    #[test]
    fn loads_stats_with_correct_level() {
        let data = StatsJoinData {
            alias: "test",
            body: &comp::Body::from(&Body {
                character_id: 0,
                species: 0,
                body_type: comp::humanoid::BodyType::Female as i16,
                hair_style: 0,
                beard: 0,
                eyes: 0,
                accessory: 0,
                hair_color: 0,
                skin: 0,
                eye_color: 0,
            }),
            stats: &Stats {
                character_id: 0,
                level: 3,
                exp: 70,
                endurance: 0,
                fitness: 2,
                willpower: 3,
            },
        };

        let stats = comp::Stats::from(data);

        assert_eq!(stats.level.level(), 3);
        assert_eq!(stats.exp.current(), 70);
    }
}
