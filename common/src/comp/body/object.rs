use crate::{
    comp::{item::Reagent, Density, Mass},
    consts::{IRON_DENSITY, WATER_DENSITY},
    make_case_elim,
};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use vek::Vec3;

make_case_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Body {
        Arrow = 0,
        Bomb = 1,
        Scarecrow = 2,
        Cauldron = 3,
        ChestVines = 4,
        Chest = 5,
        ChestDark = 6,
        ChestDemon = 7,
        ChestGold = 8,
        ChestLight = 9,
        ChestOpen = 10,
        ChestSkull = 11,
        Pumpkin = 12,
        Pumpkin2 = 13,
        Pumpkin3 = 14,
        Pumpkin4 = 15,
        Pumpkin5 = 16,
        Campfire = 17,
        LanternGround = 18,
        LanternGroundOpen = 19,
        LanternStanding2 = 20,
        LanternStanding = 21,
        PotionBlue = 22,
        PotionGreen = 23,
        PotionRed = 24,
        Crate = 25,
        Tent = 26,
        WindowSpooky = 27,
        DoorSpooky = 28,
        Anvil = 29,
        Gravestone = 30,
        Gravestone2 = 31,
        Bench = 32,
        Chair = 33,
        Chair2 = 34,
        Chair3 = 35,
        Table = 36,
        Table2 = 37,
        Table3 = 38,
        Drawer = 39,
        BedBlue = 40,
        Carpet = 41,
        Bedroll = 42,
        CarpetHumanRound = 43,
        CarpetHumanSquare = 44,
        CarpetHumanSquare2 = 45,
        CarpetHumanSquircle = 46,
        Pouch = 47,
        CraftingBench = 48,
        BoltFire = 49,
        ArrowSnake = 50,
        CampfireLit = 51,
        BoltFireBig = 52,
        TrainingDummy = 53,
        FireworkBlue = 54,
        FireworkGreen = 55,
        FireworkPurple = 56,
        FireworkRed = 57,
        FireworkWhite = 58,
        FireworkYellow = 59,
        MultiArrow = 60,
        BoltNature = 61,
        ToughMeat = 62,
        BeastMeat = 63,
        Crossbow = 64,
        ArrowTurret = 65,
        Coins = 66,
        GoldOre = 67,
        SilverOre = 68,
        ClayRocket = 69,
        HaniwaSentry = 70,
        SeaLantern = 71,
        Snowball = 72,
        BirdMeat = 73,
        FishMeat = 74,
        SmallMeat = 75,
        Tornado = 76,
        Apple = 77,
        Hive = 78,
        Coconut = 79,
        SpitPoison = 80,
        BoltIcicle = 81,
        Dart = 82,
        GnarlingTotemRed = 83,
        GnarlingTotemGreen = 84,
        GnarlingTotemWhite = 85,
        DagonBomb = 86,
        BarrelOrgan = 87,
        IceBomb = 88,
    }
);

impl Body {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        *ALL_OBJECTS.choose(&mut rng).unwrap()
    }
}

pub const ALL_OBJECTS: [Body; 89] = [
    Body::Arrow,
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
    Body::CampfireLit,
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
    Body::Pouch,
    Body::CraftingBench,
    Body::BoltFire,
    Body::BoltFireBig,
    Body::ArrowSnake,
    Body::TrainingDummy,
    Body::FireworkBlue,
    Body::FireworkGreen,
    Body::FireworkPurple,
    Body::FireworkRed,
    Body::FireworkWhite,
    Body::FireworkYellow,
    Body::MultiArrow,
    Body::BoltNature,
    Body::ToughMeat,
    Body::BeastMeat,
    Body::Crossbow,
    Body::ArrowTurret,
    Body::Coins,
    Body::SilverOre,
    Body::GoldOre,
    Body::ClayRocket,
    Body::HaniwaSentry,
    Body::SeaLantern,
    Body::Snowball,
    Body::BirdMeat,
    Body::FishMeat,
    Body::SmallMeat,
    Body::Tornado,
    Body::Apple,
    Body::Hive,
    Body::Coconut,
    Body::SpitPoison,
    Body::BoltIcicle,
    Body::Dart,
    Body::GnarlingTotemRed,
    Body::GnarlingTotemWhite,
    Body::GnarlingTotemGreen,
    Body::DagonBomb,
    Body::BarrelOrgan,
    Body::IceBomb,
];

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::Object(body) }
}

