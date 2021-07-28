use crate::{
    gitfragments::{
        read_file_from_path, transform_fragment, LocalizationEntryState, LocalizationState,
    },
    i18n_directories,
    raw::{self, RawFragment, RawLanguage},
    stats::{
        print_csv_file, print_overall_stats, print_translation_stats, LocalizationAnalysis,
        LocalizationStats,
    },
    REFERENCE_LANG,
};
use hashbrown::{hash_map::Entry, HashMap, HashSet};
use ron::de::from_bytes;
use std::path::Path;

/// Fill the entry State base information (except `state`) for a complete
/// language
fn gather_entry_state<'a>(
    repo: &'a git2::Repository,
    head_ref: &git2::Reference,
    language_identifier: &str,
    root_path: &Path,
    relative_i18n_root_path: &Path,
) -> RawLanguage<LocalizationEntryState> {
    println!("-> {:?}", &language_identifier);
    let i18n_root_path = root_path.join(relative_i18n_root_path);
    // load standard manifest
    let manifest = raw::load_manifest(&i18n_root_path, language_identifier)
        .expect("failed to load language manifest");
    // transform language into LocalizationEntryState
    let mut fragments = HashMap::new();

    // For each file in directory
    let files = raw::fragments_pathes_in_language(&i18n_root_path, language_identifier)
        .expect("failed to get all files in language");
    for subpath in files {
        let path = relative_i18n_root_path
            .join(language_identifier)
            .join(&subpath);
        println!("  -> {:?}", &subpath);
        let i18n_blob = read_file_from_path(repo, head_ref, &path);
        let fragment: RawFragment<String> = from_bytes(i18n_blob.content()).unwrap_or_else(|e| {
            panic!(
                "Could not parse {} RON file, skipping: {}",
                subpath.to_string_lossy(),
                e
            )
        });
        let frag = transform_fragment(repo, (&path, fragment), &i18n_blob);
        fragments.insert(subpath.to_path_buf(), frag);
    }

    RawLanguage::<LocalizationEntryState> {
        manifest,
        fragments,
    }
}

/// fills in the `state`
fn compare_lang_with_reference(
    current_i18n: &mut RawLanguage<LocalizationEntryState>,
    i18n_references: &RawLanguage<LocalizationEntryState>,
    repo: &git2::Repository,
) {
    // git graph decendent of is slow, so we cache it
    let mut graph_decendent_of_cache = HashMap::new();

    let mut cached_graph_descendant_of = |commit, ancestor| -> bool {
        let key = (commit, ancestor);
        match graph_decendent_of_cache.entry(key) {
            Entry::Occupied(entry) => {
                return *entry.get();
            },
            Entry::Vacant(entry) => {
                let value = repo.graph_descendant_of(commit, ancestor).unwrap_or(false);
                *entry.insert(value)
            },
        }
    };

    // match files
    for (ref_path, ref_fragment) in i18n_references.fragments.iter() {
        let cur_fragment = match current_i18n.fragments.get_mut(ref_path) {
            Some(c) => c,
            None => {
                eprintln!(
                    "language {} is missing file: {:?}",
                    current_i18n.manifest.metadata.language_identifier, ref_path
                );
                continue;
            },
        };

        for (ref_key, ref_state) in ref_fragment.string_map.iter() {
            match cur_fragment.string_map.get_mut(ref_key) {
                Some(state) => {
                    let commit_id = match state.commit_id {
                        Some(c) => c,
                        None => {
                            eprintln!(
                                "Commit ID of key {} in i18n file {} is missing! Skipping key.",
                                ref_key,
                                ref_path.to_string_lossy()
                            );
                            continue;
                        },
                    };
                    let ref_commit_id = match ref_state.commit_id {
                        Some(c) => c,
                        None => {
                            eprintln!(
                                "Commit ID of key {} in reference i18n file is missing! Skipping \
                                 key.",
                                ref_key
                            );
                            continue;
                        },
                    };
                    if commit_id != ref_commit_id
                        && !cached_graph_descendant_of(commit_id, ref_commit_id)
                    {
                        state.state = Some(LocalizationState::Outdated);
                    } else {
                        state.state = Some(LocalizationState::UpToDate);
                    }
                },
                None => {
                    cur_fragment
                        .string_map
                        .insert(ref_key.to_owned(), LocalizationEntryState {
                            key_line: None,
                            chuck_line_range: None,
                            commit_id: None,
                            state: Some(LocalizationState::NotFound),
                        });
                },
            }
        }

        let ref_keys: HashSet<&String> = ref_fragment.string_map.keys().collect();
        for (_, state) in cur_fragment
            .string_map
            .iter_mut()
            .filter(|&(k, _)| !ref_keys.contains(k))
        {
            state.state = Some(LocalizationState::Unused);
        }
    }
}

