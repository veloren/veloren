extern crate diesel;

use super::{
    establish_connection,
    models::{Body, Character, NewCharacter},
    schema, Error,
};
use crate::comp;
use common::character::{Character as CharacterData, CharacterItem, MAX_CHARACTERS_PER_PLAYER};
use diesel::prelude::*;

type CharacterListResult = Result<Vec<CharacterItem>, Error>;

// Loading of characters happens immediately after login, and the data is only
// for the purpose of rendering the character and their level in the character
// list.
pub fn load_characters(uuid: &str) -> CharacterListResult {
    use schema::{body, character::dsl::*};

    let data: Vec<(Character, Body)> = character
        .filter(player_uuid.eq(uuid))
        .order(id.desc())
        .inner_join(body::table)
        .load::<(Character, Body)>(&establish_connection())?;

    Ok(data
        .iter()
        .map(|(character_data, body_data)| CharacterItem {
            character: CharacterData::from(character_data),
            body: comp::Body::from(body_data),
        })
        .collect())
}

/// Create a new character with provided comp::Character and comp::Body data.
/// Note that sqlite does not suppport returning the inserted data after a
/// successful insert. To workaround, we wrap this in a transaction which
/// inserts, queries for the newly created chaacter id, then uses the character
/// id for insertion of the `body` table entry
pub fn create_character(
    uuid: &str,
    alias: String,
    tool: Option<String>,
    body: &comp::Body,
) -> CharacterListResult {
    check_character_limit(uuid)?;

    let new_character = NewCharacter {
        player_uuid: uuid,
        alias: &alias,
        tool: tool.as_deref(),
    };

    let connection = establish_connection();

    connection.transaction::<_, diesel::result::Error, _>(|| {
        use schema::{body, character, character::dsl::*};

        match body {
            comp::Body::Humanoid(body_data) => {
                diesel::insert_into(character::table)
                    .values(&new_character)
                    .execute(&connection)?;

                let inserted_character = character
                    .filter(player_uuid.eq(uuid))
                    .order(id.desc())
                    .first::<Character>(&connection)?;

                let new_body = Body {
                    character_id: inserted_character.id as i32,
                    race: body_data.race as i16,
                    body_type: body_data.body_type as i16,
                    hair_style: body_data.hair_style as i16,
                    beard: body_data.beard as i16,
                    eyebrows: body_data.eyebrows as i16,
                    accessory: body_data.accessory as i16,
                    hair_color: body_data.hair_color as i16,
                    skin: body_data.skin as i16,
                    eye_color: body_data.eye_color as i16,
                };

                diesel::insert_into(body::table)
                    .values(&new_body)
                    .execute(&connection)?;
            },
            _ => log::warn!("Creating non-humanoid characters is not supported."),
        };

        Ok(())
    })?;

    load_characters(uuid)
}

pub fn delete_character(uuid: &str, character_id: i32) -> CharacterListResult {
    use schema::character::dsl::*;

    diesel::delete(character.filter(id.eq(character_id))).execute(&establish_connection())?;

    load_characters(uuid)
}

fn check_character_limit(uuid: &str) -> Result<(), Error> {
    use diesel::dsl::count_star;
    use schema::character::dsl::*;

    let connection = establish_connection();

    let character_count = character
        .select(count_star())
        .filter(player_uuid.eq(uuid))
        .load::<i64>(&connection)?;

    match character_count.first() {
        Some(count) => {
            if count < &(MAX_CHARACTERS_PER_PLAYER as i64) {
                Ok(())
            } else {
                Err(Error::CharacterLimitReached)
            }
        },
        _ => Ok(()),
    }
}
