use common::{
    assets,
    assets::{load_expect, load_glob, Asset},
};
use deunicode::deunicode;
use ron::de::from_reader;
use serde_derive::*;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
};
use tracing::warn;

/// The reference language, aka the more up-to-date localization data.
/// Also the default language at first startup.
pub const REFERENCE_LANG: &str = "en";

/// How a language can be described
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageMetadata {
    /// A human friendly language name (e.g. "English (US)")
    pub language_name: String,

    /// A short text identifier for this language (e.g. "en_US")
    ///
    /// On the opposite of `language_name` that can change freely,
    /// `language_identifier` value shall be stable in time as it
    /// is used by setting components to store the language
    /// selected by the user.
    pub language_identifier: String,
}

/// Store font metadata
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Font {
    /// Key to retrieve the font in the asset system
    pub asset_key: String,

    /// Scale ratio to resize the UI text dynamicly
    pub scale_ratio: f32,
}

impl Font {
    /// Scale input size to final UI size
    pub fn scale(&self, value: u32) -> u32 { (value as f32 * self.scale_ratio).round() as u32 }
}

/// Store font metadata
pub type VoxygenFonts = HashMap<String, Font>;

/// Store internationalization data
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoxygenLocalization {
    /// A map storing the localized texts
    ///
    /// Localized content can be accessed using a String key.
    pub string_map: HashMap<String, String>,

    /// A map for storing variations of localized texts, for example multiple
    /// ways of saying "Help, I'm under attack". Used primarily for npc
    /// dialogue.
    pub vector_map: HashMap<String, Vec<String>>,

    /// Whether to convert the input text encoded in UTF-8
    /// into a ASCII version by using the `deunicode` crate.
    pub convert_utf8_to_ascii: bool,

    /// Font configuration is stored here
    pub fonts: VoxygenFonts,

    pub metadata: LanguageMetadata,
}

impl VoxygenLocalization {
    /// Get a localized text from the given key
    ///
    /// If the key is not present in the localization object
    /// then the key is returned.
    pub fn get<'a>(&'a self, key: &'a str) -> &str {
        match self.string_map.get(key) {
            Some(localized_text) => localized_text,
            None => key,
        }
    }

    /// Get a variation of localized text from the given key
    ///
    /// `index` should be a random number from `0` to `u16::max()`
    ///
    /// If the key is not present in the localization object
    /// then the key is returned.
    pub fn get_variation<'a>(&'a self, key: &'a str, index: u16) -> &str {
        match self.vector_map.get(key) {
            Some(v) if !v.is_empty() => &v[index as usize % v.len()],
            _ => key,
        }
    }

    /// Return the missing keys compared to the reference language
    pub fn list_missing_entries(&self) -> (HashSet<String>, HashSet<String>) {
        let reference_localization =
            load_expect::<VoxygenLocalization>(i18n_asset_key(REFERENCE_LANG).as_ref());

        let reference_string_keys: HashSet<_> =
            reference_localization.string_map.keys().cloned().collect();
        let string_keys: HashSet<_> = self.string_map.keys().cloned().collect();
        let strings = reference_string_keys
            .difference(&string_keys)
            .cloned()
            .collect();

        let reference_vector_keys: HashSet<_> =
            reference_localization.vector_map.keys().cloned().collect();
        let vector_keys: HashSet<_> = self.vector_map.keys().cloned().collect();
        let vectors = reference_vector_keys
            .difference(&vector_keys)
            .cloned()
            .collect();

        (strings, vectors)
    }

    /// Log missing entries (compared to the reference language) as warnings
    pub fn log_missing_entries(&self) {
        let (missing_strings, missing_vectors) = self.list_missing_entries();
        for missing_key in missing_strings {
            warn!(
                "[{:?}] Missing string key {:?}",
                self.metadata.language_identifier, missing_key
            );
        }
        for missing_key in missing_vectors {
            warn!(
                "[{:?}] Missing vector key {:?}",
                self.metadata.language_identifier, missing_key
            );
        }
    }
}

impl Asset for VoxygenLocalization {
    const ENDINGS: &'static [&'static str] = &["ron"];

    /// Load the translations located in the input buffer and convert them
    /// into a `VoxygenLocalization` object.
    #[allow(clippy::into_iter_on_ref)] // TODO: Pending review in #587
    fn parse(buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        let mut asked_localization: VoxygenLocalization =
            from_reader(buf_reader).map_err(assets::Error::parse_error)?;

        // Update the text if UTF-8 to ASCII conversion is enabled
        if asked_localization.convert_utf8_to_ascii {
            for value in asked_localization.string_map.values_mut() {
                *value = deunicode(value);
            }

            for value in asked_localization.vector_map.values_mut() {
                *value = value.into_iter().map(|s| deunicode(s)).collect();
            }
        }
        asked_localization.metadata.language_name =
            deunicode(&asked_localization.metadata.language_name);

        Ok(asked_localization)
    }
}

/// Load all the available languages located in the Voxygen asset directory
pub fn list_localizations() -> Vec<LanguageMetadata> {
    let voxygen_locales_assets = "voxygen.i18n.*";
    let lang_list = load_glob::<VoxygenLocalization>(voxygen_locales_assets).unwrap();
    lang_list.iter().map(|e| (*e).metadata.clone()).collect()
}

/// Return the asset associated with the language_id
pub fn i18n_asset_key(language_id: &str) -> String { "voxygen.i18n.".to_string() + language_id }

