use ron::de::from_bytes;
use std::path::{Path, PathBuf};

use crate::data::{
    i18n_directories, LocalizationFragment, RawLocalization, LANG_MANIFEST_FILE, REFERENCE_LANG,
};
use hashbrown::{HashMap, HashSet};

#[derive(Copy, Clone, Eq, Hash, Debug, PartialEq)]
enum LocalizationState {
    UpToDate,
    NotFound,
    Outdated,
    Unknown,
    Unused,
}

#[derive(Debug, PartialEq)]
struct LocalizationStats {
    uptodate_entries: usize,
    outdated_entries: usize,
    unused_entries: usize,
    notfound_entries: usize,
    errors: usize,
    real_entry_count: usize,
}

#[derive(Default)]
struct LocalizationAnalysis {
    notfound: Vec<(String, Option<git2::Oid>)>,
    unused: Vec<(String, Option<git2::Oid>)>,
    outdated: Vec<(String, Option<git2::Oid>)>,
    unknown: Vec<(String, Option<git2::Oid>)>,
}

impl LocalizationAnalysis {
    fn get_mut(
        &mut self,
        state: LocalizationState,
    ) -> Option<&mut Vec<(String, Option<git2::Oid>)>> {
        match state {
            LocalizationState::NotFound => Some(&mut self.notfound),
            LocalizationState::Unused => Some(&mut self.unused),
            LocalizationState::Outdated => Some(&mut self.outdated),
            LocalizationState::Unknown => Some(&mut self.unknown),
            _ => None,
        }
    }

