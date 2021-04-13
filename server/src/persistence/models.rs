pub struct Character {
    pub character_id: i64,
    pub player_uuid: String,
    pub alias: String,
    pub waypoint: Option<String>,
}

#[derive(Debug)]
pub struct Item {
    pub item_id: i64,
    pub parent_container_item_id: i64,
    pub item_definition_id: String,
    pub stack_size: i32,
    pub position: String,
}

pub struct Body {
    pub body_id: i64,
    pub variant: String,
    pub body_data: String,
}

pub struct Skill {
    pub entity_id: i64,
    pub skill: String,
    pub level: Option<i32>,
}

pub struct SkillGroup {
    pub entity_id: i64,
    pub skill_group_kind: String,
    pub exp: i32,
    pub available_sp: i32,
    pub earned_sp: i32,
}
