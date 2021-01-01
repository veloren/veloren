extern crate serde_json;

use super::schema::{body, character, entity, item, skill, skill_group};

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
    pub waypoint: Option<String>,
}

#[derive(Identifiable, Queryable, Debug)]
#[primary_key(character_id)]
#[table_name = "character"]
pub struct Character {
    pub character_id: i64,
    pub player_uuid: String,
    pub alias: String,
    pub waypoint: Option<String>,
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

#[derive(Associations, Identifiable, Insertable, Queryable, Debug)]
#[primary_key(body_id)]
#[table_name = "body"]
pub struct Body {
    pub body_id: i64,
    pub variant: String,
    pub body_data: String,
}

#[derive(Associations, Identifiable, Insertable, Queryable, Debug)]
#[primary_key(character_id, skill_type)]
#[table_name = "skill"]
pub struct Skill {
    pub character_id: i64,
    pub skill_type: String,
    pub level: Option<i32>,
}

#[derive(Associations, Identifiable, Insertable, Queryable, Debug)]
#[primary_key(character_id, skill_group_type)]
#[table_name = "skill_group"]
pub struct SkillGroup {
    pub character_id: i64,
    pub skill_group_type: String,
    pub exp: i32,
    pub available_sp: i32,
}
