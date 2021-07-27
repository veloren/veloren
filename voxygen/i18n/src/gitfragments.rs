//! fragment attached with git versioning information
use hashbrown::{HashMap};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::sync::Arc;
use crate::raw::{RawFragment};

struct GitCache<'a> {
    pub root_dir: PathBuf,
    pub blobs: RwLock<HashMap<PathBuf, Arc<git2::Blob<'a>>>>,
    pub repo: git2::Repository,
    //pub head_ref: git2::Reference<'a>,
}

impl<'a> GitCache<'a> {
    pub fn new(root_dir: &Path) -> Self {
        let repo = git2::Repository::discover(&root_dir)
            .unwrap_or_else(|_| panic!("Failed to open the Git repository at {:?}", &root_dir));
        //let head_ref = repo.head().expect("Impossible to get the HEAD reference");

        let root_dir = root_dir.to_path_buf();
        let blobs = RwLock::new(HashMap::new());
        Self {
            root_dir,
            blobs,
            repo,
            //head_ref,
        }
    }
    /// Returns the Git blob associated with the given reference and path
    fn read_file_from_path(
        &'a self,
        reference: &git2::Reference,
        path: &std::path::Path,
    ) -> Arc<git2::Blob<'a>> {
        // return from cache
        let lock = self.blobs.read().unwrap();
        if let Some(blob) = lock.get(path) {
            return blob.clone();
        }
        drop(lock);
        // load file not in cache
        let tree = reference
            .peel_to_tree()
            .expect("Impossible to peel HEAD to a tree object");
        let blob = Arc::new(tree.get_path(path)
            .unwrap_or_else(|_| {
                panic!(
                    "Impossible to find the file {:?} in reference {:?}",
                    path,
                    reference.name()
                )
            })
            .to_object(&self.repo)
            .unwrap()
            .peel_to_blob()
            .expect("Impossible to fetch the Git object"));
        let mut lock = self.blobs.write().unwrap();
        let pathbuf = path.to_path_buf();
        lock.insert(pathbuf, blob.clone());
        blob
    }
}

/*
/// Extend a Fragment with historical git data
/// The actual translation gets dropped
fn generate_key_version<'a>(
    repo: &'a GitCache,
    path: &Path,
    fragment: RawFragment<String>,
) -> RawFragment<LocalizationEntryState> {
    let file_blob = repo.read_file_from_path(path);
    // Find key start lines
    let file_content = std::str::from_utf8(file_blob.content()).expect("Got non UTF-8 file");
    let mut to_process: HashSet<&String> = localization.string_map.keys().collect();
    for (line_nb, line) in file_content.lines().enumerate() {
        let mut found_key = None;

        for key in to_process.iter() {
            if correspond(line, key) {
                found_key = Some(key.to_owned());
            }
        }

        if let Some(key) = found_key {
            keys.get_mut(key).unwrap().key_line = Some(line_nb);
            to_process.remove(key);
        };
    }


}*/

/*

fn generate_key_version<'a>(
    repo: &'a git2::Repository,
    fragment: &RawFragment<String>,
    path: &std::path::Path,
    file_blob: &git2::Blob,
) -> HashMap<String, LocalizationEntryState> {
    let mut keys: HashMap<String, LocalizationEntryState> = localization
        .string_map
        .keys()
        .map(|k| (k.to_owned(), LocalizationEntryState::new()))
        .collect();
    // Find key start lines
    let file_content = std::str::from_utf8(file_blob.content()).expect("Got non UTF-8 file");
    let mut to_process: HashSet<&String> = localization.string_map.keys().collect();
    for (line_nb, line) in file_content.lines().enumerate() {
        let mut found_key = None;

        for key in to_process.iter() {
            if correspond(line, key) {
                found_key = Some(key.to_owned());
            }
        }

        if let Some(key) = found_key {
            keys.get_mut(key).unwrap().key_line = Some(line_nb);
            to_process.remove(key);
        };
    }

    let mut error_check_set: Vec<String> = vec![];
    // Find commit for each keys
    repo.blame_file(path, None)
        .expect("Impossible to generate the Git blame")
        .iter()
        .for_each(|e: git2::BlameHunk| {
            for (key, state) in keys.iter_mut() {
                let line = match state.key_line {
                    Some(l) => l,
                    None => {
                        if !error_check_set.contains(key) {
                            eprintln!(
                                "Key {} does not have a git line in it's state! Skipping key.",
                                key
                            );
                            error_check_set.push(key.clone());
                        }
                        continue;
                    },
                };

                if line + 1 >= e.final_start_line()
                    && line + 1 < e.final_start_line() + e.lines_in_hunk()
                {
                    state.chuck_line_range = Some((
                        e.final_start_line(),
                        e.final_start_line() + e.lines_in_hunk(),
                    ));
                    state.commit_id = match state.commit_id {
                        Some(existing_commit) => {
                            match repo.graph_descendant_of(e.final_commit_id(), existing_commit) {
                                Ok(true) => Some(e.final_commit_id()),
                                Ok(false) => Some(existing_commit),
                                Err(err) => panic!("{}", err),
                            }
                        },
                        None => Some(e.final_commit_id()),
                    };
                }
            }
        });

    keys
}


 */