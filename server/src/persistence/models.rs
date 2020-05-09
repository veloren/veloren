use super::schema::{body, character};
use crate::comp;
use common::character::Character as CharacterData;

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
