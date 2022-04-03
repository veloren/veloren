use hashbrown::HashMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};
use veloren_common::{
    comp::{
        agent::Alignment,
        inventory::slot::{ArmorSlot, EquipSlot},
        Body,
    },
    lottery::LootSpec,
};

/// Old version.
mod loadout_v1 {
    use super::*;
    pub type Config = LoadoutSpec;

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum ItemSpec {
        /// One specific item.
        /// Example:
        /// Item("common.items.armor.steel.foot")
        Item(String),
        /// Choice from items with weights.
        /// Example:
        /// Choice([
        ///  (1.0, Some(Item("common.items.lantern.blue_0"))),
        ///  (1.0, None),
        /// ])
        Choice(Vec<(f32, Option<ItemSpec>)>),
    }

    #[derive(Debug, Deserialize, Clone)]
    pub struct LoadoutSpec(pub HashMap<EquipSlot, ItemSpec>);
}

/// New version.
mod loadout_v2 {
    use super::{loadout_v1::ItemSpec, *};

    type OldConfig = super::loadout_v1::Config;
    pub type Config = LoadoutSpecNew;
    type Weight = u8;

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum Base {
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
        pub inherit: Option<Base>,
        // Armor
        #[serde(skip_serializing_if = "Option::is_none")]
        pub head: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub neck: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shoulders: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub chest: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub gloves: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub ring1: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub ring2: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub back: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub belt: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub legs: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub feet: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tabard: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bag1: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bag2: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bag3: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bag4: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub lantern: Option<ItemSpecNew>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub glider: Option<ItemSpecNew>,
        // Weapons
        #[serde(skip_serializing_if = "Option::is_none")]
        pub active_hands: Option<Hands>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub inactive_hands: Option<Hands>,
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
            let super::loadout_v1::LoadoutSpec(old) = old;
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

mod entity_v1 {
    use super::*;
    pub type Config = EntityConfig;
    type Weight = u8;

    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
    pub enum NameKind {
        Name(String),
        Automatic,
        Uninit,
    }

    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
    pub enum BodyBuilder {
        RandomWith(String),
        Exact(Body),
        Uninit,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum AlignmentMark {
        Alignment(Alignment),
        Uninit,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum Meta {
        SkillSetAsset(String),
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum Hands {
        TwoHanded(super::loadout_v1::ItemSpec),
        Paired(super::loadout_v1::ItemSpec),
        Mix {
            mainhand: super::loadout_v1::ItemSpec,
            offhand: super::loadout_v1::ItemSpec,
        },
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum LoadoutAsset {
        Loadout(String),
        Choice(Vec<(Weight, String)>),
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum LoadoutKind {
        FromBody,
        Asset(LoadoutAsset),
        Hands(Hands),
        Extended {
            hands: Hands,
            base_asset: LoadoutAsset,
            inventory: Vec<(u32, String)>,
        },
    }

    #[derive(Debug, Deserialize, Clone)]
    pub struct EntityConfig {
        pub name: NameKind,
        pub body: BodyBuilder,
        pub alignment: AlignmentMark,
        pub loot: LootSpec<String>,
        pub loadout: LoadoutKind,
        #[serde(default)]
        pub meta: Vec<Meta>,
    }
}

mod entity_v2 {
    use super::{
        entity_v1::{Hands as OldHands, LoadoutAsset, LoadoutKind},
        loadout_v1::ItemSpec,
        loadout_v2::{Base, Hands, ItemSpecNew, LoadoutSpecNew},
        *,
    };
    pub type OldConfig = super::entity_v1::Config;
    pub type Config = EntityConfig;

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum LoadoutKindNew {
        FromBody,
        Asset(String),
        Inline(super::loadout_v2::LoadoutSpecNew),
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct InventorySpec {
        loadout: LoadoutKindNew,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        items: Vec<(u32, String)>,
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct EntityConfig {
        pub name: super::entity_v1::NameKind,
        pub body: super::entity_v1::BodyBuilder,
        pub alignment: super::entity_v1::AlignmentMark,
        pub loot: LootSpec<String>,
        pub inventory: InventorySpec,
        #[serde(default)]
        pub meta: Vec<super::entity_v1::Meta>,
    }

    impl From<LoadoutAsset> for LoadoutKindNew {
        fn from(old: LoadoutAsset) -> Self {
            match old {
                LoadoutAsset::Loadout(s) => LoadoutKindNew::Asset(s),
                LoadoutAsset::Choice(bases) => LoadoutKindNew::Inline(LoadoutSpecNew {
                    inherit: Some(Base::Choice(
                        bases
                            .iter()
                            .map(|(w, s)| (*w, Base::Asset(s.to_owned())))
                            .collect(),
                    )),
                    ..Default::default()
                }),
            }
        }
    }

    impl From<OldHands> for Hands {
        fn from(old: OldHands) -> Self {
            match old {
                OldHands::TwoHanded(spec) => Hands::InHands((Some(spec.into()), None)),
                OldHands::Mix { mainhand, offhand } => {
                    Hands::InHands((Some(mainhand.into()), Some(offhand.into())))
                },
                OldHands::Paired(spec) => match spec {
                    ItemSpec::Item(name) => Hands::InHands((
                        Some(ItemSpecNew::Item(name.clone())),
                        Some(ItemSpecNew::Item(name)),
                    )),
                    ItemSpec::Choice(choices) => {
                        let smallest = choices
                            .iter()
                            .map(|(w, _)| *w)
                            .min_by(|x, y| x.partial_cmp(y).expect("floats are evil"))
                            .expect("choice shouldn't empty");
                        // Very imprecise algo, but it works
                        let new_choices = choices
                            .into_iter()
                            .map(|(w, i)| {
                                let new_weight = (w / smallest) as u8;
                                let choice =
                                    Hands::InHands((i.clone().map(Into::into), i.map(Into::into)));

                                (new_weight, choice)
                            })
                            .collect();

                        Hands::Choice(new_choices)
                    },
                },
            }
        }
    }

    impl InventorySpec {
        fn with_hands(
            hands: OldHands,
            loadout: Option<LoadoutAsset>,
            items: Vec<(u32, String)>,
        ) -> Self {
            Self {
                loadout: LoadoutKindNew::Inline(LoadoutSpecNew {
                    inherit: loadout.map(|asset| match asset {
                        LoadoutAsset::Loadout(s) => Base::Asset(s.to_owned()),
                        LoadoutAsset::Choice(bases) => Base::Choice(
                            bases
                                .iter()
                                .map(|(w, s)| (*w, Base::Asset(s.to_owned())))
                                .collect(),
                        ),
                    }),
                    active_hands: Some(hands.into()),
                    ..Default::default()
                }),
                items,
            }
        }
    }

    impl From<OldConfig> for Config {
        fn from(old: OldConfig) -> Self {
            let just_loadout = |loadout| InventorySpec {
                loadout,
                items: Vec::new(),
            };

            Self {
                name: old.name,
                body: old.body,
                alignment: old.alignment,
                loot: old.loot,
                inventory: match old.loadout {
                    LoadoutKind::FromBody => just_loadout(LoadoutKindNew::FromBody),
                    LoadoutKind::Asset(asset) => just_loadout(asset.into()),
                    LoadoutKind::Hands(hands) => InventorySpec::with_hands(hands, None, Vec::new()),
                    LoadoutKind::Extended {
                        hands,
                        base_asset,
                        inventory,
                    } => InventorySpec::with_hands(hands, Some(base_asset), inventory),
                },
                meta: old.meta,
            }
        }
    }
}

mod old {
    pub type Config = super::loadout_v1::Config;
}

mod new {
    pub type Config = super::loadout_v2::Config;
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
    use std::io::{BufRead, BufReader};
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
            let config_string =
                ron::ser::to_string_pretty(&new, pretty_config).expect("serialize shouldn't fail");
            let comments_string = if comments.is_empty() {
                String::new()
            } else {
                let mut comments = comments.join("\n");
                // insert newline for other config content
                comments.push_str("\n");
                comments
            };

            let mut target = fs::File::create(to.join(&path))?;
            write!(&mut target, "{comments_string}{config_string}")
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
    walk_with_migrate::<old::Config, new::Config>(files, Path::new(from), Path::new(to)).unwrap();
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
