use super::img_ids;
use common::{
    comp::{
        inventory::trade_pricing::TradePricing,
        item::{
            armor::{Armor, ArmorKind, Protection},
            tool::{Hands, Tool, ToolKind},
            ItemDefinitionId, ItemDesc, ItemKind, MaterialKind, MaterialStatManifest,
        },
        BuffKind,
    },
    effect::Effect,
    trade::{Good, SitePrices},
};
use conrod_core::image;
use i18n::{fluent_args, Localization};
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
        ItemKind::Throwable { .. } => i18n.get_msg("common-kind-throwable"),
        ItemKind::Utility { .. } => i18n.get_msg("common-kind-utility"),
        ItemKind::Ingredient { .. } => i18n.get_msg("common-kind-ingredient"),
        ItemKind::Lantern { .. } => i18n.get_msg("common-kind-lantern"),
        ItemKind::TagExamples { .. } => Cow::Borrowed(""),
    }
}

pub fn material_kind_text<'a>(kind: &MaterialKind, i18n: &'a Localization) -> Cow<'a, str> {
    match kind {
        MaterialKind::Metal { .. } => i18n.get_msg("common-material-metal"),
        MaterialKind::Wood { .. } => i18n.get_msg("common-material-wood"),
        MaterialKind::Stone { .. } => i18n.get_msg("common-material-stone"),
        MaterialKind::Cloth { .. } => i18n.get_msg("common-material-cloth"),
        MaterialKind::Hide { .. } => i18n.get_msg("common-material-hide"),
    }
}

pub fn stats_count(item: &dyn ItemDesc, msm: &MaterialStatManifest) -> usize {
    match &*item.kind() {
        ItemKind::Armor(armor) => {
            let armor_stats = armor.stats(msm);
            armor_stats.energy_reward.is_some() as usize
                + armor_stats.energy_max.is_some() as usize
                + armor_stats.stealth.is_some() as usize
                + armor_stats.crit_power.is_some() as usize
                + armor_stats.poise_resilience.is_some() as usize
                + armor_stats.protection.is_some() as usize
                + (item.num_slots() > 0) as usize
        },
        ItemKind::Tool(_) => 7,
        ItemKind::Consumable { effects, .. } => effects.len(),
        ItemKind::ModularComponent { .. } => 7,
        _ => 0,
    }
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
                    .get_msg_ctx("buff-stat-health", &i18n::fluent_args! {
                        "str_total" => format_float(str_total),
                    }),
                BuffKind::EnergyRegen => {
                    i18n.get_msg_ctx("buff-stat-energy_regen", &i18n::fluent_args! {
                        "str_total" => format_float(str_total),
                    })
                },
                BuffKind::IncreaseMaxEnergy => {
                    i18n.get_msg_ctx("buff-stat-increase_max_energy", &i18n::fluent_args! {
                        "strength" => format_float(strength),
                    })
                },
                BuffKind::IncreaseMaxHealth => {
                    i18n.get_msg_ctx("buff-stat-increase_max_health", &i18n::fluent_args! {
                        "strength" => format_float(strength),
                    })
                },
                BuffKind::Invulnerability => i18n.get_msg("buff-stat-invulnerability"),
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
                | BuffKind::Poisoned
                | BuffKind::Hastened
                | BuffKind::Fortitude
                | BuffKind::Parried => Cow::Borrowed(""),
            };

            write!(&mut description, "{}", buff_desc).unwrap();

            let dur_desc = if let Some(dur_secs) = dur_secs {
                match buff.kind {
                    BuffKind::Saturation | BuffKind::Regeneration | BuffKind::EnergyRegen => i18n
                        .get_msg_ctx("buff-text-over_seconds", &i18n::fluent_args! {
                            "dur_secs" => dur_secs
                        }),
                    BuffKind::IncreaseMaxEnergy
                    | BuffKind::IncreaseMaxHealth
                    | BuffKind::Invulnerability => {
                        i18n.get_msg_ctx("buff-text-for_seconds", &i18n::fluent_args! {
                            "dur_secs" => dur_secs
                        })
                    },
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
                    | BuffKind::Poisoned
                    | BuffKind::Hastened
                    | BuffKind::Fortitude
                    | BuffKind::Parried => Cow::Borrowed(""),
                }
            } else if let BuffKind::Saturation | BuffKind::Regeneration | BuffKind::EnergyRegen =
                buff.kind
            {
                i18n.get_msg("buff-text-every_second")
            } else {
                Cow::Borrowed("")
            };

            write!(&mut description, " {}", dur_desc).unwrap();
        }
        descriptions.push(description);
    }

    descriptions
}