#[cfg(test)]
mod tests {
    use super::VoxygenLocalization;
    use git2::Repository;
    use ron::de::from_bytes;
    use std::{
        collections::{HashMap, HashSet},
        fs,
        path::{Path, PathBuf},
    };

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
        let mut to_process: HashSet<&String> = localization.string_map.keys().collect();
        // Find key start lines
        let file_content = std::str::from_utf8(file_blob.content()).expect("Got non UTF-8 file");
        for (line_nb, line) in file_content.lines().enumerate() {
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

                    if line >= e.final_start_line()
                        && line < e.final_start_line() + e.lines_in_hunk()
                    {
                        state.chuck_line_range = Some((
                            e.final_start_line(),
                            e.final_start_line() + e.lines_in_hunk(),
                        ));
                        state.commit_id = match state.commit_id {
                            Some(existing_commit) => {
                                match repo.graph_descendant_of(e.final_commit_id(), existing_commit)
                                {
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
    #[allow(clippy::expect_fun_call)]
    /// Test to verify all languages and print missing and faulty localisation
    fn test_all_localizations() {
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
        let loc: VoxygenLocalization = from_bytes(i18n_en_blob.content())
            .expect("Expect to parse reference i18n RON file, can't proceed without it");
        let i18n_references: HashMap<String, LocalizationEntryState> =
            generate_key_version(&repo, &loc, &en_i18n_path, &i18n_en_blob);

        // Compare to other reference files
        let i18n_files = i18n_files(&i18n_path);
        let mut i18n_entry_counts: HashMap<PathBuf, (usize, usize, usize, usize)> = HashMap::new();
        for file in &i18n_files {
            let relfile = file.strip_prefix(&root_dir).unwrap();
            if relfile == en_i18n_path {
                continue;
            }
            println!("\n-----------------------------------");
            println!("{:?}", relfile);
            println!("-----------------------------------");

            // Find the localization entry state
            let current_blob = read_file_from_path(&repo, &head_ref, &relfile);
            let current_loc: VoxygenLocalization = match from_bytes(current_blob.content()) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "Could not parse {} RON file, skipping: {}",
                        relfile.to_string_lossy(),
                        e
                    );
                    continue;
                },
            };
            let mut current_i18n =
                generate_key_version(&repo, &current_loc, &relfile, &current_blob);
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
                                    "Commit ID of key {} in reference i18n file is missing! \
                                     Skipping key.",
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

            // Display
            println!(
                "\n{:10}  | {:60}| {:40} | {:40}\n",
                "State",
                "Key name",
                relfile.to_str().unwrap(),
                en_i18n_path.to_str().unwrap()
            );

            let mut sorted_keys: Vec<&String> = current_i18n.keys().collect();
            sorted_keys.sort();

            let current_i18n_entry_count = current_i18n.len();
            let mut uptodate_entries = 0;
            let mut outdated_entries = 0;
            let mut unused_entries = 0;
            let mut notfound_entries = 0;
            let mut unknown_entries = 0;

            for key in sorted_keys {
                let state = current_i18n.get(key).unwrap();
                if state.state != LocalizationState::UpToDate {
                    match state.state {
                        LocalizationState::Outdated => outdated_entries += 1,
                        LocalizationState::NotFound => notfound_entries += 1,
                        LocalizationState::Unknown => unknown_entries += 1,
                        LocalizationState::Unused => unused_entries += 1,
                        LocalizationState::UpToDate => unreachable!(),
                    };

                    println!(
                        "[{:9}] | {:60}| {:40} | {:40}",
                        format!("{:?}", state.state),
                        key,
                        state
                            .commit_id
                            .map(|s| format!("{}", s))
                            .unwrap_or_else(|| "None".to_string()),
                        i18n_references
                            .get(key)
                            .map(|s| s.commit_id)
                            .flatten()
                            .map(|s| format!("{}", s))
                            .unwrap_or_else(|| "None".to_string()),
                    );
                } else {
                    uptodate_entries += 1;
                }
            }

            println!(
                "\n{} up-to-date, {} outdated, {} unused, {} not found, {} unknown entries",
                uptodate_entries,
                outdated_entries,
                unused_entries,
                notfound_entries,
                unknown_entries
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

            i18n_entry_counts.insert(
                file.clone(),
                (
                    uptodate_entries,
                    outdated_entries,
                    notfound_entries + unknown_entries,
                    real_entry_count,
                ),
            );
        }

        let mut overall_uptodate_entry_count = 0;
        let mut overall_outdated_entry_count = 0;
        let mut overall_untranslated_entry_count = 0;
        let mut overall_real_entry_count = 0;

        println!("-----------------------------------------------------------------------------");
        println!("Overall Translation Status");
        println!("-----------------------------------------------------------------------------");
        println!(
            "{:12}| {:8} | {:8} | {:8}",
            "", "up-to-date", "outdated", "untranslated"
        );

        for (path, (uptodate, outdated, untranslated, real)) in i18n_entry_counts {
            overall_uptodate_entry_count += uptodate;
            overall_outdated_entry_count += outdated;
            overall_untranslated_entry_count += untranslated;
            overall_real_entry_count += real;

            println!(
                "{:12}|{:8}    |{:6}    |{:8}",
                path.file_name().unwrap().to_string_lossy(),
                uptodate,
                outdated,
                untranslated
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
}
