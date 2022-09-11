#![deny(clippy::clone_on_ref_ptr)]
#![allow(clippy::expect_fun_call)] //TODO: evaluate to remove this and use `unwrap_or_else(panic!(...))` instead

use hashbrown::HashMap;
use ron::ser::{to_string_pretty, PrettyConfig};
use serde::Serialize;
use std::{error::Error, fs::File, io::Write};
use structopt::StructOpt;

use veloren_common::{
    assets::ASSETS_PATH,
    comp::{
        self,
        item::{
            armor::{ArmorKind, Protection, StatsSource},
            tool::{AbilitySpec, Hands, Stats},
            ItemDefinitionId, ItemKind, ItemTag, Quality,
        },
    },
};

#[derive(StructOpt)]
struct Cli {
    /// Available arguments: "armor-stats", "weapon-stats"
    function: String,
}

#[derive(Serialize)]
struct FakeItemDef {
    name: String,
    description: String,
    kind: ItemKind,
    quality: Quality,
    tags: Vec<ItemTag>,
    ability_spec: Option<AbilitySpec>,
}

impl FakeItemDef {
    fn new(
        name: String,
        description: String,
        kind: ItemKind,
        quality: Quality,
        tags: Vec<ItemTag>,
        ability_spec: Option<AbilitySpec>,
    ) -> Self {
        Self {
            name,
            description,
            kind,
            quality,
            tags,
            ability_spec,
        }
    }
}

