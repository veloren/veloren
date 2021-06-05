#![deny(clippy::clone_on_ref_ptr)]

use std::{
    error::Error,
    io::Write,
    ops::{Div, Mul},
};
use structopt::StructOpt;

use veloren_common::{
    assets::AssetExt,
    comp::{
        self,
        item::{
            armor::{ArmorKind, Protection},
            tool::{Hands, MaterialStatManifest, Tool, ToolKind},
            ItemKind,
        },
    },
    lottery::{LootSpec, Lottery},
};

#[derive(StructOpt)]
struct Cli {
    /// Available arguments: "armor-stats", "weapon-stats", "all-items",
    /// "loot-table"
    function: String,
}

fn armor_stats() -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path("armorstats.csv")?;
    wtr.write_record(&[
        "Path",
        "Kind",
        "Name",
        "Quality",
        "Protection",
        "Poise Resilience",
        "Max Energy",
        "Energy Reward",
        "Crit Power",
        "Stealth",
        "Description",
    ])?;

    for item in comp::item::Item::new_from_asset_glob("common.items.armor.*")
        .expect("Failed to iterate over item folders!")
    {
        match item.kind() {
            comp::item::ItemKind::Armor(armor) => {
                let kind = get_armor_kind(&armor.kind);
                if kind == "Bag" {
                    continue;
                }

                let protection = match armor.get_protection() {
                    Protection::Invincible => "Invincible".to_string(),
                    Protection::Normal(value) => value.to_string(),
                };
                let poise_resilience = match armor.get_poise_resilience() {
                    Protection::Invincible => "Invincible".to_string(),
                    Protection::Normal(value) => value.to_string(),
                };
                let max_energy = armor.get_energy_max().to_string();
                let energy_recovery = armor.get_energy_recovery().to_string();
                let crit_power = armor.get_crit_power().to_string();
                let stealth = armor.get_stealth().to_string();

                wtr.write_record(&[
                    item.item_definition_id(),
                    &kind,
                    item.name(),
                    &format!("{:?}", item.quality()),
                    &protection,
                    &poise_resilience,
                    &max_energy,
                    &energy_recovery,
                    &crit_power,
                    &stealth,
                    item.description(),
                ])?;
            },
            _ => println!("Skipping non-armor item: {:?}\n", item),
        }
    }

    wtr.flush()?;
    Ok(())
}

fn weapon_stats() -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path("weaponstats.csv")?;
    wtr.write_record(&[
        "Path",
        "Kind",
        "Name",
        "Hands",
        "Quality",
        "Power",
        "Poise Strength",
        "Speed",
        "Crit Chance",
        "Crit Mult",
        "Equip Time (s)",
        "Description",
    ])?;

    let msm = MaterialStatManifest::default();

    let mut items: Vec<comp::Item> = comp::Item::new_from_asset_glob("common.items.weapons.*")
        .expect("Failed to iterate over item folders!");
    items.extend(
        comp::Item::new_from_asset_glob("common.items.npc_weapons.*")
            .expect("Failed to iterate over npc weapons!"),
    );

    for item in items.iter() {
        match item.kind() {
            comp::item::ItemKind::Tool(tool) => {
                let power = tool.base_power(&msm, &[]).to_string();
                let poise_strength = tool.base_poise_strength(&msm, &[]).to_string();
                let speed = tool.base_speed(&msm, &[]).to_string();
                let crit_chance = tool.base_crit_chance(&msm, &[]).to_string();
                let crit_mult = tool.base_crit_mult(&msm, &[]).to_string();
                let equip_time = tool.equip_time(&msm, &[]).as_secs_f32().to_string();
                let kind = get_tool_kind(&tool.kind);
                let hands = get_tool_hands(&tool);

                wtr.write_record(&[
                    item.item_definition_id(),
                    &kind,
                    item.name(),
                    &hands,
                    &format!("{:?}", item.quality()),
                    &power,
                    &poise_strength,
                    &speed,
                    &crit_chance,
                    &crit_mult,
                    &equip_time,
                    item.description(),
                ])?;
            },
            _ => println!("Skipping non-weapon item: {:?}\n", item),
        }
    }

    wtr.flush()?;
    Ok(())
}

fn get_tool_kind(kind: &ToolKind) -> String {
    match kind {
        ToolKind::Sword => "Sword".to_string(),
        ToolKind::Axe => "Axe".to_string(),
        ToolKind::Hammer => "Hammer".to_string(),
        ToolKind::Bow => "Bow".to_string(),
        ToolKind::Dagger => "Dagger".to_string(),
        ToolKind::Staff => "Staff".to_string(),
        ToolKind::Sceptre => "Sceptre".to_string(),
        ToolKind::Shield => "Shield".to_string(),
        ToolKind::Spear => "Spear".to_string(),
        ToolKind::Debug => "Debug".to_string(),
        ToolKind::Farming => "Farming".to_string(),
        ToolKind::Pick => "Pick".to_string(),
        ToolKind::Natural => "Natural".to_string(),
        ToolKind::Empty => "Empty".to_string(),
    }
}

