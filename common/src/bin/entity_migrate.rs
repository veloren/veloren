use serde::{de::DeserializeOwned, Deserialize, Serialize};
use veloren_common::{
    comp::{inventory::loadout_builder::ItemSpec, Alignment, Body},
    lottery::LootSpec,
};

use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

/// First "stable" version.
mod v1 {
    pub(super) use super::*;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub enum NameKind {
        Name(String),
        Automatic,
        Uninit,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub enum BodyBuilder {
        RandomWith(String),
        Exact(Body),
        Uninit,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub enum AlignmentMark {
        Alignment(Alignment),
        Uninit,
    }

    impl Default for AlignmentMark {
        fn default() -> Self { Self::Alignment(Alignment::Wild) }
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub enum Hands {
        TwoHanded(ItemSpec),
        Paired(ItemSpec),
        Mix {
            mainhand: ItemSpec,
            offhand: ItemSpec,
        },
        Uninit,
    }

    impl Default for Hands {
        fn default() -> Self { Self::Uninit }
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub enum Meta {
        LoadoutAsset(String),
        SkillSetAsset(String),
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct EntityConfig {
        pub name: NameKind,
        pub body: BodyBuilder,
        pub alignment: AlignmentMark,
        pub loot: LootSpec<String>,
        pub hands: Hands,
        #[serde(default)]
        pub meta: Vec<Meta>,
    }
}

/// Loadout update.
/// 1) Added ability to randomize loadout for entity.
/// 2) Simplified logic by squashing hands, loadout and inventory into one pack.
mod v2 {
    pub(super) use super::*;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub enum LoadoutAsset {
        Loadout(String),
        Choice(Vec<(f32, String)>),
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub enum Hands {
        TwoHanded(ItemSpec),
        Paired(ItemSpec),
        Mix {
            mainhand: ItemSpec,
            offhand: ItemSpec,
        },
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
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

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub enum Meta {
        SkillSetAsset(String),
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct EntityConfig {
        pub name: super::v1::NameKind,
        pub body: super::v1::BodyBuilder,
        pub alignment: super::v1::AlignmentMark,
        pub loadout: LoadoutKind,
        pub loot: super::v1::LootSpec<String>,
        #[serde(default)]
        pub meta: Vec<Meta>,
    }

    impl From<super::v1::EntityConfig> for EntityConfig {
        fn from(old_config: super::v1::EntityConfig) -> Self {
            let mut loadout_asset = None;
            let mut meta = Vec::new();

            for item in old_config.meta {
                match item {
                    super::v1::Meta::SkillSetAsset(asset) => {
                        meta.push(Meta::SkillSetAsset(asset));
                    },
                    super::v1::Meta::LoadoutAsset(asset) => {
                        if loadout_asset == None {
                            loadout_asset = Some(asset);
                        } else {
                            tracing::error!("multiple loadout assets in meta[], bad");
                        }
                    },
                }
            }

            let loadout_kind = match loadout_asset {
                Some(asset) => match old_config.hands {
                    super::v1::Hands::TwoHanded(spec) => LoadoutKind::Extended {
                        hands: Hands::TwoHanded(spec),
                        base_asset: LoadoutAsset::Loadout(asset),
                        inventory: vec![],
                    },
                    super::v1::Hands::Paired(spec) => LoadoutKind::Extended {
                        hands: Hands::Paired(spec),
                        base_asset: LoadoutAsset::Loadout(asset),
                        inventory: vec![],
                    },
                    super::v1::Hands::Mix { mainhand, offhand } => LoadoutKind::Extended {
                        hands: Hands::Mix { mainhand, offhand },
                        base_asset: LoadoutAsset::Loadout(asset),
                        inventory: vec![],
                    },
                    super::v1::Hands::Uninit => LoadoutKind::Asset(LoadoutAsset::Loadout(asset)),
                },
                None => match old_config.hands {
                    super::v1::Hands::TwoHanded(spec) => LoadoutKind::Hands(Hands::TwoHanded(spec)),
                    super::v1::Hands::Paired(spec) => LoadoutKind::Hands(Hands::Paired(spec)),
                    super::v1::Hands::Mix { mainhand, offhand } => {
                        LoadoutKind::Hands(Hands::Mix { mainhand, offhand })
                    },
                    super::v1::Hands::Uninit => LoadoutKind::FromBody,
                },
            };

            Self {
                name: old_config.name,
                body: old_config.body,
                alignment: old_config.alignment,
                loadout: loadout_kind,
                loot: old_config.loot,
                meta,
            }
        }
    }
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
    match tree {
        Walk::Dir { path, content } => {
            let target_dir = to.join(path);
            fs::create_dir_all(target_dir)?;
            for entry in content {
                walk_with_migrate::<OldV, NewV>(entry, from, to)?;
            }
        },
        Walk::File(path) => {
            let source = fs::File::open(from.join(&path))?;
            let old: OldV = ron::de::from_reader(source).unwrap();
            let new: NewV = old.into();
            let target = fs::File::create(to.join(&path))?;
            let pretty_config = ron::ser::PrettyConfig::new();
            ron::ser::to_writer_pretty(target, &new, pretty_config).unwrap();
            println!("{path:?} done");
        },
    }
    Ok(())
}

fn convert_loop(from: &str, to: &str, old_ver: &str, new_ver: &str) {
    #[rustfmt::skip]
    println!(
        "\nRequest info:\n\
        {old_ver} -> {new_ver}.\n\
        Get data from {from} and store in {to}."
    );

    let root = Path::new(from);
    let files = Walk::Dir {
        path: Path::new("").to_owned(),
        content: walk_tree(&root, &root).unwrap(),
    };
    if old_ver == "v1" && new_ver == "v2" {
        walk_with_migrate::<v1::EntityConfig, v2::EntityConfig>(
            files,
            Path::new(from),
            Path::new(to),
        )
        .unwrap();
    } else {
        eprintln!("Unexpected versions")
    }
}

fn main() {
    println!(
        r#"
Hello, this tool can convert all your entity configs to newer version.
Currently it supports converting from "v1" to "v2".
    "#
    );

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

    let old_version =
        input_validated_string("Please input old version to migrate from:", &|version| {
            let olds = ["v1"];
            if !olds.contains(&version) {
                eprintln!("Unexpected version {version}. Available: {olds:?}");
                false
            } else {
                true
            }
        });
    let new_version = input_validated_string("Please input new version:", &|version| {
        let news = ["v2"];
        if !news.contains(&version) {
            eprintln!("Unexpected version {version}. Available: {news:?}");
            false
        } else {
            true
        }
    });

    convert_loop(&old_dir, &new_dir, &old_version, &new_version)
}
