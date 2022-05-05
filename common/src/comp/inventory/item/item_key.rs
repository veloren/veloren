use crate::{
    assets::AssetExt,
    comp::inventory::item::{
        armor::{Armor, ArmorKind},
        modular, Glider, ItemDef, ItemDefinitionId, ItemDesc, ItemKind, Lantern, Throwable,
        Utility,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ItemKey {
    Tool(String),
    ModularWeapon(modular::ModularWeaponKey),
    ModularWeaponComponent(modular::ModularWeaponComponentKey),
    Lantern(String),
    Glider(String),
    Armor(ArmorKind),
    Utility(Utility),
    Consumable(String),
    Throwable(Throwable),
    Ingredient(String),
    TagExamples(Vec<ItemKey>),
    Empty,
}

impl<T: ItemDesc> From<&T> for ItemKey {
    fn from(item_desc: &T) -> Self {
        let item_definition_id = item_desc.item_definition_id();

        match &*item_desc.kind() {
            ItemKind::Tool(_) => match item_definition_id {
                ItemDefinitionId::Simple(id) => ItemKey::Tool(id.to_string()),
                ItemDefinitionId::Modular { .. } => {
                    ItemKey::ModularWeapon(modular::weapon_to_key(item_desc))
                },
            },
            ItemKind::ModularComponent(mod_comp) => {
                use modular::ModularComponent;
                match mod_comp {
                    ModularComponent::ToolPrimaryComponent { .. } => {
                        if let Some(id) = item_definition_id.raw() {
                            match modular::weapon_component_to_key(id, item_desc.components()) {
                                Ok(key) => ItemKey::ModularWeaponComponent(key),
                                // TODO: Maybe use a different ItemKey?
                                Err(_) => ItemKey::Tool(id.to_owned()),
                            }
                        } else {
                            ItemKey::Empty
                        }
                    },
                    ModularComponent::ToolSecondaryComponent { .. } => {
                        if let Some(id) = item_definition_id.raw() {
                            // TODO: Maybe use a different ItemKey?
                            ItemKey::Tool(id.to_owned())
                        } else {
                            ItemKey::Empty
                        }
                    },
                }
            },
            ItemKind::Lantern(Lantern { kind, .. }) => ItemKey::Lantern(kind.clone()),
            ItemKind::Glider(Glider { kind, .. }) => ItemKey::Glider(kind.clone()),
            ItemKind::Armor(Armor { kind, .. }) => ItemKey::Armor(kind.clone()),
            ItemKind::Utility { kind, .. } => ItemKey::Utility(*kind),
            ItemKind::Consumable { .. } => {
                if let Some(id) = item_definition_id.raw() {
                    ItemKey::Consumable(id.to_owned())
                } else {
                    ItemKey::Empty
                }
            },
            ItemKind::Throwable { kind, .. } => ItemKey::Throwable(*kind),
            ItemKind::Ingredient { kind, .. } => ItemKey::Ingredient(kind.clone()),
            ItemKind::TagExamples { item_ids } => ItemKey::TagExamples(
                item_ids
                    .iter()
                    .map(|id| ItemKey::from(&*Arc::<ItemDef>::load_expect_cloned(id)))
                    .collect(),
            ),
        }
    }
}
