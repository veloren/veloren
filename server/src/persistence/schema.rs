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

table! {
    stats (character_id) {
        character_id -> Integer,
        level -> Integer,
        exp -> Integer,
        endurance -> Integer,
        fitness -> Integer,
        willpower -> Integer,
    }
}

joinable!(body -> character (character_id));
joinable!(stats -> character (character_id));

allow_tables_to_appear_in_same_query!(body, character, stats,);
