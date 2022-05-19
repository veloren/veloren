use crate::comp::{
    inventory::item::{
        armor,
        armor::{ArmorKind, Protection},
        tool::AbilityMap,
        ItemBase, ItemDef, ItemKind, MaterialStatManifest, Quality,
    },
    Item,
};
use std::sync::Arc;

pub(super) fn get_test_bag(slots: u16) -> Item {
    let item_def = ItemDef::new_test(
        "common.items.testing.test_bag".to_string(),
        ItemKind::Armor(armor::Armor::test_armor(
            ArmorKind::Bag,
            Protection::Normal(0.0),
            Protection::Normal(0.0),
        )),
        Quality::Common,
        Vec::new(),
        slots,
    );

    Item::new_from_item_base(
        ItemBase::Simple(Arc::new(item_def)),
        Vec::new(),
        &AbilityMap::load().read(),
        &MaterialStatManifest::load().read(),
    )
}
