use super::{
    hotbar::{self, Slot as HotbarSlot},
    img_ids,
    item_imgs::{ItemImgs, ItemKey},
};
use crate::ui::slot::{self, SlotKey, SumSlot};
use common::comp::{
    item::{
        tool::{AbilityMap, Hands, ToolKind},
        ItemKind,
    },
    slot::InvSlotId,
    Energy, Inventory,
};
use conrod_core::{image, Color};

pub use common::comp::slot::{ArmorSlot, EquipSlot};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SlotKind {
    Inventory(InventorySlot),
    Equip(EquipSlot),
    Hotbar(HotbarSlot),
    Trade(TradeSlot),
    /* Spellbook(SpellbookSlot), TODO */
}

pub type SlotManager = slot::SlotManager<SlotKind>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InventorySlot(pub InvSlotId);

impl SlotKey<Inventory, ItemImgs> for InventorySlot {
    type ImageKey = ItemKey;

    fn image_key(&self, source: &Inventory) -> Option<(Self::ImageKey, Option<Color>)> {
        source.get(self.0).map(|i| (i.into(), None))
    }

    fn amount(&self, source: &Inventory) -> Option<u32> {
        source
            .get(self.0)
            .map(|item| item.amount())
            .filter(|amount| *amount > 1)
    }

    fn image_ids(key: &Self::ImageKey, source: &ItemImgs) -> Vec<image::Id> {
        source.img_ids_or_not_found_img(key.clone())
    }
}

impl SlotKey<Inventory, ItemImgs> for EquipSlot {
    type ImageKey = ItemKey;

    fn image_key(&self, source: &Inventory) -> Option<(Self::ImageKey, Option<Color>)> {
        let item = source.equipped(*self);
        item.map(|i| (i.into(), None))
    }

    fn amount(&self, _: &Inventory) -> Option<u32> { None }

