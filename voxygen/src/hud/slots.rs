use super::{
    hotbar::{self, Slot as HotbarSlot},
    img_ids,
    item_imgs::{ItemImgs, ItemKey},
};
use crate::ui::slot::{self, SlotKey, SumSlot};
use common::comp::{
    self,
    controller::InputKind,
    item::{
        tool::{Tool, ToolKind},
        ItemKind,
    },
    slot::InvSlotId,
    AbilityPool, Body, Energy, Inventory, SkillSet,
};
use conrod_core::{image, Color};
use specs::Entity as EcsEntity;

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
pub struct InventorySlot {
    pub slot: InvSlotId,
    pub entity: EcsEntity,
    pub ours: bool,
}

impl SlotKey<Inventory, ItemImgs> for InventorySlot {
    type ImageKey = ItemKey;

    fn image_key(&self, source: &Inventory) -> Option<(Self::ImageKey, Option<Color>)> {
        source.get(self.slot).map(|i| (i.into(), None))
    }

    fn amount(&self, source: &Inventory) -> Option<u32> {
        source
            .get(self.slot)
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
    pub entity: EcsEntity,
    pub ours: bool,
}

impl SlotKey<Inventory, ItemImgs> for TradeSlot {
    type ImageKey = ItemKey;

    fn image_key(&self, source: &Inventory) -> Option<(Self::ImageKey, Option<Color>)> {
        self.invslot.and_then(|inv_id| {
            InventorySlot {
                slot: inv_id,
                ours: self.ours,
                entity: self.entity,
            }
            .image_key(source)
        })
    }

    fn amount(&self, source: &Inventory) -> Option<u32> {
        self.invslot
            .and_then(|inv_id| {
                InventorySlot {
                    slot: inv_id,
                    ours: self.ours,
                    entity: self.entity,
                }
                .amount(source)
            })
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
    SceptreAura,
}

type HotbarSource<'a> = (
    &'a hotbar::State,
    &'a Inventory,
    &'a Energy,
    &'a SkillSet,
    &'a AbilityPool,
    &'a Body,
);
type HotbarImageSource<'a> = (&'a ItemImgs, &'a img_ids::Imgs);

impl<'a> SlotKey<HotbarSource<'a>, HotbarImageSource<'a>> for HotbarSlot {
    type ImageKey = HotbarImage;

    fn image_key(
        &self,
        (hotbar, inventory, energy, skillset, ability_pool, body): &HotbarSource<'a>,
    ) -> Option<(Self::ImageKey, Option<Color>)> {
        hotbar.get(*self).and_then(|contents| match contents {
            hotbar::SlotContents::Inventory(idx) => inventory
                .get(idx)
                .map(|item| HotbarImage::Item(item.into()))
                .map(|i| (i, None)),
            hotbar::SlotContents::Ability(i) => {
                use comp::Ability;
                let tool_kind = ability_pool
                    .abilities
                    .get(i)
                    .and_then(|a| match a {
                        Ability::MainWeaponAbility(_) => Some(EquipSlot::ActiveMainhand),
                        Ability::OffWeaponAbility(_) => Some(EquipSlot::ActiveOffhand),
                        _ => None,
                    })
                    .and_then(|equip_slot| inventory.equipped(equip_slot))
                    .and_then(|item| match &item.kind {
                        ItemKind::Tool(Tool { kind, .. }) => Some(kind),
                        _ => None,
                    });
                tool_kind
                    .and_then(|kind| hotbar_image(*kind))
                    .and_then(|image| {
                        ability_pool
                            .activate_ability(
                                InputKind::Ability(i),
                                Some(inventory),
                                skillset,
                                body,
                            )
                            .map(|(ability, _)| {
                                (
                                    image,
                                    if energy.current() > ability.get_energy_cost() {
                                        Some(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                                    } else {
                                        Some(Color::Rgba(0.3, 0.3, 0.3, 0.8))
                                    },
                                )
                            })
                    })
            },
        })
    }

    fn amount(&self, (hotbar, inventory, _, _, _, _): &HotbarSource<'a>) -> Option<u32> {
        hotbar
            .get(*self)
            .and_then(|content| match content {
                hotbar::SlotContents::Inventory(idx) => inventory.get(idx),
                hotbar::SlotContents::Ability(_) => None,
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
            HotbarImage::SceptreAura => vec![imgs.skill_sceptre_aura],
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
        ToolKind::Sceptre => Some(HotbarImage::SceptreAura),
        _ => None,
    }
}
