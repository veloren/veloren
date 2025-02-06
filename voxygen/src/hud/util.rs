use super::img_ids;
use common::{
    comp::{
        BuffData, BuffKind,
        inventory::trade_pricing::TradePricing,
        item::{
            Effects, Item, ItemDefinitionId, ItemDesc, ItemI18n, ItemKind, MaterialKind,
            MaterialStatManifest,
            armor::{Armor, ArmorKind, Protection},
            tool::{Hands, Tool, ToolKind},
        },
    },
    effect::Effect,
    trade::{Good, SitePrices},
};
use conrod_core::image;
use i18n::{FluentValue, Localization, fluent_args};
use std::{borrow::Cow, fmt::Write};

pub fn price_desc<'a>(
    prices: &Option<SitePrices>,
    item_definition_id: ItemDefinitionId<'_>,
    i18n: &'a Localization,
) -> Option<(Cow<'a, str>, Cow<'a, str>, f32)> {
    let prices = prices.as_ref()?;
    let materials = TradePricing::get_materials(&item_definition_id)?;
    let coinprice = prices.values.get(&Good::Coin).cloned().unwrap_or(1.0);
    let buyprice: f32 = materials
        .iter()
        .map(|e| prices.values.get(&e.1).cloned().unwrap_or_default() * e.0)
        .sum();
    let sellprice: f32 = materials
        .iter()
        .map(|e| prices.values.get(&e.1).cloned().unwrap_or_default() * e.0 * e.1.trade_margin())
        .sum();

    let deal_goodness: f32 = materials
        .iter()
        .map(|e| prices.values.get(&e.1).cloned().unwrap_or(0.0))
        .sum::<f32>()
        / prices.values.get(&Good::Coin).cloned().unwrap_or(1.0)
        / (materials.len() as f32);
    let deal_goodness = deal_goodness.log(2.0);

    let buy_string = i18n.get_msg_ctx("hud-trade-buy", &fluent_args! {
        "coin_num" => buyprice / coinprice,
        "coin_formatted" => format!("{:0.1}", buyprice / coinprice),
    });
    let sell_string = i18n.get_msg_ctx("hud-trade-sell", &fluent_args! {
        "coin_num" => sellprice / coinprice,
        "coin_formatted" => format!("{:0.1}", sellprice / coinprice),
    });

    let deal_goodness = match deal_goodness {
        x if x < -2.5 => 0.0,
        x if x < -1.05 => 0.25,
        x if x < -0.95 => 0.5,
        x if x < 0.0 => 0.75,
        _ => 1.0,
    };
    Some((buy_string, sell_string, deal_goodness))
}

pub fn item_text<'a, I: ItemDesc + ?Sized>(
    item: &I,
    i18n: &'a Localization,
    i18n_spec: &'a ItemI18n,
) -> (String, String) {
    let (title, desc) = item.i18n(i18n_spec);

    (i18n.get_content(&title), i18n.get_content(&desc))
}

pub fn describe<'a, I: ItemDesc + ?Sized>(
    item: &I,
    i18n: &'a Localization,
    i18n_spec: &'a ItemI18n,
) -> String {
    let (title, _) = item_text(item, i18n, i18n_spec);
    let amount = item.amount();

    if amount.get() > 1 {
        format!("{amount} x {title}")
    } else {
        title
    }
}

pub fn kind_text<'a>(kind: &ItemKind, i18n: &'a Localization) -> Cow<'a, str> {
    match kind {
        ItemKind::Armor(armor) => armor_kind(armor, i18n),
        ItemKind::Tool(tool) => Cow::Owned(format!(
            "{} ({})",
            tool_kind(tool, i18n),
            tool_hands(tool, i18n)
        )),
        ItemKind::ModularComponent(mc) => {
            if let Some(toolkind) = mc.toolkind() {
                Cow::Owned(format!(
                    "{} {}",
                    i18n.get_msg(&format!("common-weapons-{}", toolkind.identifier_name())),
                    i18n.get_msg("common-kind-modular_component_partial")
                ))
            } else {
                i18n.get_msg("common-kind-modular_component")
            }
        },
        ItemKind::Glider => i18n.get_msg("common-kind-glider"),
        ItemKind::Consumable { .. } => i18n.get_msg("common-kind-consumable"),
        ItemKind::Utility { .. } => i18n.get_msg("common-kind-utility"),
        ItemKind::Ingredient { .. } => i18n.get_msg("common-kind-ingredient"),
        ItemKind::Lantern { .. } => i18n.get_msg("common-kind-lantern"),
        ItemKind::TagExamples { .. } => Cow::Borrowed(""),
        ItemKind::RecipeGroup { .. } => i18n.get_msg("common-kind-recipegroup"),
    }
}

