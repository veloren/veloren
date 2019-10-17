use crate::{
    comp,
    effect::Effect,
    terrain::{Block, BlockKind},
};
use rand::prelude::*;
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tool {
    Dagger,
    Shield,
    Sword,
    Axe,
    Hammer,
    Bow,
    Staff,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Dagger => "Dagger",
            Tool::Shield => "Shield",
            Tool::Sword => "Sword",
            Tool::Axe => "Axe",
            Tool::Hammer => "Hammer",
            Tool::Bow => "Bow",
            Tool::Staff => "Staff",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Tool::Dagger => {
                "A basic kitchen knife.\n\
                 NOT YET AVAILABLE."
            }
            Tool::Shield => {
                "This shield belonged to many adventurers.\n\
                 Now it's yours.\n\
                 NOT YET AVAILABLE."
            }
            Tool::Sword => "When closing one eye it's nearly like it wasn't rusty at all!",
            Tool::Axe => {
                "It has a name written on it.\n\
                 Sounds dwarvish."
            }
            Tool::Hammer => "Use with caution around nails.",
            Tool::Bow => "An old but sturdy hunting bow.",
            Tool::Staff => {
                "A carved stick.\n\
                 The wood smells like magic.\n\
                 NOT YET AVAILABLE."
            }
        }
    }
}

pub const ALL_TOOLS: [Tool; 7] = [
    Tool::Dagger,
    Tool::Shield,
    Tool::Sword,
    Tool::Axe,
    Tool::Hammer,
    Tool::Bow,
    Tool::Staff,
];

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

impl Armor {
    pub fn name(&self) -> &'static str {
        match self {
            Armor::Helmet => "Helmet",
            Armor::Shoulders => "Shoulder Pads",
            Armor::Chestplate => "Chestplate",
            Armor::Belt => "Belt",
            Armor::Gloves => "Gloves",
            Armor::Pants => "Pants",
            Armor::Boots => "Boots",
            Armor::Back => "Back",
            Armor::Tabard => "Tabard",
            Armor::Gem => "Gem",
            Armor::Necklace => "Necklace",
        }
    }

    pub fn description(&self) -> &'static str {
        self.name()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Consumable {
    Apple,
    Potion,
    Mushroom,
    Velorite,
    VeloriteFrag,
}

