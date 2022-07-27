use std::{ffi::OsStr, fs, io::Write, path::Path};

use serde::Deserialize;

use common_assets::{walk_tree, Walk};

/// Structure representing file for old .ron format
#[derive(Deserialize)]
struct RawFragment {
    #[serde(with = "tuple_vec_map")]
    string_map: Vec<(String, String)>,
    #[serde(with = "tuple_vec_map")]
    vector_map: Vec<(String, Vec<String>)>,
}

impl RawFragment {
    fn read(path: &Path) -> Self {
        let source = fs::File::open(path).unwrap();
        ron::de::from_reader(source).unwrap()
    }
}

/// Message value, may contain interpolated variables
struct Pattern {
    view: String,
}

impl Pattern {
    fn expand(self) -> String {
        let mut buff = String::new();
        if self.view.contains('\n') {
            let mut first = true;
            for line in self.view.lines() {
                if line.is_empty() && first {
                    // fluent ignores space characters at the beginning
                    // so we need to encode \n explicitly
                    buff.push_str(r#"{"\u000A"}"#);
                } else {
                    buff.push_str("\n    ");
                }
                if first {
                    first = false;
                }
                buff.push_str(line);
            }
        } else {
            buff.push_str(" ");
            buff.push_str(&self.view);
        }

        buff
    }
}

/// Fluent entry
struct Message {
    value: Option<Pattern>,
    attributes: Vec<(String, Pattern)>,
}

impl Message {
    fn stringify(self) -> String {
        let mut buff = String::new();
        // append equal sign
        buff.push_str(" =");
        // display value if any
        if let Some(value) = self.value {
            buff.push_str(&value.expand());
        }
        // add attributes
        for (attr_name, attr) in self.attributes {
            // new line and append tab
            buff.push_str("\n    ");
            // print attrname
            buff.push('.');
            buff.push_str(&attr_name);
            // equal sign
            buff.push_str(" =");
            // display attr
            buff.push_str(&attr.expand());
        }

        buff
    }
}

/// Structure representing file for new .ftl format
struct Source {
    entries: Vec<(String, Message)>,
}

impl Source {
    fn write(self, path: &Path) {
        let mut source = fs::File::create(path).unwrap();
        let mut first = true;
        for (key, msg) in self.entries {
            if !first {
                source.write_all(b"\n").unwrap();
            } else {
                first = false;
            }
            source.write_all(key.as_bytes()).unwrap();
            source.write_all(msg.stringify().as_bytes()).unwrap();
        }
    }
}

// Convert old i18n string to new fluent format
fn to_pattern(old: String) -> Pattern {
    let mut buff = String::new();

    let mut in_capture = false;
    let mut need_sign = false;

    for ch in old.chars() {
        if ch == '{' {
            if !in_capture {
                in_capture = true;
            } else {
                panic!("double {{");
            }
            need_sign = true;

            buff.push(ch);
            buff.push(' ');
        } else if ch == '}' {
            if in_capture {
                in_capture = false;
            } else {
                panic!("}} without opening {{");
            }

            buff.push(' ');
            buff.push(ch);
        } else {
            if need_sign {
                buff.push('$');
                need_sign = false;
            }
            if ch == '.' && in_capture {
                buff.push('-')
            } else {
                buff.push(ch)
            }
        }
    }

    Pattern { view: buff }
}

fn to_attributes(old: Vec<String>) -> Message {
    let mut attributes = Vec::new();
    for (i, string) in old.iter().enumerate() {
        let attr_name = format!("a{i}");
        let attr = to_pattern(string.to_owned());
        attributes.push((attr_name, attr))
    }

    Message {
        value: None,
        attributes,
    }
}

fn convert(old: RawFragment) -> Source {
    let mut entries = Vec::new();
    let mut cache = Vec::new();
    for (key, string) in old.string_map.into_iter() {
        if cache.contains(&key) {
            continue;
        } else {
            cache.push(key.clone());
        }
        // common.weapon.tool -> common-weapon-tool
        let key = key.replace('.', "-").to_owned();
        let msg = Message {
            value: Some(to_pattern(string.to_owned())),
            attributes: Vec::new(),
        };
        entries.push((key, msg))
    }

    for (key, variation) in old.vector_map.into_iter() {
        if cache.contains(&key) {
            continue;
        } else {
            cache.push(key.clone());
        }
        // common.weapon.tool -> common-weapon-tool
        let key = key.replace('.', "-").to_owned();
        let msg = to_attributes(variation);
        entries.push((key, msg))
    }

    Source { entries }
}

fn migrate(tree: Walk, from: &Path, to: &Path) {
    match tree {
        Walk::Dir { path, content } => {
            println!("{:?}", path);
            let target_dir = to.join(path);
            fs::create_dir(target_dir).unwrap();
            for entry in content {
                migrate(entry, from, to);
            }
        },
        Walk::File(path) => {
            if path.file_name() == Some(OsStr::new("_manifest.ron"))
                || path.file_name() == Some(OsStr::new("README.md"))
            {
                fs::copy(from.join(&path), to.join(path)).unwrap();
            } else {
                let old = RawFragment::read(&from.join(&path));
                let new = convert(old);
                new.write(&to.join(path).with_extension("ftl"));
            }
        },
    }
}

fn main() {
    // it assumes that you have old i18n files in i18n-ron directory
    let old_path = Path::new("assets/voxygen/i18n-ron");
    let new_path = Path::new("assets/voxygen/i18n");
    let tree = walk_tree(&old_path, &old_path).unwrap();
    let tree = Walk::Dir {
        path: Path::new("").to_owned(),
        content: tree,
    };
    migrate(tree, &old_path, &new_path);
}
