use super::schema::{body, character, stats};
use crate::comp;
use common::character::Character as CharacterData;

/// When we want to build player stats from database data, we need data from the
/// character, body and stats tables
pub struct StatsJoinData<'a> {
    pub character: &'a CharacterData,
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
    pub race: i16,
    pub body_type: i16,
    pub hair_style: i16,
    pub beard: i16,
    pub eyebrows: i16,
    pub accessory: i16,
    pub hair_color: i16,
    pub skin: i16,
    pub eye_color: i16,
}

impl From<&Body> for comp::Body {
    fn from(body: &Body) -> comp::Body {
        comp::Body::Humanoid(comp::humanoid::Body {
            race: comp::humanoid::ALL_RACES[body.race as usize],
            body_type: comp::humanoid::ALL_BODY_TYPES[body.body_type as usize],
            hair_style: body.hair_style as u8,
            beard: body.beard as u8,
            eyebrows: body.eyebrows as u8,
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
        let mut base_stats = comp::Stats::new(String::from(&data.character.alias), *data.body);

        base_stats.level.set_level(data.stats.level as u32);
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

#[derive(AsChangeset)]
#[primary_key(character_id)]
#[table_name = "stats"]
pub struct StatsUpdate {
    pub level: i32,
    pub exp: i32,
    pub endurance: i32,
    pub fitness: i32,
    pub willpower: i32,
}
