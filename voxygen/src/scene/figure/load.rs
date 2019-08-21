use crate::{
    mesh::Meshable,
    render::{FigurePipeline, Mesh},
};
use common::{
    assets,
    comp::{
        humanoid::{Belt, BodyType, Chest, Foot, Hand, Pants, Race, Shoulder},
        item::Tool,
        object, quadruped, quadruped_medium, Item,
    },
    figure::Segment,
};
use dot_vox::DotVoxData;
use vek::*;

pub fn load_segment(mesh_name: &str) -> Segment {
    let full_specifier: String = ["voxygen.voxel.", mesh_name].concat();
    Segment::from(assets::load_expect::<DotVoxData>(full_specifier.as_str()).as_ref())
}
pub fn load_mesh(mesh_name: &str, position: Vec3<f32>) -> Mesh<FigurePipeline> {
    Meshable::<FigurePipeline, FigurePipeline>::generate_mesh(&load_segment(mesh_name), position).0
}

pub fn load_head(race: Race, body_type: BodyType) -> Mesh<FigurePipeline> {
    let (name, offset) = match (race, body_type) {
        // z-value should be 0.25 of the total z
        (Race::Human, BodyType::Male) => {
            ("figure.head.head_human_male", Vec3::new(-7.0, -5.0, -2.25))
        }
        (Race::Human, BodyType::Female) => (
            "figure.head.head_human_female",
            Vec3::new(-7.0, -7.5, -3.25),
        ),
        (Race::Elf, BodyType::Male) => ("figure.head.head_elf_male", Vec3::new(-8.0, -5.0, -2.25)),
        (Race::Elf, BodyType::Female) => {
            ("figure.head.head_elf_female", Vec3::new(-8.0, -5.5, -3.0))
        }
        (Race::Dwarf, BodyType::Male) => {
            ("figure.head.head_dwarf_male", Vec3::new(-6.0, -5.0, -12.5))
        }
        (Race::Dwarf, BodyType::Female) => (
            "figure.head.head_dwarf_female",
            Vec3::new(-6.0, -6.0, -9.25),
        ),
        (Race::Orc, BodyType::Male) => ("figure.head.head_orc_male", Vec3::new(-8.0, -5.0, -2.50)),
        (Race::Orc, BodyType::Female) => {
            ("figure.head.head_orc_female", Vec3::new(-8.0, -8.0, -3.5))
        }
        (Race::Undead, BodyType::Male) => {
            ("figure.head.head_undead_male", Vec3::new(-5.5, -5.0, -2.5))
        }
        (Race::Undead, BodyType::Female) => (
            "figure.head.head_undead_female",
            Vec3::new(-6.0, -5.0, -2.5),
        ),
        (Race::Danari, BodyType::Male) => {
            ("figure.head.head_danari_male", Vec3::new(-9.0, -5.0, -2.75))
        }
        (Race::Danari, BodyType::Female) => (
            "figure.head.head_danari_female",
            Vec3::new(-9.0, -7.5, -3.0),
        ),
    };
    load_mesh(name, offset)
}
// loads models with different offsets
//    pub fn load_beard(beard: Beard) -> Mesh<FigurePipeline> {
//        let (name, offset) = match beard {
//            Beard::None => ("figure/body/empty", Vec3::new(0.0, 0.0, 0.0)),
//            Beard::Human1 => ("figure/empty", Vec3::new(0.0, 0.0, 0.0)),
//        };
//        load_mesh(name, offset)
//    }

pub fn load_chest(chest: Chest) -> Mesh<FigurePipeline> {
    let color = match chest {
        Chest::Brown => (125, 53, 0),
        Chest::Dark => (0, 38, 43),
        Chest::Green => (0, 255, 34),
        Chest::Orange => (255, 106, 0),
        Chest::Blue => (0, 38, 255),
    };

    let bare_chest = load_segment("figure.body.chest");
    let chest_armor = load_segment("armor.chest.generic");
    let chest = bare_chest.union(&chest_armor.chromify(Rgb::from(color)), Vec3::new(0, 0, 0));

    Meshable::<FigurePipeline, FigurePipeline>::generate_mesh(&chest, Vec3::new(-6.0, -3.5, 0.0)).0
}

pub fn load_belt(belt: Belt) -> Mesh<FigurePipeline> {
    load_mesh(
        match belt {
            //Belt::Default => "figure/body/belt_male",
            Belt::Dark => "armor.belt.belt_dark",
        },
        Vec3::new(-5.0, -3.5, 0.0),
    )
}

