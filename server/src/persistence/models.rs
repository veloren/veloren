extern crate serde_json;

use super::schema::{body, character, inventory, loadout, stats};
use crate::comp;
use common::character::Character as CharacterData;
use diesel::sql_types::Text;
use serde::{Deserialize, Serialize};
use tracing::warn;

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

/// `Body` represents the body variety for a character, which has a one-to-one
/// relationship with Characters. This data is set during player creation, and
/// while there is currently no in-game functionality to modify it, it will
/// likely be added in the future.
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

/// `Stats` represents the stats for a character, and have a one-to-one
/// relationship with `Character`.
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
    pub skills: SkillSetData,
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
        base_stats.skill_set = data.stats.skills.0.clone();
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
    pub skills: SkillSetData,
}

impl From<&comp::Stats> for StatsUpdate {
    fn from(stats: &comp::Stats) -> StatsUpdate {
        StatsUpdate {
            level: stats.level.level() as i32,
            exp: stats.exp.current() as i32,
            endurance: stats.endurance as i32,
            fitness: stats.fitness as i32,
            willpower: stats.willpower as i32,
            skills: SkillSetData(stats.skill_set.clone()),
        }
    }
}

/// A wrapper type for the SkillSet of a character used to serialise to and from
/// JSON If the column contains malformed JSON, a default skillset is returned
#[derive(AsExpression, Debug, Deserialize, Serialize, PartialEq, FromSqlRow)]
#[sql_type = "Text"]
pub struct SkillSetData(pub comp::SkillSet);

impl<DB> diesel::deserialize::FromSql<Text, DB> for SkillSetData
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
            Err(e) => {
                warn!(?e, "Failed to deserialize skill set data");
                Ok(Self(comp::SkillSet::default()))
            },
        }
    }
}

impl<DB> diesel::serialize::ToSql<Text, DB> for SkillSetData
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

/// Inventory storage and conversion. Inventories have a one-to-one relationship
/// with characters.
///
/// We store inventory rows as a (character_id, json) tuples, where the json is
/// a serialised Inventory component.
#[derive(Associations, AsChangeset, Identifiable, Queryable, Debug, Insertable)]
#[belongs_to(Character)]
#[primary_key(character_id)]
#[table_name = "inventory"]
pub struct Inventory {
    character_id: i32,
    items: InventoryData,
}

/// A wrapper type for Inventory components used to serialise to and from JSON
/// If the column contains malformed JSON, a default inventory is returned
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
        serde_json::from_str(&t).map_err(Box::from)
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

/// Loadout holds the armor and weapons owned by a character. This data is
/// seperate from the inventory. At the moment, characters have a single Loadout
/// which is loaded with their character data, however there are plans for each
/// character to have multiple Loadouts which they can switch between during
/// gameplay. Due to this Loadouts have a many to one relationship with
/// characetrs, and a distinct `id`.
#[derive(Associations, Queryable, Debug, Identifiable)]
#[belongs_to(Character)]
#[primary_key(id)]
#[table_name = "loadout"]
pub struct Loadout {
    pub id: i32,
    pub character_id: i32,
    pub items: LoadoutData,
}

/// A wrapper type for Loadout components used to serialise to and from JSON
/// If the column contains malformed JSON, a default loadout is returned, with
/// the starter sword set as the main weapon
#[derive(SqlType, AsExpression, Debug, Deserialize, Serialize, FromSqlRow, PartialEq)]
#[sql_type = "Text"]
pub struct LoadoutData(comp::Loadout);

impl<DB> diesel::deserialize::FromSql<Text, DB> for LoadoutData
where
    DB: diesel::backend::Backend,
    String: diesel::deserialize::FromSql<Text, DB>,
{
    fn from_sql(
        bytes: Option<&<DB as diesel::backend::Backend>::RawValue>,
    ) -> diesel::deserialize::Result<Self> {
        let t = String::from_sql(bytes)?;
        serde_json::from_str(&t).map_err(Box::from)
    }
}

impl<DB> diesel::serialize::ToSql<Text, DB> for LoadoutData
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

impl From<&Loadout> for comp::Loadout {
    fn from(loadout: &Loadout) -> comp::Loadout { loadout.items.0.clone() }
}

#[derive(Insertable, PartialEq, Debug)]
#[table_name = "loadout"]
pub struct NewLoadout {
    pub character_id: i32,
    pub items: LoadoutData,
}

impl From<(i32, &comp::Loadout)> for NewLoadout {
    fn from(data: (i32, &comp::Loadout)) -> NewLoadout {
        let (character_id, loadout) = data;

        NewLoadout {
            character_id,
            items: LoadoutData(loadout.clone()),
        }
    }
}

#[derive(Insertable, PartialEq, Debug, AsChangeset)]
#[table_name = "loadout"]
pub struct LoadoutUpdate {
    pub character_id: i32,
    pub items: LoadoutData,
}

impl From<(i32, &comp::Loadout)> for LoadoutUpdate {
    fn from(data: (i32, &comp::Loadout)) -> LoadoutUpdate {
        let (character_id, loadout) = data;

        LoadoutUpdate {
            character_id,
            items: LoadoutData(loadout.clone()),
        }
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
            skills: SkillSetData(stats.skill_set)
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
                skills: SkillSetData(comp::SkillSet::new()),
            },
        };

        let stats = comp::Stats::from(data);

        assert_eq!(stats.level.level(), 3);
        assert_eq!(stats.exp.current(), 70);
    }
}