    fn show(
        &mut self,
        state: LocalizationState,
        be_verbose: bool,
        ref_i18n_map: &HashMap<String, LocalizationEntryState>,
    ) {
        let entries = self
            .get_mut(state)
            .unwrap_or_else(|| panic!("called on invalid state: {:?}", state));
        if entries.is_empty() {
            return;
        }
        println!("\n\t[{:?}]", state);
        entries.sort();
        for (key, commit_id) in entries {
            if be_verbose {
                let our_commit = commit_id
                    .map(|s| format!("{}", s))
                    .unwrap_or_else(|| "None".to_owned());
                let ref_commit = ref_i18n_map
                    .get(key)
                    .and_then(|s| s.commit_id)
                    .map(|s| format!("{}", s))
                    .unwrap_or_else(|| "None".to_owned());
                println!("{:60}| {:40} | {:40}", key, our_commit, ref_commit,);
            } else {
                println!("{}", key);
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
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

fn complete_key_versions<'a>(
    repo: &'a git2::Repository,
    head_ref: &git2::Reference,
    i18n_key_versions: &mut HashMap<String, LocalizationEntryState>,
    root_dir: &Path,
    lang_dir: &Path,
) {
    //TODO: review unwraps in this file

    // For each file (if it's not a directory) in directory
    for i18n_file in root_dir.join(&lang_dir).read_dir().unwrap().flatten() {
        if let Ok(file_type) = i18n_file.file_type() {
            if file_type.is_file() {
                println!("-> {:?}", i18n_file.file_name());

                let full_path = i18n_file.path();
                let path = full_path.strip_prefix(root_dir).unwrap();
                let i18n_blob = read_file_from_path(&repo, &head_ref, &path);
                let i18n: LocalizationFragment =
                    from_bytes(i18n_blob.content()).unwrap_or_else(|e| {
                        panic!(
                            "Could not parse {} RON file, skipping: {}",
                            i18n_file.path().to_string_lossy(),
                            e
                        )
                    });
                i18n_key_versions.extend(generate_key_version(&repo, &i18n, &path, &i18n_blob));
            }
        }
    }
}

fn gather_state(
    loc: &RawLocalization,
    i18n_blob: &git2::Blob,
    ref_manifest: &Path,
    root_dir: &Path,
    lang_dir: &Path,
    repo: &git2::Repository,
    head_ref: &git2::Reference,
) -> HashMap<String, LocalizationEntryState> {
    // Generate map
    let mut i18n_map = generate_key_version(
        repo,
        &LocalizationFragment::from(loc.clone()),
        ref_manifest,
        i18n_blob,
    );

    // Gathering info about keys from language
    complete_key_versions(repo, head_ref, &mut i18n_map, root_dir, lang_dir);

    // read HEAD for the subfolders
    for sub_directory in loc.sub_directories.iter() {
        let subdir_path = &lang_dir.join(sub_directory);
        complete_key_versions(repo, head_ref, &mut i18n_map, root_dir, subdir_path);
    }

    i18n_map
}

// Helper function to test localization directory
//  `lang_dir` - path to localization directory. Relative from root of the
// repo.
//  `root_dir` - absolute path to repo
//  `ref_manifest` - path to reference manifest
//  `i18n_references` - keys from reference language
//  `repo` - git object for main repo
//  `head_ref` - HEAD
fn test_localization_directory(
    lang_dir: &Path,
    root_dir: &Path,
    ref_manifest: &Path,
    i18n_references: &HashMap<String, LocalizationEntryState>,
    be_verbose: bool,
    repo: &git2::Repository,
    head_ref: &git2::Reference,
) -> Option<LocalizationStats> {
    let relfile = lang_dir.join(&(LANG_MANIFEST_FILE.to_string() + ".ron"));
    if relfile == ref_manifest {
        return None;
    }
    println!("\n-----------------------------------");
    println!("{:?}", relfile);
    println!("-----------------------------------");

    // Find the localization entry state
    let current_blob = read_file_from_path(&repo, &head_ref, &relfile);
    let current_loc: RawLocalization = from_bytes(current_blob.content()).unwrap_or_else(|e| {
        panic!(
            "Could not parse {} RON file, skipping: {}",
            relfile.to_string_lossy(),
            e
        )
    });

    // Gather state of current localization
    let mut current_i18n = gather_state(
        &current_loc,
        &current_blob,
        ref_manifest,
        root_dir,
        lang_dir,
        repo,
        head_ref,
    );

    // Comparing with reference localization
    fill_info(&mut current_i18n, &i18n_references, repo, &relfile);

    let mut state_map = LocalizationAnalysis::default();
    let result = gather_results(current_i18n, &mut state_map);
    print_translation_stats(
        &i18n_references,
        &result,
        &mut state_map,
        be_verbose,
        relfile,
        ref_manifest,
    );
    Some(result)
}

fn fill_info(
    current_i18n: &mut HashMap<String, LocalizationEntryState>,
    i18n_references: &HashMap<String, LocalizationEntryState>,
    repo: &git2::Repository,
    relfile: &Path,
) {
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
}

fn gather_results(
    current_i18n: HashMap<String, LocalizationEntryState>,
    state_map: &mut LocalizationAnalysis,
) -> LocalizationStats {
    let mut uptodate_entries = 0;
    let mut outdated_entries = 0;
    let mut unused_entries = 0;
    let mut notfound_entries = 0;
    let mut unknown_entries = 0;

    let keys: Vec<&String> = current_i18n.keys().collect();
    for key in keys {
        let entry = current_i18n.get(key).unwrap();
        match entry.state {
            LocalizationState::Outdated => outdated_entries += 1,
            LocalizationState::NotFound => notfound_entries += 1,
            LocalizationState::Unknown => unknown_entries += 1,
            LocalizationState::Unused => unused_entries += 1,
            LocalizationState::UpToDate => uptodate_entries += 1,
        };
        if entry.state != LocalizationState::UpToDate {
            let state_keys = state_map
                .get_mut(entry.state)
                .expect("vectors must be added");
            state_keys.push((key.to_owned(), entry.commit_id));
        }
    }

    // Calculate key count that actually matter for the status of the translation
    // Unused entries don't break the game
    let current_i18n_entry_count = current_i18n.len();
    let real_entry_count = current_i18n_entry_count - unused_entries;

    LocalizationStats {
        uptodate_entries,
        unused_entries,
        outdated_entries,
        notfound_entries,
        errors: unknown_entries,
        real_entry_count,
    }
}

fn print_translation_stats(
    ref_i18n_map: &HashMap<String, LocalizationEntryState>,
    stats: &LocalizationStats,
    state_map: &mut LocalizationAnalysis,
    be_verbose: bool,
    relfile: PathBuf,
    ref_manifest: &Path,
) {
    let uptodate_percent =
        (stats.uptodate_entries as f32 / stats.real_entry_count as f32) * 100_f32;
    let outdated_percent =
        (stats.outdated_entries as f32 / stats.real_entry_count as f32) * 100_f32;
    let untranslated_percent =
        ((stats.errors + stats.errors) as f32 / stats.real_entry_count as f32) * 100_f32;

    // Display
    if be_verbose {
        println!(
            "\n{:60}| {:40} | {:40}",
            "Key name",
            relfile.to_str().unwrap(),
            ref_manifest.to_str().unwrap(),
        );
    } else {
        println!("\nKey name");
    }

    state_map.show(LocalizationState::NotFound, be_verbose, ref_i18n_map);
    state_map.show(LocalizationState::Unused, be_verbose, ref_i18n_map);
    state_map.show(LocalizationState::Outdated, be_verbose, ref_i18n_map);
    state_map.show(LocalizationState::Unknown, be_verbose, ref_i18n_map);

    println!(
        "\n{} up-to-date, {} outdated, {} unused, {} not found, {} unknown entries",
        stats.uptodate_entries,
        stats.outdated_entries,
        stats.unused_entries,
        stats.notfound_entries,
        stats.errors,
    );

    println!(
        "{:.2}% up-to-date, {:.2}% outdated, {:.2}% untranslated\n",
        uptodate_percent, outdated_percent, untranslated_percent,
    );
}

/// Test one language
/// `code` - name of the directory in assets (de_DE for example)
/// `root_dir` - absolute path to main repo
/// `assets_path` - relative path to asset directory (right now it is
/// 'assets/voxygen/i18n')
pub fn test_specific_localization(
    code: &str,
    root_dir: &Path,
    assets_path: &Path,
    be_verbose: bool,
) {
    // Relative paths from root of repo to assets
    let ref_lang_dir = assets_path.join(REFERENCE_LANG);
    let ref_manifest = ref_lang_dir.join(LANG_MANIFEST_FILE.to_string() + ".ron");

    // Initialize Git objects
    let repo = git2::Repository::discover(&root_dir)
        .unwrap_or_else(|_| panic!("Failed to open the Git repository at {:?}", &root_dir));
    let head_ref = repo.head().expect("Impossible to get the HEAD reference");

    // Read HEAD for the reference language manifest
    let ref_manifest_blob = read_file_from_path(&repo, &head_ref, &ref_manifest);
    let loc: RawLocalization = from_bytes(ref_manifest_blob.content())
        .expect("Expect to parse reference i18n RON file, can't proceed without it");

    // Gathering info about keys from reference language
    let reference_i18n = gather_state(
        &loc,
        &ref_manifest_blob,
        &ref_manifest,
        root_dir,
        &ref_lang_dir,
        &repo,
        &head_ref,
    );

    // Testing how specific language is localized
    let dir = assets_path.join(code);
    test_localization_directory(
        &dir,
        root_dir,
        &ref_manifest,
        &reference_i18n,
        be_verbose,
        &repo,
        &head_ref,
    );
}

/// Test all localizations
/// `root_dir` - absolute path to main repo
/// `assets_path` - relative path to asset directory (right now it is
/// 'assets/voxygen/i18n')
pub fn test_all_localizations(root_dir: &Path, assets_path: &Path, be_verbose: bool) {
    let ref_lang_dir = assets_path.join(REFERENCE_LANG);
    let ref_manifest = ref_lang_dir.join(LANG_MANIFEST_FILE.to_string() + ".ron");

    if !root_dir.join(&ref_lang_dir).is_dir() {
        panic!("Reference language folder not found {:?}", &ref_lang_dir)
    }
    if !root_dir.join(&ref_manifest).is_file() {
        panic!("Reference language file not found {:?}", &ref_manifest)
    }

    // Initialize Git objects
    let repo = git2::Repository::discover(&root_dir)
        .unwrap_or_else(|_| panic!("Failed to open the Git repository at {:?}", &root_dir));
    let head_ref = repo.head().expect("Impossible to get the HEAD reference");

    // Read HEAD for the reference language file
    let ref_manifest_blob = read_file_from_path(&repo, &head_ref, &ref_manifest);
    let loc: RawLocalization = from_bytes(ref_manifest_blob.content())
        .expect("Expect to parse reference i18n RON file, can't proceed without it");

    // Gathering info about keys from reference language
    let reference_i18n = gather_state(
        &loc,
        &ref_manifest_blob,
        &ref_manifest,
        root_dir,
        &ref_lang_dir,
        &repo,
        &head_ref,
    );

    // Compare to other reference files
    let i18n_directories = i18n_directories(&root_dir.join(assets_path));
    let mut i18n_entry_counts: HashMap<PathBuf, LocalizationStats> = HashMap::new();
    for dir in &i18n_directories {
        let rel_dir = dir.strip_prefix(root_dir).unwrap();
        let result = test_localization_directory(
            rel_dir,
            root_dir,
            &ref_manifest,
            &reference_i18n,
            be_verbose,
            &repo,
            &head_ref,
        );
        if let Some(values) = result {
            i18n_entry_counts.insert(dir.clone(), values);
        }
    }

    print_overall_stats(i18n_entry_counts);
}

fn print_overall_stats(i18n_entry_counts: HashMap<PathBuf, LocalizationStats>) {
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

    let mut i18n_stats: Vec<(&PathBuf, &LocalizationStats)> = i18n_entry_counts.iter().collect();
    i18n_stats.sort_by_key(|(_, result)| result.notfound_entries);

    for (path, test_result) in i18n_stats {
        let LocalizationStats {
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