fn armor_stats() -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::Reader::from_path("armorstats.csv")?;

    let headers: HashMap<String, usize> = rdr
        .headers()
        .expect("Failed to read CSV headers")
        .iter()
        .enumerate()
        .map(|(i, x)| (x.to_string(), i))
        .collect();

    for record in rdr.records() {
        for item in comp::item::Item::new_from_asset_glob("common.items.armor.*")
            .expect("Failed to iterate over item folders!")
        {
            match &*item.kind() {
                ItemKind::Armor(armor) => {
                    if let ArmorKind::Bag = armor.kind {
                        continue;
                    }

                    if let Ok(ref record) = record {
                        if item.item_definition_id()
                            == ItemDefinitionId::Simple(
                                record.get(headers["Path"]).expect("No file path in csv?"),
                            )
                        {
                            let protection =
                                if let Some(protection_raw) = record.get(headers["Protection"]) {
                                    if protection_raw == "Invincible" {
                                        Some(Protection::Invincible)
                                    } else {
                                        let value: f32 = protection_raw.parse().unwrap();
                                        if value == 0.0 {
                                            None
                                        } else {
                                            Some(Protection::Normal(value))
                                        }
                                    }
                                } else {
                                    eprintln!(
                                        "Could not unwrap protection value for {:?}",
                                        item.item_definition_id()
                                    );
                                    None
                                };

                            let poise_resilience = if let Some(poise_resilience_raw) =
                                record.get(headers["Poise Resilience"])
                            {
                                if poise_resilience_raw == "Invincible" {
                                    Some(Protection::Invincible)
                                } else {
                                    let value: f32 = poise_resilience_raw.parse().unwrap();
                                    if value == 0.0 {
                                        None
                                    } else {
                                        Some(Protection::Normal(value))
                                    }
                                }
                            } else {
                                eprintln!(
                                    "Could not unwrap poise protection value for {:?}",
                                    item.item_definition_id()
                                );
                                None
                            };

                            let energy_max =
                                if let Some(max_energy_raw) = record.get(headers["Max Energy"]) {
                                    let value = max_energy_raw.parse().unwrap();
                                    if value == 0.0 { None } else { Some(value) }
                                } else {
                                    eprintln!(
                                        "Could not unwrap max energy value for {:?}",
                                        item.item_definition_id()
                                    );
                                    None
                                };

                            let energy_reward = if let Some(energy_reward_raw) =
                                record.get(headers["Energy Reward"])
                            {
                                let value = energy_reward_raw.parse().unwrap();
                                if value == 0.0 { None } else { Some(value) }
                            } else {
                                eprintln!(
                                    "Could not unwrap energy recovery value for {:?}",
                                    item.item_definition_id()
                                );
                                None
                            };

                            let crit_power =
                                if let Some(crit_power_raw) = record.get(headers["Crit Power"]) {
                                    let value = crit_power_raw.parse().unwrap();
                                    if value == 0.0 { None } else { Some(value) }
                                } else {
                                    eprintln!(
                                        "Could not unwrap crit power value for {:?}",
                                        item.item_definition_id()
                                    );
                                    None
                                };

                            let stealth = if let Some(stealth_raw) = record.get(headers["Stealth"])
                            {
                                let value = stealth_raw.parse().unwrap();
                                if value == 0.0 { None } else { Some(value) }
                            } else {
                                eprintln!(
                                    "Could not unwrap stealth value for {:?}",
                                    item.item_definition_id()
                                );
                                None
                            };

                            let kind = armor.kind;
                            let armor_stats = comp::item::armor::Stats {
                                protection,
                                poise_resilience,
                                energy_max,
                                energy_reward,
                                crit_power,
                                stealth,
                                ground_contact: Default::default(),
                            };
                            let armor = comp::item::armor::Armor::new(
                                kind,
                                StatsSource::Direct(armor_stats),
                            );
                            let quality = if let Some(quality_raw) = record.get(headers["Quality"])
                            {
                                match quality_raw {
                                    "Low" => Quality::Low,
                                    "Common" => Quality::Common,
                                    "Moderate" => Quality::Moderate,
                                    "High" => Quality::High,
                                    "Epic" => Quality::Epic,
                                    "Legendary" => Quality::Legendary,
                                    "Artifact" => Quality::Artifact,
                                    "Debug" => Quality::Debug,
                                    _ => {
                                        eprintln!(
                                            "Unknown quality variant for {:?}",
                                            item.item_definition_id()
                                        );
                                        Quality::Debug
                                    },
                                }
                            } else {
                                eprintln!(
                                    "Could not unwrap quality for {:?}",
                                    item.item_definition_id()
                                );
                                Quality::Debug
                            };

                            let description = record
                                .get(headers["Description"])
                                .expect(&format!(
                                    "Error unwrapping description for {:?}",
                                    item.item_definition_id()
                                ))
                                .replace("\\'", "'");

                            let fake_item = FakeItemDef::new(
                                item.name().to_string(),
                                description.to_string(),
                                ItemKind::Armor(armor),
                                quality,
                                item.tags().to_vec(),
                                item.ability_spec().map(|spec| spec.into_owned()),
                            );

                            let pretty_config = PrettyConfig::new()
                                .depth_limit(4)
                                .separate_tuple_members(true)
                                .decimal_floats(true)
                                .enumerate_arrays(true);

                            let mut path = ASSETS_PATH.clone();
                            for part in item
                                .item_definition_id()
                                .itemdef_id()
                                .expect("Csv import only works on simple items, not modular items")
                                .split('.')
                            {
                                path.push(part);
                            }
                            path.set_extension("ron");

                            let path_str = path.to_str().expect("File path not unicode?!");
                            let mut writer = File::create(path_str)?;
                            write!(
                                writer,
                                "ItemDef{}",
                                to_string_pretty(&fake_item, pretty_config)?.replace("\\'", "'")
                            )?;
                        }
                    }
                },
                _ => println!("Skipping non-armor item: {:?}\n", item),
            }
        }
    }

    Ok(())
}