pub fn material_kind_text<'a>(kind: &MaterialKind, i18n: &'a Localization) -> Cow<'a, str> {
    match kind {
        MaterialKind::Metal => i18n.get_msg("common-material-metal"),
        MaterialKind::Gem => i18n.get_msg("common-material-gem"),
        MaterialKind::Wood => i18n.get_msg("common-material-wood"),
        MaterialKind::Stone => i18n.get_msg("common-material-stone"),
        MaterialKind::Cloth => i18n.get_msg("common-material-cloth"),
        MaterialKind::Hide => i18n.get_msg("common-material-hide"),
    }
}

pub fn stats_count(item: &dyn ItemDesc, msm: &MaterialStatManifest) -> usize {
    let mut count = match &*item.kind() {
        ItemKind::Armor(armor) => {
            let armor_stats = armor.stats(msm, item.stats_durability_multiplier());
            armor_stats.energy_reward.is_some() as usize
                + armor_stats.energy_max.is_some() as usize
                + armor_stats.stealth.is_some() as usize
                + armor_stats.precision_power.is_some() as usize
                + armor_stats.poise_resilience.is_some() as usize
                + armor_stats.protection.is_some() as usize
                + (item.num_slots() > 0) as usize
        },
        ItemKind::Tool(_) => 6,
        ItemKind::Consumable { effects, .. } => match effects {
            Effects::Any(_) | Effects::One(_) => 1,
            Effects::All(effects) => effects.len(),
        },
        ItemKind::RecipeGroup { recipes } => {
            /* Add one for recipe known/not known message */
            recipes.len() + 1
        },
        ItemKind::ModularComponent { .. } => 6,
        _ => 0,
    };
    if item.has_durability() {
        count += 1;
    }
    count
}

pub fn line_count(item: &dyn ItemDesc, msm: &MaterialStatManifest, i18n: &Localization) -> usize {
    match &*item.kind() {
        ItemKind::Consumable { effects, .. } => {
            let descs = consumable_desc(effects, i18n);
            let mut lines = 0;
            for desc in descs {
                lines += desc.matches('\n').count() + 1;
            }

            lines
        },
        _ => stats_count(item, msm),
    }
}

/// Returns i18n key for a buff with title, .desc and optionally .stat
///
/// NOTE: not to be confused with buff key for buff's kill message
fn buff_key(buff: BuffKind) -> &'static str {
    match buff {
        // Buffs
        BuffKind::Regeneration => "buff-heal",
        BuffKind::Saturation => "buff-saturation",
        BuffKind::Potion => "buff-potion",
        BuffKind::Agility => "buff-agility",
        BuffKind::RestingHeal => "buff-resting_heal",
        BuffKind::EnergyRegen => "buff-energy_regen",
        BuffKind::ComboGeneration => "buff-combo_generation",
        BuffKind::IncreaseMaxHealth => "buff-increase_max_health",
        BuffKind::IncreaseMaxEnergy => "buff-increase_max_energy",
        BuffKind::Invulnerability => "buff-invulnerability",
        BuffKind::ProtectingWard => "buff-protectingward",
        BuffKind::Frenzied => "buff-frenzied",
        BuffKind::Hastened => "buff-hastened",
        BuffKind::Fortitude => "buff-fortitude",
        BuffKind::Reckless => "buff-reckless",
        // BuffKind::SalamanderAspect => "buff-salamanderaspect",
        BuffKind::Flame => "buff-burn",
        BuffKind::Frigid => "buff-frigid",
        BuffKind::Lifesteal => "buff-lifesteal",
        BuffKind::ImminentCritical => "buff-imminentcritical",
        BuffKind::Fury => "buff-fury",
        BuffKind::Sunderer => "buff-sunderer",
        BuffKind::Defiance => "buff-defiance",
        BuffKind::Bloodfeast => "buff-bloodfeast",
        BuffKind::Berserk => "buff-berserk",
        BuffKind::ScornfulTaunt => "buff-scornfultaunt",
        BuffKind::Tenacity => "buff-tenacity",
        BuffKind::Resilience => "buff-resilience",
        BuffKind::SnareShot => "buff-snareshot",
        BuffKind::OwlTalon => "buff-owltalon",
        BuffKind::HeavyNock => "buff-heavynock",
        BuffKind::Heartseeker => "buff-heartseeker",
        // Debuffs
        BuffKind::Bleeding => "buff-bleed",
        BuffKind::Cursed => "buff-cursed",
        BuffKind::Burning => "buff-burn",
        BuffKind::Crippled => "buff-crippled",
        BuffKind::Frozen => "buff-frozen",
        BuffKind::Wet => "buff-wet",
        BuffKind::Ensnared => "buff-ensnared",
        BuffKind::Poisoned => "buff-poisoned",
        BuffKind::Parried => "buff-parried",
        BuffKind::PotionSickness => "buff-potionsickness",
        BuffKind::Heatstroke => "buff-heatstroke",
        BuffKind::Rooted => "buff-rooted",
        BuffKind::Winded => "buff-winded",
        BuffKind::Amnesia => "buff-amnesia",
        BuffKind::OffBalance => "buff-offbalance",
        // Neutral
        BuffKind::Polymorphed => "buff-polymorphed",
    }
}

