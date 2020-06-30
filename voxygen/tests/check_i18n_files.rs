use git2::Repository;
use ron::de::from_bytes;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use veloren_voxygen::i18n::VoxygenLocalization;

/// List localization files as a PathBuf vector
fn i18n_files(i18n_dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(i18n_dir)
        .unwrap()
        .map(|res| res.map(|e| e.path()).unwrap())
        .filter(|e| match e.extension() {
            Some(ext) => ext == "ron",
            None => false,
        })
        .collect()
}

#[derive(Debug, PartialEq)]
enum LocalizationState {
    UpToDate,
    NotFound,
    Outdated,
    Unknown,
    Unused,
}

#[derive(Debug)]
struct LocalizationEntryState {
    pub key_line: Option<usize>,
    pub chuck_line_range: Option<(usize, usize)>,
    pub commit_id: Option<git2::Oid>,
    pub state: LocalizationState,
}

impl LocalizationEntryState {
    pub fn new() -> LocalizationEntryState {
        LocalizationEntryState {
            key_line: None,
            chuck_line_range: None,
            commit_id: None,
            state: LocalizationState::Unknown,
        }
    }
}

/// Returns the Git blob associated with the given reference and path
#[allow(clippy::expect_fun_call)] // TODO: Pending review in #587
fn read_file_from_path<'a>(
    repo: &'a git2::Repository,
    reference: &git2::Reference,
    path: &std::path::Path,
) -> git2::Blob<'a> {
    let tree = reference
        .peel_to_tree()
        .expect("Impossible to peel HEAD to a tree object");
    tree.get_path(path)
        .expect(&format!(
            "Impossible to find the file {:?} in reference {:?}",
            path,
            reference.name()
        ))
        .to_object(&repo)
        .unwrap()
        .peel_to_blob()
        .expect("Impossible to fetch the Git object")
}

fn generate_key_version<'a>(
    repo: &'a git2::Repository,
    localization: &VoxygenLocalization,
    path: &std::path::Path,
    file_blob: &git2::Blob,
) -> HashMap<String, LocalizationEntryState> {
    let mut keys: HashMap<String, LocalizationEntryState> = localization
        .string_map
        .keys()
        .map(|k| (k.to_owned(), LocalizationEntryState::new()))
        .collect();
    let mut to_process: HashSet<&String> = localization.string_map.keys().map(|k| k).collect();
    // Find key start lines
    for (line_nb, line) in std::str::from_utf8(file_blob.content())
        .expect("UTF-8 file")
        .split('\n')
        .enumerate()
    {
        let mut found_key = None;

        for key in to_process.iter() {
            if line.contains(key.as_str()) {
                found_key = Some(key.to_owned());
                break;
            }
        }

        if let Some(key) = found_key {
            keys.get_mut(key).unwrap().key_line = Some(line_nb);
            to_process.remove(&key);
        };
    }

    // Find commit for each keys
    repo.blame_file(path, None)
        .expect("Impossible to generate the Git blame")
        .iter()
        .for_each(|e: git2::BlameHunk| {
            for state in keys.values_mut() {
                let line = state.key_line.unwrap();

                if line >= e.final_start_line() && line < e.final_start_line() + e.lines_in_hunk() {
                    state.chuck_line_range = Some((
                        e.final_start_line(),
                        e.final_start_line() + e.lines_in_hunk(),
                    ));
                    state.commit_id = match state.commit_id {
                        Some(existing_commit) => {
                            match repo.graph_descendant_of(e.final_commit_id(), existing_commit) {
                                Ok(true) => Some(e.final_commit_id()),
                                Ok(false) => Some(existing_commit),
                                Err(err) => panic!(err),
                            }
                        },
                        None => Some(e.final_commit_id()),
                    };
                }
            }
        });

    keys
}

