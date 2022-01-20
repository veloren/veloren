use super::img_ids;
use common::{
    comp::{
        inventory::trade_pricing::TradePricing,
        item::{
            armor::{Armor, ArmorKind, Protection},
            tool::{Hands, StatKind, Stats, Tool, ToolKind},
            Item, ItemDesc, ItemKind, MaterialKind, MaterialStatManifest, ModularComponent,
        },
        BuffKind,
    },
    effect::Effect,
    trade::{Good, SitePrices},
};
use conrod_core::image;
use i18n::Localization;
use std::{borrow::Cow, fmt::Write};

pub fn price_desc(
    prices: &Option<SitePrices>,
    item_definition_id: &str,
    i18n: &Localization,
) -> Option<(String, String, f32)> {
    if let Some(prices) = prices {
        let (material, factor) = TradePricing::get_material(item_definition_id);
        let coinprice = prices.values.get(&Good::Coin).cloned().unwrap_or(1.0);
        let buyprice = prices.values.get(&material).cloned().unwrap_or_default() * factor;
        let sellprice = buyprice * material.trade_margin();

        let deal_goodness = prices.values.get(&material).cloned().unwrap_or(0.0)
            / prices.values.get(&Good::Coin).cloned().unwrap_or(1.0);
        let deal_goodness = deal_goodness.log(2.0);
        let buy_string = format!(
            "{} : {:0.1} {}",
            i18n.get("hud.trade.buy_price"),
            buyprice / coinprice,
            i18n.get("hud.trade.coin"),
        );
        let sell_string = format!(
            "{} : {:0.1} {}",
            i18n.get("hud.trade.sell_price"),
            sellprice / coinprice,
            i18n.get("hud.trade.coin"),
        );
        let deal_goodness = match deal_goodness {
            x if x < -2.5 => 0.0,
            x if x < -1.05 => 0.25,
            x if x < -0.95 => 0.5,
            x if x < 0.0 => 0.75,
            _ => 1.0,
        };
        Some((buy_string, sell_string, deal_goodness))
    } else {
        None
    }
}

