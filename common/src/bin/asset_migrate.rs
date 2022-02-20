use hashbrown::HashMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};
use veloren_common::comp::inventory::{
    loadout_builder::ItemSpec,
    slot::{ArmorSlot, EquipSlot},
};

/// Old version.
mod v1 {
    use super::*;
    pub type Config = LoadoutSpec;

    #[derive(Debug, Deserialize, Clone)]
    pub struct LoadoutSpec(pub HashMap<EquipSlot, ItemSpec>);
}

/// New version.
mod v2 {
    use super::*;

    type OldConfig = super::v1::Config;
    pub type Config = LoadoutSpecNew;
    type Weight = u8;

    #[derive(Debug, Deserialize, Serialize, Clone)]
    enum Base {
        Asset(String),
        /// NOTE: If you have the same item in multiple configs,
        /// first one will have the priority
        Combine(Vec<Base>),
        Choice(Vec<(Weight, Base)>),
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum ItemSpecNew {
        Item(String),
        Choice(Vec<(Weight, Option<ItemSpecNew>)>),
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum Hands {
        /// Allows to specify one pair
        // TODO: add link to tests with example
        InHands((Option<ItemSpecNew>, Option<ItemSpecNew>)),
        /// Allows specify range of choices
        // TODO: add link to tests with example
        Choice(Vec<(Weight, Hands)>),
    }

    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct LoadoutSpecNew {
        // Meta fields
        #[serde(skip_serializing_if = "Option::is_none")]
        inherit: Option<Base>,
        // Armor
        #[serde(skip_serializing_if = "Option::is_none")]
        head: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        neck: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        shoulders: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        chest: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        gloves: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ring1: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ring2: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        back: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        belt: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        legs: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        feet: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tabard: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bag1: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bag2: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bag3: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bag4: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        lantern: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        glider: Option<ItemSpecNew>,
        // Weapons
        #[serde(skip_serializing_if = "Option::is_none")]
        active_hands: Option<Hands>,
        #[serde(skip_serializing_if = "Option::is_none")]
        inactive_hands: Option<Hands>,
    }

    impl From<(Option<ItemSpec>, Option<ItemSpec>)> for Hands {
        fn from((mainhand, offhand): (Option<ItemSpec>, Option<ItemSpec>)) -> Self {
            Hands::InHands((mainhand.map(|i| i.into()), offhand.map(|i| i.into())))
        }
    }

    impl From<ItemSpec> for ItemSpecNew {
        fn from(old: ItemSpec) -> Self {
            match old {
                ItemSpec::Item(s) => ItemSpecNew::Item(s),
                ItemSpec::Choice(choices) => {
                    let smallest = choices
                        .iter()
                        .map(|(w, _)| *w)
                        .min_by(|x, y| x.partial_cmp(y).expect("floats are evil"))
                        .expect("choice shouldn't empty");
                    // Very imprecise algo, but it works
                    let new_choices = choices
                        .into_iter()
                        .map(|(w, i)| ((w / smallest) as u8, i.map(|i| i.into())))
                        .collect();

                    ItemSpecNew::Choice(new_choices)
                },
            }
        }
    }

    impl From<OldConfig> for Config {
        fn from(old: OldConfig) -> Self {
            let super::v1::LoadoutSpec(old) = old;
            let to_new_item = |slot: &EquipSlot| -> Option<ItemSpecNew> {
                old.get(slot).cloned().map(|i| i.into())
            };

            let active_mainhand = old.get(&EquipSlot::ActiveMainhand).cloned();
            let active_offhand = old.get(&EquipSlot::ActiveOffhand).cloned();
            let inactive_mainhand = old.get(&EquipSlot::InactiveMainhand).cloned();
            let inactive_offhand = old.get(&EquipSlot::InactiveOffhand).cloned();

            let to_hands =
                |mainhand: Option<ItemSpec>, offhand: Option<ItemSpec>| -> Option<Hands> {
                    if mainhand.is_none() && offhand.is_none() {
                        None
                    } else {
                        Some((mainhand, offhand).into())
                    }
                };
            Self {
                inherit: None,
                head: to_new_item(&EquipSlot::Armor(ArmorSlot::Head)),
                neck: to_new_item(&EquipSlot::Armor(ArmorSlot::Neck)),
                shoulders: to_new_item(&EquipSlot::Armor(ArmorSlot::Shoulders)),
                chest: to_new_item(&EquipSlot::Armor(ArmorSlot::Chest)),
                gloves: to_new_item(&EquipSlot::Armor(ArmorSlot::Hands)),
                ring1: to_new_item(&EquipSlot::Armor(ArmorSlot::Ring1)),
                ring2: to_new_item(&EquipSlot::Armor(ArmorSlot::Ring2)),
                back: to_new_item(&EquipSlot::Armor(ArmorSlot::Back)),
                belt: to_new_item(&EquipSlot::Armor(ArmorSlot::Belt)),
                legs: to_new_item(&EquipSlot::Armor(ArmorSlot::Legs)),
                feet: to_new_item(&EquipSlot::Armor(ArmorSlot::Feet)),
                tabard: to_new_item(&EquipSlot::Armor(ArmorSlot::Tabard)),
                bag1: to_new_item(&EquipSlot::Armor(ArmorSlot::Bag1)),
                bag2: to_new_item(&EquipSlot::Armor(ArmorSlot::Bag2)),
                bag3: to_new_item(&EquipSlot::Armor(ArmorSlot::Bag3)),
                bag4: to_new_item(&EquipSlot::Armor(ArmorSlot::Bag4)),
                lantern: to_new_item(&EquipSlot::Lantern),
                glider: to_new_item(&EquipSlot::Glider),
                active_hands: to_hands(active_mainhand, active_offhand),
                inactive_hands: to_hands(inactive_mainhand, inactive_offhand),
            }
        }
    }
}

#[derive(Debug)]
enum Walk {
    File(PathBuf),
    Dir { path: PathBuf, content: Vec<Walk> },
}

fn walk_tree(dir: &Path, root: &Path) -> io::Result<Vec<Walk>> {
    let mut buff = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            buff.push(Walk::Dir {
                path: path
                    .strip_prefix(root)
                    .expect("strip can't fail, this path is created from root")
                    .to_owned(),
                content: walk_tree(&path, root)?,
            });
        } else {
            let filename = path
                .strip_prefix(root)
                .expect("strip can't fail, this file is created from root")
                .to_owned();
            buff.push(Walk::File(filename));
        }
    }

