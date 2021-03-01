use common::comp::item::{
    armor::{Armor, ArmorKind, Protection},
    tool::{Hands, StatKind, Stats, Tool, ToolKind},
    Item, ItemDesc, ItemKind, MaterialStatManifest, ModularComponent,
};
use std::{borrow::Cow, fmt::Write};

pub fn loadout_slot_text<'a>(
    item: Option<&'a impl ItemDesc>,
    mut empty: impl FnMut() -> (&'a str, &'a str),
    msm: &'a MaterialStatManifest,
) -> (&'a str, Cow<'a, str>) {
    item.map_or_else(
        || {
            let (title, desc) = empty();
            (title, Cow::Borrowed(desc))
        },
        |item| item_text(item, msm),
    )
}

pub fn item_text<'a>(
    item: &'a impl ItemDesc,
    msm: &'a MaterialStatManifest,
) -> (&'a str, Cow<'a, str>) {
    let desc: Cow<str> = match item.kind() {
        ItemKind::Armor(armor) => {
            Cow::Owned(armor_desc(armor, item.description(), item.num_slots()))
        },
        ItemKind::Tool(tool) => Cow::Owned(tool_desc(
            &tool,
            item.components(),
            &msm,
            item.description(),
        )),
        ItemKind::ModularComponent(mc) => Cow::Owned(modular_component_desc(
            mc,
            item.components(),
            &msm,
            item.description(),
        )),
        ItemKind::Glider(_glider) => Cow::Owned(glider_desc(item.description())),
        ItemKind::Consumable { .. } => Cow::Owned(consumable_desc(item.description())),
        ItemKind::Throwable { .. } => Cow::Owned(throwable_desc(item.description())),
        ItemKind::Utility { .. } => Cow::Owned(utility_desc(item.description())),
        ItemKind::Ingredient { .. } => Cow::Owned(ingredient_desc(
            item.description(),
            item.item_definition_id(),
            msm,
        )),
        ItemKind::Lantern { .. } => Cow::Owned(lantern_desc(item.description())),
        ItemKind::TagExamples { .. } => Cow::Borrowed(item.description()),
        //_ => Cow::Borrowed(item.description()),
    };

    (item.name(), desc)
}

// TODO: localization
fn modular_component_desc(
    mc: &ModularComponent,
    components: &[Item],
    msm: &MaterialStatManifest,
    description: &str,
) -> String {
    let stats = StatKind::Direct(mc.stats).resolve_stats(msm, components);
    let statblock = statblock_desc(&stats);
    let mut result = format!("Modular Component\n\n{}\n\n{}", statblock, description);
    if !components.is_empty() {
        result += "\n\nMade from:\n";
        for component in components {
            result += component.name();
            result += "\n"
        }
        result += "\n";
    }
    result
}
fn glider_desc(desc: &str) -> String { format!("Glider\n\n{}\n\n<Right-Click to use>", desc) }

fn consumable_desc(desc: &str) -> String {
    format!("Consumable\n\n{}\n\n<Right-Click to use>", desc)
}

fn throwable_desc(desc: &str) -> String {
    format!("Can be thrown\n\n{}\n\n<Right-Click to use>", desc)
}

fn utility_desc(desc: &str) -> String { format!("{}\n\n<Right-Click to use>", desc) }

fn ingredient_desc(desc: &str, item_id: &str, msm: &MaterialStatManifest) -> String {
    let mut result = format!("Crafting Ingredient\n\n{}", desc);
    if let Some(stats) = msm.0.get(item_id) {
        result += "\n\nStat multipliers:\n";
        result += &statblock_desc(stats);
    }
    result
}

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

fn tool_desc(tool: &Tool, components: &[Item], msm: &MaterialStatManifest, desc: &str) -> String {
    let kind = match tool.kind {
        ToolKind::Sword => "Sword",
        ToolKind::Axe => "Axe",
        ToolKind::Hammer => "Hammer",
        ToolKind::Bow => "Bow",
        ToolKind::Dagger => "Dagger",
        ToolKind::Staff => "Staff",
        ToolKind::Sceptre => "Sceptre",
        ToolKind::Shield => "Shield",
        ToolKind::Spear => "Spear",
        ToolKind::HammerSimple => "HammerSimple",
        ToolKind::SwordSimple => "SwordSimple",
        ToolKind::StaffSimple => "StaffSimple",
        ToolKind::AxeSimple => "AxeSimple",
        ToolKind::BowSimple => "BowSimple",
        ToolKind::Unique(_) => "Unique",
        ToolKind::Debug => "Debug",
        ToolKind::Farming => "Farming Tool",
        ToolKind::Empty => "Empty",
    };

    // Get tool stats
    let stats = tool.stats.resolve_stats(msm, components).clamp_speed();

    //let poise_strength = tool.base_poise_strength();
    let hands = match tool.hands {
        Hands::One => "One",
        Hands::Two => "Two",
    };

    let mut result = format!("{}-Handed {}\n\n", hands, kind);
    result += &statblock_desc(&stats);
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

fn statblock_desc(stats: &Stats) -> String {
    format!(
        "DPS: {:0.1}\n\nPower: {:0.1}\n\nSpeed: {:0.1}\n\n",
        // add back when ready for poise
        //"{}\n\nDPS: {:0.1}\n\nPower: {:0.1}\n\nPoise Strength: {:0.1}\n\nSpeed: \
        // {:0.1}\n\n{}\n\n<Right-Click to use>",
        stats.speed * stats.power * 10.0, // Damage per second
        stats.power * 10.0,
        //stats.poise_strength * 10.0,
        stats.speed
    )
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
        let mut testmsm = MaterialStatManifest(hashbrown::HashMap::new());
        testmsm.0.insert(
            "common.items.crafting_ing.bronze_ingot".to_string(),
            Stats {
                equip_time_secs: 0.0,
                power: 3.0,
                poise_strength: 5.0,
                speed: 7.0,
            },
        );

        assert_eq!(
            "Crafting Ingredient\n\nmushrooms",
            ingredient_desc("mushrooms", "common.items.food.mushroom", &testmsm)
        );
        assert_eq!(
            "Crafting Ingredient\n\nA bronze ingot.\n\nStat multipliers:\nDPS: 210.0\n\nPower: \
             30.0\n\nSpeed: 7.0\n\n",
            ingredient_desc(
                "A bronze ingot.",
                "common.items.crafting_ing.bronze_ingot",
                &testmsm
            )
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