fn weapon_stats() -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::Reader::from_path("weaponstats.csv")?;

    let headers: HashMap<String, usize> = rdr
        .headers()
        .expect("Failed to read CSV headers")
        .iter()
        .enumerate()
        .map(|(i, x)| (x.to_string(), i))
        .collect();

    for record in rdr.records() {
        // Does all items even outside weapons since we check if itemkind was a tool
        // anyways
        let items: Vec<comp::Item> = comp::Item::new_from_asset_glob("common.items.*")
            .expect("Failed to iterate over item folders!");

        for item in items.iter() {
            if let ItemKind::Tool(tool) = &*item.kind() {
                if let Ok(ref record) = record {
                    if item.item_definition_id()
                        == ItemDefinitionId::Simple(
                            record.get(headers["Path"]).expect("No file path in csv?"),
                        )
                    {
                        let kind = tool.kind;
                        let equip_time_secs: f32 = record
                            .get(headers["Equip Time (s)"])
                            .expect(&format!(
                                "Error unwrapping equip time for {:?}",
                                item.item_definition_id()
                            ))
                            .parse()
                            .expect(&format!("Not a u32? {:?}", item.item_definition_id()));
                        let power: f32 = record
                            .get(headers["Power"])
                            .expect(&format!(
                                "Error unwrapping power for {:?}",
                                item.item_definition_id()
                            ))
                            .parse()
                            .expect(&format!("Not a f32? {:?}", item.item_definition_id()));
                        let effect_power: f32 = record
                            .get(headers["Effect Power"])
                            .expect(&format!(
                                "Error unwrapping effect power for {:?}",
                                item.item_definition_id()
                            ))
                            .parse()
                            .expect(&format!("Not a f32? {:?}", item.item_definition_id()));

                        let speed: f32 = record
                            .get(headers["Speed"])
                            .expect(&format!(
                                "Error unwrapping speed for {:?}",
                                item.item_definition_id()
                            ))
                            .parse()
                            .expect(&format!("Not a f32? {:?}", item.item_definition_id()));

                        let hands = if let Some(hands_raw) = record.get(headers["Hands"]) {
                            match hands_raw {
                                "One" | "1" | "1h" => Hands::One,
                                "Two" | "2" | "2h" => Hands::Two,
                                _ => {
                                    eprintln!(
                                        "Unknown hand variant for {:?}",
                                        item.item_definition_id()
                                    );
                                    Hands::Two
                                },
                            }
                        } else {
                            eprintln!("Could not unwrap hand for {:?}", item.item_definition_id());
                            Hands::Two
                        };

                        let crit_chance: f32 = record
                            .get(headers["Crit Chance"])
                            .expect(&format!(
                                "Error unwrapping crit_chance for {:?}",
                                item.item_definition_id()
                            ))
                            .parse()
                            .expect(&format!("Not a f32? {:?}", item.item_definition_id()));

                        let range: f32 = record
                            .get(headers["Range"])
                            .expect(&format!(
                                "Error unwrapping range for {:?}",
                                item.item_definition_id()
                            ))
                            .parse()
                            .expect(&format!("Not a f32? {:?}", item.item_definition_id()));

                        let energy_efficiency: f32 = record
                            .get(headers["Energy Efficiency"])
                            .expect(&format!(
                                "Error unwrapping energy efficiency for {:?}",
                                item.item_definition_id()
                            ))
                            .parse()
                            .expect(&format!("Not a f32? {:?}", item.item_definition_id()));

                        let buff_strength: f32 = record
                            .get(headers["Buff Strength"])
                            .expect(&format!(
                                "Error unwrapping buff strength for {:?}",
                                item.item_definition_id()
                            ))
                            .parse()
                            .expect(&format!("Not a f32? {:?}", item.item_definition_id()));

                        let tool = comp::item::tool::Tool::new(kind, hands, Stats {
                            equip_time_secs,
                            power,
                            effect_power,
                            speed,
                            crit_chance,
                            range,
                            energy_efficiency,
                            buff_strength,
                        });

                        let quality = if let Some(quality_raw) = record.get(headers["Quality"]) {
                            match quality_raw {
                                "Low" => Quality::Low,
                                "Common" => Quality::Common,
                                "Moderate" => Quality::Moderate,
                                "High" => Quality::High,
                                "Epic" => Quality::Epic,
                                "Legendary" => Quality::Legendary,
                                "Artifact" => Quality::Artifact,
                                "Debug" => Quality::Debug,
                                _ => {
                                    eprintln!(
                                        "Unknown quality variant for {:?}",
                                        item.item_definition_id()
                                    );
                                    Quality::Debug
                                },
                            }
                        } else {
                            eprintln!(
                                "Could not unwrap quality for {:?}",
                                item.item_definition_id()
                            );
                            Quality::Debug
                        };

                        let description = record.get(headers["Description"]).expect(&format!(
                            "Error unwrapping description for {:?}",
                            item.item_definition_id()
                        ));

                        let fake_item = FakeItemDef::new(
                            item.name().to_string(),
                            description.to_string(),
                            ItemKind::Tool(tool),
                            quality,
                            item.tags().to_vec(),
                            item.ability_spec().map(|spec| spec.into_owned()),
                        );

                        let pretty_config = PrettyConfig::new()
                            .depth_limit(4)
                            .separate_tuple_members(true)
                            .decimal_floats(true)
                            .enumerate_arrays(true);

                        let mut path = ASSETS_PATH.clone();
                        for part in item
                            .item_definition_id()
                            .itemdef_id()
                            .expect("Csv import only works on simple items, not modular items")
                            .split('.')
                        {
                            path.push(part);
                        }
                        path.set_extension("ron");

                        let path_str = path.to_str().expect("File path not unicode?!");
                        let mut writer = File::create(path_str)?;
                        write!(
                            writer,
                            "ItemDef{}",
                            to_string_pretty(&fake_item, pretty_config)?.replace("\\'", "'")
                        )?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn main() {
    let args = Cli::from_args();
    if args.function.eq_ignore_ascii_case("armor-stats") {
        if get_input(
            "
-------------------------------------------------------------------------------
|                                 DISCLAIMER                                  |
-------------------------------------------------------------------------------
|                                                                             |
|   This script will wreck the RON files for armor if it messes up.           |
|   You might want to save a back up of the weapon files or be prepared to    |
|   use `git checkout HEAD -- ../assets/common/items/armor/*` if needed.      |
|   If this script does mess up your files, please fix it. Otherwise your     |
|   files will be yeeted away and you will get a bonk on the head.            |
|                                                                             |
-------------------------------------------------------------------------------

In order for this script to work, you need to have first run the csv exporter.
Once you have armorstats.csv you can make changes to stats, quality, and
description in your preferred editor. Save the csv file and then run this
script to import your changes back to RON.

Would you like to continue? (y/n)
> ",
        )
        .to_lowercase()
            == *"y"
        {
            if let Err(e) = armor_stats() {
                println!("Error: {}\n", e)
            }
        }
    } else if args.function.eq_ignore_ascii_case("weapon-stats") {
        if get_input(
            "
-------------------------------------------------------------------------------
|                                 DISCLAIMER                                  |
-------------------------------------------------------------------------------
|                                                                             |
|   This script will wreck the RON files for weapons if it messes up.         |
|   You might want to save a back up of the weapon files or be prepared to    |
|   use `git checkout HEAD -- ../assets/common/items/weapons/*` if needed.    |
|   If this script does mess up your files, please fix it. Otherwise your     |
|   files will be yeeted away and you will get a bonk on the head.            |
|                                                                             |
-------------------------------------------------------------------------------

In order for this script to work, you need to have first run the csv exporter.
Once you have weaponstats.csv you can make changes to stats, quality, and
description in your preferred editor. Save the csv file and then run this
script to import your changes back to RON.

Would you like to continue? (y/n)
> ",
        )
        .to_lowercase()
            == *"y"
        {
            if let Err(e) = weapon_stats() {
                println!("Error: {}\n", e)
            }
        }
    } else {
        println!(
            "Invalid argument, available \
             arguments:\n\"armor-stats\"\n\"weapon-stats\"\n\"loot-table\"\n\""
        )
    }
}

pub fn get_input(prompt: &str) -> String {
    // Function to get input from the user with a prompt
    let mut input = String::new();
    print!("{}", prompt);
    std::io::stdout().flush().unwrap();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Error reading input");

    String::from(input.trim())
}
