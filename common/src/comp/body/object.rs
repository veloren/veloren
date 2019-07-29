use bimap::BiMap;
use rand::{seq::SliceRandom, thread_rng};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Bomb,
    Scarecrow,
    Cauldron,
    ChestVines,
    Chest,
    ChestDark,
    ChestDemon,
    ChestGold,
    ChestLight,
    ChestOpen,
    ChestSkull,
    Pumpkin,
    Pumpkin2,
    Pumpkin3,
    Pumpkin4,
    Pumpkin5,
    Campfire,
    LanternGround,
    LanternGroundOpen,
    LanternStanding,
    LanternStanding2,
    PotionRed,
    PotionGreen,
    PotionBlue,
    Crate,
    Tent,
    WindowSpooky,
    DoorSpooky,
    Anvil,
    Gravestone,
    Gravestone2,
    Bench,
    Chair,
    Chair2,
    Chair3,
    Table,
    Table2,
    Table3,
    Drawer,
    BedBlue,
    Carpet,
    Bedroll,
    CarpetHumanRound,
    CarpetHumanSquare,
    CarpetHumanSquare2,
    CarpetHumanSquircle,
}

pub struct BodyElements<L: Eq, R: Eq>(pub BiMap<L, R>);

impl BodyElements<&str, Body> {
    pub fn new() -> Self {
        let mut elements = BiMap::new();
        elements.insert("bomb", Body::Bomb);
        elements.insert("scarecrow", Body::Scarecrow);
        elements.insert("cauldron", Body::Cauldron);
        elements.insert("chest_vines", Body::ChestVines);
        elements.insert("chest", Body::Chest);
        elements.insert("chest_dark", Body::ChestDark);
        elements.insert("chest_demon", Body::ChestDemon);
        elements.insert("chest_gold", Body::ChestGold);
        elements.insert("chest_light", Body::ChestLight);
        elements.insert("chest_open", Body::ChestOpen);
        elements.insert("chest_skull", Body::ChestSkull);
        elements.insert("pumpkin", Body::Pumpkin);
        elements.insert("pumpkin_2", Body::Pumpkin2);
        elements.insert("pumpkin_3", Body::Pumpkin3);
        elements.insert("pumpkin_4", Body::Pumpkin4);
        elements.insert("pumpkin_5", Body::Pumpkin5);
        elements.insert("campfire", Body::Campfire);
        elements.insert("lantern_ground", Body::LanternGround);
        elements.insert("lantern_ground_open", Body::LanternGroundOpen);
        elements.insert("lantern", Body::LanternStanding);
        elements.insert("lantern_2", Body::LanternStanding2);
        elements.insert("potion_red", Body::PotionRed);
        elements.insert("potion_green", Body::PotionGreen);
        elements.insert("potion_blue", Body::PotionBlue);
        elements.insert("crate", Body::Crate);
        elements.insert("tent", Body::Tent);
        elements.insert("window_spooky", Body::WindowSpooky);
        elements.insert("door_spooky", Body::DoorSpooky);
        elements.insert("anvil", Body::Anvil);
        elements.insert("gravestone", Body::Gravestone);
        elements.insert("gravestone_2", Body::Gravestone2);
        elements.insert("bench", Body::Bench);
        elements.insert("chair", Body::Chair);
        elements.insert("chair_2", Body::Chair2);
        elements.insert("chair_3", Body::Chair3);
        elements.insert("table_human", Body::Table);
        elements.insert("table_human_2", Body::Table2);
        elements.insert("table_human_3", Body::Table3);
        elements.insert("drawer", Body::Drawer);
        elements.insert("bed_human_blue", Body::BedBlue);
        elements.insert("carpet", Body::Carpet);
        elements.insert("bedroll", Body::Bedroll);
        elements.insert("carpet_human_round", Body::CarpetHumanRound);
        elements.insert("carpet_human_square", Body::CarpetHumanSquare);
        elements.insert("carpet_human_square_2", Body::CarpetHumanSquare2);
        elements.insert("carpet_human_squircle", Body::CarpetHumanSquircle);
        BodyElements(elements)
    }
    pub fn get_body(&self, request_object: &str) -> Option<Body> {
        self.0.get_by_left(&request_object).map(|x| x.to_owned())
    }
    pub fn to_string(&self, request_body: &Body) -> Option<String> {
        self.0.get_by_right(&request_body).map(|s| s.to_string())
    }
}

impl Body {
    pub fn all() -> [Body; 46] {
        ALL_OBJECTS
    }
}

#[cfg(test)]
mod body_tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_body_enum_to_string_and_string_to_enum() {
        let body = BodyElements::new();
        for (enum_str, body_value) in &body.0 {
            let cmp_enum_body = body
                .get_body(&enum_str)
                .expect(format!("Should be able to find: {}", enum_str).as_ref());
            let cmp_enum_str_from_body = body
                .to_string(&body_value)
                .expect("Should find string from Body");

            assert_eq!(body_value, &cmp_enum_body);
            assert_eq!(cmp_enum_str_from_body, enum_str.to_string());
        }
    }
}

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        *(&ALL_OBJECTS).choose(&mut rng).unwrap()
    }
}

const ALL_OBJECTS: [Body; 46] = [
    Body::Bomb,
    Body::Scarecrow,
    Body::Cauldron,
    Body::ChestVines,
    Body::Chest,
    Body::ChestDark,
    Body::ChestDemon,
    Body::ChestGold,
    Body::ChestLight,
    Body::ChestOpen,
    Body::ChestSkull,
    Body::Pumpkin,
    Body::Pumpkin2,
    Body::Pumpkin3,
    Body::Pumpkin4,
    Body::Pumpkin5,
    Body::Campfire,
    Body::LanternGround,
    Body::LanternGroundOpen,
    Body::LanternStanding,
    Body::LanternStanding2,
    Body::PotionRed,
    Body::PotionBlue,
    Body::PotionGreen,
    Body::Crate,
    Body::Tent,
    Body::WindowSpooky,
    Body::DoorSpooky,
    Body::Anvil,
    Body::Gravestone,
    Body::Gravestone2,
    Body::Bench,
    Body::Chair,
    Body::Chair2,
    Body::Chair3,
    Body::Table,
    Body::Table2,
    Body::Table3,
    Body::Drawer,
    Body::BedBlue,
    Body::Carpet,
    Body::Bedroll,
    Body::CarpetHumanRound,
    Body::CarpetHumanSquare,
    Body::CarpetHumanSquare2,
    Body::CarpetHumanSquircle,
];
