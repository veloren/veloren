use common::comp::item::{
    armor::{Armor, ArmorKind, Protection},
    Item, ItemKind,
};
use std::borrow::Cow;

pub fn loadout_slot_text<'a>(
    item: Option<&'a Item>,
    mut empty: impl FnMut() -> (&'a str, &'a str),
) -> (&'a str, Cow<'a, str>) {
    item.map_or_else(
        || {
            let (title, desc) = empty();
            (title, Cow::Borrowed(desc))
        },
        item_text,
    )
}

pub fn item_text<'a>(item: &'a Item) -> (&'_ str, Cow<'a, str>) {
    let desc = match item.kind {
        ItemKind::Armor(armor) => Cow::Owned(armor_desc(armor, item.description())),
        // ItemKind::Tool => {},
        /*ItemKind::Consumable(kind, effect, ..) => {
            Cow::Owned(consumable_desc(consumable, item.description()))
        },*/
        // ItemKind::Throwable => {},
        // ItemKind::Utility => {},
        // ItemKind::Ingredient => {},
        // ItemKind::Lantern => {},
        _ => Cow::Borrowed(item.description()),
    };

    (item.name(), desc)
}
// Armor Description
fn armor_desc(armor: Armor, desc: &str) -> String {
    // TODO: localization
    let kind = match armor.kind {
        ArmorKind::Shoulder(_) => "Shoulders",
        ArmorKind::Chest(_) => "Chest",
        ArmorKind::Belt(_) => "Belt",
        ArmorKind::Hand(_) => "Hands",
        ArmorKind::Pants(_) => "Legs",
        ArmorKind::Foot(_) => "Feet",
        ArmorKind::Back(_) => "Back",
        ArmorKind::Ring(_) => "Ring",
        ArmorKind::Neck(_) => "Neck",
        ArmorKind::Head(_) => "Head",
        ArmorKind::Tabard(_) => "Tabard",
    };
    let armor = match armor.get_protection() {
        Protection::Normal(a) => a.to_string(),
        Protection::Invincible => "Inf".to_string(),
    };

    if !desc.is_empty() {
        format!(
            "{}\n\nArmor: {}\n\n{}\n\n<Right-Click to use>",
            kind, armor, desc
        )
    } else {
        format!("{}\n\nArmor: {}\n\n<Right-Click to use>", kind, armor)
    }
}
// Weapon/Tool Description

// Consumable Description
/*fn consumable_desc(consumable: Consumable, desc: &str) -> String {
    // TODO: localization
    let kind = "Consumable";
    if !desc.is_empty() {
        format!("{}\n\n{}\n\n<Right-Click to use>", kind, desc)
    } else {
        format!("{}\n\n<Right-Click to use>", kind)
    }
}*/

// Throwable Description

// Utility Description

// Ingredient Description

// Lantern Description