fn gather_results(
    current_i18n: &RawLanguage<LocalizationEntryState>,
) -> (LocalizationAnalysis, LocalizationStats) {
    let mut state_map =
        LocalizationAnalysis::new(&current_i18n.manifest.metadata.language_identifier);
    let mut stats = LocalizationStats::default();

    for (file, fragments) in &current_i18n.fragments {
        for (key, entry) in &fragments.string_map {
            match entry.state {
                Some(LocalizationState::Outdated) => stats.outdated_entries += 1,
                Some(LocalizationState::NotFound) => stats.notfound_entries += 1,
                None => stats.errors += 1,
                Some(LocalizationState::Unused) => stats.unused_entries += 1,
                Some(LocalizationState::UpToDate) => stats.uptodate_entries += 1,
            };
            if entry.state != Some(LocalizationState::UpToDate) {
                let state_keys = state_map.data.get_mut(&entry.state).expect("prefiled");
                state_keys.push((file.clone(), key.to_owned(), entry.commit_id));
            }
        }
    }

    for (_, entries) in state_map.data.iter_mut() {
        entries.sort();
    }

    (state_map, stats)
}

/// completely analysis multiple languages without printing
fn complete_analysis(
    language_identifiers: &[&str],
    root_path: &Path,
    relative_i18n_root_path: &Path,
) -> (
    HashMap<String, (LocalizationAnalysis, LocalizationStats)>,
    /* ref lang */ RawLanguage<LocalizationEntryState>,
) {
    let mut result = HashMap::new();
    // Initialize Git objects
    let repo = git2::Repository::discover(&root_path)
        .unwrap_or_else(|_| panic!("Failed to open the Git repository at {:?}", &root_path));
    let head_ref = repo.head().expect("Impossible to get the HEAD reference");

    // Read Reference Language
    let ref_language = gather_entry_state(
        &repo,
        &head_ref,
        REFERENCE_LANG,
        root_path,
        relative_i18n_root_path,
    );
    for &language_identifier in language_identifiers {
        let mut cur_language = gather_entry_state(
            &repo,
            &head_ref,
            language_identifier,
            root_path,
            relative_i18n_root_path,
        );
        compare_lang_with_reference(&mut cur_language, &ref_language, &repo);
        let (state_map, stats) = gather_results(&cur_language);
        result.insert(language_identifier.to_owned(), (state_map, stats));
    }
    (result, ref_language)
}

/// Test one language
/// - `code`: name of the directory in assets (de_DE for example)
/// - `root_path`: absolute path to main repo
/// - `relative_i18n_root_path`: relative path to asset directory (right now it
///   is 'assets/voxygen/i18n')
/// - be_verbose: print extra info
/// - csv_enabled: generate csv files in target folder
pub fn test_specific_localizations(
    language_identifiers: &[&str],
    root_path: &Path,
    relative_i18n_root_path: &Path,
    be_verbose: bool,
    csv_enabled: bool,
) {
    let (analysis, reference_language) =
        complete_analysis(language_identifiers, root_path, relative_i18n_root_path);
    for (language_identifier, (state_map, stats)) in &analysis {
        if csv_enabled {
            print_csv_file(state_map);
        } else {
            print_translation_stats(
                language_identifier,
                &reference_language,
                stats,
                state_map,
                be_verbose,
            );
        }
    }
    if analysis.len() > 1 {
        print_overall_stats(analysis);
    }
}

/// Test all localizations
pub fn test_all_localizations(
    root_path: &Path,
    relative_i18n_root_path: &Path,
    be_verbose: bool,
    csv_enabled: bool,
) {
    let i18n_root_path = root_path.join(relative_i18n_root_path);
    // Compare to other reference files
    let language_identifiers = i18n_directories(&i18n_root_path)
        .into_iter()
        .map(|p| {
            p.strip_prefix(&i18n_root_path)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    test_specific_localizations(
        &language_identifiers
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
        root_path,
        relative_i18n_root_path,
        be_verbose,
        csv_enabled,
    );
}
