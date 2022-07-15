//! fragment attached with git versioning information
use crate::raw::RawFragment;
use hashbrown::HashMap;
use std::path::Path;

#[derive(Copy, Clone, Eq, Hash, Debug, PartialEq)]
pub(crate) enum LocalizationState {
    UpToDate,
    NotFound,
    Outdated,
    Unused,
}

pub(crate) const ALL_LOCALIZATION_STATES: [Option<LocalizationState>; 5] = [
    Some(LocalizationState::UpToDate),
    Some(LocalizationState::NotFound),
    Some(LocalizationState::Outdated),
    Some(LocalizationState::Unused),
    None,
];

#[derive(Clone, Debug)]
pub(crate) struct LocalizationEntryState {
    pub(crate) key_line: Option<usize>,
    pub(crate) chuck_line_range: Option<(usize, usize)>,
    pub(crate) commit_id: Option<git2::Oid>,
    pub(crate) state: Option<LocalizationState>,
}

impl LocalizationState {
    pub(crate) fn print(this: &Option<Self>) -> String {
        match this {
            Some(LocalizationState::UpToDate) => "UpToDate",
            Some(LocalizationState::NotFound) => "NotFound",
            Some(LocalizationState::Outdated) => "Outdated",
            Some(LocalizationState::Unused) => "Unused",
            None => "Unknown",
        }
        .to_owned()
    }
}

impl LocalizationEntryState {
    fn new(key_line: Option<usize>) -> LocalizationEntryState {
        LocalizationEntryState {
            key_line,
            chuck_line_range: None,
            commit_id: None,
            state: None,
        }
    }
}

/// Returns the Git blob associated with the given reference and path
pub(crate) fn read_file_from_path<'a>(
    repo: &'a git2::Repository,
    reference: &git2::Reference,
    path: &Path,
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
        .to_object(repo)
        .unwrap()
        .peel_to_blob()
        .expect("Impossible to fetch the Git object")
}

/// Extend a Fragment with historical git data
/// The actual translation gets dropped
/// TODO: transform vector_map too
pub(crate) fn transform_fragment<'a>(
    repo: &'a git2::Repository,
    fragment: (&Path, RawFragment<String>),
    file_blob: &git2::Blob,
) -> RawFragment<LocalizationEntryState> {
    let (path, fragment) = fragment;
    // Find key start lines by searching all lines which have `:` in them (as they
    // are probably keys) and getting the first part of such line trimming
    // whitespace and quotes. Quite buggy heuristic
    let file_content = std::str::from_utf8(file_blob.content()).expect("Got non UTF-8 file");
    // we only need the key part of the file to process
    let file_content_keys = file_content.lines().enumerate().filter_map(|(no, line)| {
        line.split_once(':').map(|(key, _)| {
            let mut key = key.trim().chars();
            key.next();
            key.next_back();
            (no, key.as_str())
        })
    });
    //speed up the search by sorting all keys!
    let mut file_content_keys_sorted = file_content_keys.into_iter().collect::<Vec<_>>();
    file_content_keys_sorted.sort_by_key(|(_, key)| *key);

    let mut result = RawFragment::<LocalizationEntryState> {
        string_map: HashMap::new(),
        vector_map: HashMap::new(),
    };

    for (original_key, _) in fragment.string_map {
        let line_nb = file_content_keys_sorted
            .binary_search_by_key(&original_key.as_str(), |(_, key)| *key)
            .map_or_else(
                |_| {
                    eprintln!(
                        "Key {} does not have a git line in it's state!",
                        original_key
                    );
                    None
                },
                |id| Some(file_content_keys_sorted[id].0),
            );

        result
            .string_map
            .insert(original_key, LocalizationEntryState::new(line_nb));
    }

    // Find commit for each keys, THIS PART IS SLOW (2s/4s)
    for e in repo
        .blame_file(path, None)
        .expect("Impossible to generate the Git blame")
        .iter()
    {
        for (_, state) in result.string_map.iter_mut() {
            if let Some(line) = state.key_line {
                let range = (
                    e.final_start_line(),
                    e.final_start_line() + e.lines_in_hunk(),
                );
                if line + 1 >= range.0 && line + 1 < range.1 {
                    state.chuck_line_range = Some(range);
                    state.commit_id = state.commit_id.map_or_else(
                        || Some(e.final_commit_id()),
                        |existing_commit| match repo
                            .graph_descendant_of(e.final_commit_id(), existing_commit)
                        {
                            Ok(true) => Some(e.final_commit_id()),
                            Ok(false) => Some(existing_commit),
                            Err(err) => panic!("{}", err),
                        },
                    );
                }
            }
        }
    }

    result
}
