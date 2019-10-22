use crate::{
    assets::{self, Asset},
    effect::Effect,
    terrain::{Block, BlockKind},
};
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use std::fs::File;
use std::io::BufReader;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tool {
    Dagger,
    Shield,
    Sword,
    Axe,
    Hammer,
    Bow,
    Staff,
    Debug(Debug),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Debug {
    Boost,
    Possess,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Armor {
    // TODO: Don't make armor be a body part. Wearing enemy's head is funny but also a creepy thing to do.
    Helmet,
    Shoulders,
    Chestplate,
    Belt,
    Gloves,
    Pants,
    Boots,
    Back,
    Tabard,
    Gem,
    Necklace,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Consumable {
    Apple,
    Potion,
    Mushroom,
    Velorite,
    VeloriteFrag,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ingredient {
    Flower,
    Grass,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    Tool { kind: Tool, power: u32 },
    Armor { kind: Armor, power: u32 },
    Consumable { kind: Consumable, effect: Effect },
    Ingredient(Ingredient),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Item {
    name: String,
    description: String,
    pub kind: ItemKind,
}

impl Asset for Item {
    const ENDINGS: &'static [&'static str] = &["ron"];
    fn parse(buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        Ok(ron::de::from_reader(buf_reader).unwrap())
    }
}

impl Item {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn try_reclaim_from_block(block: Block) -> Option<Self> {
        match block.kind() {
            BlockKind::Apple => Some(assets::load_expect_cloned("common.items.apple")),
            BlockKind::Mushroom => Some(assets::load_expect_cloned("common.items.mushroom")),
            BlockKind::Velorite => Some(assets::load_expect_cloned("common.items.velorite")),
            BlockKind::BlueFlower => Some(assets::load_expect_cloned("common.items.flowers.blue")),
            BlockKind::PinkFlower => Some(assets::load_expect_cloned("common.items.flowers.pink")),
            BlockKind::PurpleFlower => {
                Some(assets::load_expect_cloned("common.items.flowers.purple"))
            }
            BlockKind::RedFlower => Some(assets::load_expect_cloned("common.items.flowers.red")),
            BlockKind::WhiteFlower => {
                Some(assets::load_expect_cloned("common.items.flowers.white"))
            }
            BlockKind::YellowFlower => {
                Some(assets::load_expect_cloned("common.items.flowers.yellow"))
            }
            BlockKind::Sunflower => Some(assets::load_expect_cloned("common.items.flowers.sun")),
            BlockKind::LongGrass => Some(assets::load_expect_cloned("common.items.grasses.long")),
            BlockKind::MediumGrass => {
                Some(assets::load_expect_cloned("common.items.grasses.medium"))
            }
            BlockKind::ShortGrass => Some(assets::load_expect_cloned("common.items.grasses.short")),
            BlockKind::Chest => Some(match rand::random::<usize>() % 4 {
                0 => assets::load_expect_cloned("common.items.apple"),
                1 => assets::load_expect_cloned("common.items.velorite"),
                // TODO: Implement random asset loading
                //2 => Item::Tool {
                //    kind: *(&ALL_TOOLS).choose(&mut rand::thread_rng()),
                //    power: 8 + rand::random::<u32>() % (rand::random::<u32>() % 29 + 1),
                //    stamina: 0,
                //    strength: 0,
                //    dexterity: 0,
                //    intelligence: 0,
                //},
                2 => assets::load_expect_cloned("common.items.apple"),
                3 => assets::load_expect_cloned("common.items.veloritefrag"),
                _ => unreachable!(),
            }),
            _ => None,
        }
    }
}

impl Component for Item {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