#[test]
#[ignore]
#[allow(clippy::expect_fun_call)] // TODO: Pending review in #587
#[allow(clippy::extra_unused_lifetimes)] // TODO: Pending review in #587
#[allow(clippy::or_fun_call)] // TODO: Pending review in #587
fn test_all_localizations<'a>() {
    // Generate paths
    let i18n_asset_path = Path::new("assets/voxygen/i18n/");
    let en_i18n_path = i18n_asset_path.join("en.ron");
    let root_dir = std::env::current_dir()
        .map(|p| p.parent().expect("").to_owned())
        .unwrap();
    let i18n_path = root_dir.join(i18n_asset_path);

    if !root_dir.join(&en_i18n_path).is_file() {
        panic!("Reference language file not found {:?}", &en_i18n_path)
    }

    // Initialize Git objects
    let repo = Repository::discover(&root_dir).expect(&format!(
        "Failed to open the Git repository at {:?}",
        &root_dir
    ));
    let head_ref = repo.head().expect("Impossible to get the HEAD reference");

    // Read HEAD for the reference language file
    let i18n_en_blob = read_file_from_path(&repo, &head_ref, &en_i18n_path);
    let loc: VoxygenLocalization =
        from_bytes(i18n_en_blob.content()).expect("Expect to parse the RON file");
    let i18n_references: HashMap<String, LocalizationEntryState> =
        generate_key_version(&repo, &loc, &en_i18n_path, &i18n_en_blob);

    // Compare to other reference files
    let i18n_files = i18n_files(&i18n_path);
    for file in i18n_files {
        let relfile = file.strip_prefix(&root_dir).unwrap();
        let mut uptodate_entries = 0;
        if relfile == en_i18n_path {
            continue;
        }
        println!("{:?}", relfile);

        // Find the localization entry state
        let current_blob = read_file_from_path(&repo, &head_ref, &relfile);
        let current_loc: VoxygenLocalization =
            from_bytes(current_blob.content()).expect("Expect to parse the RON file");
        let mut current_i18n = generate_key_version(&repo, &current_loc, &relfile, &current_blob);
        for (ref_key, ref_state) in i18n_references.iter() {
            match current_i18n.get_mut(ref_key) {
                Some(state) => {
                    let commit_id = state.commit_id.unwrap();
                    let ref_commit_id = ref_state.commit_id.unwrap();
                    if commit_id != ref_commit_id
                        && !repo
                            .graph_descendant_of(commit_id, ref_commit_id)
                            .unwrap_or(false)
                    {
                        state.state = LocalizationState::Outdated;
                    } else {
                        state.state = LocalizationState::UpToDate;
                    }
                },
                None => {
                    current_i18n.insert(ref_key.to_owned(), LocalizationEntryState {
                        key_line: None,
                        chuck_line_range: None,
                        commit_id: None,
                        state: LocalizationState::NotFound,
                    });
                },
            }
        }

        let ref_keys: HashSet<&String> = i18n_references.keys().collect();
        for (_, state) in current_i18n
            .iter_mut()
            .filter(|&(k, _)| !ref_keys.contains(k))
        {
            state.state = LocalizationState::Unused;
        }

        // Display
        println!(
            "{:10}  {:60}{:40} {:40}",
            "State",
            "Key name",
            relfile.to_str().unwrap(),
            en_i18n_path.to_str().unwrap()
        );

        let mut sorted_keys: Vec<&String> = current_i18n.keys().collect();
        sorted_keys.sort();
        for key in sorted_keys {
            let state = current_i18n.get(key).unwrap();
            if state.state != LocalizationState::UpToDate {
                println!(
                    "[{:9}] {:60}{:40} {:40}",
                    format!("{:?}", state.state),
                    key,
                    state
                        .commit_id
                        .map(|s| format!("{}", s))
                        .unwrap_or("None".to_string()),
                    i18n_references
                        .get(key)
                        .map(|s| s.commit_id)
                        .flatten()
                        .map(|s| format!("{}", s))
                        .unwrap_or("None".to_string()),
                );
            } else {
                uptodate_entries += 1;
            }
        }
        println!("{} entries are up-to-date\n", uptodate_entries);
    }
}
