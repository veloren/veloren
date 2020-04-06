use super::item_imgs::{ItemImgs, ItemKey};
use crate::ui::slot::{ContentKey, SlotKinds, SlotManager};
use common::comp::{item::ItemKind, Inventory, Loadout};
use conrod_core::image;

#[derive(Clone, Copy, PartialEq)]
pub enum HudSlotKinds {
    Inventory(InventorySlot),
    Armor(ArmorSlot),
    Hotbar(HotbarSlot),
    //Spellbook(SpellbookSlot), TODO
}

pub type HudSlotManager = SlotManager<HudSlotKinds>;

#[derive(Clone, Copy, PartialEq)]
pub struct InventorySlot(pub usize);

#[derive(Clone, Copy, PartialEq)]
pub enum ArmorSlot {
    Head,
    Neck,
    Shoulders,
    Chest,
    Hands,
    Ring,
    Lantern,
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
}

impl ContentKey for ArmorSlot {
    type ContentSource = Loadout;
    type ImageKey = ItemKey;
    type ImageSource = ItemImgs;

    fn image_key(&self, source: &Self::ContentSource) -> Option<Self::ImageKey> {
        let item = match self {
            ArmorSlot::Shoulders => source.shoulder.as_ref(),
            ArmorSlot::Chest => source.chest.as_ref(),
            ArmorSlot::Belt => source.belt.as_ref(),
            ArmorSlot::Hands => source.hand.as_ref(),
            ArmorSlot::Legs => source.pants.as_ref(),
            ArmorSlot::Feet => source.foot.as_ref(),
            ArmorSlot::Back => source.back.as_ref(),
            ArmorSlot::Ring => source.ring.as_ref(),
            ArmorSlot::Neck => source.neck.as_ref(),
            ArmorSlot::Head => source.head.as_ref(),
            ArmorSlot::Lantern => source.lantern.as_ref(),
            ArmorSlot::Tabard => source.tabard.as_ref(),
            ArmorSlot::Mainhand => source.active_item.as_ref().map(|i| &i.item),
            ArmorSlot::Offhand => source.second_item.as_ref().map(|i| &i.item),
            _ => None,
        };

        item.map(Into::into)
    }

    fn amount(&self, _: &Self::ContentSource) -> Option<u32> { None }

    fn image_id(key: &Self::ImageKey, source: &Self::ImageSource) -> image::Id {
        source.img_id_or_not_found_img(key.clone())
    }
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
