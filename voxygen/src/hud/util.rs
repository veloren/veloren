use common::comp::item::{
    armor::{Armor, ArmorKind, Protection},
    tool::{Tool, ToolKind},
    ItemDesc, ItemKind,
};
use std::borrow::Cow;

pub fn loadout_slot_text<'a>(
    item: Option<&'a impl ItemDesc>,
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

pub fn item_text<'a>(item: &'a impl ItemDesc) -> (&'_ str, Cow<'a, str>) {
    let desc: Cow<str> = match item.kind() {
        ItemKind::Armor(armor) => Cow::Owned(armor_desc(&armor, item.description())),
        ItemKind::Tool(tool) => Cow::Owned(tool_desc(&tool, item.description())),
        ItemKind::Glider(_glider) => Cow::Owned(glider_desc(item.description())),
        ItemKind::Consumable { .. } => Cow::Owned(consumable_desc(item.description())),
        ItemKind::Throwable { .. } => Cow::Owned(throwable_desc(item.description())),
        ItemKind::Utility { .. } => Cow::Owned(utility_desc(item.description())),
        ItemKind::Ingredient { .. } => Cow::Owned(ingredient_desc(item.description())),
        ItemKind::Lantern { .. } => Cow::Owned(lantern_desc(item.description())),
        //_ => Cow::Borrowed(item.description()),
    };

    (item.name(), desc)
}

fn glider_desc(desc: &str) -> String { format!("Glider\n\n{}\n\n<Right-Click to use>", desc) }

fn consumable_desc(desc: &str) -> String {
    format!("Consumable\n\n{}\n\n<Right-Click to use>", desc)
}

fn throwable_desc(desc: &str) -> String {
    format!("Can be thrown\n\n{}\n\n<Right-Click to use>", desc)
}

fn utility_desc(desc: &str) -> String { format!("{}\n\n<Right-Click to use>", desc) }

fn ingredient_desc(desc: &str) -> String { format!("Crafting Ingredient\n\n{}", desc) }

fn lantern_desc(desc: &str) -> String { format!("Lantern\n\n{}\n\n<Right-Click to use>", desc) }

// Armor Description
fn armor_desc(armor: &Armor, desc: &str) -> String {
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
fn tool_desc(tool: &Tool, desc: &str) -> String {
    // TODO: localization
    let kind = match tool.kind {
        ToolKind::Sword => "Sword",
        ToolKind::Axe => "Axe",
        ToolKind::Hammer => "Hammer",
        ToolKind::Bow => "Bow",
        ToolKind::Dagger => "Dagger",
        ToolKind::Staff => "Staff",
        ToolKind::Sceptre => "Sceptre",
        ToolKind::Shield => "Shield",
        ToolKind::Unique(_) => "Unique",
        ToolKind::Debug => "Debug",
        ToolKind::Farming => "Farming Tool",
        ToolKind::Empty => "Empty",
    };
    let power = tool.base_power();

    if !desc.is_empty() {
        format!(
            "{}\n\nPower: {:0.1}\n\n{}\n\n<Right-Click to use>",
            kind,
            power * 10.0,
            desc
        )
    } else {
        format!(
            "{}\n\nPower: {:0.1}\n\n<Right-Click to use>",
            kind,
            power * 10.0
        )
    }
}
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
