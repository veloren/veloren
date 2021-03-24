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

use crate::i18n::Localization;

pub fn loadout_slot_text<'a>(
    item: Option<&'a impl ItemDesc>,
    mut empty: impl FnMut() -> (&'a str, &'a str),
    msm: &'a MaterialStatManifest,
    i18n: &'a Localization,
) -> (&'a str, Cow<'a, str>) {
    item.map_or_else(
        || {
            let (title, desc) = empty();
            (title, Cow::Borrowed(desc))
        },
        |item| item_text(item, msm, i18n),
    )
}

pub fn item_text<'a>(
    item: &'a dyn ItemDesc,
    msm: &'a MaterialStatManifest,
    i18n: &'a Localization,
) -> (&'a str, Cow<'a, str>) {
    let desc: Cow<str> = match item.kind() {
        ItemKind::Armor(armor) => Cow::Owned(armor_desc(
            armor,
            item.description(),
            item.num_slots(),
            i18n,
        )),
        ItemKind::Tool(tool) => Cow::Owned(tool_desc(
            &tool,
            item.components(),
            &msm,
            item.description(),
            i18n,
        )),
        ItemKind::ModularComponent(mc) => Cow::Owned(modular_component_desc(
            mc,
            item.components(),
            &msm,
            item.description(),
        )),
        ItemKind::Glider(_glider) => Cow::Owned(generic_desc(item, i18n)),
        ItemKind::Consumable { effect, .. } => Cow::Owned(consumable_desc(effect, i18n)),
        ItemKind::Throwable { .. } => Cow::Owned(generic_desc(item, i18n)),
        ItemKind::Utility { .. } => Cow::Owned(generic_desc(item, i18n)),
        ItemKind::Ingredient { .. } => Cow::Owned(ingredient_desc(
            item.description(),
            item.item_definition_id(),
            msm,
        )),
        ItemKind::Lantern { .. } => Cow::Owned(generic_desc(item, i18n)),
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

pub fn kind_text(kind: &ItemKind, i18n: &Localization) -> String {
    match kind {
        ItemKind::Armor(armor) => armor_kind(&armor, &i18n),
        ItemKind::Tool(tool) => {
            format!("{} ({})", tool_kind(&tool, &i18n), tool_hands(&tool, i18n))
        },
        ItemKind::ModularComponent(_mc) => i18n.get("common.bag.shoulders").to_string(),
        ItemKind::Glider(_glider) => i18n.get("common.kind.glider").to_string(),
        ItemKind::Consumable { .. } => i18n.get("common.kind.consumable").to_string(),
        ItemKind::Throwable { .. } => i18n.get("common.kind.throwable").to_string(),
        ItemKind::Utility { .. } => i18n.get("common.kind.utility").to_string(),
        ItemKind::Ingredient { .. } => i18n.get("common.kind.ingredient").to_string(),
        ItemKind::Lantern { .. } => i18n.get("common.kind.lantern").to_string(),
        ItemKind::TagExamples { .. } => "".to_string(),
    }
}

fn generic_desc(desc: &dyn ItemDesc, i18n: &Localization) -> String {
    format!(
        "{}\n\n{}\n\n{}",
        kind_text(desc.kind(), i18n),
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

pub fn consumable_desc(effects: &[Effect], i18n: &Localization) -> String {
    let mut description = "".to_string();

    for effect in effects {
        if let Effect::Buff(buff) = effect {
            let strength = buff.data.strength * 0.1;
            let dur_secs = buff.data.duration.map(|d| d.as_secs_f32());
            let str_total = dur_secs.map_or(strength, |secs| strength * secs);

            let buff_desc = match buff.kind {
                BuffKind::Saturation | BuffKind::Regeneration | BuffKind::Potion => i18n
                    .get("buff.stat.health")
                    .replace("{str_total}", &str_total.to_string()),
                BuffKind::IncreaseMaxEnergy => i18n
                    .get("buff.stat.increase_max_stamina")
                    .replace("{strength}", &strength.to_string()),
                BuffKind::IncreaseMaxHealth => i18n
                    .get("buff.stat.increase_max_health")
                    .replace("{strength}", &strength.to_string()),
                BuffKind::Invulnerability => i18n.get("buff.stat.invulenrability").to_string(),
                BuffKind::Bleeding
                | BuffKind::CampfireHeal
                | BuffKind::Cursed
                | BuffKind::ProtectingWard => continue,
            };

            write!(&mut description, "{}", buff_desc).unwrap();

            let dur_desc = if let Some(dur_secs) = dur_secs {
                match buff.kind {
                    BuffKind::Saturation | BuffKind::Regeneration => i18n
                        .get("buff.text.over_seconds")
                        .replace("{dur_secs}", &dur_secs.to_string()),
                    BuffKind::IncreaseMaxEnergy
                    | BuffKind::IncreaseMaxHealth
                    | BuffKind::Invulnerability => i18n
                        .get("buff.text.for_seconds")
                        .replace("{dur_secs}", &dur_secs.to_string()),
                    BuffKind::Bleeding
                    | BuffKind::Potion
                    | BuffKind::CampfireHeal
                    | BuffKind::Cursed
                    | BuffKind::ProtectingWard => continue,
                }
            } else if let BuffKind::Saturation | BuffKind::Regeneration = buff.kind {
                i18n.get("buff.text.every_second").to_string()
            } else {
                continue;
            };

            write!(&mut description, " {}", dur_desc).unwrap();
        }
    }

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
fn armor_kind(armor: &Armor, i18n: &Localization) -> String {
    let kind = match armor.kind {
        ArmorKind::Shoulder(_) => i18n.get("hud.bag.shoulders"),
        ArmorKind::Chest(_) => i18n.get("hud.bag.chest"),
        ArmorKind::Belt(_) => i18n.get("hud.bag.belt"),
        ArmorKind::Hand(_) => i18n.get("hud.bag.hands"),
        ArmorKind::Pants(_) => i18n.get("hud.bag.legs"),
        ArmorKind::Foot(_) => i18n.get("hud.bag.feet"),
        ArmorKind::Back(_) => i18n.get("hud.bag.back"),
        ArmorKind::Ring(_) => i18n.get("hud.bag.ring"),
        ArmorKind::Neck(_) => i18n.get("hud.bag.neck"),
        ArmorKind::Head(_) => i18n.get("hud.bag.head"),
        ArmorKind::Tabard(_) => i18n.get("hud.bag.tabard"),
        ArmorKind::Bag(_) => i18n.get("hud.bag.bag"),
    };
    kind.to_string()
}

fn armor_protection(armor: &Armor) -> String {
    match armor.get_protection() {
        Protection::Normal(a) => format!("Protection: {}", a.to_string()),
        Protection::Invincible => "Protection: Inf".to_string(),
    }
}

pub fn armor_desc(armor: &Armor, desc: &str, slots: u16, i18n: &Localization) -> String {
    // TODO: localization
    let kind = armor_kind(armor, i18n);
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

pub fn tool_kind(tool: &Tool, i18n: &Localization) -> String {
    let kind = match tool.kind {
        ToolKind::Sword => i18n.get("common.weapons.sword"),
        ToolKind::Axe => i18n.get("common.weapons.axe"),
        ToolKind::Hammer => i18n.get("common.weapons.hammer"),
        ToolKind::Bow => i18n.get("common.weapons.bow"),
        ToolKind::Dagger => i18n.get("common.weapons.dagger"),
        ToolKind::Staff => i18n.get("common.weapons.staff"),
        ToolKind::Sceptre => i18n.get("common.weapons.sceptre"),
        ToolKind::Shield => i18n.get("common.weapons.shield"),
        ToolKind::Spear => i18n.get("common.weapons.spear"),
        ToolKind::HammerSimple => i18n.get("common.weapons.hammer_simple"),
        ToolKind::SwordSimple => i18n.get("common.weapons.sword_simple"),
        ToolKind::StaffSimple => i18n.get("common.weapons.staff_simple"),
        ToolKind::AxeSimple => i18n.get("common.weapons.axe_simple"),
        ToolKind::BowSimple => i18n.get("common.weapons.bow_simple"),
        ToolKind::Unique(_) => i18n.get("common.weapons.unique_simple"),
        ToolKind::Debug => i18n.get("common.tool.debug"),
        ToolKind::Farming => i18n.get("common.tool.farming"),
        ToolKind::Pick => i18n.get("common.tool.pick"),
        ToolKind::Empty => i18n.get("common.empty"),
    };
    kind.to_string()
}

pub fn tool_stats(tool: &Tool, components: &[Item], msm: &MaterialStatManifest) -> String {
    let stats = tool.stats.resolve_stats(msm, components).clamp_speed();
    statblock_desc(&stats)
}

pub fn tool_hands(tool: &Tool, i18n: &Localization) -> String {
    let hands = match tool.hands {
        Hands::One => i18n.get("common.hands.one"),
        Hands::Two => i18n.get("common.hands.two"),
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
    i18n: &Localization,
) -> String {
    let kind = tool_kind(tool, i18n);
    //let poise_strength = tool.base_poise_strength();
    let hands = tool_hands(tool, i18n);
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
        "Power: {:0.1}\n\nPoise Strength: {:0.1}\n\nSpeed: {:0.1}\n\n",
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
pub fn comparison<T: PartialOrd>(first: T, other: T) -> (String, conrod_core::color::Color) {
    if first == other {
        ("•".to_string(), conrod_core::color::GREY)
    } else if other < first {
        ("▲".to_string(), conrod_core::color::GREEN)
    } else {
        ("▼".to_string(), conrod_core::color::RED)
    }
}

pub fn protec2string(stat: Protection) -> String {
    match stat {
        Protection::Normal(a) => format!("{:.1}", a),
        Protection::Invincible => "Inf".to_string(),
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
