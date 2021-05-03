use ron::de::{from_bytes, from_reader};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

use std::collections::{HashMap, HashSet};

/// The reference language, aka the more up-to-date localization data.  Also the
/// default language at first startup.
const REFERENCE_LANG: &str = "en";

const LANG_MANIFEST_FILE: &str = "_manifest";

/// How a language can be described
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LanguageMetadata {
    /// A human friendly language name (e.g. "English (US)")
    language_name: String,

    /// A short text identifier for this language (e.g. "en_US")
    ///
    /// On the opposite of `language_name` that can change freely,
    /// `language_identifier` value shall be stable in time as it
    /// is used by setting components to store the language
    /// selected by the user.
    language_identifier: String,
}

/// Store font metadata
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Font {
    /// Key to retrieve the font in the asset system
    asset_key: String,

    /// Scale ratio to resize the UI text dynamicly
    scale_ratio: f32,
}

/// Store font metadata
type Fonts = HashMap<String, Font>;

/// Raw localization data, expect the strings to not be loaded here
/// However, metadata informations are correct
/// See `Localization` for more info on each attributes
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct RawLocalization {
    sub_directories: Vec<String>,
    string_map: HashMap<String, String>,
    vector_map: HashMap<String, Vec<String>>,
    convert_utf8_to_ascii: bool,
    fonts: Fonts,
    metadata: LanguageMetadata,
}

/// Store internationalization data
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Localization {
    /// A list of subdirectories to lookup for localization files
    sub_directories: Vec<String>,

    /// A map storing the localized texts
    ///
    /// Localized content can be accessed using a String key.
    string_map: HashMap<String, String>,

    /// A map for storing variations of localized texts, for example multiple
    /// ways of saying "Help, I'm under attack". Used primarily for npc
    /// dialogue.
    vector_map: HashMap<String, Vec<String>>,

    /// Whether to convert the input text encoded in UTF-8
    /// into a ASCII version by using the `deunicode` crate.
    convert_utf8_to_ascii: bool,

    /// Font configuration is stored here
    fonts: Fonts,

    metadata: LanguageMetadata,
}

/// Store internationalization maps
/// These structs are meant to be merged into a Localization
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct LocalizationFragment {
    /// A map storing the localized texts
    ///
    /// Localized content can be accessed using a String key.
    string_map: HashMap<String, String>,

    /// A map for storing variations of localized texts, for example multiple
    /// ways of saying "Help, I'm under attack". Used primarily for npc
    /// dialogue.
    vector_map: HashMap<String, Vec<String>>,
}

impl Localization {}

impl From<RawLocalization> for Localization {
    fn from(raw: RawLocalization) -> Self {
        Self {
            sub_directories: raw.sub_directories,
            string_map: raw.string_map,
            vector_map: raw.vector_map,
            convert_utf8_to_ascii: raw.convert_utf8_to_ascii,
            fonts: raw.fonts,
            metadata: raw.metadata,
        }
    }
}
impl From<RawLocalization> for LocalizationFragment {
    fn from(raw: RawLocalization) -> Self {
        Self {
            string_map: raw.string_map,
            vector_map: raw.vector_map,
        }
    }
}

#[derive(Clone, Debug)]
struct LocalizationList(Vec<LanguageMetadata>);

/// List localization directories as a PathBuf vector
fn i18n_directories(i18n_dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(i18n_dir)
        .unwrap()
        .map(|res| res.map(|e| e.path()).unwrap())
        .filter(|e| e.is_dir())
        .collect()
}

#[derive(Eq, Hash, Debug, PartialEq)]
enum LocalizationState {
    UpToDate,
    NotFound,
    Outdated,
    Unknown,
    Unused,
}

#[derive(Debug, PartialEq)]
struct FindLocalization {
    uptodate_entries: usize,
    outdated_entries: usize,
    unused_entries: usize,
    notfound_entries: usize,
    errors: usize,
    real_entry_count: usize,
}

#[derive(Debug)]
struct LocalizationEntryState {
    key_line: Option<usize>,
    chuck_line_range: Option<(usize, usize)>,
    commit_id: Option<git2::Oid>,
    state: LocalizationState,
}

impl LocalizationEntryState {
    fn new() -> LocalizationEntryState {
        LocalizationEntryState {
            key_line: None,
            chuck_line_range: None,
            commit_id: None,
            state: LocalizationState::Unknown,
        }
    }
}

