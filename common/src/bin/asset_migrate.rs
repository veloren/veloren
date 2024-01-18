use common_assets::{walk_tree, Walk};
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io, io::Write, path::Path};

// If you want to migrate assets.
// 1) Copy-paste old asset type to own module
// 2) Copy-paste new asset type to own module
// (don't forget to add serde derive-s, import if needed)
// 3) impl From<old asset> for new asset.
// 4) Reference old and new assets in old and new modules
mod old {
    pub type Config = ();
}

mod new {
    pub type Config = ();
}

fn walk_with_migrate<OldV, NewV>(tree: Walk, from: &Path, to: &Path) -> io::Result<()>
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
            for line in BufReader::new(source).lines().map_while(Result::ok) {
                if let Some(idx) = line.find("//") {
                    let comment = &line[idx..line.len()];
                    comments.push(comment.to_owned());
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
                comments.push('\n');
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
    let prompt = r"
        Stub implementation.
        If you want to migrate new assets, edit `v1` and `v2` modules.
        If you want to migrate old assets, check commit history.
    ";
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
