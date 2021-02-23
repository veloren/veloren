use crate::comp::{
    inventory::item::{
        armor,
        armor::{ArmorKind, Protection},
        tool::AbilityMap,
        ItemDef, ItemKind, MaterialStatManifest, Quality,
    },
    Item,
};
use std::{default::Default, sync::Arc};

pub(super) fn get_test_bag(slots: u16) -> Item {
    let item_def = ItemDef::new_test(
        "common.items.testing.test_bag".to_string(),
        ItemKind::Armor(armor::Armor::test_armor(
            ArmorKind::Bag("Test Bag".to_string()),
            Protection::Normal(0.0),
            Protection::Normal(0.0),
        )),
        Quality::Common,
        Vec::new(),
        slots,
        AbilityMap::default(),
    );

    Item::new_from_item_def(Arc::new(item_def), &[], &MaterialStatManifest::default())
}
