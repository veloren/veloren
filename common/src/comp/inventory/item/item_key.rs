use crate::{
    assets::AssetExt,
    comp::inventory::item::{modular, ItemDef, ItemDefinitionId, ItemDesc, ItemKind},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ItemKey {
    Simple(String),
    ModularWeapon(modular::ModularWeaponKey),
    ModularWeaponComponent(modular::ModularWeaponComponentKey),
    TagExamples(Vec<ItemKey>),
    Empty,
}

impl<T: ItemDesc> From<&T> for ItemKey {
    fn from(item_desc: &T) -> Self {
        let item_definition_id = item_desc.item_definition_id();

        if let ItemKind::TagExamples { item_ids } = &*item_desc.kind() {
            ItemKey::TagExamples(
                item_ids
                    .iter()
                    .map(|id| ItemKey::from(&*Arc::<ItemDef>::load_expect_cloned(id)))
                    .collect(),
            )
        } else {
            match item_definition_id {
                ItemDefinitionId::Simple(id) => ItemKey::Simple(String::from(id)),
                ItemDefinitionId::Compound { simple_base, .. } => {
                    if let Ok(key) =
                        modular::weapon_component_to_key(simple_base, item_desc.components())
                    {
                        ItemKey::ModularWeaponComponent(key)
                    } else {
                        ItemKey::Simple(simple_base.to_owned())
                    }
                },
                ItemDefinitionId::Modular { .. } => {
                    ItemKey::ModularWeapon(modular::weapon_to_key(item_desc))
                },
            }
        }
    }
}