/// Returns localized buff title
pub fn get_buff_title(buff: BuffKind, i18n: &Localization) -> Cow<'_, str> {
    let key = buff_key(buff);

    i18n.get_msg(key)
}

/// Returns localized buff description
pub fn get_buff_desc(buff: BuffKind, data: BuffData, i18n: &Localization) -> Cow<'_, str> {
    let key = buff_key(buff);
    if let BuffKind::RestingHeal = buff {
        i18n.get_attr_ctx(key, "desc", &i18n::fluent_args! {
            "rate" => data.strength * 100.0
        })
    } else {
        i18n.get_attr(key, "desc")
    }
}

fn almost_integer(number: &f32) -> FluentValue<'_> {
    let epsilon = 0.001;
    if number.fract() < epsilon {
        FluentValue::from(number.round() as usize)
    } else {
        FluentValue::from(format!("{:.<3}", number))
    }
}
/// Takes N `effects` and returns N effect descriptions
/// If effect isn't intended to have description, returns empty string
///
/// FIXME: handle which effects should have description in `stats_count`
/// to not waste space in item box
pub fn consumable_desc(effects: &Effects, i18n: &Localization) -> Vec<String> {
    let mut descriptions = Vec::new();
    match effects {
        Effects::Any(_) => {
            descriptions.push(i18n.get_msg("buff-mysterious").into_owned());
        },
        Effects::All(_) | Effects::One(_) => {
            for effect in effects.effects() {
                let mut description = String::new();
                if let Effect::Buff(buff) = effect {
                    let strength = buff.data.strength;
                    let duration = buff.data.duration.map(|d| d.0 as f32);
                    let str_total = duration.map_or(strength, |secs| strength * secs);
                    let str_duration = duration.unwrap_or(0.0);
                    let fluent_duration = almost_integer(&str_duration);

                    let format_float =
                        |input: f32| format!("{:.1}", input).trim_end_matches(".0").to_string();

                    let buff_desc = match buff.kind {
                        // These share common buff-key and show full possible regen
                        BuffKind::Saturation | BuffKind::Regeneration | BuffKind::Potion => {
                            let key = "buff-heal";
                            i18n.get_attr_ctx(key, "stat", &i18n::fluent_args! {
                                "str_total" => format_float(str_total),
                                "duration" => fluent_duration,
                            })
                        },
                        // Shows its full possible regen
                        BuffKind::EnergyRegen => {
                            let key = buff_key(buff.kind);
                            i18n.get_attr_ctx(key, "stat", &i18n::fluent_args! {
                                "str_total" => format_float(str_total),
                                "duration" => fluent_duration,
                            })
                        },
                        BuffKind::ComboGeneration => {
                            let key = buff_key(buff.kind);
                            i18n.get_attr_ctx(key, "stat", &i18n::fluent_args! {
                                "str_total" => format_float(str_total),
                                "duration" => fluent_duration,
                            })
                        },
                        // Show buff strength
                        BuffKind::IncreaseMaxEnergy
                        | BuffKind::IncreaseMaxHealth => {
                            let key = buff_key(buff.kind);
                            i18n.get_attr_ctx(key, "stat", &i18n::fluent_args! {
                                "strength" => format_float(strength),
                                "duration" => fluent_duration,
                            })
                        },
                        // Show percentage
                        BuffKind::PotionSickness
                        | BuffKind::Agility => {
                            let key = buff_key(buff.kind);
                            i18n.get_attr_ctx(key, "stat", &i18n::fluent_args! {
                                "strength" => format_float(strength * 100.0),
                                "duration" => fluent_duration,
                            })
                        },
                        // Independent of strength, still has duration
                        BuffKind::Invulnerability => {
                            let key = buff_key(buff.kind);
                            i18n.get_attr_ctx(key, "stat", &i18n::fluent_args! {
                                "duration" => fluent_duration,
                            })
                        },
                        // Have no stat description
                        BuffKind::Bleeding
                        | BuffKind::Burning
                        | BuffKind::RestingHeal
                        | BuffKind::Cursed
                        | BuffKind::ProtectingWard
                        | BuffKind::Crippled
                        | BuffKind::Frenzied
                        | BuffKind::Frozen
                        | BuffKind::Wet
                        | BuffKind::Ensnared
                        | BuffKind::Poisoned
                        | BuffKind::Hastened
                        | BuffKind::Fortitude
                        | BuffKind::Parried
                        | BuffKind::Reckless
                        | BuffKind::Polymorphed
                        | BuffKind::Flame
                        | BuffKind::Frigid
                        | BuffKind::Lifesteal
                        // | BuffKind::SalamanderAspect
                        | BuffKind::ImminentCritical
                        | BuffKind::Fury
                        | BuffKind::Sunderer
                        | BuffKind::Defiance
                        | BuffKind::Bloodfeast
                        | BuffKind::Berserk
                        | BuffKind::Heatstroke
                        | BuffKind::ScornfulTaunt
                        | BuffKind::Rooted
                        | BuffKind::Winded
                        | BuffKind::Amnesia
                        | BuffKind::OffBalance
                        | BuffKind::Tenacity
                        | BuffKind::Resilience
                        | BuffKind::SnareShot
                        | BuffKind::OwlTalon
                        | BuffKind::HeavyNock
                        | BuffKind::Heartseeker => Cow::Borrowed(""),
                    };

                    write!(&mut description, "{}", buff_desc).unwrap();
                }
                descriptions.push(description);
            }
        },
    }
    descriptions
}