    Ok(buff)
}

fn walk_with_migrate<OldV, NewV>(tree: Walk, from: &Path, to: &Path) -> std::io::Result<()>
where
    NewV: From<OldV>,
    OldV: DeserializeOwned,
    NewV: Serialize,
{
    use std::io::BufReader;
    use std::io::BufRead;
    match tree {
        Walk::Dir { path, content } => {
            let target_dir = to.join(path);
            fs::create_dir_all(target_dir)?;
            for entry in content {
                walk_with_migrate::<OldV, NewV>(entry, from, to)?;
            }
        },
        Walk::File(path) => {
            // Grab all comments from old file
            let source = fs::File::open(from.join(&path))?;
            let mut comments = Vec::new();
            for line in BufReader::new(source).lines() {
                if let Ok(line) = line {
                    if let Some(idx) = line.find("//") {
                        let comment = &line[idx..line.len()];
                        comments.push(comment.to_owned());
                    }
                }
            }
            // Parse the file
            let source = fs::File::open(from.join(&path))?;
            let old: OldV = ron::de::from_reader(source).unwrap();
            // Convert it to new format
            let new: NewV = old.into();
            // Write it all back
            let pretty_config = ron::ser::PrettyConfig::new()
                .extensions(ron::extensions::Extensions::IMPLICIT_SOME);
            let config_string = ron::ser::to_string_pretty(&new, pretty_config)
                .expect("serialize shouldn't fail");
            let comments_string = comments.join("\n");

            let mut target = fs::File::create(to.join(&path))?;
            write!(&mut target, "{comments_string}\n{config_string}")
                .expect("fail to write to the file");
            println!("{path:?} done");
        },
    }
    Ok(())
}

fn convert_loop(from: &str, to: &str) {
    let root = Path::new(from);
    let files = Walk::Dir {
        path: Path::new("").to_owned(),
        content: walk_tree(root, root).unwrap(),
    };
    walk_with_migrate::<v1::Config, v2::Config>(files, Path::new(from), Path::new(to)).unwrap();
}

fn input_string(prompt: &str) -> String { input_validated_string(prompt, &|_| true) }

fn input_validated_string(prompt: &str, check: &dyn Fn(&str) -> bool) -> String {
    println!("{}", prompt);

    print!("> ");
    io::stdout().flush().unwrap();

    let mut buff = String::new();
    io::stdin().read_line(&mut buff).unwrap();

    while !check(buff.trim()) {
        buff.clear();
        print!("> ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut buff).unwrap();
    }

    buff.trim().to_owned()
}

fn main() {
    let prompt = r#"
        Stub implementation.
        If you want to migrate new assets, edit `v1` and `v2` modules.
        If you want to migrate old assets, check commit history.
    "#;
    println!("{prompt}");

    let old_dir = input_validated_string(
        "Please input directory path with old entity configs:",
        &|path| {
            if !Path::new(path).exists() {
                eprintln!("Source directory '{path}' does not exists.");
                false
            } else {
                true
            }
        },
    );
    let new_dir = input_string("Please input directory path to place new entity configs:");

    convert_loop(&old_dir, &new_dir)
}
