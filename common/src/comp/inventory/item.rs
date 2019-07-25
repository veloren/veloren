use specs::Component;
use specs_idvs::IDVStorage;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Weapon {
    Daggers,
    SwordShield,
    Sword,
    Axe,
    Hammer,
    Bow,
    Staff,
}
pub const ALL_WEAPONS: [Weapon; 7] = [
    Weapon::Daggers,
    Weapon::SwordShield,
    Weapon::Sword,
    Weapon::Axe,
    Weapon::Hammer,
    Weapon::Bow,
    Weapon::Staff,
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Item {
    Weapon {
        kind: Weapon,
        damage: i32,
        strength: i32,
    },
    Armor {
        kind: Armor,
        defense: i32,
        health_bonus: i32,
    },
}

impl Default for Item {
    fn default() -> Self {
        Item::Weapon {
            kind: Weapon::Hammer,
            damage: 0,
            strength: 0,
        }
    }
}

impl Component for Item {
    type Storage = IDVStorage<Self>;
}