// Armor
fn armor_kind<'a>(armor: &Armor, i18n: &'a Localization) -> Cow<'a, str> {
    match armor.kind {
        ArmorKind::Shoulder => i18n.get_msg("hud-bag-shoulders"),
        ArmorKind::Chest => i18n.get_msg("hud-bag-chest"),
        ArmorKind::Belt => i18n.get_msg("hud-bag-belt"),
        ArmorKind::Hand => i18n.get_msg("hud-bag-hands"),
        ArmorKind::Pants => i18n.get_msg("hud-bag-legs"),
        ArmorKind::Foot => i18n.get_msg("hud-bag-feet"),
        ArmorKind::Back => i18n.get_msg("hud-bag-back"),
        ArmorKind::Backpack => i18n.get_msg("hud-bag-backpack"),
        ArmorKind::Ring => i18n.get_msg("hud-bag-ring"),
        ArmorKind::Neck => i18n.get_msg("hud-bag-neck"),
        ArmorKind::Head => i18n.get_msg("hud-bag-head"),
        ArmorKind::Tabard => i18n.get_msg("hud-bag-tabard"),
        ArmorKind::Bag => i18n.get_msg("hud-bag-bag"),
    }
}

// Tool
fn tool_kind<'a>(tool: &Tool, i18n: &'a Localization) -> Cow<'a, str> {
    match tool.kind {
        ToolKind::Sword => i18n.get_msg("common-weapons-sword"),
        ToolKind::Axe => i18n.get_msg("common-weapons-axe"),
        ToolKind::Hammer => i18n.get_msg("common-weapons-hammer"),
        ToolKind::Bow => i18n.get_msg("common-weapons-bow"),
        ToolKind::Dagger => i18n.get_msg("common-weapons-dagger"),
        ToolKind::Staff => i18n.get_msg("common-weapons-staff"),
        ToolKind::Sceptre => i18n.get_msg("common-weapons-sceptre"),
        ToolKind::Shield => i18n.get_msg("common-weapons-shield"),
        ToolKind::Spear => i18n.get_msg("common-weapons-spear"),
        ToolKind::Blowgun => i18n.get_msg("common-weapons-blowgun"),
        ToolKind::Natural => i18n.get_msg("common-weapons-natural"),
        ToolKind::Debug => i18n.get_msg("common-tool-debug"),
        ToolKind::Farming => i18n.get_msg("common-tool-farming"),
        ToolKind::Instrument => i18n.get_msg("common-tool-instrument"),
        ToolKind::Throwable => i18n.get_msg("common-tool-throwable"),
        ToolKind::Pick => i18n.get_msg("common-tool-pick"),
        ToolKind::Shovel => i18n.get_msg("common-tool-shovel"),
        ToolKind::Empty => i18n.get_msg("common-empty"),
    }
}

/// Output the number of hands needed to hold a tool
pub fn tool_hands<'a>(tool: &Tool, i18n: &'a Localization) -> Cow<'a, str> {
    match tool.hands {
        Hands::One => i18n.get_msg("common-hands-one"),
        Hands::Two => i18n.get_msg("common-hands-two"),
    }
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

/// Gets the durability of an item in a format more intuitive for UI
pub fn item_durability(item: &dyn ItemDesc) -> Option<u32> {
    let durability = item
        .durability_lost()
        .or_else(|| item.has_durability().then_some(0));
    durability.map(|d| Item::MAX_DURABILITY - d)
}