/// Returns the Git blob associated with the given reference and path
fn read_file_from_path<'a>(
    repo: &'a git2::Repository,
    reference: &git2::Reference,
    path: &std::path::Path,
) -> git2::Blob<'a> {
    let tree = reference
        .peel_to_tree()
        .expect("Impossible to peel HEAD to a tree object");
    tree.get_path(path)
        .unwrap_or_else(|_| {
            panic!(
                "Impossible to find the file {:?} in reference {:?}",
                path,
                reference.name()
            )
        })
        .to_object(&repo)
        .unwrap()
        .peel_to_blob()
        .expect("Impossible to fetch the Git object")
}

fn correspond(line: &str, key: &str) -> bool {
    let pat = {
        // Get left part of split
        let mut begin = line
            .split(':')
            .next()
            .expect("split always produces value")
            .trim()
            .chars();
        // Remove quotes
        begin.next();
        begin.next_back();
        begin.as_str()
    };

    pat == key
}

fn generate_key_version<'a>(
    repo: &'a git2::Repository,
    localization: &LocalizationFragment,
    path: &std::path::Path,
    file_blob: &git2::Blob,
) -> HashMap<String, LocalizationEntryState> {
    let mut keys: HashMap<String, LocalizationEntryState> = localization
        .string_map
        .keys()
        .map(|k| (k.to_owned(), LocalizationEntryState::new()))
        .collect();
    let mut to_process: HashSet<&String> = localization.string_map.keys().collect();
    // Find key start lines
    let file_content = std::str::from_utf8(file_blob.content()).expect("Got non UTF-8 file");

    // Make the file hot
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

fn complete_key_versions<'a>(
    repo: &'a git2::Repository,
    head_ref: &git2::Reference,
    i18n_key_versions: &mut HashMap<String, LocalizationEntryState>,
    root_dir: &Path,
    asset_path: &Path,
) {
    //TODO: review unwraps in this file

    // For each file (if it's not a directory) in directory
    for i18n_file in root_dir.join(&asset_path).read_dir().unwrap().flatten() {
        if let Ok(file_type) = i18n_file.file_type() {
            if file_type.is_file() {
                println!("-> {:?}", i18n_file.file_name());

                let full_path = i18n_file.path();
                let path = full_path.strip_prefix(root_dir).unwrap();
                let i18n_blob = read_file_from_path(&repo, &head_ref, &path);
                let i18n: LocalizationFragment = match from_bytes(i18n_blob.content()) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!(
                            "Could not parse {} RON file, skipping: {}",
                            i18n_file.path().to_string_lossy(),
                            e
                        );
                        continue;
                    },
                };
                i18n_key_versions.extend(generate_key_version(&repo, &i18n, &path, &i18n_blob));
            }
        }
    }
}

fn verify_localization_directory(root_dir: &Path, directory_path: &Path) {
    // Walk through each file in the directory
    for i18n_file in root_dir.join(&directory_path).read_dir().unwrap().flatten() {
        if let Ok(file_type) = i18n_file.file_type() {
            // Skip folders and the manifest file (which does not contain the same struct we
            // want to load)
            if file_type.is_file()
                && i18n_file.file_name().to_string_lossy()
                    != (LANG_MANIFEST_FILE.to_string() + ".ron")
            {
                let full_path = i18n_file.path();
                println!("-> {:?}", full_path.strip_prefix(&root_dir).unwrap());
                let f = fs::File::open(&full_path).expect("Failed opening file");
                let _: LocalizationFragment = match from_reader(f) {
                    Ok(v) => v,
                    Err(e) => {
                        panic!(
                            "Could not parse {} RON file, error: {}",
                            full_path.to_string_lossy(),
                            e
                        );
                    },
                };
            }
        }
    }
}