pub fn load_pants(pants: Pants) -> Mesh<FigurePipeline> {
    load_mesh(
        match pants {
            Pants::Blue => "armor.pants.pants_blue",
            Pants::Brown => "armor.pants.pants_brown",
            Pants::Dark => "armor.pants.pants_dark",
            Pants::Green => "armor.pants.pants_green",
            Pants::Orange => "armor.pants.pants_orange",
        },
        Vec3::new(-5.0, -3.5, 0.0),
    )
}

pub fn load_left_hand(hand: Hand) -> Mesh<FigurePipeline> {
    load_mesh(
        match hand {
            Hand::Default => "figure.body.hand",
        },
        Vec3::new(-2.0, -2.5, -2.0),
    )
}

pub fn load_right_hand(hand: Hand) -> Mesh<FigurePipeline> {
    load_mesh(
        match hand {
            Hand::Default => "figure.body.hand",
        },
        Vec3::new(-2.0, -2.5, -2.0),
    )
}

pub fn load_left_foot(foot: Foot) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot {
            Foot::Dark => "armor.foot.foot_dark",
        },
        Vec3::new(-2.5, -3.5, -9.0),
    )
}

pub fn load_right_foot(foot: Foot) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot {
            Foot::Dark => "armor.foot.foot_dark",
        },
        Vec3::new(-2.5, -3.5, -9.0),
    )
}

pub fn load_main(item: Option<&Item>) -> Mesh<FigurePipeline> {
    if let Some(item) = item {
        let (name, offset) = match item {
            Item::Tool { kind, .. } => match kind {
                Tool::Sword => ("weapon.sword.rusty_2h", Vec3::new(-1.5, -6.5, -4.0)),
                Tool::Axe => ("weapon.axe.rusty_2h", Vec3::new(-1.5, -5.0, -4.0)),
                Tool::Hammer => ("weapon.hammer.rusty_2h", Vec3::new(-2.5, -5.5, -4.0)),
                Tool::Daggers => ("weapon.hammer.rusty_2h", Vec3::new(-2.5, -5.5, -4.0)),
                Tool::SwordShield => ("weapon.axe.rusty_2h", Vec3::new(-2.5, -6.5, -2.0)),
                Tool::Bow => ("weapon.hammer.rusty_2h", Vec3::new(-2.5, -5.5, -4.0)),
                Tool::Staff => ("weapon.axe.rusty_2h", Vec3::new(-2.5, -6.5, -2.0)),
            },
            Item::Debug(_) => ("weapon.debug_wand", Vec3::new(-1.5, -9.5, -4.0)),
            _ => ("figure.empty", Vec3::default()),
        };
        load_mesh(name, offset)
    } else {
        load_mesh("figure.empty", Vec3::default())
    }
}

pub fn load_left_shoulder(shoulder: Shoulder) -> Mesh<FigurePipeline> {
    load_mesh(
        match shoulder {
            Shoulder::None => "figure.empty",
            Shoulder::Brown1 => "armor.shoulder.shoulder_l_brown",
        },
        Vec3::new(-2.5, -3.5, -1.5),
    )
}

pub fn load_right_shoulder(shoulder: Shoulder) -> Mesh<FigurePipeline> {
    load_mesh(
        match shoulder {
            Shoulder::None => "figure.empty",
            Shoulder::Brown1 => "armor.shoulder.shoulder_r_brown",
        },
        Vec3::new(-2.5, -3.5, -1.5),
    )
}

// TODO: Inventory
pub fn load_draw() -> Mesh<FigurePipeline> {
    load_mesh("object.glider", Vec3::new(-26.0, -26.0, -5.0))
}

//pub fn load_right_equip(hand: Hand) -> Mesh<FigurePipeline> {
//    load_mesh(
//        match hand {
//            Hand::Default => "figure/body/hand",
//        },
//        Vec3::new(-2.0, -2.5, -5.0),
//    )
//}

/////////
pub fn load_pig_head(head: quadruped::Head) -> Mesh<FigurePipeline> {
    load_mesh(
        match head {
            quadruped::Head::Default => "npc.pig_purple.pig_head",
        },
        Vec3::new(-6.0, 4.5, 3.0),
    )
}

pub fn load_pig_chest(chest: quadruped::Chest) -> Mesh<FigurePipeline> {
    load_mesh(
        match chest {
            quadruped::Chest::Default => "npc.pig_purple.pig_chest",
        },
        Vec3::new(-5.0, 4.5, 0.0),
    )
}

pub fn load_pig_leg_lf(leg_l: quadruped::LegL) -> Mesh<FigurePipeline> {
    load_mesh(
        match leg_l {
            quadruped::LegL::Default => "npc.pig_purple.pig_leg_l",
        },
        Vec3::new(0.0, -1.0, -1.5),
    )
}