pub fn ability_image(imgs: &img_ids::Imgs, ability_id: &str) -> image::Id {
    match ability_id {
        // Debug stick
        "common.abilities.debug.forwardboost" => imgs.flyingrod_m1,
        "common.abilities.debug.upboost" => imgs.flyingrod_m2,
        "common.abilities.debug.possess" => imgs.snake_arrow_0,
        // Sword
        "veloren.core.pseudo_abilities.sword.heavy_stance" => imgs.sword_heavy_stance,
        "veloren.core.pseudo_abilities.sword.agile_stance" => imgs.sword_agile_stance,
        "veloren.core.pseudo_abilities.sword.defensive_stance" => imgs.sword_defensive_stance,
        "veloren.core.pseudo_abilities.sword.crippling_stance" => imgs.sword_crippling_stance,
        "veloren.core.pseudo_abilities.sword.cleaving_stance" => imgs.sword_cleaving_stance,
        "veloren.core.pseudo_abilities.sword.double_slash" => imgs.sword_double_slash,
        "common.abilities.sword.basic_double_slash" => imgs.sword_basic_double_slash,
        "common.abilities.sword.heavy_double_slash" => imgs.sword_heavy_double_slash,
        "common.abilities.sword.agile_double_slash" => imgs.sword_agile_double_slash,
        "common.abilities.sword.defensive_double_slash" => imgs.sword_defensive_double_slash,
        "common.abilities.sword.crippling_double_slash" => imgs.sword_crippling_double_slash,
        "common.abilities.sword.cleaving_double_slash" => imgs.sword_cleaving_double_slash,
        "veloren.core.pseudo_abilities.sword.secondary_ability" => imgs.sword_secondary_ability,
        "common.abilities.sword.basic_thrust" => imgs.sword_basic_thrust,
        "common.abilities.sword.heavy_slam" => imgs.sword_heavy_slam,
        "common.abilities.sword.agile_perforate" => imgs.sword_agile_perforate,
        "common.abilities.sword.agile_dual_perforate" => imgs.sword_agile_perforate,
        "common.abilities.sword.defensive_vital_jab" => imgs.sword_defensive_vital_jab,
        "common.abilities.sword.crippling_deep_rend" => imgs.sword_crippling_deep_rend,
        "common.abilities.sword.cleaving_spiral_slash" => imgs.sword_cleaving_spiral_slash,
        "common.abilities.sword.cleaving_dual_spiral_slash" => imgs.sword_cleaving_spiral_slash,
        "veloren.core.pseudo_abilities.sword.crescent_slash" => imgs.sword_crescent_slash,
        "common.abilities.sword.basic_crescent_slash" => imgs.sword_basic_crescent_slash,
        "common.abilities.sword.heavy_crescent_slash" => imgs.sword_heavy_crescent_slash,
        "common.abilities.sword.agile_crescent_slash" => imgs.sword_agile_crescent_slash,
        "common.abilities.sword.defensive_crescent_slash" => imgs.sword_defensive_crescent_slash,
        "common.abilities.sword.crippling_crescent_slash" => imgs.sword_crippling_crescent_slash,
        "common.abilities.sword.cleaving_crescent_slash" => imgs.sword_cleaving_crescent_slash,
        "veloren.core.pseudo_abilities.sword.fell_strike" => imgs.sword_fell_strike,
        "common.abilities.sword.basic_fell_strike" => imgs.sword_basic_fell_strike,
        "common.abilities.sword.heavy_fell_strike" => imgs.sword_heavy_fell_strike,
        "common.abilities.sword.agile_fell_strike" => imgs.sword_agile_fell_strike,
        "common.abilities.sword.defensive_fell_strike" => imgs.sword_defensive_fell_strike,
        "common.abilities.sword.crippling_fell_strike" => imgs.sword_crippling_fell_strike,
        "common.abilities.sword.cleaving_fell_strike" => imgs.sword_cleaving_fell_strike,
        "veloren.core.pseudo_abilities.sword.skewer" => imgs.sword_skewer,
        "common.abilities.sword.basic_skewer" => imgs.sword_basic_skewer,
        "common.abilities.sword.heavy_skewer" => imgs.sword_heavy_skewer,
        "common.abilities.sword.agile_skewer" => imgs.sword_agile_skewer,
        "common.abilities.sword.defensive_skewer" => imgs.sword_defensive_skewer,
        "common.abilities.sword.crippling_skewer" => imgs.sword_crippling_skewer,
        "common.abilities.sword.cleaving_skewer" => imgs.sword_cleaving_skewer,
        "veloren.core.pseudo_abilities.sword.cascade" => imgs.sword_cascade,
        "common.abilities.sword.basic_cascade" => imgs.sword_basic_cascade,
        "common.abilities.sword.heavy_cascade" => imgs.sword_heavy_cascade,
        "common.abilities.sword.agile_cascade" => imgs.sword_agile_cascade,
        "common.abilities.sword.defensive_cascade" => imgs.sword_defensive_cascade,
        "common.abilities.sword.crippling_cascade" => imgs.sword_crippling_cascade,
        "common.abilities.sword.cleaving_cascade" => imgs.sword_cleaving_cascade,
        "veloren.core.pseudo_abilities.sword.cross_cut" => imgs.sword_cross_cut,
        "common.abilities.sword.basic_cross_cut" => imgs.sword_basic_cross_cut,
        "common.abilities.sword.heavy_cross_cut" => imgs.sword_heavy_cross_cut,
        "common.abilities.sword.agile_cross_cut" => imgs.sword_agile_cross_cut,
        "common.abilities.sword.defensive_cross_cut" => imgs.sword_defensive_cross_cut,
        "common.abilities.sword.crippling_cross_cut" => imgs.sword_crippling_cross_cut,
        "common.abilities.sword.cleaving_cross_cut" => imgs.sword_cleaving_cross_cut,
        "common.abilities.sword.basic_dual_cross_cut" => imgs.sword_basic_cross_cut,
        "common.abilities.sword.heavy_dual_cross_cut" => imgs.sword_heavy_cross_cut,
        "common.abilities.sword.agile_dual_cross_cut" => imgs.sword_agile_cross_cut,
        "common.abilities.sword.defensive_dual_cross_cut" => imgs.sword_defensive_cross_cut,
        "common.abilities.sword.crippling_dual_cross_cut" => imgs.sword_crippling_cross_cut,
        "common.abilities.sword.cleaving_dual_cross_cut" => imgs.sword_cleaving_cross_cut,
        "veloren.core.pseudo_abilities.sword.finisher" => imgs.sword_finisher,
        "common.abilities.sword.basic_mighty_strike" => imgs.sword_basic_mighty_strike,
        "common.abilities.sword.heavy_guillotine" => imgs.sword_heavy_guillotine,
        "common.abilities.sword.agile_hundred_cuts" => imgs.sword_agile_hundred_cuts,
        "common.abilities.sword.defensive_counter" => imgs.sword_defensive_counter,
        "common.abilities.sword.crippling_mutilate" => imgs.sword_crippling_mutilate,
        "common.abilities.sword.cleaving_bladestorm" => imgs.sword_cleaving_bladestorm,
        "common.abilities.sword.cleaving_dual_bladestorm" => imgs.sword_cleaving_bladestorm,
        "common.abilities.sword.heavy_sweep" => imgs.sword_heavy_sweep,
        "common.abilities.sword.heavy_pommel_strike" => imgs.sword_heavy_pommel_strike,
        "common.abilities.sword.agile_quick_draw" => imgs.sword_agile_quick_draw,
        "common.abilities.sword.agile_feint" => imgs.sword_agile_feint,
        "common.abilities.sword.defensive_riposte" => imgs.sword_defensive_riposte,
        "common.abilities.sword.defensive_disengage" => imgs.sword_defensive_disengage,
        "common.abilities.sword.crippling_gouge" => imgs.sword_crippling_gouge,
        "common.abilities.sword.crippling_hamstring" => imgs.sword_crippling_hamstring,
        "common.abilities.sword.cleaving_whirlwind_slice" => imgs.sword_cleaving_whirlwind_slice,
        "common.abilities.sword.cleaving_dual_whirlwind_slice" => {
            imgs.sword_cleaving_whirlwind_slice
        },
        "common.abilities.sword.cleaving_earth_splitter" => imgs.sword_cleaving_earth_splitter,
        "common.abilities.sword.heavy_fortitude" => imgs.sword_heavy_fortitude,
        "common.abilities.sword.heavy_pillar_thrust" => imgs.sword_heavy_pillar_thrust,
        "common.abilities.sword.agile_dancing_edge" => imgs.sword_agile_dancing_edge,
        "common.abilities.sword.agile_flurry" => imgs.sword_agile_flurry,
        "common.abilities.sword.agile_dual_flurry" => imgs.sword_agile_flurry,
        "common.abilities.sword.defensive_stalwart_sword" => imgs.sword_defensive_stalwart_sword,
        "common.abilities.sword.defensive_deflect" => imgs.sword_defensive_deflect,
        "common.abilities.sword.crippling_eviscerate" => imgs.sword_crippling_eviscerate,
        "common.abilities.sword.crippling_bloody_gash" => imgs.sword_crippling_bloody_gash,
        "common.abilities.sword.cleaving_blade_fever" => imgs.sword_cleaving_blade_fever,
        "common.abilities.sword.cleaving_sky_splitter" => imgs.sword_cleaving_sky_splitter,
        // Axe
        "common.abilities.axe.triple_chop" => imgs.axe_triple_chop,
        "common.abilities.axe.cleave" => imgs.axe_cleave,
        "common.abilities.axe.brutal_swing" => imgs.axe_brutal_swing,
        "common.abilities.axe.berserk" => imgs.axe_berserk,
        "common.abilities.axe.rising_tide" => imgs.axe_rising_tide,
        "common.abilities.axe.savage_sense" => imgs.axe_savage_sense,
        "common.abilities.axe.adrenaline_rush" => imgs.axe_adrenaline_rush,
        "common.abilities.axe.execute" => imgs.axe_execute,
        "common.abilities.axe.maelstrom" => imgs.axe_maelstrom,
        "common.abilities.axe.rake" => imgs.axe_rake,
        "common.abilities.axe.bloodfeast" => imgs.axe_bloodfeast,
        "common.abilities.axe.fierce_raze" => imgs.axe_fierce_raze,
        "common.abilities.axe.dual_fierce_raze" => imgs.axe_fierce_raze,
        "common.abilities.axe.furor" => imgs.axe_furor,
        "common.abilities.axe.fracture" => imgs.axe_fracture,
        "common.abilities.axe.lacerate" => imgs.axe_lacerate,
        "common.abilities.axe.riptide" => imgs.axe_riptide,
        "common.abilities.axe.skull_bash" => imgs.axe_skull_bash,
        "common.abilities.axe.sunder" => imgs.axe_sunder,
        "common.abilities.axe.plunder" => imgs.axe_plunder,
        "common.abilities.axe.defiance" => imgs.axe_defiance,
        "common.abilities.axe.keelhaul" => imgs.axe_keelhaul,
        "common.abilities.axe.bulkhead" => imgs.axe_bulkhead,
        "common.abilities.axe.capsize" => imgs.axe_capsize,
        // Hammer
        "common.abilities.hammer.solid_smash" => imgs.hammer_solid_smash,
        "common.abilities.hammer.wide_wallop" => imgs.hammer_wide_wallop,
        "common.abilities.hammer.scornful_swipe" => imgs.hammer_scornful_swipe,
        "common.abilities.hammer.tremor" => imgs.hammer_tremor,
        "common.abilities.hammer.vigorous_bash" => imgs.hammer_vigorous_bash,
        "common.abilities.hammer.heavy_whorl" => imgs.hammer_heavy_whorl,
        "common.abilities.hammer.dual_heavy_whorl" => imgs.hammer_heavy_whorl,
        "common.abilities.hammer.intercept" => imgs.hammer_intercept,
        "common.abilities.hammer.dual_intercept" => imgs.hammer_intercept,
        "common.abilities.hammer.retaliate" => imgs.hammer_retaliate,
        "common.abilities.hammer.spine_cracker" => imgs.hammer_spine_cracker,
        "common.abilities.hammer.breach" => imgs.hammer_breach,
        "common.abilities.hammer.pile_driver" => imgs.hammer_pile_driver,
        "common.abilities.hammer.lung_pummel" => imgs.hammer_lung_pummel,
        "common.abilities.hammer.helm_crusher" => imgs.hammer_helm_crusher,
        "common.abilities.hammer.iron_tempest" => imgs.hammer_iron_tempest,
        "common.abilities.hammer.dual_iron_tempest" => imgs.hammer_iron_tempest,
        "common.abilities.hammer.upheaval" => imgs.hammer_upheaval,
        "common.abilities.hammer.dual_upheaval" => imgs.hammer_upheaval,
        "common.abilities.hammer.rampart" => imgs.hammer_rampart,
        "common.abilities.hammer.tenacity" => imgs.hammer_tenacity,
        "common.abilities.hammer.thunderclap" => imgs.hammer_thunderclap,
        "common.abilities.hammer.seismic_shock" => imgs.hammer_seismic_shock,
        "common.abilities.hammer.earthshaker" => imgs.hammer_earthshaker,
        "common.abilities.hammer.judgement" => imgs.hammer_judgement,
        // Bow
        "common.abilities.bow.arrow_shot" => imgs.bow_arrow_shot,
        "common.abilities.bow.broadhead" => imgs.bow_broadhead,
        "common.abilities.bow.foothold" => imgs.bow_foothold,
        "common.abilities.bow.snare_shot" => imgs.bow_snare_shot,
        "common.abilities.bow.barrage" => imgs.bow_barrage,
        "common.abilities.bow.barrage_shot" => imgs.bow_barrage,
        "common.abilities.bow.owl_talon" => imgs.bow_owl_talon,
        "common.abilities.bow.heavy_nock" => imgs.bow_heavy_nock,
        "common.abilities.bow.heartseeker" => imgs.bow_heartseeker,
        "common.abilities.bow.scatterburst" => imgs.bow_scatterburst,
        "common.abilities.bow.lesser_scatterburst" => imgs.bow_lesser_scatterburst,
        "common.abilities.bow.greater_scatterburst" => imgs.bow_greater_scatterburst,
        // Staff
        "common.abilities.staff.firebomb" => imgs.fireball,
        "common.abilities.staff.flamethrower" => imgs.flamethrower,
        "common.abilities.staff.fireshockwave" => imgs.fire_aoe,
        // Sceptre
        "common.abilities.sceptre.lifestealbeam" => imgs.skill_sceptre_lifesteal,
        "common.abilities.sceptre.healingaura" => imgs.skill_sceptre_heal,
        "common.abilities.sceptre.wardingaura" => imgs.skill_sceptre_aura,
        // Shield
        "common.abilities.shield.singlestrike" => imgs.onehshield_m1,
        "common.abilities.shield.power_guard" => imgs.onehshield_m1,
        // Dagger
        "common.abilities.dagger.tempbasic" => imgs.onehdagger_m1,
        // Pickaxe
        "common.abilities.pick.swing" => imgs.mining,
        // Shovel
        "common.abilities.shovel.dig" => imgs.dig,
        // Instruments
        "common.abilities.music.bass" => imgs.instrument,
        "common.abilities.music.flute" => imgs.instrument,
        "common.abilities.music.harp" => imgs.instrument,
        "common.abilities.music.perc" => imgs.instrument,
        "common.abilities.music.kalimba" => imgs.instrument,
        "common.abilities.music.melodica" => imgs.instrument,
        "common.abilities.music.lute" => imgs.instrument,
        "common.abilities.music.oud" => imgs.instrument,
        "common.abilities.music.guitar" => imgs.instrument,
        "common.abilities.music.dark_guitar" => imgs.instrument,
        "common.abilities.music.sitar" => imgs.instrument,
        "common.abilities.music.double_bass" => imgs.instrument,
        "common.abilities.music.glass_flute" => imgs.instrument,
        "common.abilities.music.lyre" => imgs.instrument,
        "common.abilities.music.wildskin_drum" => imgs.instrument,
        "common.abilities.music.icy_talharpa" => imgs.instrument,
        "common.abilities.music.washboard" => imgs.instrument,
        "common.abilities.music.steeltonguedrum" => imgs.instrument,
        "common.abilities.music.shamisen" => imgs.instrument,
        "common.abilities.music.kora" => imgs.instrument,
        "common.abilities.music.banjo" => imgs.instrument,
        "common.abilities.music.viola_pizzicato" => imgs.instrument,
        "common.abilities.music.starlight_conch" => imgs.instrument,
        "common.abilities.music.timbrel_of_chaos" => imgs.instrument,
        "common.abilities.music.rhythmo" => imgs.instrument,
        // Glider
        "common.abilities.debug.glide_boost" => imgs.flyingrod_m2,
        "common.abilities.debug.glide_speeder" => imgs.flyingrod_m1,
        _ => imgs.not_found,
    }
}