pub fn kind_text<'a>(kind: &ItemKind, i18n: &'a Localization) -> Cow<'a, str> {
    match kind {
        ItemKind::Armor(armor) => Cow::Borrowed(armor_kind(armor, i18n)),
        ItemKind::Tool(tool) => Cow::Owned(format!(
            "{} ({})",
            tool_kind(tool, i18n),
            tool_hands(tool, i18n)
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

pub fn material_kind_text<'a>(kind: &MaterialKind, i18n: &'a Localization) -> &'a str {
    match kind {
        MaterialKind::Metal { .. } => i18n.get("common.material.metal"),
        MaterialKind::Wood { .. } => i18n.get("common.material.wood"),
        MaterialKind::Stone { .. } => i18n.get("common.material.stone"),
        MaterialKind::Cloth { .. } => i18n.get("common.material.cloth"),
        MaterialKind::Hide { .. } => i18n.get("common.material.hide"),
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

pub fn stats_count(item: &dyn ItemDesc) -> usize {
    let mut count = match item.kind() {
        ItemKind::Armor(armor) => {
            if matches!(armor.kind, ArmorKind::Bag(_)) {
                0
            } else {
                armor.stats.energy_reward().is_some() as usize
                    + armor.stats.energy_max().is_some() as usize
                    + armor.stats.stealth().is_some() as usize
                    + armor.stats.crit_power().is_some() as usize
                    + armor.stats.poise_resilience().is_some() as usize
            }
        },
        ItemKind::Tool(_) => 4,
        ItemKind::Consumable { effects, .. } => effects.len(),
        ItemKind::ModularComponent { .. } => 1,
        _ => 0,
    };

    let is_bag = match item.kind() {
        ItemKind::Armor(armor) => matches!(armor.kind, ArmorKind::Bag(_)),
        _ => false,
    };
    if item.num_slots() != 0 && !is_bag {
        count += 1
    }
    count as usize
}

/// Takes N `effects` and returns N effect descriptions
/// If effect isn't intended to have description, returns empty string
///
/// FIXME: handle which effects should have description in `stats_count`
/// to not waste space in item box
pub fn consumable_desc(effects: &[Effect], i18n: &Localization) -> Vec<String> {
    let mut descriptions = Vec::new();

    for effect in effects {
        let mut description = String::new();
        if let Effect::Buff(buff) = effect {
            let strength = buff.data.strength;
            let dur_secs = buff.data.duration.map(|d| d.as_secs_f32());
            let str_total = dur_secs.map_or(strength, |secs| strength * secs);

            let format_float =
                |input: f32| format!("{:.1}", input).trim_end_matches(".0").to_string();

            let buff_desc = match buff.kind {
                BuffKind::Saturation | BuffKind::Regeneration | BuffKind::Potion => i18n
                    .get("buff.stat.health")
                    .replace("{str_total}", &format_float(str_total)),
                BuffKind::IncreaseMaxEnergy => i18n
                    .get("buff.stat.increase_max_energy")
                    .replace("{strength}", &format_float(strength)),
                BuffKind::IncreaseMaxHealth => i18n
                    .get("buff.stat.increase_max_health")
                    .replace("{strength}", &format_float(strength)),
                BuffKind::Invulnerability => i18n.get("buff.stat.invulnerability").to_string(),
                BuffKind::Bleeding
                | BuffKind::Burning
                | BuffKind::CampfireHeal
                | BuffKind::Cursed
                | BuffKind::ProtectingWard
                | BuffKind::Crippled
                | BuffKind::Frenzied
                | BuffKind::Frozen
                | BuffKind::Wet
                | BuffKind::Ensnared
                | BuffKind::Poisoned => "".to_owned(),
            };

            write!(&mut description, "{}", buff_desc).unwrap();

            let dur_desc = if let Some(dur_secs) = dur_secs {
                match buff.kind {
                    BuffKind::Saturation | BuffKind::Regeneration => i18n
                        .get("buff.text.over_seconds")
                        .replace("{dur_secs}", &format_float(dur_secs)),
                    BuffKind::IncreaseMaxEnergy
                    | BuffKind::IncreaseMaxHealth
                    | BuffKind::Invulnerability => i18n
                        .get("buff.text.for_seconds")
                        .replace("{dur_secs}", &format_float(dur_secs)),
                    BuffKind::Bleeding
                    | BuffKind::Burning
                    | BuffKind::Potion
                    | BuffKind::CampfireHeal
                    | BuffKind::Cursed
                    | BuffKind::ProtectingWard
                    | BuffKind::Crippled
                    | BuffKind::Frenzied
                    | BuffKind::Frozen
                    | BuffKind::Wet
                    | BuffKind::Ensnared
                    | BuffKind::Poisoned => "".to_owned(),
                }
            } else if let BuffKind::Saturation | BuffKind::Regeneration = buff.kind {
                i18n.get("buff.text.every_second").to_string()
            } else {
                "".to_owned()
            };

            write!(&mut description, " {}", dur_desc).unwrap();
        }
        descriptions.push(description);
    }

    descriptions
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

// Tool
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
        ToolKind::Blowgun => i18n.get("common.weapons.blowgun"),
        ToolKind::Natural => i18n.get("common.weapons.natural"),
        ToolKind::Debug => i18n.get("common.tool.debug"),
        ToolKind::Farming => i18n.get("common.tool.farming"),
        ToolKind::Pick => i18n.get("common.tool.pick"),
        ToolKind::Empty => i18n.get("common.empty"),
    };
    kind
}

/// Output the number of hands needed to hold a tool
pub fn tool_hands<'a>(tool: &Tool, i18n: &'a Localization) -> &'a str {
    let hands = match tool.hands {
        Hands::One => i18n.get("common.hands.one"),
        Hands::Two => i18n.get("common.hands.two"),
    };
    hands
}

fn statblock_desc(stats: &Stats) -> String {
    format!(
        // TODO: Change display of Effect Power based on toolkind equipped and what effect power is
        // affecting
        "Power: {:0.1}\n\nPoise Strength: {:0.1}\n\nSpeed: {:0.1}\n\n",
        stats.power * 10.0,
        stats.effect_power * 10.0,
        stats.speed,
    ) + &format!("Crit chance: {:0.1}%\n\n", stats.crit_chance * 100.0,)
}

/// Compare two type, output a colored character to show comparison
pub fn comparison<T: PartialOrd>(first: T, other: T) -> (&'static str, conrod_core::Color) {
    if first == other {
        ("•", conrod_core::color::GREY)
    } else if other < first {
        ("▲", conrod_core::color::GREEN)
    } else {
        ("▼", conrod_core::color::RED)
    }
}

