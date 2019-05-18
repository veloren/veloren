use crate::comp::actor;
use specs::{Component, VecStorage};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Armor {
    Helmet(actor::Head),
    Shoulders(actor::Shoulder),
    Chestplate(actor::Chest),
    Belt(actor::Belt),
    Gloves(actor::Hand),
    Pants(actor::Pants),
    Boots(actor::Foot),
    Back,
    Tabard,
    Gem,
    Necklace,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Item {
    Weapon {
        damage: i16,
        strength: i16,
        rarity: Rarity,
        variant: actor::Weapon,
    },
    Armor {
        defense: i16,
        health_bonus: i16,
        rarity: Rarity,
        variant: Armor,
    },
}

impl Component for Item {
    type Storage = VecStorage<Self>;
}
