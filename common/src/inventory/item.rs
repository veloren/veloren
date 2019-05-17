use specs::{Component, VecStorage};
use crate::comp::actor;

#[Derive(Clone)]
pub enum Armor {
    Helmet: Head,
    Shoulders: Shoulder,
    Chestplate: Chest,
    Belt: Belt;
    Gloves: Hand,
    Pants: Pants,
    Boots: Foot,
}

#[Derive(Clone)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

#[Derive(Clone)]
pub enum Item {
    Weapon { damage: i8, strength: i8, rarity: Rarity, variant: Weapon},
    Armor { defense: i8, health_bonus: i8, rarity: Rarity, variant: Armor },
}

impl Component for Item {
    type Storage = VecStorage<Self>;
}