use super::{
    hotbar::{self, Slot as HotbarSlot},
    img_ids,
    item_imgs::ItemImgs,
    util,
};
use crate::ui::slot::{self, SlotKey, SumSlot};
use common::{
    comp::{
        ability::{Ability, AbilityInput, AuxiliaryAbility},
        item::tool::{AbilityContext, ToolKind},
        slot::InvSlotId,
        ActiveAbilities, Body, CharacterState, Combo, Energy, Inventory, Item, ItemKey, SkillSet,
        Stance,
    },
    recipe::ComponentRecipeBook,
};
use conrod_core::{image, Color};
use specs::Entity as EcsEntity;
use std::fmt::{Debug, Formatter};

pub use common::comp::slot::{ArmorSlot, EquipSlot};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SlotKind {
    Inventory(InventorySlot),
    Equip(EquipSlot),
    Hotbar(HotbarSlot),
    Trade(TradeSlot),
    Ability(AbilitySlot),
    Crafting(CraftSlot),
    /* Spellbook(SpellbookSlot), TODO */
}

pub type SlotManager = slot::SlotManager<SlotKind>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, PartialEq, Eq)]
pub enum HotbarImage {
    Item(ItemKey),
    Ability(String),
}

type HotbarSource<'a> = (
    &'a hotbar::State,
    &'a Inventory,
    &'a Energy,
    &'a SkillSet,
    Option<&'a ActiveAbilities>,
    &'a Body,
    AbilityContext,
    Option<&'a Combo>,
    Option<&'a CharacterState>,
    Option<&'a Stance>,
);
type HotbarImageSource<'a> = (&'a ItemImgs, &'a img_ids::Imgs);

impl<'a> SlotKey<HotbarSource<'a>, HotbarImageSource<'a>> for HotbarSlot {
    type ImageKey = HotbarImage;

    fn image_key(
        &self,
        (
            hotbar,
            inventory,
            energy,
            skillset,
            active_abilities,
            body,
            context,
            combo,
            char_state,
            stance,
        ): &HotbarSource<'a>,
    ) -> Option<(Self::ImageKey, Option<Color>)> {
        const GREYED_OUT: Color = Color::Rgba(0.3, 0.3, 0.3, 0.8);
        hotbar.get(*self).and_then(|contents| match contents {
            hotbar::SlotContents::Inventory(item_hash, item_key) => {
                let item = inventory.get_by_hash(item_hash);
                match item {
                    Some(item) => Some((HotbarImage::Item(item.into()), None)),
                    None => Some((HotbarImage::Item(item_key), Some(GREYED_OUT))),
                }
            },
            hotbar::SlotContents::Ability(i) => {
                let ability_id = active_abilities.and_then(|a| {
                    a.auxiliary_set(Some(inventory), Some(skillset))
                        .get(i)
                        .and_then(|a| {
                            Ability::from(*a).ability_id(Some(inventory), Some(skillset), *context)
                        })
                });

                ability_id
                    .map(|id| HotbarImage::Ability(id.to_string()))
                    .and_then(|image| {
                        active_abilities
                            .and_then(|a| {
                                a.activate_ability(
                                    AbilityInput::Auxiliary(i),
                                    Some(inventory),
                                    skillset,
                                    Some(body),
                                    *char_state,
                                    *context,
                                )
                            })
                            .map(|(ability, _)| {
                                (
                                    image,
                                    if energy.current() >= ability.energy_cost()
                                        && combo
                                            .map_or(false, |c| c.counter() >= ability.combo_cost())
                                        && ability
                                            .ability_meta()
                                            .requirements
                                            .requirements_met(*stance)
                                    {
                                        Some(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                                    } else {
                                        Some(GREYED_OUT)
                                    },
                                )
                            })
                    })
            },
        })
    }

    fn amount(&self, (hotbar, inventory, ..): &HotbarSource<'a>) -> Option<u32> {
        hotbar
            .get(*self)
            .and_then(|content| match content {
                hotbar::SlotContents::Inventory(item_hash, _) => inventory.get_by_hash(item_hash),
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
            HotbarImage::Ability(ability_id) => vec![util::ability_image(imgs, ability_id)],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilitySlot {
    Slot(usize),
    Ability(AuxiliaryAbility),
}

type AbilitiesSource<'a> = (
    &'a ActiveAbilities,
    &'a Inventory,
    &'a SkillSet,
    AbilityContext,
);

impl<'a> SlotKey<AbilitiesSource<'a>, img_ids::Imgs> for AbilitySlot {
    type ImageKey = String;

    fn image_key(
        &self,
        (active_abilities, inventory, skillset, context): &AbilitiesSource<'a>,
    ) -> Option<(Self::ImageKey, Option<Color>)> {
        let ability_id = match self {
            Self::Slot(index) => active_abilities
                .get_ability(
                    AbilityInput::Auxiliary(*index),
                    Some(inventory),
                    Some(skillset),
                )
                .ability_id(Some(inventory), Some(skillset), *context),
            Self::Ability(ability) => {
                Ability::from(*ability).ability_id(Some(inventory), Some(skillset), *context)
            },
        };

        ability_id.map(|id| (String::from(id), None))
    }

    fn amount(&self, _source: &AbilitiesSource) -> Option<u32> { None }

    fn image_ids(ability_id: &Self::ImageKey, imgs: &img_ids::Imgs) -> Vec<image::Id> {
        vec![util::ability_image(imgs, ability_id)]
    }
}

#[derive(Clone, Copy)]
pub struct CraftSlot {
    pub index: u32,
    pub invslot: Option<InvSlotId>,
    pub requirement: fn(&Item, &ComponentRecipeBook, Option<CraftSlotInfo>) -> bool,
    pub info: Option<CraftSlotInfo>,
}

#[derive(Clone, Copy, Debug)]
pub enum CraftSlotInfo {
    Tool(ToolKind),
}

impl PartialEq for CraftSlot {
    fn eq(&self, other: &Self) -> bool {
        (self.index, self.invslot) == (other.index, other.invslot)
    }
}

impl Debug for CraftSlot {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("CraftSlot")
            .field("index", &self.index)
            .field("invslot", &self.invslot)
            .field("requirement", &"fn ptr")
            .finish()
    }
}

impl SlotKey<Inventory, ItemImgs> for CraftSlot {
    type ImageKey = ItemKey;

    fn image_key(&self, source: &Inventory) -> Option<(Self::ImageKey, Option<Color>)> {
        self.invslot
            .and_then(|invslot| source.get(invslot))
            .map(|i| (i.into(), None))
    }

    fn amount(&self, source: &Inventory) -> Option<u32> {
        self.invslot
            .and_then(|invslot| source.get(invslot))
            .map(|item| item.amount())
            .filter(|amount| *amount > 1)
    }

    fn image_ids(key: &Self::ImageKey, source: &ItemImgs) -> Vec<image::Id> {
        source.img_ids_or_not_found_img(key.clone())
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

impl From<AbilitySlot> for SlotKind {
    fn from(ability: AbilitySlot) -> Self { Self::Ability(ability) }
}

impl From<CraftSlot> for SlotKind {
    fn from(craft: CraftSlot) -> Self { Self::Crafting(craft) }
}

impl SumSlot for SlotKind {
    fn drag_size(&self) -> Option<[f64; 2]> {
        Some(match self {
            Self::Ability(_) => [80.0; 2],
            _ => return None,
        })
    }
}
