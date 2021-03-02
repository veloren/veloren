use common::{
    comp::{
        inventory::trade_pricing::TradePricing,
        item::{
            armor::{Armor, ArmorKind, Protection},
            tool::{Hands, StatKind, Stats, Tool, ToolKind},
            Item, ItemDesc, ItemKind, MaterialStatManifest, ModularComponent,
        },
        BuffKind,
    },
    effect::Effect,
    trade::{Good, SitePrices},
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
    item: &'a dyn ItemDesc,
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
        ItemKind::Glider(_glider) => Cow::Owned(generic_desc(item)),
        ItemKind::Consumable { effect, .. } => {
            Cow::Owned(consumable_desc(effect, item.description()))
        },
        ItemKind::Throwable { .. } => Cow::Owned(generic_desc(item)),
        ItemKind::Utility { .. } => Cow::Owned(generic_desc(item)),
        ItemKind::Ingredient { .. } => Cow::Owned(ingredient_desc(
            item.description(),
            item.item_definition_id(),
            msm,
        )),
        ItemKind::Lantern { .. } => Cow::Owned(generic_desc(item)),
        ItemKind::TagExamples { .. } => Cow::Borrowed(item.description()),
        //_ => Cow::Borrowed(item.description()),
    };

    (item.name(), desc)
}

pub fn append_price_desc(desc: &mut String, prices: &Option<SitePrices>, item_definition_id: &str) {
    if let Some(prices) = prices {
        let (material, factor) = TradePricing::get_material(item_definition_id);
        let coinprice = prices.values.get(&Good::Coin).cloned().unwrap_or(1.0);
        let buyprice = prices.values.get(&material).cloned().unwrap_or_default() * factor;
        let sellprice = buyprice * material.trade_margin();
        *desc += &format!(
            "\n\nBuy price: {:0.1} coins\nSell price: {:0.1} coins",
            buyprice / coinprice,
            sellprice / coinprice
        );
    }
}

fn use_text(kind: &ItemKind) -> String {
    let text = match kind {
        ItemKind::Armor(_)
        | ItemKind::Tool(_)
        | ItemKind::ModularComponent(_)
        | ItemKind::Glider(_)
        | ItemKind::Consumable { .. }
        | ItemKind::Utility { .. }
        | ItemKind::Ingredient { .. }
        | ItemKind::Lantern { .. } => "<Right-Click to use>",
        ItemKind::Throwable { .. } => "<Right-Click to throw>",
        ItemKind::TagExamples { .. } => "",
    };
    text.to_string()
}

pub fn kind_text(kind: &ItemKind) -> String {
    match kind {
        ItemKind::Armor(armor) => format!("Armor ({})", armor_kind(&armor)),
        ItemKind::Tool(tool) => format!("{} {}", tool_hands(&tool), tool_kind(&tool)),
        ItemKind::ModularComponent(_mc) => "Modular Component".to_string(),
        ItemKind::Glider(_glider) => "Glider".to_string(),
        ItemKind::Consumable { .. } => "Consumable".to_string(),
        ItemKind::Throwable { .. } => "Can be thrown".to_string(),
        ItemKind::Utility { .. } => "Utility".to_string(),
        ItemKind::Ingredient { .. } => "Ingredient".to_string(),
        ItemKind::Lantern { .. } => "Lantern".to_string(),
        ItemKind::TagExamples { .. } => "".to_string(),
    }
}