// Armor
fn armor_kind<'a>(armor: &Armor, i18n: &'a Localization) -> Cow<'a, str> {
    let kind = match armor.kind {
        ArmorKind::Shoulder => i18n.get_msg("hud-bag-shoulders"),
        ArmorKind::Chest => i18n.get_msg("hud-bag-chest"),
        ArmorKind::Belt => i18n.get_msg("hud-bag-belt"),
        ArmorKind::Hand => i18n.get_msg("hud-bag-hands"),
        ArmorKind::Pants => i18n.get_msg("hud-bag-legs"),
        ArmorKind::Foot => i18n.get_msg("hud-bag-feet"),
        ArmorKind::Back => i18n.get_msg("hud-bag-back"),
        ArmorKind::Ring => i18n.get_msg("hud-bag-ring"),
        ArmorKind::Neck => i18n.get_msg("hud-bag-neck"),
        ArmorKind::Head => i18n.get_msg("hud-bag-head"),
        ArmorKind::Tabard => i18n.get_msg("hud-bag-tabard"),
        ArmorKind::Bag => i18n.get_msg("hud-bag-bag"),
    };
    kind
}

// Tool
fn tool_kind<'a>(tool: &Tool, i18n: &'a Localization) -> Cow<'a, str> {
    let kind = match tool.kind {
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
        ToolKind::Pick => i18n.get_msg("common-tool-pick"),
        ToolKind::Empty => i18n.get_msg("common-empty"),
    };
    kind
}

/// Output the number of hands needed to hold a tool
pub fn tool_hands<'a>(tool: &Tool, i18n: &'a Localization) -> Cow<'a, str> {
    let hands = match tool.hands {
        Hands::One => i18n.get_msg("common-hands-one"),
        Hands::Two => i18n.get_msg("common-hands-two"),
    };
    hands
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
        "common.abilities.sword.balanced_combo" => imgs.sword_balanced_combo,
        "common.abilities.sword.balanced_thrust" => imgs.sword_balanced_thrust,
        "common.abilities.sword.balanced_finisher" => imgs.sword_balanced_finisher,
        "common.abilities.sword.offensive_combo" => imgs.sword_offensive_combo,
        "common.abilities.sword.offensive_finisher" => imgs.sword_offensive_finisher,
        "common.abilities.sword.offensive_advance" => imgs.sword_offensive_advance,
        "common.abilities.sword.crippling_combo" => imgs.sword_crippling_combo,
        "common.abilities.sword.crippling_finisher" => imgs.sword_crippling_finisher,
        "common.abilities.sword.crippling_strike" => imgs.sword_crippling_strike,
        "common.abilities.sword.crippling_gouge" => imgs.sword_crippling_gouge,
        "common.abilities.sword.cleaving_combo" => imgs.sword_cleaving_combo,
        "common.abilities.sword.cleaving_finisher" => imgs.sword_cleaving_finisher,
        "common.abilities.sword.cleaving_spin" => imgs.sword_cleaving_spin,
        "common.abilities.sword.cleaving_dive" => imgs.sword_cleaving_dive,
        "common.abilities.sword.defensive_combo" => imgs.sword_defensive_combo,
        "common.abilities.sword.defensive_bulwark" => imgs.sword_defensive_bulwark,
        "common.abilities.sword.defensive_retreat" => imgs.sword_defensive_retreat,
        "common.abilities.sword.parrying_combo" => imgs.sword_parrying_combo,
        "common.abilities.sword.parrying_parry" => imgs.sword_parrying_parry,
        "common.abilities.sword.parrying_riposte" => imgs.sword_parrying_riposte,
        "common.abilities.sword.parrying_counter" => imgs.sword_parrying_counter,
        "common.abilities.sword.heavy_combo" => imgs.sword_heavy_combo,
        "common.abilities.sword.heavy_finisher" => imgs.sword_heavy_finisher,
        "common.abilities.sword.heavy_pommelstrike" => imgs.sword_heavy_pommelstrike,
        "common.abilities.sword.heavy_fortitude" => imgs.sword_heavy_fortitude,
        "common.abilities.sword.mobility_combo" => imgs.sword_mobility_combo,
        "common.abilities.sword.mobility_feint" => imgs.sword_mobility_feint,
        "common.abilities.sword.mobility_agility" => imgs.sword_mobility_agility,
        "common.abilities.sword.reaching_combo" => imgs.sword_reaching_combo,
        "common.abilities.sword.reaching_charge" => imgs.sword_reaching_charge,
        "common.abilities.sword.reaching_flurry" => imgs.sword_reaching_flurry,
        "common.abilities.sword.reaching_skewer" => imgs.sword_reaching_skewer,
        "veloren.core.pseudo_abilities.sword.stance_ability" => imgs.sword,
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
        // Instruments
        "common.abilities.music.bass" => imgs.instrument,
        "common.abilities.music.flute" => imgs.instrument,
        "common.abilities.music.harp" => imgs.instrument,
        "common.abilities.music.perc" => imgs.instrument,
        "common.abilities.music.kalimba" => imgs.instrument,
        "common.abilities.music.melodica" => imgs.instrument,
        "common.abilities.music.lute" => imgs.instrument,
        "common.abilities.music.guitar" => imgs.instrument,
        "common.abilities.music.sitar" => imgs.instrument,
        _ => imgs.not_found,
    }
}

pub fn ability_description<'a>(
    ability_id: &str,
    loc: &'a Localization,
) -> (Cow<'a, str>, Cow<'a, str>) {
    let ability = ability_id.replace('.', "-");

    (loc.get_msg(&ability), loc.get_attr(&ability, "desc"))
}