    fn image_ids(key: &Self::ImageKey, source: &ItemImgs) -> Vec<image::Id> {
        source.img_ids_or_not_found_img(key.clone())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TradeSlot {
    pub index: usize,
    pub quantity: u32,
    pub invslot: Option<InvSlotId>,
}

impl SlotKey<Inventory, ItemImgs> for TradeSlot {
    type ImageKey = ItemKey;

    fn image_key(&self, source: &Inventory) -> Option<(Self::ImageKey, Option<Color>)> {
        self.invslot
            .and_then(|inv_id| InventorySlot(inv_id).image_key(source))
    }

    fn amount(&self, source: &Inventory) -> Option<u32> {
        self.invslot
            .and_then(|inv_id| InventorySlot(inv_id).amount(source))
            .map(|x| x.min(self.quantity))
    }

    fn image_ids(key: &Self::ImageKey, source: &ItemImgs) -> Vec<image::Id> {
        source.img_ids_or_not_found_img(key.clone())
    }
}

#[derive(Clone, PartialEq)]
pub enum HotbarImage {
    Item(ItemKey),
    FireAoe,
    SnakeArrow,
    SwordWhirlwind,
    HammerLeap,
    AxeLeapSlash,
    BowJumpBurst,
}

type HotbarSource<'a> = (&'a hotbar::State, &'a Inventory, &'a Energy, &'a AbilityMap);
type HotbarImageSource<'a> = (&'a ItemImgs, &'a img_ids::Imgs);

impl<'a> SlotKey<HotbarSource<'a>, HotbarImageSource<'a>> for HotbarSlot {
    type ImageKey = HotbarImage;

    fn image_key(
        &self,
        (hotbar, inventory, energy, ability_map): &HotbarSource<'a>,
    ) -> Option<(Self::ImageKey, Option<Color>)> {
        hotbar.get(*self).and_then(|contents| match contents {
            hotbar::SlotContents::Inventory(idx) => inventory
                .get(idx)
                .map(|item| HotbarImage::Item(item.into()))
                .map(|i| (i, None)),
            hotbar::SlotContents::Ability3 => {
                let tool = match inventory
                    .equipped(EquipSlot::Mainhand)
                    .map(|i| (i, i.kind()))
                {
                    Some((item, ItemKind::Tool(tool))) => Some((item, tool)),
                    _ => None,
                };

                tool.and_then(|(item, tool)| {
                    hotbar_image(tool.kind).map(|i| {
                        (
                            i,
                            if let Some(skill) = tool
                                .get_abilities(item.components(), ability_map)
                                .abilities
                                .get(0)
                            {
                                if energy.current() >= skill.1.get_energy_cost() {
                                    Some(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                                } else {
                                    Some(Color::Rgba(0.3, 0.3, 0.3, 0.8))
                                }
                            } else {
                                Some(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                            },
                        )
                    })
                })
            },
            hotbar::SlotContents::Ability4 => {
                let hands = |equip_slot| match inventory.equipped(equip_slot).map(|i| i.kind()) {
                    Some(ItemKind::Tool(tool)) => Some(tool.hands),
                    _ => None,
                };

                let active_tool_hands = hands(EquipSlot::Mainhand);
                let second_tool_hands = hands(EquipSlot::Offhand);

                let (equip_slot, skill_index) = match (active_tool_hands, second_tool_hands) {
                    (Some(Hands::Two), _) => (Some(EquipSlot::Mainhand), 1),
                    (_, Some(Hands::One)) => (Some(EquipSlot::Offhand), 0),
                    (Some(Hands::One), _) => (Some(EquipSlot::Mainhand), 1),
                    (_, _) => (None, 0),
                };

                let tool =
                    match equip_slot.and_then(|es| inventory.equipped(es).map(|i| (i, i.kind()))) {
                        Some((item, ItemKind::Tool(tool))) => Some((item, tool)),
                        _ => None,
                    };

                tool.and_then(|(item, tool)| {
                    hotbar_image(tool.kind).map(|i| {
                        (
                            i,
                            if let Some(skill) = tool
                                .get_abilities(item.components(), ability_map)
                                .abilities
                                .get(skill_index)
                            {
                                if energy.current() >= skill.1.get_energy_cost() {
                                    Some(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                                } else {
                                    Some(Color::Rgba(0.3, 0.3, 0.3, 0.8))
                                }
                            } else {
                                Some(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                            },
                        )
                    })
                })
            },
        })
    }

    fn amount(&self, (hotbar, inventory, _, _): &HotbarSource<'a>) -> Option<u32> {
        hotbar
            .get(*self)
            .and_then(|content| match content {
                hotbar::SlotContents::Inventory(idx) => inventory.get(idx),
                hotbar::SlotContents::Ability3 => None,
                hotbar::SlotContents::Ability4 => None,
            })
            .map(|item| item.amount())
            .filter(|amount| *amount > 1)
    }

    fn image_ids(
        key: &Self::ImageKey,
        (item_imgs, imgs): &HotbarImageSource<'a>,
    ) -> Vec<image::Id> {
        match key {
            HotbarImage::Item(key) => item_imgs.img_ids_or_not_found_img(key.clone()),
            HotbarImage::SnakeArrow => vec![imgs.snake_arrow_0],
            HotbarImage::FireAoe => vec![imgs.fire_aoe],
            HotbarImage::SwordWhirlwind => vec![imgs.sword_whirlwind],
            HotbarImage::HammerLeap => vec![imgs.hammerleap],
            HotbarImage::AxeLeapSlash => vec![imgs.skill_axe_leap_slash],
            HotbarImage::BowJumpBurst => vec![imgs.skill_bow_jump_burst],
        }
    }
}

impl From<InventorySlot> for SlotKind {
    fn from(inventory: InventorySlot) -> Self { Self::Inventory(inventory) }
}

impl From<EquipSlot> for SlotKind {
    fn from(equip: EquipSlot) -> Self { Self::Equip(equip) }
}

impl From<HotbarSlot> for SlotKind {
    fn from(hotbar: HotbarSlot) -> Self { Self::Hotbar(hotbar) }
}
impl From<TradeSlot> for SlotKind {
    fn from(trade: TradeSlot) -> Self { Self::Trade(trade) }
}

impl SumSlot for SlotKind {}

fn hotbar_image(tool: ToolKind) -> Option<HotbarImage> {
    match tool {
        ToolKind::Staff => Some(HotbarImage::FireAoe),
        ToolKind::Hammer => Some(HotbarImage::HammerLeap),
        ToolKind::Axe => Some(HotbarImage::AxeLeapSlash),
        ToolKind::Bow => Some(HotbarImage::BowJumpBurst),
        ToolKind::Debug => Some(HotbarImage::SnakeArrow),
        ToolKind::Sword => Some(HotbarImage::SwordWhirlwind),
        _ => None,
    }
}
