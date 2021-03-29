use common::{
    comp::{
        inventory::trade_pricing::TradePricing,
        item::{
            armor::{Armor, ArmorKind, Protection},
            tool::{Hands, StatKind, Stats, Tool, ToolKind},
            Item, ItemKind, MaterialStatManifest, ModularComponent,
        },
        BuffKind,
    },
    effect::Effect,
    trade::{Good, SitePrices},
};
use std::{borrow::Cow, fmt::Write};

use crate::i18n::Localization;

pub fn price_desc(
    prices: &Option<SitePrices>,
    item_definition_id: &str,
    i18n: &Localization,
) -> Option<String> {
    if let Some(prices) = prices {
        let (material, factor) = TradePricing::get_material(item_definition_id);
        let coinprice = prices.values.get(&Good::Coin).cloned().unwrap_or(1.0);
        let buyprice = prices.values.get(&material).cloned().unwrap_or_default() * factor;
        let sellprice = buyprice * material.trade_margin();
        Some(format!(
            "{} : {:0.1} {}\n{} : {:0.1} {}",
            i18n.get("hud.trade.buy_price"),
            buyprice / coinprice,
            i18n.get("hud.trade.coin"),
            i18n.get("hud.trade.sell_price"),
            sellprice / coinprice,
            i18n.get("hud.trade.coin"),
        ))
    } else {
        None
    }
}

pub fn kind_text<'a>(kind: &ItemKind, i18n: &'a Localization) -> Cow<'a, str> {
    match kind {
        ItemKind::Armor(armor) => Cow::Borrowed(armor_kind(&armor, &i18n)),
        ItemKind::Tool(tool) => Cow::Owned(format!(
            "{} ({})",
            tool_kind(&tool, i18n),
            tool_hands(&tool, i18n)
        )),
        ItemKind::ModularComponent(_mc) => Cow::Borrowed(i18n.get("common.bag.shoulders")),
        ItemKind::Glider(_glider) => Cow::Borrowed(i18n.get("common.kind.glider")),
        ItemKind::Consumable { .. } => Cow::Borrowed(i18n.get("common.kind.consumable")),
        ItemKind::Throwable { .. } => Cow::Borrowed(i18n.get("common.kind.throwable")),
        ItemKind::Utility { .. } => Cow::Borrowed(i18n.get("common.kind.utility")),
        ItemKind::Ingredient { .. } => Cow::Borrowed(i18n.get("common.kind.ingredient")),
        ItemKind::Lantern { .. } => Cow::Borrowed(i18n.get("common.kind.lantern")),
        ItemKind::TagExamples { .. } => Cow::Borrowed(""),
    }
}

// TODO: localization, refactor when mc are player facing
pub fn modular_component_desc(
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
    let mut description = String::new();

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

// Armor
fn armor_kind<'a>(armor: &Armor, i18n: &'a Localization) -> &'a str {
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
    kind
}

//Tool

fn tool_kind<'a>(tool: &Tool, i18n: &'a Localization) -> &'a str {
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
    kind
}

pub fn tool_hands<'a>(tool: &Tool, i18n: &'a Localization) -> &'a str {
    let hands = match tool.hands {
        Hands::One => i18n.get("common.hands.one"),
        Hands::Two => i18n.get("common.hands.two"),
    };
    hands
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
pub fn comparison<T: PartialOrd>(first: T, other: T) -> (&'static str, conrod_core::Color) {
    if first == other {
        ("•", conrod_core::color::GREY)
    } else if other < first {
        ("▲", conrod_core::color::GREEN)
    } else {
        ("▼", conrod_core::color::RED)
    }
}

pub fn protec2string(stat: Protection) -> String {
    match stat {
        Protection::Normal(a) => format!("{:.1}", a),
        Protection::Invincible => "Inf".to_string(),
    }
}
