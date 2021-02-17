use common::comp::item::{
    armor::{Armor, ArmorKind, Protection},
    tool::{Hands, Tool, ToolKind},
    Item, ItemDesc, ItemKind, ModularComponent,
};
use std::{borrow::Cow, fmt::Write};

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
        ItemKind::Armor(armor) => {
            Cow::Owned(armor_desc(armor, item.description(), item.num_slots()))
        },
        ItemKind::Tool(tool) => Cow::Owned(tool_desc(&tool, item.components(), item.description())),
        ItemKind::ModularComponent(mc) => Cow::Owned(modular_component_desc(mc)),
        ItemKind::Glider(_glider) => Cow::Owned(glider_desc(item.description())),
        ItemKind::Consumable { .. } => Cow::Owned(consumable_desc(item.description())),
        ItemKind::Throwable { .. } => Cow::Owned(throwable_desc(item.description())),
        ItemKind::Utility { .. } => Cow::Owned(utility_desc(item.description())),
        ItemKind::Ingredient { .. } => Cow::Owned(ingredient_desc(item.description())),
        ItemKind::Lantern { .. } => Cow::Owned(lantern_desc(item.description())),
        ItemKind::TagExamples { .. } => Cow::Borrowed(item.description()),
        //_ => Cow::Borrowed(item.description()),
    };

    (item.name(), desc)
}

// TODO: localization
fn modular_component_desc(mc: &ModularComponent) -> String {
    format!("Modular Component\n\n{:?}", mc)
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

fn armor_desc(armor: &Armor, desc: &str, slots: u16) -> String {
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
        ArmorKind::Bag(_) => "Bag",
    };
    let armor_protection = match armor.get_protection() {
        Protection::Normal(a) => a.to_string(),
        Protection::Invincible => "Inf".to_string(),
    };
    //let armor_poise_resilience = match armor.get_poise_resilience() {
    //    Protection::Normal(a) => a.to_string(),
    //    Protection::Invincible => "Inf".to_string(),
    //};

    let mut description = format!(
        "{}\n\nArmor: {}",
        //"{}\n\nArmor: {}\n\nPoise Resilience: {}",
        kind,
        armor_protection, /* armor_poise_resilience // Add back when we are ready for poise */
    );

    if !desc.is_empty() {
        write!(&mut description, "\n\n{}", desc).unwrap();
    }

    if slots > 0 {
        write!(&mut description, "\n\nSlots: {}", slots).unwrap();
    }

    write!(&mut description, "\n\n<Right-Click to use>").unwrap();
    description
}

fn tool_desc(tool: &Tool, components: &[Item], desc: &str) -> String {
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

    // Get tool stats
    let power = tool.base_power(components);
    //let poise_strength = tool.base_poise_strength();
    let hands = match tool.hands {
        Hands::One => "One",
        Hands::Two => "Two",
    };
    let speed = tool.base_speed(components);

    let mut result = format!(
        "{}-Handed {}\n\nDPS: {:0.1}\n\nPower: {:0.1}\n\nSpeed: {:0.1}\n\n",
        // add back when ready for poise
        //"{}\n\nDPS: {:0.1}\n\nPower: {:0.1}\n\nPoise Strength: {:0.1}\n\nSpeed: \
        // {:0.1}\n\n{}\n\n<Right-Click to use>",
        hands,
        kind,
        speed * power * 10.0, // Damage per second
        power * 10.0,
        //poise_strength * 10.0,
        speed
    );
    if !components.is_empty() {
        result += "Made from:\n";
        for component in components {
            result += component.name();
            result += "\n"
        }
        result += "\n";
    }
    if !desc.is_empty() {
        result += &format!("{}\n\n", desc);
    }
    result += "<Right-Click to use>";
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glider_desc() {
        let item_description = "mushrooms";

        assert_eq!(
            "Glider\n\nmushrooms\n\n<Right-Click to use>",
            glider_desc(item_description)
        );
    }

    #[test]
    fn test_consumable_desc() {
        let item_description = "mushrooms";

        assert_eq!(
            "Consumable\n\nmushrooms\n\n<Right-Click to use>",
            consumable_desc(item_description)
        );
    }

    #[test]
    fn test_throwable_desc() {
        let item_description = "mushrooms";

        assert_eq!(
            "Can be thrown\n\nmushrooms\n\n<Right-Click to use>",
            throwable_desc(item_description)
        );
    }

    #[test]
    fn test_utility_desc() {
        let item_description = "mushrooms";

        assert_eq!(
            "mushrooms\n\n<Right-Click to use>",
            utility_desc(item_description)
        );
    }

    #[test]
    fn test_ingredient_desc() {
        let item_description = "mushrooms";

        assert_eq!(
            "Crafting Ingredient\n\nmushrooms",
            ingredient_desc(item_description)
        );
    }

    #[test]
    fn test_lantern_desc() {
        let item_description = "mushrooms";

        assert_eq!(
            "Lantern\n\nmushrooms\n\n<Right-Click to use>",
            lantern_desc(item_description)
        );
    }
}