pub fn load_pig_leg_rf(leg_r: quadruped::LegR) -> Mesh<FigurePipeline> {
    load_mesh(
        match leg_r {
            quadruped::LegR::Default => "npc.pig_purple.pig_leg_r",
        },
        Vec3::new(0.0, -1.0, -1.5),
    )
}

pub fn load_pig_leg_lb(leg_l: quadruped::LegL) -> Mesh<FigurePipeline> {
    load_mesh(
        match leg_l {
            quadruped::LegL::Default => "npc.pig_purple.pig_leg_l",
        },
        Vec3::new(0.0, -1.0, -1.5),
    )
}

pub fn load_pig_leg_rb(leg_r: quadruped::LegR) -> Mesh<FigurePipeline> {
    load_mesh(
        match leg_r {
            quadruped::LegR::Default => "npc.pig_purple.pig_leg_r",
        },
        Vec3::new(0.0, -1.0, -1.5),
    )
}
//////
pub fn load_wolf_head_upper(upper_head: quadruped_medium::HeadUpper) -> Mesh<FigurePipeline> {
    load_mesh(
        match upper_head {
            quadruped_medium::HeadUpper::Default => "npc.wolf.wolf_head_upper",
        },
        Vec3::new(-7.0, -6.0, -5.5),
    )
}

pub fn load_wolf_jaw(jaw: quadruped_medium::Jaw) -> Mesh<FigurePipeline> {
    load_mesh(
        match jaw {
            quadruped_medium::Jaw::Default => "npc.wolf.wolf_jaw",
        },
        Vec3::new(-3.0, -3.0, -2.5),
    )
}

pub fn load_wolf_head_lower(head_lower: quadruped_medium::HeadLower) -> Mesh<FigurePipeline> {
    load_mesh(
        match head_lower {
            quadruped_medium::HeadLower::Default => "npc.wolf.wolf_head_lower",
        },
        Vec3::new(-7.0, -6.0, -5.5),
    )
}

pub fn load_wolf_tail(tail: quadruped_medium::Tail) -> Mesh<FigurePipeline> {
    load_mesh(
        match tail {
            quadruped_medium::Tail::Default => "npc.wolf.wolf_tail",
        },
        Vec3::new(-2.0, -12.0, -5.0),
    )
}

pub fn load_wolf_torso_back(torso_back: quadruped_medium::TorsoBack) -> Mesh<FigurePipeline> {
    load_mesh(
        match torso_back {
            quadruped_medium::TorsoBack::Default => "npc.wolf.wolf_torso_back",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

pub fn load_wolf_torso_mid(torso_mid: quadruped_medium::TorsoMid) -> Mesh<FigurePipeline> {
    load_mesh(
        match torso_mid {
            quadruped_medium::TorsoMid::Default => "npc.wolf.wolf_torso_mid",
        },
        Vec3::new(-8.0, -5.5, -6.0),
    )
}

pub fn load_wolf_ears(ears: quadruped_medium::Ears) -> Mesh<FigurePipeline> {
    load_mesh(
        match ears {
            quadruped_medium::Ears::Default => "npc.wolf.wolf_ears",
        },
        Vec3::new(-4.0, -1.0, -1.0),
    )
}

pub fn load_wolf_foot_lf(foot_lf: quadruped_medium::FootLF) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot_lf {
            quadruped_medium::FootLF::Default => "npc.wolf.wolf_foot_lf",
        },
        Vec3::new(-2.5, -4.0, -2.5),
    )
}

pub fn load_wolf_foot_rf(foot_rf: quadruped_medium::FootRF) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot_rf {
            quadruped_medium::FootRF::Default => "npc.wolf.wolf_foot_rf",
        },
        Vec3::new(-2.5, -4.0, -2.5),
    )
}

pub fn load_wolf_foot_lb(foot_lb: quadruped_medium::FootLB) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot_lb {
            quadruped_medium::FootLB::Default => "npc.wolf.wolf_foot_lb",
        },
        Vec3::new(-2.5, -4.0, -2.5),
    )
}

pub fn load_wolf_foot_rb(foot_rb: quadruped_medium::FootRB) -> Mesh<FigurePipeline> {
    load_mesh(
        match foot_rb {
            quadruped_medium::FootRB::Default => "npc.wolf.wolf_foot_rb",
        },
        Vec3::new(-2.5, -4.0, -2.5),
    )
}

