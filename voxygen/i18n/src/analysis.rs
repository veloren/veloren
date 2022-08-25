use crate::{assets::Walk, error::ResourceErr};
use fluent_syntax::{ast, parser};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Generate tree of i18n files, path should be absolute.
/// We assume that all i18n directories should have the same tree structure,
/// so that we can generate tree once and reuse for all languages.
fn i18n_tree(reference: &Path) -> io::Result<Walk> { Walk::generate(reference) }

/// Grab keys from one file
fn keys_from_file(filepath: &Path) -> Vec<MsgId> {
    use ast::Entry;

    let file = format!("{}", filepath.display());

    let content = match fs::read_to_string(filepath) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("failed to read from {filepath:?}. err={e}");
            return Vec::new();
        },
    };

    let ast = parser::parse(&*content).unwrap_or_else(|(_parsed, errs)| {
        panic!(
            "{}",
            ResourceErr::parsing_error(errs, file.clone(), &content)
        )
    });

    let mut keys = Vec::new();
    for entry in ast.body {
        match entry {
            Entry::Message(m) => {
                keys.push(MsgId {
                    key: m.id.name.to_owned(),
                    file: file.clone(),
                });
            },
            Entry::Term(_)
            | Entry::Comment(_)
            | Entry::GroupComment(_)
            | Entry::ResourceComment(_)
            | Entry::Junk { .. } => {
                // these are not part of "public" API so do nothing
                // comments linked to message are part of Message entry
                // and we are not interested in global comments either, for now
            },
        }
    }
    keys
}

/// Grab keys from one language sitting at `from`.
///
/// Tree of files assumed to have only .ftl files.
fn keys(from: &Path, tree: &Walk) -> Vec<MsgId> {
    let mut keys = Vec::new();

    tree.for_each_file(from, &mut |filepath| {
        if !filepath.ends_with("_manifest.ron") {
            keys.extend(keys_from_file(filepath));
        }
    });

    keys
}

// TODO:
// Add versioning
// TODO:
// Do something with attributes?
//
// For some messages it makes sense to require that all attributes
// should match ones in reference language.
// For some it doesn't as of now.
#[derive(Clone, Debug)]
pub struct MsgId {
    pub key: String,
    pub file: String,
}

// TODO:
// Add versioning
#[derive(Debug)]
pub struct Stats {
    pub up_to_date: Vec<MsgId>,
    pub not_found: Vec<MsgId>,
    pub unused: Vec<MsgId>,
}

pub struct ReferenceLanguage {
    /// All keys.
    pub keys: Vec<MsgId>,
    /// Cached tree of files.
    tree: Walk,
}

impl ReferenceLanguage {
    /// Generate reference language, path should be absolute.
    pub fn at(path: &Path) -> Self {
        let tree = i18n_tree(path)
            .unwrap_or_else(|e| panic!("{path:?}\nfailed to build file tree\n{e:?}"));
        let keys = keys(path, &tree);
        Self { keys, tree }
    }

    /// Compare with other language
    pub fn compare_with(&self, lang: &Language) -> Stats {
        let keys = keys(&lang.path, &self.tree);

        let mut stats = Stats {
            up_to_date: Vec::new(),
            not_found: Vec::new(),
            unused: Vec::new(),
        };

        for ref_key in &self.keys {
            if let Some(key) = keys.iter().find(|MsgId { key, .. }| &ref_key.key == key) {
                stats.up_to_date.push(key.clone());
            } else {
                stats.not_found.push(ref_key.clone());
            }
        }

        for key in &keys {
            if !self
                .keys
                .iter()
                .any(|MsgId { key: ref_key, .. }| ref_key == &key.key)
            {
                stats.unused.push(key.clone())
            }
        }

        stats
    }
}

pub struct Language {
    pub code: String,
    pub path: PathBuf,
}
