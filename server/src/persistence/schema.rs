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
        waypoint -> Nullable<Text>,
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
    skill (entity_id, skill_type) {
        entity_id -> BigInt,
        #[sql_name = "skill"]
        skill_type -> Text,
        level -> Nullable<Integer>,
    }
}

table! {
    skill_group (entity_id, skill_group_kind) {
        entity_id -> BigInt,
        skill_group_kind -> Text,
        exp -> Integer,
        available_sp -> Integer,
        earned_sp -> Integer,
    }
}

joinable!(character -> body (character_id));

allow_tables_to_appear_in_same_query!(body, character, entity, item);