pub fn ability_description<'a>(
    ability_id: &str,
    loc: &'a Localization,
) -> (Cow<'a, str>, Cow<'a, str>) {
    let ability_i18n_key = ability_id.replace('.', "-");
    match ability_i18n_key.as_str() {
        "common-abilities-axe-execute"
        | "common-abilities-axe-maelstrom"
        | "common-abilities-axe-lacerate"
        | "common-abilities-axe-riptide"
        | "common-abilities-axe-bulkhead"
        | "common-abilities-axe-capsize" => (
            loc.get_msg(&ability_i18n_key),
            loc.get_attr_ctx(&ability_i18n_key, "desc", &i18n::fluent_args! {
                "min_combo" => 25,
                "min_combo_upg" => 40,
            }),
        ),
        "common-abilities-hammer-earthshaker"
        | "common-abilities-hammer-judgement"
        | "common-abilities-hammer-seismic_shock"
        | "common-abilities-hammer-thunderclap" => (
            loc.get_msg(&ability_i18n_key),
            loc.get_attr_ctx(&ability_i18n_key, "desc", &i18n::fluent_args! {
                "min_combo" => 20,
            }),
        ),
        "common-abilities-hammer-lung_pummel" | "common-abilities-hammer-spine_cracker" => (
            loc.get_msg(&ability_i18n_key),
            loc.get_attr_ctx(&ability_i18n_key, "desc", &i18n::fluent_args! {
                "min_combo" => 5,
            }),
        ),
        "common-abilities-hammer-helm_crusher" => (
            loc.get_msg(&ability_i18n_key),
            loc.get_attr_ctx(&ability_i18n_key, "desc", &i18n::fluent_args! {
                "min_combo" => 10,
            }),
        ),
        // Default case, no input values
        ability_i18n_key => (
            loc.get_msg(ability_i18n_key),
            loc.get_attr(ability_i18n_key, "desc"),
        ),
    }
}