/// Compare two Option type, output a colored character to show comparison
pub fn option_comparison<T: PartialOrd>(
    first: &Option<T>,
    other: &Option<T>,
) -> (&'static str, conrod_core::Color) {
    if let Some(first) = first {
        if let Some(other) = other {
            if first == other {
                ("•", conrod_core::color::GREY)
            } else if other < first {
                ("▲", conrod_core::color::GREEN)
            } else {
                ("▼", conrod_core::color::RED)
            }
        } else {
            ("▲", conrod_core::color::GREEN)
        }
    } else if other.is_some() {
        ("▼", conrod_core::color::RED)
    } else {
        ("•", conrod_core::color::GREY)
    }
}

/// Output protection as a string
pub fn protec2string(stat: Protection) -> String {
    match stat {
        Protection::Normal(a) => format!("{:.1}", a),
        Protection::Invincible => "Inf".to_string(),
    }
}

pub fn ability_image(imgs: &img_ids::Imgs, ability_id: &str) -> image::Id {
    match ability_id {
        // Debug stick
        "common.abilities.debug.forwardboost" => imgs.flyingrod_m1,
        "common.abilities.debug.upboost" => imgs.flyingrod_m2,
        "common.abilities.debug.possess" => imgs.snake_arrow_0,
        // Sword
        "common.abilities.sword.triplestrike" => imgs.twohsword_m1,
        "common.abilities.sword.dash" => imgs.twohsword_m2,
        "common.abilities.sword.spin" => imgs.sword_whirlwind,
        // Axe
        "common.abilities.axe.doublestrike" => imgs.twohaxe_m1,
        "common.abilities.axe.spin" => imgs.axespin,
        "common.abilities.axe.leap" => imgs.skill_axe_leap_slash,
        // Hammer
        "common.abilities.hammer.singlestrike" => imgs.twohhammer_m1,
        "common.abilities.hammer.charged" => imgs.hammergolf,
        "common.abilities.hammer.leap" => imgs.hammerleap,
        // Bow
        "common.abilities.bow.charged" => imgs.bow_m1,
        "common.abilities.bow.repeater" => imgs.bow_m2,
        "common.abilities.bow.shotgun" => imgs.skill_bow_jump_burst,
        // Staff
        "common.abilities.staff.firebomb" => imgs.fireball,
        "common.abilities.staff.flamethrower" => imgs.flamethrower,
        "common.abilities.staff.fireshockwave" => imgs.fire_aoe,
        // Sceptre
        "common.abilities.sceptre.lifestealbeam" => imgs.skill_sceptre_lifesteal,
        "common.abilities.sceptre.healingaura" => imgs.skill_sceptre_heal,
        "common.abilities.sceptre.wardingaura" => imgs.skill_sceptre_aura,
        // Shield
        "common.abilities.shield.tempbasic" => imgs.onehshield_m1,
        "common.abilities.shield.block" => imgs.onehshield_m2,
        // Dagger
        "common.abilities.dagger.tempbasic" => imgs.onehdagger_m1,
        // Pickaxe
        "common.abilities.pick.swing" => imgs.mining,

        _ => imgs.not_found,
    }
}

#[rustfmt::skip]
pub fn ability_description(ability_id: &str) -> (&str, &str) {
    match ability_id {
        // Debug stick
        "common.abilities.debug.possess" => (
            "Possessing Arrow",
            "Shoots a poisonous arrow. Lets you control your target.",
        ),
        // Sword
        "common.abilities.sword.spin" => (
            "Whirlwind",
            "Move forward while spinning with your sword.",
        ),
        // Axe
        "common.abilities.axe.leap" => (
            "Axe Jump",
            "A jump with the slashing leap to position of cursor.",
        ),
        // Hammer
        "common.abilities.hammer.leap" => (
            "Smash of Doom",
            "An AOE attack with knockback. Leaps to position of cursor.",
        ),
        // Bow
        "common.abilities.bow.shotgun" => (
            "Burst",
            "Launches a burst of arrows",
        ),
        // Staff
        "common.abilities.staff.fireshockwave" => (
            "Ring of Fire",
            "Ignites the ground with fiery shockwave.",
        ),
        // Sceptre
        "common.abilities.sceptre.wardingaura" => (
            "Thorn Bulwark",
            "Protects you and your group with thorns for a short amount of time.",
        ),
        _ => (
            "Ability has no title",
            "Ability has no description."
        ),
    }
}