impl Consumable {
    pub fn name(&self) -> &'static str {
        match self {
            Consumable::Apple => "Apple",
            Consumable::Potion => "Potion",
            Consumable::Mushroom => "Mushroom",
            Consumable::Velorite => "Velorite",
            Consumable::VeloriteFrag => "Glowing Fragment",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Consumable::Apple => "A tasty Apple.",
            Consumable::Potion => "This Potion contains the essence of Life.",
            Consumable::Mushroom => "A common Mushroom.",
            Consumable::Velorite => "Has a subtle turqoise glow.",
            Consumable::VeloriteFrag => "Seems to be the fragment of a bigger piece...",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ingredient {
    Flower,
    Grass,
}

impl Ingredient {
    pub fn name(&self) -> &'static str {
        match self {
            Ingredient::Flower => "Flower",
            Ingredient::Grass => "Grass",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Ingredient::Flower => "It smells great.",
            Ingredient::Grass => "Greener than an orc's snout.",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Debug {
    Boost,
    Possess,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Item {
    Tool {
        kind: Tool,
        power: u32,
        stamina: i32,
        strength: i32,
        dexterity: i32,
        intelligence: i32,
    },
    Armor {
        kind: Armor,
        stamina: i32,
        strength: i32,
        dexterity: i32,
        intelligence: i32,
    },
    Consumable {
        kind: Consumable,
        effect: Effect,
    },
    Ingredient {
        kind: Ingredient,
    },
    Debug(Debug),
}

impl Item {
    pub fn name(&self) -> &'static str {
        match self {
            Item::Tool { kind, .. } => kind.name(),
            Item::Armor { kind, .. } => kind.name(),
            Item::Consumable { kind, .. } => kind.name(),
            Item::Ingredient { kind, .. } => kind.name(),
            Item::Debug(_) => "Debugging item",
        }
    }

    pub fn title(&self) -> String {
        format!("{} ({})", self.name(), self.category())
    }

    pub fn info(&self) -> String {
        match self {
            Item::Tool { power, .. } => format!("{:+} attack", power),
            Item::Armor { .. } => String::new(),
            Item::Consumable { effect, .. } => format!("{}", effect.info()),
            Item::Ingredient { .. } => String::new(),
            Item::Debug(_) => format!("+99999 insanity"),
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Item::Tool { .. } => "Tool",
            Item::Armor { .. } => "Armor",
            Item::Consumable { .. } => "Consumable",
            Item::Ingredient { .. } => "Ingredient",
            Item::Debug(_) => "Debug",
        }
    }

    pub fn description(&self) -> String {
        match self {
            Item::Tool { kind, .. } => format!("{}", kind.description()),
            Item::Armor { kind, .. } => format!("{}", kind.description()),
            Item::Consumable { kind, .. } => format!("{}", kind.description()),
            Item::Ingredient { kind, .. } => format!("{}", kind.description()),
            Item::Debug(_) => format!("Debugging item"),
        }
    }

    pub fn try_reclaim_from_block(block: Block) -> Option<Self> {
        match block.kind() {
            BlockKind::Apple => Some(Self::apple()),
            BlockKind::Mushroom => Some(Self::mushroom()),
            BlockKind::Velorite => Some(Self::velorite()),
            BlockKind::BlueFlower => Some(Self::flower()),
            BlockKind::PinkFlower => Some(Self::flower()),
            BlockKind::PurpleFlower => Some(Self::flower()),
            BlockKind::RedFlower => Some(Self::flower()),
            BlockKind::WhiteFlower => Some(Self::flower()),
            BlockKind::YellowFlower => Some(Self::flower()),
            BlockKind::Sunflower => Some(Self::flower()),
            BlockKind::LongGrass => Some(Self::grass()),
            BlockKind::MediumGrass => Some(Self::grass()),
            BlockKind::ShortGrass => Some(Self::grass()),
            BlockKind::Chest => Some(match rand::random::<usize>() % 4 {
                0 => Self::apple(),
                1 => Self::velorite(),
                2 => Item::Tool {
                    kind: *(&ALL_TOOLS).choose(&mut rand::thread_rng()).unwrap(),
                    power: 8 + rand::random::<u32>() % (rand::random::<u32>() % 29 + 1),
                    stamina: 0,
                    strength: 0,
                    dexterity: 0,
                    intelligence: 0,
                },
                3 => Self::veloritefrag(),
                _ => unreachable!(),
            }),
            _ => None,
        }
    }

    // General item constructors

    pub fn apple() -> Self {
        Item::Consumable {
            kind: Consumable::Apple,
            effect: Effect::Health(comp::HealthChange {
                amount: 50,
                cause: comp::HealthSource::Item,
            }),
        }
    }

    pub fn mushroom() -> Self {
        Item::Consumable {
            kind: Consumable::Mushroom,
            effect: Effect::Health(comp::HealthChange {
                amount: 10,
                cause: comp::HealthSource::Item,
            }),
        }
    }

    pub fn velorite() -> Self {
        Item::Consumable {
            kind: Consumable::Velorite,
            effect: Effect::Xp(50),
        }
    }
    pub fn veloritefrag() -> Self {
        Item::Consumable {
            kind: Consumable::VeloriteFrag,
            effect: Effect::Xp(20),
        }
    }

    pub fn flower() -> Self {
        Item::Ingredient {
            kind: Ingredient::Flower,
        }
    }

    pub fn grass() -> Self {
        Item::Ingredient {
            kind: Ingredient::Grass,
        }
    }
}

impl Default for Item {
    fn default() -> Self {
        Item::Tool {
            kind: Tool::Hammer,
            power: 0,
            stamina: 0,
            strength: 0,
            dexterity: 0,
            intelligence: 0,
        }
    }
}

impl Component for Item {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