pub fn load_object(obj: object::Body) -> Mesh<FigurePipeline> {
    use object::Body;

    let (name, offset) = match obj {
        Body::Bomb => ("object.bomb", Vec3::new(-5.5, -5.5, 0.0)),
        Body::Scarecrow => ("object.scarecrow", Vec3::new(-9.5, -4.0, 0.0)),
        Body::Cauldron => ("object.cauldron", Vec3::new(-10.0, -10.0, 0.0)),
        Body::ChestVines => ("object.chest_vines", Vec3::new(-7.5, -6.0, 0.0)),
        Body::Chest => ("object.chest", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestDark => ("object.chest_dark", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestDemon => ("object.chest_demon", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestGold => ("object.chest_gold", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestLight => ("object.chest_light", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestOpen => ("object.chest_open", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestSkull => ("object.chest_skull", Vec3::new(-7.5, -6.0, 0.0)),
        Body::Pumpkin => ("object.pumpkin", Vec3::new(-5.5, -4.0, 0.0)),
        Body::Pumpkin2 => ("object.pumpkin_2", Vec3::new(-5.0, -4.0, 0.0)),
        Body::Pumpkin3 => ("object.pumpkin_3", Vec3::new(-5.0, -4.0, 0.0)),
        Body::Pumpkin4 => ("object.pumpkin_4", Vec3::new(-5.0, -4.0, 0.0)),
        Body::Pumpkin5 => ("object.pumpkin_5", Vec3::new(-4.0, -5.0, 0.0)),
        Body::Campfire => ("object.campfire", Vec3::new(-9.0, -10.0, 0.0)),
        Body::LanternGround => ("object.lantern_ground", Vec3::new(-3.5, -3.5, 0.0)),
        Body::LanternGroundOpen => ("object.lantern_ground_open", Vec3::new(-3.5, -3.5, 0.0)),
        Body::LanternStanding => ("object.lantern_standing", Vec3::new(-7.5, -3.5, 0.0)),
        Body::LanternStanding2 => ("object.lantern_standing_2", Vec3::new(-11.5, -3.5, 0.0)),
        Body::PotionRed => ("object.potion_red", Vec3::new(-2.0, -2.0, 0.0)),
        Body::PotionBlue => ("object.potion_blue", Vec3::new(-2.0, -2.0, 0.0)),
        Body::PotionGreen => ("object.potion_green", Vec3::new(-2.0, -2.0, 0.0)),
        Body::Crate => ("object.crate", Vec3::new(-7.0, -7.0, 0.0)),
        Body::Tent => ("object.tent", Vec3::new(-18.5, -19.5, 0.0)),
        Body::WindowSpooky => ("object.window_spooky", Vec3::new(-15.0, -1.5, -1.0)),
        Body::DoorSpooky => ("object.door_spooky", Vec3::new(-15.0, -4.5, 0.0)),
        Body::Table => ("object.table", Vec3::new(-12.0, -8.0, 0.0)),
        Body::Table2 => ("object.table_2", Vec3::new(-8.0, -8.0, 0.0)),
        Body::Table3 => ("object.table_3", Vec3::new(-10.0, -10.0, 0.0)),
        Body::Drawer => ("object.drawer", Vec3::new(-11.0, -7.5, 0.0)),
        Body::BedBlue => ("object.bed_human_blue", Vec3::new(-11.0, -15.0, 0.0)),
        Body::Anvil => ("object.anvil", Vec3::new(-3.0, -7.0, 0.0)),
        Body::Gravestone => ("object.gravestone", Vec3::new(-5.0, -2.0, 0.0)),
        Body::Gravestone2 => ("object.gravestone_2", Vec3::new(-8.5, -3.0, 0.0)),
        Body::Chair => ("object.chair", Vec3::new(-5.0, -4.5, 0.0)),
        Body::Chair2 => ("object.chair_2", Vec3::new(-5.0, -4.5, 0.0)),
        Body::Chair3 => ("object.chair_3", Vec3::new(-5.0, -4.5, 0.0)),
        Body::Bench => ("object.bench", Vec3::new(-8.8, -5.0, 0.0)),
        Body::Carpet => ("object.carpet", Vec3::new(-14.0, -14.0, -0.5)),
        Body::Bedroll => ("object.bedroll", Vec3::new(-11.0, -19.5, -0.5)),
        Body::CarpetHumanRound => ("object.carpet_human_round", Vec3::new(-14.0, -14.0, -0.5)),
        Body::CarpetHumanSquare => ("object.carpet_human_square", Vec3::new(-13.5, -14.0, -0.5)),
        Body::CarpetHumanSquare2 => (
            "object.carpet_human_square_2",
            Vec3::new(-13.5, -14.0, -0.5),
        ),
        Body::CarpetHumanSquircle => (
            "object.carpet_human_squircle",
            Vec3::new(-21.0, -21.0, -0.5),
        ),
        Body::Pouch => ("object.pouch", Vec3::new(-5.5, -4.5, 0.0)),
    };
    load_mesh(name, offset)
}
