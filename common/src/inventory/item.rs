use specs::{Component, VecStorage};
use crate::comp::actor;

#[derive(Clone)]
pub enum Armor {
    Helmet(actor::Head),
    Shoulders(actor::Shoulder),
    Chestplate(actor::Chest),
    Belt(actor::Belt),
    Gloves(actor::Hand),
    Pants(actor::Pants),
    Boots(actor::Foot),
}

#[derive(Clone)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

#[derive(Clone)]
pub enum Item {
    Weapon { damage: i8, strength: i8, rarity: Rarity, variant: actor::Weapon},
    Armor { defense: i8, health_bonus: i8, rarity: Rarity, variant: Armor },
}

impl Component for Item {
    type Storage = VecStorage<Self>;
}