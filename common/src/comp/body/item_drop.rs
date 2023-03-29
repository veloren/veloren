use crate::{
    comp::{
        item::{
            armor::ArmorKind,
            tool::{Tool, ToolKind},
            Item, ItemKind, Utility,
        },
        Density, Mass, Ori,
    },
    consts::WATER_DENSITY,
    make_case_elim,
    util::Dir,
};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use vek::Vec3;

make_case_elim!(
    armor,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum ItemDropArmorKind {
        Shoulder = 0,
        Chest = 1,
        Belt = 2,
        Hand = 3,
        Pants = 4,
        Foot = 5,
        Back = 6,
        Ring = 7,
        Neck = 8,
        Head = 9,
        Tabard = 10,
        Bag = 11,
    }
);

make_case_elim!(
    body,
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum Body {
        Tool(tool: ToolKind) = 0,
        ModularComponent = 1,
        Lantern = 2,
        Glider = 3,
        Armor(armor: ItemDropArmorKind) = 4,
        Utility = 5,
        Consumable = 6,
        Throwable = 7,
        Ingredient = 8,
        Coins = 9,
        CoinPouch = 10,
        Empty = 11,
    }
);

impl From<Body> for super::Body {
    fn from(body: Body) -> Self { super::Body::ItemDrop(body) }
}

impl From<&Item> for Body {
    fn from(item: &Item) -> Self {
        match &*item.kind() {
            ItemKind::Tool(Tool { kind, .. }) => Body::Tool(*kind),
            ItemKind::ModularComponent(_) => Body::ModularComponent,
            ItemKind::Lantern(_) => Body::Lantern,
            ItemKind::Glider => Body::Glider,
            ItemKind::Armor(armor) => match armor.kind {
                ArmorKind::Shoulder => Body::Armor(ItemDropArmorKind::Shoulder),
                ArmorKind::Chest => Body::Armor(ItemDropArmorKind::Chest),
                ArmorKind::Belt => Body::Armor(ItemDropArmorKind::Belt),
                ArmorKind::Hand => Body::Armor(ItemDropArmorKind::Hand),
                ArmorKind::Pants => Body::Armor(ItemDropArmorKind::Pants),
                ArmorKind::Foot => Body::Armor(ItemDropArmorKind::Foot),
                ArmorKind::Back => Body::Armor(ItemDropArmorKind::Back),
                ArmorKind::Ring => Body::Armor(ItemDropArmorKind::Ring),
                ArmorKind::Neck => Body::Armor(ItemDropArmorKind::Neck),
                ArmorKind::Head => Body::Armor(ItemDropArmorKind::Head),
                ArmorKind::Tabard => Body::Armor(ItemDropArmorKind::Tabard),
                ArmorKind::Bag => Body::Armor(ItemDropArmorKind::Bag),
            },
            ItemKind::Utility { kind, .. } => match kind {
                Utility::Coins => {
                    if item.amount() > 100 {
                        Body::CoinPouch
                    } else {
                        Body::Coins
                    }
                },
                _ => Body::Utility,
            },
            ItemKind::Consumable { .. } => Body::Consumable,
            ItemKind::Throwable { .. } => Body::Throwable,
            ItemKind::Ingredient { .. } => Body::Ingredient,
            _ => Body::Empty,
        }
    }
}

impl Body {
    pub fn to_string(self) -> &'static str {
        match self {
            Body::Tool(_) => "tool",
            Body::ModularComponent => "modular_component",
            Body::Lantern => "lantern",
            Body::Glider => "glider",
            Body::Armor(_) => "armor",
            Body::Utility => "utility",
            Body::Consumable => "consumable",
            Body::Throwable => "throwable",
            Body::Ingredient => "ingredient",
            Body::Coins => "coins",
            Body::CoinPouch => "coin_pouch",
            Body::Empty => "empty",
        }
    }

    pub fn density(&self) -> Density { Density(1.1 * WATER_DENSITY) }

    pub fn mass(&self) -> Mass { Mass(2.0) }

    pub fn dimensions(&self) -> Vec3<f32> { Vec3::new(0.0, 0.1, 0.0) }

    pub fn orientation(&self, rng: &mut impl Rng) -> Ori {
        let random = rng.gen_range(-1.0..1.0f32);
        let default = Ori::default();
        match self {
            Body::Tool(_) => default
                .pitched_down(PI / 2.0)
                .yawed_left(PI / 2.0)
                .pitched_towards(
                    Dir::from_unnormalized(Vec3::new(
                        random,
                        rng.gen_range(-1.0..1.0f32),
                        rng.gen_range(-1.0..1.0f32),
                    ))
                    .unwrap_or_default(),
                ),
            Body::Armor(kind) => match kind {
                ItemDropArmorKind::Neck | ItemDropArmorKind::Back | ItemDropArmorKind::Tabard => {
                    default.yawed_left(random).pitched_down(PI / 2.0)
                },
                _ => default.yawed_left(random),
            },
            _ => default.yawed_left(random),
        }
    }
}
