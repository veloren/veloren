extern crate serde_json;

use super::schema::{body, character, entity, item, stats};

#[derive(Debug, Insertable, PartialEq)]
#[table_name = "entity"]
pub struct Entity {
    pub entity_id: i64,
}

#[derive(Insertable)]
#[table_name = "character"]
pub struct NewCharacter<'a> {
    pub character_id: i64,
    pub player_uuid: &'a str,
    pub alias: &'a str,
}

#[derive(Identifiable, Queryable, Debug)]
#[primary_key(character_id)]
#[table_name = "character"]
pub struct Character {
    pub character_id: i64,
    pub player_uuid: String,
    pub alias: String,
}

#[primary_key(item_id)]
#[table_name = "item"]
#[derive(Debug, Insertable, Queryable, AsChangeset)]
pub struct Item {
    pub item_id: i64,
    pub parent_container_item_id: i64,
    pub item_definition_id: String,
    pub stack_size: i32,
    pub position: String,
}

#[derive(Associations, AsChangeset, Identifiable, Queryable, Debug, Insertable)]
#[primary_key(stats_id)]
#[table_name = "stats"]
pub struct Stats {
    pub stats_id: i64,
    pub level: i32,
    pub exp: i32,
    pub endurance: i32,
    pub fitness: i32,
    pub willpower: i32,
}

#[derive(Associations, Identifiable, Insertable, Queryable, Debug)]
#[primary_key(body_id)]
#[table_name = "body"]
pub struct Body {
    pub body_id: i64,
    pub variant: String,
    pub body_data: String,
}
