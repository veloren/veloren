pub struct Character {
    pub character_id: i64,
    #[expect(dead_code)]
    pub player_uuid: String,
    pub alias: String,
    pub waypoint: Option<String>,
    pub hardcore: i64,
}

#[derive(Debug)]
pub struct Item {
    pub item_id: i64,
    pub parent_container_item_id: i64,
    pub item_definition_id: String,
    /// `u32::MAX` must fit inside this type
    pub stack_size: i64,
    pub position: String,
    pub properties: String,
}

pub struct Body {
    #[expect(dead_code)]
    pub body_id: i64,
    pub variant: String,
    pub body_data: String,
}

pub struct SkillGroup {
    pub entity_id: i64,
    pub skill_group_kind: String,
    pub earned_exp: i64,
    pub spent_exp: i64,
    pub skills: String,
    pub hash_val: Vec<u8>,
}

pub struct Pet {
    pub database_id: i64,
    pub name: String,
    pub body_variant: String,
    pub body_data: String,
}

pub struct AbilitySets {
    #[expect(dead_code)]
    pub entity_id: i64,
    pub ability_sets: String,
}
