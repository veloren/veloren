table! {
    body (character_id) {
        character_id -> Integer,
        race -> SmallInt,
        body_type -> SmallInt,
        hair_style -> SmallInt,
        beard -> SmallInt,
        eyebrows -> SmallInt,
        accessory -> SmallInt,
        hair_color -> SmallInt,
        skin -> SmallInt,
        eye_color -> SmallInt,
    }
}

table! {
    character (id) {
        id -> Integer,
        player_uuid -> Text,
        alias -> Text,
        tool -> Nullable<Text>,
    }
}

joinable!(body -> character (character_id));

allow_tables_to_appear_in_same_query!(body, character,);