fn generic_desc(desc: &dyn ItemDesc) -> String {
    format!(
        "{}\n\n{}\n\n{}",
        kind_text(desc.kind()),
        desc.description(),
        use_text(desc.kind())
    )
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

fn consumable_desc(effects: &[Effect], desc: &str) -> String {
    // TODO: localization
    let mut description = "Consumable".to_string();

    for effect in effects {
        if let Effect::Buff(buff) = effect {
            let strength = buff.data.strength * 0.1;
            let dur_secs = buff.data.duration.map(|d| d.as_secs_f32());
            let str_total = dur_secs.map_or(strength, |secs| strength * secs);

            let buff_desc = match buff.kind {
                BuffKind::Saturation | BuffKind::Regeneration | BuffKind::Potion => {
                    format!("Restores {} Health", str_total)
                },
                BuffKind::IncreaseMaxEnergy => {
                    format!("Raises Maximum Stamina by {}", strength)
                },
                BuffKind::IncreaseMaxHealth => {
                    format!("Raises Maximum Health by {}", strength)
                },
                BuffKind::Invulnerability => "Grants invulnerability".to_string(),
                BuffKind::Bleeding
                | BuffKind::CampfireHeal
                | BuffKind::Cursed
                | BuffKind::ProtectingWard => continue,
            };

            write!(&mut description, "\n\n{}", buff_desc).unwrap();

            let dur_desc = if dur_secs.is_some() {
                match buff.kind {
                    BuffKind::Saturation | BuffKind::Regeneration => {
                        format!("over {} seconds", dur_secs.unwrap())
                    },
                    BuffKind::IncreaseMaxEnergy
                    | BuffKind::IncreaseMaxHealth
                    | BuffKind::Invulnerability => {
                        format!("for {} seconds", dur_secs.unwrap())
                    },
                    BuffKind::Bleeding
                    | BuffKind::Potion
                    | BuffKind::CampfireHeal
                    | BuffKind::Cursed
                    | BuffKind::ProtectingWard => continue,
                }
            } else if let BuffKind::Saturation | BuffKind::Regeneration = buff.kind {
                "every second".to_string()
            } else {
                continue;
            };

            write!(&mut description, " {}", dur_desc).unwrap();
        }
    }

    if !desc.is_empty() {
        write!(&mut description, "\n\n{}", desc).unwrap();
    }

    write!(&mut description, "\n\n<Right-Click to use>").unwrap();
    description
}

fn ingredient_desc(desc: &str, item_id: &str, msm: &MaterialStatManifest) -> String {
    let mut result = format!("Crafting Ingredient\n\n{}", desc);
    if let Some(stats) = msm.0.get(item_id) {
        result += "\n\nStat multipliers:\n";
        result += &statblock_desc(stats);
    }
    result
}

// Armor

fn armor_kind(armor: &Armor) -> String {
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
    kind.to_string()
}

fn armor_protection(armor: &Armor) -> String {
    match armor.get_protection() {
        Protection::Normal(a) => format!("Protection: {}", a.to_string()),
        Protection::Invincible => "Protection: Inf".to_string(),
    }
}

pub fn armor_desc(armor: &Armor, desc: &str, slots: u16) -> String {
    // TODO: localization
    let kind = armor_kind(armor);
    let armor_protection = armor_protection(armor);
    //let armor_poise_resilience = match armor.get_poise_resilience() {
    //    Protection::Normal(a) => a.to_string(),
    //    Protection::Invincible => "Inf".to_string(),
    //};

    let mut desctext: String = "".to_string();
    if !desc.is_empty() {
        desctext = desc.to_string();
    }

    let mut slottext: String = "".to_string();
    if slots > 0 {
        slottext = format!("Slots: {}", slots)
    }

    let usetext = use_text(&ItemKind::Armor(armor.clone()));
    format!(
        "{} {}\n\n{}\n{}\n{}",
        kind, armor_protection, slottext, desctext, usetext
    )
}

//Tool

pub fn tool_kind(tool: &Tool) -> String {
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
        ToolKind::Pick => "Pickaxe",
        ToolKind::Empty => "Empty",
    };
    kind.to_string()
}

pub fn tool_stats(tool: &Tool, components: &[Item], msm: &MaterialStatManifest) -> String {
    let stats = tool.stats.resolve_stats(msm, components).clamp_speed();
    statblock_desc(&stats)
}

pub fn tool_hands(tool: &Tool) -> String {
    let hands = match tool.hands {
        Hands::One => "One-Handed",
        Hands::Two => "Two-Handed",
    };
    hands.to_string()
}

fn components_list(components: &[Item]) -> String {
    let mut text: String = "Made from:\n".to_string();
    for component in components {
        text += component.name();
        text += "\n"
    }
    text
}

pub fn tool_desc(
    tool: &Tool,
    components: &[Item],
    msm: &MaterialStatManifest,
    desc: &str,
) -> String {
    let kind = tool_kind(tool);
    //let poise_strength = tool.base_poise_strength();
    let hands = tool_hands(tool);
    let stats = tool_stats(tool, components, msm);
    let usetext = use_text(&ItemKind::Tool(tool.clone()));
    let mut componentstext: String = "".to_string();
    if !components.is_empty() {
        componentstext = components_list(components);
    }
    let mut desctext: String = "".to_string();
    if !desc.is_empty() {
        desctext = desc.to_string();
    }
    format!(
        "{} {}\n\n{}\n{}\n{}\n{}",
        hands, kind, stats, componentstext, desctext, usetext
    )
}

fn statblock_desc(stats: &Stats) -> String {
    format!(
        "DPS: {:0.1}\nPower: {:0.1}\nSpeed: {:0.1}\n",
        // add back when ready for poise
        //"{}\n\nDPS: {:0.1}\n\nPower: {:0.1}\n\nPoise Strength: {:0.1}\n\nSpeed: \
        // {:0.1}\n\n{}\n\n<Right-Click to use>",
        stats.speed * stats.power * 10.0, // Damage per second
        stats.power * 10.0,
        stats.poise_strength * 10.0,
        stats.speed,
    ) + &format!(
        "Crit chance: {:0.1}%\n\nCrit damage: x{:0.1}\n\n",
        stats.crit_chance * 100.0,
        stats.crit_mult,
    )
}

// Compare two type, output a colored character to show comparison
pub fn comparaison<T: PartialOrd>(first: T, other: T) -> (String, conrod_core::color::Color) {
    if first == other {
        (".".to_string(), conrod_core::color::GREY)
    } else if other < first {
        ("^".to_string(), conrod_core::color::GREEN)
    } else {
        ("v".to_string(), conrod_core::color::RED)
    }
}

pub fn protec2string(stat: Protection) -> String {
    match stat {
        Protection::Normal(a) => format!("{:.1}", a),
        Protection::Invincible => "Infinite".to_string(),
    }
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
            consumable_desc(&[], item_description)
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
                crit_chance: 0.5,
                crit_mult: 2.0,
            },
        );

        assert_eq!(
            "Crafting Ingredient\n\nmushrooms",
            ingredient_desc("mushrooms", "common.items.food.mushroom", &testmsm)
        );
        assert_eq!(
            "Crafting Ingredient\n\nA bronze ingot.\n\nStat multipliers:\nPower: 30.0\n\nPoise \
             Strength: 50.0\n\nSpeed: 7.0\n\nCrit chance: 50.0%\n\nCrit damage: x2.0\n\n",
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
