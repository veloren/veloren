use crate::{
    assets::AssetExt,
    comp::inventory::item::{
        armor::{Armor, ArmorKind},
        Glider, ItemDef, ItemDesc, ItemKind, Lantern, Throwable, Utility,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ItemKey {
    Tool(String),
    ModularComponent(String),
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
        let item_kind = item_desc.kind();
        let item_definition_id = item_desc.item_definition_id();

        match item_kind {
            ItemKind::Tool(_) => ItemKey::Tool(item_definition_id.to_owned()),
            ItemKind::ModularComponent(_) => {
                ItemKey::ModularComponent(item_definition_id.to_owned())
            },
            ItemKind::Lantern(Lantern { kind, .. }) => ItemKey::Lantern(kind.clone()),
            ItemKind::Glider(Glider { kind, .. }) => ItemKey::Glider(kind.clone()),
            ItemKind::Armor(Armor { kind, .. }) => ItemKey::Armor(kind.clone()),
            ItemKind::Utility { kind, .. } => ItemKey::Utility(*kind),
            ItemKind::Consumable { .. } => ItemKey::Consumable(item_definition_id.to_owned()),
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
