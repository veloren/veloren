use crate::comp::actor;
use specs::{Component, VecStorage};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Armor {
    //TODO: Don't make armor be a body part. Wearing enemy's head is funny but also creepy thing to do.
    Helmet(actor::Head),
    Shoulders(actor::Shoulder),
    ChestPlate(actor::Chest),
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
        damage: i32,
        strength: i32,
        rarity: Rarity,
        variant: actor::Weapon,
    },
    Armor {
        defense: i32,
        health_bonus: i32,
        rarity: Rarity,
        variant: Armor,
    },
}

impl Component for Item {
    type Storage = VecStorage<Self>;
}
