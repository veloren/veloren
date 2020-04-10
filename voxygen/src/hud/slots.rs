use super::item_imgs::{ItemImgs, ItemKey};
use crate::ui::slot::{self, SlotKey, SumSlot};
use common::comp::{item::ItemKind, Inventory, Loadout};
use conrod_core::image;

pub use common::comp::slot::{ArmorSlot, EquipSlot};

#[derive(Clone, Copy, PartialEq)]
pub enum SlotKind {
    Inventory(InventorySlot),
    Equip(EquipSlot),
    /*Hotbar(HotbarSlot),
     *Spellbook(SpellbookSlot), TODO */
}

pub type SlotManager = slot::SlotManager<SlotKind>;

#[derive(Clone, Copy, PartialEq)]
pub struct InventorySlot(pub usize);

/*#[derive(Clone, Copy, PartialEq)]
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
}*/

impl SlotKey<Inventory, ItemImgs> for InventorySlot {
    type ImageKey = ItemKey;

    fn image_key(&self, source: &Inventory) -> Option<Self::ImageKey> {
        source.get(self.0).map(Into::into)
    }

    fn amount(&self, source: &Inventory) -> Option<u32> {
        source
            .get(self.0)
            .and_then(|item| match item.kind {
                ItemKind::Tool { .. } | ItemKind::Lantern(_) | ItemKind::Armor { .. } => None,
                ItemKind::Utility { amount, .. }
                | ItemKind::Consumable { amount, .. }
                | ItemKind::Ingredient { amount, .. } => Some(amount),
            })
            .filter(|amount| *amount > 1)
    }

    fn image_id(key: &Self::ImageKey, source: &ItemImgs) -> image::Id {
        source.img_id_or_not_found_img(key.clone())
    }
}

impl SlotKey<Loadout, ItemImgs> for EquipSlot {
    type ImageKey = ItemKey;

    fn image_key(&self, source: &Loadout) -> Option<Self::ImageKey> {
        let item = match self {
            EquipSlot::Armor(ArmorSlot::Shoulders) => source.shoulder.as_ref(),
            EquipSlot::Armor(ArmorSlot::Chest) => source.chest.as_ref(),
            EquipSlot::Armor(ArmorSlot::Belt) => source.belt.as_ref(),
            EquipSlot::Armor(ArmorSlot::Hands) => source.hand.as_ref(),
            EquipSlot::Armor(ArmorSlot::Legs) => source.pants.as_ref(),
            EquipSlot::Armor(ArmorSlot::Feet) => source.foot.as_ref(),
            EquipSlot::Armor(ArmorSlot::Back) => source.back.as_ref(),
            EquipSlot::Armor(ArmorSlot::Ring) => source.ring.as_ref(),
            EquipSlot::Armor(ArmorSlot::Neck) => source.neck.as_ref(),
            EquipSlot::Armor(ArmorSlot::Head) => source.head.as_ref(),
            EquipSlot::Armor(ArmorSlot::Tabard) => source.tabard.as_ref(),
            EquipSlot::Mainhand => source.active_item.as_ref().map(|i| &i.item),
            EquipSlot::Offhand => source.second_item.as_ref().map(|i| &i.item),
            EquipSlot::Lantern => source.lantern.as_ref(),
        };

        item.map(Into::into)
    }

    fn amount(&self, _: &Loadout) -> Option<u32> { None }

    fn image_id(key: &Self::ImageKey, source: &ItemImgs) -> image::Id {
        source.img_id_or_not_found_img(key.clone())
    }
}

/*impl SlotKey<Hotbar, ItemImgs> for HotbarSlot {
    type ImageKey = ItemKey;

    fn image_key(&self, source: &Inventory) -> Option<Self::ImageKey> {
        source.get(self.0).map(Into::into)
    }

    fn amount(&self, source: &Inventory) -> Option<u32> {
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

    fn image_id(key: &Self::ImageKey, source: &ItemImgs) -> image::Id {
        source.img_id_or_not_found_img(key.clone())
    }
}*/

impl From<InventorySlot> for SlotKind {
    fn from(inventory: InventorySlot) -> Self { Self::Inventory(inventory) }
}

impl From<EquipSlot> for SlotKind {
    fn from(equip: EquipSlot) -> Self { Self::Equip(equip) }
}

//impl From<HotbarSlot> for SlotKind {
//    fn from(hotbar: HotbarSlot) -> Self { Self::Hotbar(hotbar) }
//}

impl SumSlot for SlotKind {}
