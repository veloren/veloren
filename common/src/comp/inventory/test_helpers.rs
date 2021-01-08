use crate::comp::{
    inventory::item::{
        armor,
        armor::{ArmorKind, Protection},
        ItemDef, ItemKind, Quality,
    },
    Item,
};
use std::sync::Arc;

pub(super) fn get_test_bag(slots: u16) -> Item {
    let item_def = ItemDef::new_test(
        "common.items.testing.test_bag".to_string(),
        None,
        ItemKind::Armor(armor::Armor::test_armor(
            ArmorKind::Bag("Test Bag".to_string()),
            Protection::Normal(0.0),
        )),
        Quality::Common,
        slots,
    );

    Item::new_from_item_def(Arc::new(item_def))
}