impl Body {
    pub fn to_string(self) -> &'static str {
        match self {
            Body::Arrow => "arrow",
            Body::Bomb => "bomb",
            Body::Scarecrow => "scarecrow",
            Body::Cauldron => "cauldron",
            Body::ChestVines => "chest_vines",
            Body::Chest => "chest",
            Body::ChestDark => "chest_dark",
            Body::ChestDemon => "chest_demon",
            Body::ChestGold => "chest_gold",
            Body::ChestLight => "chest_light",
            Body::ChestOpen => "chest_open",
            Body::ChestSkull => "chest_skull",
            Body::Pumpkin => "pumpkin",
            Body::Pumpkin2 => "pumpkin_2",
            Body::Pumpkin3 => "pumpkin_3",
            Body::Pumpkin4 => "pumpkin_4",
            Body::Pumpkin5 => "pumpkin_5",
            Body::Campfire => "campfire",
            Body::CampfireLit => "campfire_lit",
            Body::LanternGround => "lantern_ground",
            Body::LanternGroundOpen => "lantern_ground_open",
            Body::LanternStanding => "lantern_standing",
            Body::LanternStanding2 => "lantern_standing_2",
            Body::PotionRed => "potion_red",
            Body::PotionBlue => "potion_blue",
            Body::PotionGreen => "potion_green",
            Body::Crate => "crate",
            Body::Tent => "tent",
            Body::WindowSpooky => "window_spooky",
            Body::DoorSpooky => "door_spooky",
            Body::Anvil => "anvil",
            Body::Gravestone => "gravestone",
            Body::Gravestone2 => "gravestone_2",
            Body::Bench => "bench",
            Body::Chair => "chair",
            Body::Chair2 => "chair_2",
            Body::Chair3 => "chair_3",
            Body::Table => "table",
            Body::Table2 => "table_2",
            Body::Table3 => "table_3",
            Body::Drawer => "drawer",
            Body::BedBlue => "bed_blue",
            Body::Carpet => "carpet",
            Body::Bedroll => "bedroll",
            Body::CarpetHumanRound => "carpet_human_round",
            Body::CarpetHumanSquare => "carpet_human_square",
            Body::CarpetHumanSquare2 => "carpet_human_square_2",
            Body::CarpetHumanSquircle => "carpet_human_squircle",
            Body::Pouch => "pouch",
            Body::CraftingBench => "crafting_bench",
            Body::BoltFire => "bolt_fire",
            Body::BoltFireBig => "bolt_fire_big",
            Body::ArrowSnake => "arrow_snake",
            Body::TrainingDummy => "training_dummy",
            Body::FireworkBlue => "firework_blue",
            Body::FireworkGreen => "firework_green",
            Body::FireworkPurple => "firework_purple",
            Body::FireworkRed => "firework_red",
            Body::FireworkWhite => "firework_white",
            Body::FireworkYellow => "firework_yellow",
            Body::MultiArrow => "multi_arrow",
            Body::BoltNature => "bolt_nature",
            Body::ToughMeat => "tough_meat",
            Body::BeastMeat => "beast_meat",
            Body::Crossbow => "crossbow",
            Body::ArrowTurret => "arrow_turret",
            Body::Coins => "coins",
            Body::SilverOre => "silver_ore",
            Body::GoldOre => "gold_ore",
            Body::ClayRocket => "clay_rocket",
            Body::HaniwaSentry => "haniwa_sentry",
            Body::SeaLantern => "sea_lantern",
            Body::Snowball => "snowball",
            Body::BirdMeat => "bird_meat",
            Body::FishMeat => "fish_meat",
            Body::SmallMeat => "small_meat",
            Body::Tornado => "tornado",
            Body::Apple => "apple",
            Body::Hive => "hive",
            Body::Coconut => "coconut",
            Body::SpitPoison => "spit_poison",
            Body::BoltIcicle => "bolt_icicle",
            Body::Dart => "dart",
            Body::GnarlingTotemRed => "gnarling_totem_red",
            Body::GnarlingTotemGreen => "gnarling_totem_green",
            Body::GnarlingTotemWhite => "gnarling_totem_white",
            Body::DagonBomb => "dagon_bomb",
            Body::BarrelOrgan => "barrel_organ",
            Body::IceBomb => "ice_bomb",
        }
    }

    pub fn for_firework(reagent: Reagent) -> Body {
        match reagent {
            Reagent::Blue => Body::FireworkBlue,
            Reagent::Green => Body::FireworkGreen,
            Reagent::Purple => Body::FireworkPurple,
            Reagent::Red => Body::FireworkRed,
            Reagent::White => Body::FireworkWhite,
            Reagent::Yellow => Body::FireworkYellow,
        }
    }

    pub fn density(&self) -> Density {
        let density = match self {
            Body::Anvil | Body::Cauldron => IRON_DENSITY,
            Body::Arrow
            | Body::ArrowSnake
            | Body::ArrowTurret
            | Body::MultiArrow
            | Body::Dart
            | Body::DagonBomb => 500.0,
            Body::Bomb => 2000.0, // I have no idea what it's supposed to be
            Body::Crate => 300.0, // let's say it's a lot of wood and maybe some contents
            Body::Scarecrow => 900.0,
            Body::TrainingDummy => 2000.0,
            Body::Snowball => 0.9 * WATER_DENSITY,
            // let them sink
            _ => 1.1 * WATER_DENSITY,
        };

        Density(density)
    }

    pub fn mass(&self) -> Mass {
        let m = match self {
            // I think MultiArrow is one of several arrows, not several arrows combined?
            Body::Anvil => 100.0,
            Body::Arrow | Body::ArrowSnake | Body::ArrowTurret | Body::MultiArrow | Body::Dart => {
                0.003
            },
            Body::BedBlue => 50.0,
            Body::Bedroll => 3.0,
            Body::Bench => 100.0,
            Body::BoltFire
            | Body::BoltFireBig
            | Body::BoltNature
            | Body::BoltIcicle
            | Body::SpitPoison => 1.0,
            Body::Bomb | Body::DagonBomb => {
                0.5 * IRON_DENSITY * std::f32::consts::PI / 6.0 * self.dimensions().x.powi(3)
            },
            Body::Campfire | Body::CampfireLit | Body::BarrelOrgan => 300.0,
            Body::Carpet
            | Body::CarpetHumanRound
            | Body::CarpetHumanSquare
            | Body::CarpetHumanSquare2
            | Body::CarpetHumanSquircle => 10.0,
            Body::Cauldron => 5.0,
            Body::Chair | Body::Chair2 | Body::Chair3 => 10.0,
            Body::Chest
            | Body::ChestDark
            | Body::ChestDemon
            | Body::ChestGold
            | Body::ChestLight
            | Body::ChestOpen
            | Body::ChestSkull
            | Body::ChestVines => 100.0,
            Body::Coins => 1.0,
            Body::CraftingBench => 100.0,
            Body::Crate => 50.0,
            Body::Crossbow => 200.0,
            Body::DoorSpooky => 20.0,
            Body::Drawer => 50.0,
            Body::FireworkBlue
            | Body::FireworkGreen
            | Body::FireworkPurple
            | Body::FireworkRed
            | Body::FireworkWhite
            | Body::FireworkYellow => 1.0,
            Body::Gravestone => 100.0,
            Body::Gravestone2 => 100.0,
            Body::LanternGround
            | Body::LanternGroundOpen
            | Body::LanternStanding
            | Body::LanternStanding2 => 3.0,
            Body::ToughMeat => 50.0,
            Body::BeastMeat => 50.0,
            Body::PotionBlue | Body::PotionGreen | Body::PotionRed => 5.0,
            Body::Pouch => 1.0,
            Body::Pumpkin | Body::Pumpkin2 | Body::Pumpkin3 | Body::Pumpkin4 | Body::Pumpkin5 => {
                10.0
            },
            Body::Scarecrow => 50.0,
            Body::Table | Body::Table2 | Body::Table3 => 50.0,
            Body::Tent => 50.0,
            Body::TrainingDummy => 60.0,
            Body::WindowSpooky => 10.0,
            Body::SilverOre => 1000.0,
            Body::GoldOre => 1000.0,
            Body::ClayRocket => 50.0,
            Body::HaniwaSentry => 300.0,
            Body::SeaLantern => 1000.0,
            Body::Snowball => 7360.0, // 2.5 m diamter
            Body::FishMeat => 10.0,
            Body::BirdMeat => 10.0,
            Body::SmallMeat => 10.0,
            Body::Tornado => 50.0,
            Body::Apple => 2.0,
            Body::Hive => 2.0,
            Body::Coconut => 2.0,
            Body::GnarlingTotemRed | Body::GnarlingTotemGreen | Body::GnarlingTotemWhite => 100.0,
            Body::IceBomb => 12298.0, // 2.5 m diamter but ice
        };

        Mass(m)
    }

    pub fn dimensions(&self) -> Vec3<f32> {
        match self {
            Body::Arrow | Body::ArrowSnake | Body::MultiArrow | Body::ArrowTurret | Body::Dart => {
                Vec3::new(0.01, 0.8, 0.01)
            },
            Body::BoltFire => Vec3::new(0.1, 0.1, 0.1),
            Body::Crossbow => Vec3::new(3.0, 3.0, 1.5),
            Body::HaniwaSentry => Vec3::new(0.8, 0.8, 1.4),
            Body::SeaLantern => Vec3::new(0.8, 0.8, 1.4),
            Body::Snowball => Vec3::broadcast(2.5),
            Body::Tornado => Vec3::new(2.0, 2.0, 3.4),
            Body::TrainingDummy => Vec3::new(1.5, 1.5, 3.0),
            Body::GnarlingTotemRed | Body::GnarlingTotemGreen | Body::GnarlingTotemWhite => {
                Vec3::new(0.8, 0.8, 1.4)
            },
            Body::BarrelOrgan => Vec3::new(4.0, 2.0, 3.0),
            Body::IceBomb => Vec3::broadcast(2.5),
            // FIXME: this *must* be exhaustive match
            _ => Vec3::broadcast(0.5),
        }
    }
}
