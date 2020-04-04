use super::item_imgs::{ItemImgs, ItemKey};
use crate::ui::slot::{ContentKey, SlotKinds, SlotManager};
use common::comp::{item::ItemKind, Inventory};
use conrod_core::image;
use vek::*;

#[derive(Clone, Copy, PartialEq)]
pub enum HudSlotKinds {
    Inventory(InventorySlot),
    Armor(ArmorSlot),
    Hotbar(HotbarSlot),
}

pub type HudSlotManager = SlotManager<HudSlotKinds>;

#[derive(Clone, Copy, PartialEq)]
pub struct InventorySlot(pub usize);

#[derive(Clone, Copy, PartialEq)]
pub enum ArmorSlot {
    Helmet,
    Neck,
    Shoulders,
    Chest,
    Hands,
    LeftRing,
    RightRing,
    Back,
    Belt,
    Legs,
    Feet,
    Mainhand,
    Offhand,
    Tabard,
}

#[derive(Clone, Copy, PartialEq)]
pub enum HotbarSlot {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
}

impl ContentKey for InventorySlot {
    type ContentSource = Inventory;
    type ImageKey = ItemKey;
    type ImageSource = ItemImgs;

    fn image_key(&self, source: &Self::ContentSource) -> Option<Self::ImageKey> {
        source.get(self.0).map(Into::into)
    }

    fn amount(&self, source: &Self::ContentSource) -> Option<u32> {
        source
            .get(self.0)
            .and_then(|item| match item.kind {
                ItemKind::Tool { .. } | ItemKind::Armor { .. } => None,
                ItemKind::Utility { amount, .. }
                | ItemKind::Consumable { amount, .. }
                | ItemKind::Ingredient { amount, .. } => Some(amount),
            })
            .filter(|amount| *amount > 1)
    }

    fn image_id(key: &Self::ImageKey, source: &Self::ImageSource) -> image::Id {
        source.img_id_or_not_found_img(key.clone())
    }

    fn back_icon(&self, _: &Self::ImageSource) -> Option<(image::Id, Vec2<f32>)> { None }
}

impl From<InventorySlot> for HudSlotKinds {
    fn from(inventory: InventorySlot) -> Self { Self::Inventory(inventory) }
}

impl From<ArmorSlot> for HudSlotKinds {
    fn from(armor: ArmorSlot) -> Self { Self::Armor(armor) }
}

impl From<HotbarSlot> for HudSlotKinds {
    fn from(hotbar: HotbarSlot) -> Self { Self::Hotbar(hotbar) }
}

impl SlotKinds for HudSlotKinds {}
