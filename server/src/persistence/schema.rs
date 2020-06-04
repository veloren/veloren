table! {
    body (character_id) {
        character_id -> Integer,
        species -> SmallInt,
        body_type -> SmallInt,
        hair_style -> SmallInt,
        beard -> SmallInt,
        eyes -> SmallInt,
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
    inventory (character_id) {
        character_id -> Integer,
        items -> Text,
    }
}

table! {
    loadout (id) {
        id -> Integer,
        character_id -> Integer,
        items -> Text,
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
joinable!(inventory -> character (character_id));
joinable!(loadout -> character (character_id));
joinable!(stats -> character (character_id));

allow_tables_to_appear_in_same_query!(body, character, inventory, loadout, stats,);
