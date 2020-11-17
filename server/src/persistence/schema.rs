table! {
    body (body_id) {
        body_id -> BigInt,
        variant -> Text,
        body_data -> Text,
    }
}

table! {
    character (character_id) {
        character_id -> BigInt,
        player_uuid -> Text,
        alias -> Text,
    }
}

table! {
    entity (entity_id) {
        entity_id -> BigInt,
    }
}

table! {
    item (item_id) {
        item_id -> BigInt,
        parent_container_item_id -> BigInt,
        item_definition_id -> Text,
        stack_size -> Integer,
        position -> Text,
    }
}

table! {
    stats (stats_id) {
        stats_id -> BigInt,
        level -> Integer,
        exp -> Integer,
        endurance -> Integer,
        fitness -> Integer,
        willpower -> Integer,
        waypoint -> Nullable<Text>,
    }
}

joinable!(character -> body (character_id));
joinable!(character -> stats (character_id));

allow_tables_to_appear_in_same_query!(body, character, entity, item, stats,);