// Test to verify all languages that they are VALID and loadable, without
// need of git just on the local assets folder
pub fn verify_all_localizations(root_dir: &Path, asset_path: &Path) {
    let ref_i18n_dir_path = asset_path.join(REFERENCE_LANG);
    let ref_i18n_path = ref_i18n_dir_path.join(LANG_MANIFEST_FILE.to_string() + ".ron");
    assert!(
        root_dir.join(&ref_i18n_dir_path).is_dir(),
        "Reference language folder doesn't exist, something is wrong!"
    );
    assert!(
        root_dir.join(&ref_i18n_path).is_file(),
        "Reference language manifest file doesn't exist, something is wrong!"
    );
    let i18n_directories = i18n_directories(&root_dir.join(asset_path));
    // This simple check  ONLY guarantees that an arbitrary minimum of translation
    // files exists. It's just to notice unintentional deletion of all
    // files, or modifying the paths. In case you want to delete all
    // language you have to adjust this number:
    assert!(
        i18n_directories.len() > 5,
        "have less than 5 translation folders, arbitrary minimum check failed. Maybe the i18n \
         folder is empty?"
    );
    for i18n_directory in i18n_directories {
        // Attempt to load the manifest file
        let manifest_path = i18n_directory.join(LANG_MANIFEST_FILE.to_string() + ".ron");
        println!(
            "verifying {:?}",
            manifest_path.strip_prefix(&root_dir).unwrap()
        );
        let f = fs::File::open(&manifest_path).expect("Failed opening file");
        let raw_localization: RawLocalization = match from_reader(f) {
            Ok(v) => v,
            Err(e) => {
                panic!(
                    "Could not parse {} RON file, error: {}",
                    i18n_directory.to_string_lossy(),
                    e
                );
            },
        };
        // Walk through each files and try to load them
        verify_localization_directory(root_dir, &i18n_directory);
        // Walk through each subdirectories and try to load files in them
        for sub_directory in raw_localization.sub_directories.iter() {
            let subdir_path = &i18n_directory.join(sub_directory);
            verify_localization_directory(root_dir, &subdir_path);
        }
    }
}