fn get_tool_hands(tool: &Tool) -> String {
    match tool.hands {
        Hands::One => "One".to_string(),
        Hands::Two => "Two".to_string(),
    }
}

fn get_armor_kind(kind: &ArmorKind) -> String {
    match kind {
        ArmorKind::Shoulder(_) => "Shoulder".to_string(),
        ArmorKind::Chest(_) => "Chest".to_string(),
        ArmorKind::Belt(_) => "Belt".to_string(),
        ArmorKind::Hand(_) => "Hand".to_string(),
        ArmorKind::Pants(_) => "Pants".to_string(),
        ArmorKind::Foot(_) => "Foot".to_string(),
        ArmorKind::Back(_) => "Back".to_string(),
        ArmorKind::Ring(_) => "Ring".to_string(),
        ArmorKind::Neck(_) => "Neck".to_string(),
        ArmorKind::Head(_) => "Head".to_string(),
        ArmorKind::Tabard(_) => "Tabard".to_string(),
        ArmorKind::Bag(_) => "Bag".to_string(),
    }
}

fn get_armor_kind_kind(kind: &ArmorKind) -> String {
    match kind {
        ArmorKind::Shoulder(x) => x.clone(),
        ArmorKind::Chest(x) => x.clone(),
        ArmorKind::Belt(x) => x.clone(),
        ArmorKind::Hand(x) => x.clone(),
        ArmorKind::Pants(x) => x.clone(),
        ArmorKind::Foot(x) => x.clone(),
        ArmorKind::Back(x) => x.clone(),
        ArmorKind::Ring(x) => x.clone(),
        ArmorKind::Neck(x) => x.clone(),
        ArmorKind::Head(x) => x.clone(),
        ArmorKind::Tabard(x) => x.clone(),
        ArmorKind::Bag(x) => x.clone(),
    }
}

fn all_items() -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path("items.csv")?;
    wtr.write_record(&["Path", "Name", "Kind"])?;

    for item in comp::item::Item::new_from_asset_glob("common.items.*")
        .expect("Failed to iterate over item folders!")
    {
        let kind = match item.kind() {
            ItemKind::Armor(armor) => get_armor_kind_kind(&armor.kind),
            ItemKind::Lantern(lantern) => lantern.kind.clone(),
            _ => "".to_owned(),
        };

        wtr.write_record(&[item.item_definition_id(), item.name(), &kind])?;
    }

    wtr.flush()?;
    Ok(())
}

fn loot_table(loot_table: &str) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path("loot_table.csv")?;
    wtr.write_record(&[
        "Relative Chance",
        "Kind",
        "Item",
        "Lower Amount",
        "Upper Amount",
    ])?;

    let loot_table = "common.loot_tables.".to_owned() + loot_table;

    let loot_table = Lottery::<LootSpec>::load_expect(&loot_table).read();

    for (i, (chance, item)) in loot_table.iter().enumerate() {
        let chance = if let Some((next_chance, _)) = loot_table.iter().nth(i + 1) {
            next_chance - chance
        } else {
            loot_table.total() - chance
        }
        .mul(10_f32.powi(5))
        .round()
        .div(10_f32.powi(5))
        .to_string();

        match item {
            LootSpec::Item(item) => wtr.write_record(&[&chance, "Item", item, "", ""])?,
            LootSpec::ItemQuantity(item, lower, upper) => wtr.write_record(&[
                &chance,
                "Item",
                item,
                &lower.to_string(),
                &upper.to_string(),
            ])?,
            LootSpec::LootTable(table) => {
                wtr.write_record(&[&chance, "LootTable", table, "", ""])?
            },
        }
    }

    wtr.flush()?;
    Ok(())
}

fn main() {
    let args = Cli::from_args();
    if args.function.eq_ignore_ascii_case("armor-stats") {
        if let Err(e) = armor_stats() {
            println!("Error: {}\n", e)
        }
    } else if args.function.eq_ignore_ascii_case("weapon-stats") {
        if let Err(e) = weapon_stats() {
            println!("Error: {}\n", e)
        }
    } else if args.function.eq_ignore_ascii_case("all-items") {
        if let Err(e) = all_items() {
            println!("Error: {}\n", e)
        }
    } else if args.function.eq_ignore_ascii_case("loot-table") {
        let loot_table_name = get_input(
            "Specify the name of the loot table to export to csv. Assumes loot table is in \
             directory: assets.common.loot_tables.\n",
        );
        if let Err(e) = loot_table(&loot_table_name) {
            println!("Error: {}\n", e)
        }
    } else {
        println!(
            "Invalid argument, available \
             arguments:\n\"armor-stats\"\n\"weapon-stats\"\n\"all-items\"\n\"loot-table [table]\""
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
