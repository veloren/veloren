use std::{
    error::Error,
    ffi::OsString,
    path::{Component, PathBuf},
};
use structopt::StructOpt;

use common::{assets, comp};
use comp::item::{
    armor::{ArmorKind, Protection},
    tool::ToolKind,
};

#[derive(StructOpt)]
struct Cli {
    /// Available arguments: "armor_stats", "weapon_stats"
    function: String,
}

fn armor_stats() -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path("armorstats.csv")?;
    wtr.write_record(&["Path", "Kind", "Name", "Protection"])?;

    for folder in
        assets::read_dir("common.items.armor").expect("Failed to iterate over armor folders!")
    {
        match folder {
            Ok(folder) => {
                let mut glob_folder = folder.path().display().to_string().replace("/", ".");
                glob_folder.push_str(".*");

                for file in std::fs::read_dir(folder.path())?.filter_map(|f| f.ok()) {
                    let asset_identifier = &file
                        .path()
                        .components()
                        .skip_while(|s| s != &Component::Normal(&OsString::from("common")))
                        .inspect(|s| {
                            dbg!(&s);
                        })
                        .collect::<PathBuf>()
                        .with_extension("")
                        .display()
                        .to_string()
                        .replace("/", ".");

                    let asset = assets::load_expect_cloned::<comp::Item>(asset_identifier);

                    match &asset.kind {
                        comp::item::ItemKind::Armor(armor) => {
                            let protection = match armor.get_protection() {
                                Protection::Invincible => "Invincible".to_string(),
                                Protection::Normal(value) => value.to_string(),
                            };
                            let kind = match armor.kind {
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
                            };

                            wtr.write_record(&[
                                asset_identifier,
                                &kind,
                                asset.name(),
                                &protection,
                            ])?;
                        },
                        // Skip non-armor
                        _ => println!("Skipping non-armor item: {:?}", asset),
                    }
                }
            },
            Err(e) => println!("Skipping folder due to {}", e),
        }
    }

    wtr.flush()?;
    Ok(())
}

fn weapon_stats() -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path("weaponstats.csv")?;
    wtr.write_record(&["Path", "Kind", "Name", "Power", "Equip Time (ms)"])?;

    for folder in
        assets::read_dir("common.items.weapons").expect("Failed to iterate over weapon folders!")
    {
        match folder {
            Ok(folder) => {
                let mut glob_folder = folder.path().display().to_string().replace("/", ".");
                glob_folder.push_str(".*");
                for file in std::fs::read_dir(folder.path())?.filter_map(|f| f.ok()) {
                    let asset_identifier = &file
                        .path()
                        .components()
                        .skip_while(|s| s != &Component::Normal(&OsString::from("common")))
                        .inspect(|s| {
                            dbg!(&s);
                        })
                        .collect::<PathBuf>()
                        .with_extension("")
                        .display()
                        .to_string()
                        .replace("/", ".");
                    let asset = assets::load_expect_cloned::<comp::Item>(asset_identifier);

                    match &asset.kind {
                        comp::item::ItemKind::Tool(tool) => {
                            let power = tool.base_power().to_string();
                            let equip_time = tool.equip_time().subsec_millis().to_string();
                            let kind = match tool.kind {
                                ToolKind::Sword(_) => "Sword".to_string(),
                                ToolKind::Axe(_) => "Axe".to_string(),
                                ToolKind::Hammer(_) => "Hammer".to_string(),
                                ToolKind::Bow(_) => "Bow".to_string(),
                                ToolKind::Dagger(_) => "Dagger".to_string(),
                                ToolKind::Staff(_) => "Staff".to_string(),
                                ToolKind::Shield(_) => "Shield".to_string(),
                                ToolKind::Debug(_) => "Debug".to_string(),
                                ToolKind::Farming(_) => "Farming".to_string(),
                                ToolKind::Empty => "Empty".to_string(),
                            };

                            wtr.write_record(&[
                                asset_identifier,
                                &kind,
                                asset.name(),
                                &power,
                                &equip_time,
                            ])?;
                        },
                        // Skip non-armor
                        _ => println!("Skipping non-weapon item: {:?}", asset),
                    }
                }
            },
            Err(e) => println!("Skipping folder due to {}", e),
        }
    }

    wtr.flush()?;
    Ok(())
}

fn main() {
    let args = Cli::from_args();
    if args.function.eq_ignore_ascii_case("armor_stats") {
        if let Err(e) = armor_stats() {
            println!("Error: {}", e)
        }
    } else if args.function.eq_ignore_ascii_case("weapon_stats") {
        if let Err(e) = weapon_stats() {
            println!("Error: {}", e)
        }
    } else {
        println!("Invalid argument, available arguments:\n\"armor_stats\"\n\"weapon_stats\"")
    }
}