///  `asset_path` - path to localization directory. Relative from root of the
/// repo.  `root_dir` - absolute path to repo
///  `ref_i18n_path` - path to reference manifest
///  `i18n_references` - keys from reference language
///  `repo` - git object for main repo
///  `head_ref` - HEAD
fn test_localization_directory(
    asset_path: &Path,
    root_dir: &Path,
    ref_i18n_path: &Path,
    i18n_references: &HashMap<String, LocalizationEntryState>,
    repo: &git2::Repository,
    head_ref: &git2::Reference,
) -> Option<FindLocalization> {
    let relfile = asset_path.join(&(LANG_MANIFEST_FILE.to_string() + ".ron"));
    if relfile == ref_i18n_path {
        return None;
    }
    println!("\n-----------------------------------");
    println!("{:?}", relfile);
    println!("-----------------------------------");

    // Find the localization entry state
    let current_blob = read_file_from_path(&repo, &head_ref, &relfile);
    let current_loc: RawLocalization = match from_bytes(current_blob.content()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "Could not parse {} RON file, skipping: {}",
                relfile.to_string_lossy(),
                e
            );
            return None;
        },
    };
    let mut current_i18n = generate_key_version(
        &repo,
        &LocalizationFragment::from(current_loc.clone()),
        &relfile,
        &current_blob,
    );
    // read HEAD for the fragment files
    complete_key_versions(&repo, &head_ref, &mut current_i18n, root_dir, &asset_path);
    // read HEAD for the subfolders
    for sub_directory in current_loc.sub_directories.iter() {
        let subdir_path = &asset_path.join(sub_directory);
        complete_key_versions(&repo, &head_ref, &mut current_i18n, root_dir, &subdir_path);
    }

    for (ref_key, ref_state) in i18n_references.iter() {
        match current_i18n.get_mut(ref_key) {
            Some(state) => {
                let commit_id = match state.commit_id {
                    Some(c) => c,
                    None => {
                        eprintln!(
                            "Commit ID of key {} in i18n file {} is missing! Skipping key.",
                            ref_key,
                            relfile.to_string_lossy()
                        );
                        continue;
                    },
                };
                let ref_commit_id = match ref_state.commit_id {
                    Some(c) => c,
                    None => {
                        eprintln!(
                            "Commit ID of key {} in reference i18n file is missing! Skipping key.",
                            ref_key
                        );
                        continue;
                    },
                };
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

    let keys: Vec<&String> = current_i18n.keys().collect();
    let mut state_map: HashMap<LocalizationState, Vec<(&String, Option<git2::Oid>)>> =
        HashMap::new();
    state_map.insert(LocalizationState::Outdated, Vec::new());
    state_map.insert(LocalizationState::NotFound, Vec::new());
    state_map.insert(LocalizationState::Unknown, Vec::new());
    state_map.insert(LocalizationState::Unused, Vec::new());

    let current_i18n_entry_count = current_i18n.len();
    let mut uptodate_entries = 0;
    let mut outdated_entries = 0;
    let mut unused_entries = 0;
    let mut notfound_entries = 0;
    let mut unknown_entries = 0;

    for key in keys {
        let entry = current_i18n.get(key).unwrap();
        if entry.state != LocalizationState::UpToDate {
            let state_keys = state_map
                .get_mut(&entry.state)
                .expect("vectors must be added");
            state_keys.push((key, entry.commit_id));
            match entry.state {
                LocalizationState::Outdated => outdated_entries += 1,
                LocalizationState::NotFound => notfound_entries += 1,
                LocalizationState::Unknown => unknown_entries += 1,
                LocalizationState::Unused => unused_entries += 1,
                LocalizationState::UpToDate => unreachable!(),
            };
        } else {
            uptodate_entries += 1;
        }
    }

    // Display
    println!(
        "\n{:60}| {:40} | {:40}\n",
        "Key name",
        relfile.to_str().unwrap(),
        ref_i18n_path.to_str().unwrap()
    );

    for (state, mut lines) in state_map {
        if lines.is_empty() {
            continue;
        }
        println!("\n\t[{:?}]", state);
        lines.sort();
        for line in lines {
            println!(
                "{:60}| {:40} | {:40}",
                line.0,
                line.1
                    .map(|s| format!("{}", s))
                    .unwrap_or_else(|| "None".to_string()),
                i18n_references
                    .get(line.0)
                    .map(|s| s.commit_id)
                    .flatten()
                    .map(|s| format!("{}", s))
                    .unwrap_or_else(|| "None".to_string()),
            );
        }
    }

    println!(
        "\n{} up-to-date, {} outdated, {} unused, {} not found, {} unknown entries",
        uptodate_entries, outdated_entries, unused_entries, notfound_entries, unknown_entries
    );

    // Calculate key count that actually matter for the status of the translation
    // Unused entries don't break the game
    let real_entry_count = current_i18n_entry_count - unused_entries;
    let uptodate_percent = (uptodate_entries as f32 / real_entry_count as f32) * 100_f32;
    let outdated_percent = (outdated_entries as f32 / real_entry_count as f32) * 100_f32;
    let untranslated_percent =
        ((notfound_entries + unknown_entries) as f32 / real_entry_count as f32) * 100_f32;

    println!(
        "{:.2}% up-to-date, {:.2}% outdated, {:.2}% untranslated\n",
        uptodate_percent, outdated_percent, untranslated_percent,
    );

    let result = FindLocalization {
        uptodate_entries,
        unused_entries,
        outdated_entries,
        notfound_entries,
        errors: unknown_entries,
        real_entry_count,
    };
    Some(result)
}

// `asset_path` - relative path to asset directory (should be
// "assets/voxygen/i18n/") `root_dir` - absolute path to main repo
pub fn test_specific_localization(code: String, root_dir: &Path, asset_path: &Path) {
    // Relative paths from root of repo to assets
    let ref_lang_dir = asset_path.join(REFERENCE_LANG);
    let ref_manifest = ref_lang_dir.join(LANG_MANIFEST_FILE.to_string() + ".ron");

    // Initialize Git objects
    let repo = git2::Repository::discover(&root_dir)
        .unwrap_or_else(|_| panic!("Failed to open the Git repository at {:?}", &root_dir));
    let head_ref = repo.head().expect("Impossible to get the HEAD reference");

    // Read HEAD for the reference language manifest
    let ref_manifest_blob = read_file_from_path(&repo, &head_ref, &ref_manifest);
    let loc: RawLocalization = from_bytes(ref_manifest_blob.content())
        .expect("Expect to parse reference i18n RON file, can't proceed without it");
    let mut i18n_references: HashMap<String, LocalizationEntryState> = generate_key_version(
        &repo,
        &LocalizationFragment::from(loc.clone()),
        &ref_manifest,
        &ref_manifest_blob,
    );

    // Gathering info about keys from reference language
    complete_key_versions(
        &repo,
        &head_ref,
        &mut i18n_references,
        root_dir,
        &ref_lang_dir,
    );
    for sub_directory in loc.sub_directories.iter() {
        let subdir_path = &ref_lang_dir.join(sub_directory);
        complete_key_versions(
            &repo,
            &head_ref,
            &mut i18n_references,
            root_dir,
            &subdir_path,
        );
    }

    // Testing how specific language is localized
    let dir = asset_path.join(code);
    test_localization_directory(
        &dir,
        root_dir,
        &ref_manifest,
        &i18n_references,
        &repo,
        &head_ref,
    );
}

pub fn test_all_localizations(root_dir: &Path, asset_path: &Path) {
    let ref_i18n_dir_path = asset_path.join(REFERENCE_LANG);
    let ref_i18n_path = ref_i18n_dir_path.join(LANG_MANIFEST_FILE.to_string() + ".ron");

    if !root_dir.join(&ref_i18n_dir_path).is_dir() {
        panic!(
            "Reference language folder not found {:?}",
            &ref_i18n_dir_path
        )
    }
    if !root_dir.join(&ref_i18n_path).is_file() {
        panic!("Reference language file not found {:?}", &ref_i18n_path)
    }

    // Initialize Git objects
    let repo = git2::Repository::discover(&root_dir)
        .unwrap_or_else(|_| panic!("Failed to open the Git repository at {:?}", &root_dir));
    let head_ref = repo.head().expect("Impossible to get the HEAD reference");

    // Read HEAD for the reference language file
    let i18n_ref_blob = read_file_from_path(&repo, &head_ref, &ref_i18n_path);
    let loc: RawLocalization = from_bytes(i18n_ref_blob.content())
        .expect("Expect to parse reference i18n RON file, can't proceed without it");
    let mut i18n_references: HashMap<String, LocalizationEntryState> = generate_key_version(
        &repo,
        &LocalizationFragment::from(loc.clone()),
        &ref_i18n_path,
        &i18n_ref_blob,
    );

    // Gathering info about keys from reference language
    complete_key_versions(
        &repo,
        &head_ref,
        &mut i18n_references,
        root_dir,
        &ref_i18n_dir_path,
    );
    // read HEAD for the subfolders
    for sub_directory in loc.sub_directories.iter() {
        let subdir_path = &ref_i18n_dir_path.join(sub_directory);
        complete_key_versions(
            &repo,
            &head_ref,
            &mut i18n_references,
            root_dir,
            &subdir_path,
        );
    }

    // Compare to other reference files
    let i18n_directories = i18n_directories(&root_dir.join(asset_path));
    let mut i18n_entry_counts: HashMap<PathBuf, FindLocalization> = HashMap::new();
    for dir in &i18n_directories {
        let rel_dir = dir.strip_prefix(root_dir).unwrap();
        let result = test_localization_directory(
            rel_dir,
            root_dir,
            &ref_i18n_path,
            &i18n_references,
            &repo,
            &head_ref,
        );
        if let Some(values) = result {
            i18n_entry_counts.insert(dir.clone(), values);
        }
    }

    let mut overall_uptodate_entry_count = 0;
    let mut overall_outdated_entry_count = 0;
    let mut overall_untranslated_entry_count = 0;
    let mut overall_real_entry_count = 0;

    println!("-----------------------------------------------------------------------------");
    println!("Overall Translation Status");
    println!("-----------------------------------------------------------------------------");
    println!(
        "{:12}| {:8} | {:8} | {:8} | {:8} | {:8}",
        "", "up-to-date", "outdated", "untranslated", "unused", "errors",
    );

    for (path, test_result) in i18n_entry_counts {
        let FindLocalization {
            uptodate_entries: uptodate,
            outdated_entries: outdated,
            unused_entries: unused,
            notfound_entries: untranslated,
            errors,
            real_entry_count: real,
        } = test_result;
        overall_uptodate_entry_count += uptodate;
        overall_outdated_entry_count += outdated;
        overall_untranslated_entry_count += untranslated;
        overall_real_entry_count += real;

        println!(
            "{:12}|{:8}    |{:6}    |{:8}      |{:6}    |{:8}",
            path.file_name().unwrap().to_string_lossy(),
            uptodate,
            outdated,
            untranslated,
            unused,
            errors,
        );
    }

    println!(
        "\n{:.2}% up-to-date, {:.2}% outdated, {:.2}% untranslated",
        (overall_uptodate_entry_count as f32 / overall_real_entry_count as f32) * 100_f32,
        (overall_outdated_entry_count as f32 / overall_real_entry_count as f32) * 100_f32,
        (overall_untranslated_entry_count as f32 / overall_real_entry_count as f32) * 100_f32,
    );
    println!("-----------------------------------------------------------------------------\n");
}
