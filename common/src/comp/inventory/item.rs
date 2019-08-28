use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tool {
    Daggers,
    SwordShield,
    Sword,
    Axe,
    Hammer,
    Bow,
    Staff,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Daggers => "daggers",
            Tool::SwordShield => "sword and shield",
            Tool::Sword => "sword",
            Tool::Axe => "axe",
            Tool::Hammer => "hammer",
            Tool::Bow => "bow",
            Tool::Staff => "staff",
        }
    }
}

pub const ALL_TOOLS: [Tool; 7] = [
    Tool::Daggers,
    Tool::SwordShield,
    Tool::Sword,
    Tool::Axe,
    Tool::Hammer,
    Tool::Bow,
    Tool::Staff,
];

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Armor {
    // TODO: Don't make armor be a body part. Wearing enemy's head is funny but also creepy thing to do.
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
            Armor::Helmet => "helmet",
            Armor::Shoulders => "shoulder pads",
            Armor::Chestplate => "chestplate",
            Armor::Belt => "belt",
            Armor::Gloves => "gloves",
            Armor::Pants => "pants",
            Armor::Boots => "boots",
            Armor::Back => "back",
            Armor::Tabard => "tabard",
            Armor::Gem => "gem",
            Armor::Necklace => "necklace",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsumptionEffect {
    Health(i32),
    Xp(i32),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Debug {
    Teleport,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Item {
    Tool {
        kind: Tool,
        power: u32,
    },
    Armor {
        kind: Armor,
        defense: i32,
        health_bonus: i32,
    },
    Consumable {
        effect: ConsumptionEffect,
    },
    Ingredient,
    Debug(Debug),
}

impl Item {
    pub fn name(&self) -> &'static str {
        match self {
            Item::Tool { kind, .. } => kind.name(),
            Item::Armor { kind, .. } => kind.name(),
            Item::Consumable { .. } => "<consumable>",
            Item::Ingredient => "<ingredient>",
            Item::Debug(_) => "Debugging item",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Item::Tool { .. } => "tool",
            Item::Armor { .. } => "armour",
            Item::Consumable { .. } => "consumable",
            Item::Ingredient => "ingredient",
            Item::Debug(_) => "debug",
        }
    }

    pub fn description(&self) -> String {
        format!("{} ({})", self.name(), self.category())
    }
}

impl Default for Item {
    fn default() -> Self {
        Item::Tool {
            kind: Tool::Hammer,
            power: 0,
        }
    }
}

impl Component for Item {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
