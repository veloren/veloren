use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

/// Old version.
mod v1 {
    use super::*;

    #[derive(Serialize, Deserialize)]
    pub struct Example {
        pub field: u8,
    }
}

/// New version.
mod v2 {
    use super::*;

    #[derive(Serialize, Deserialize)]
    pub struct Example {
        pub field: f64,
    }

    impl From<super::v1::Example> for Example {
        fn from(old: super::v1::Example) -> Self {
            Self {
                field: f64::from(old.field),
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

fn convert_loop(from: &str, to: &str) {
    let root = Path::new(from);
    let files = Walk::Dir {
        path: Path::new("").to_owned(),
        content: walk_tree(root, root).unwrap(),
    };
    walk_with_migrate::<v1::Example, v2::Example>(files, Path::new(from), Path::new(to)).unwrap();
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
