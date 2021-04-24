use common::assets::{self, AssetExt, AssetGuard, AssetHandle};
use deunicode::deunicode;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use tracing::warn;

/// The reference language, aka the more up-to-date localization data.
/// Also the default language at first startup.
pub const REFERENCE_LANG: &str = "en";

pub const LANG_MANIFEST_FILE: &str = "_manifest";

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
pub type Fonts = HashMap<String, Font>;

/// Raw localization data, expect the strings to not be loaded here
/// However, metadata informations are correct
/// See `Language` for more info on each attributes
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RawLocalization {
    pub sub_directories: Vec<String>,
    pub string_map: HashMap<String, String>,
    pub vector_map: HashMap<String, Vec<String>>,
    pub convert_utf8_to_ascii: bool,
    pub fonts: Fonts,
    pub metadata: LanguageMetadata,
}

/// Store internationalization data
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Language {
    /// A list of subdirectories to lookup for localization files
    pub sub_directories: Vec<String>,

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
    pub fonts: Fonts,

    pub metadata: LanguageMetadata,
}

/// Store internationalization maps
/// These structs are meant to be merged into a Language
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalizationFragment {
    /// A map storing the localized texts
    ///
    /// Localized content can be accessed using a String key.
    pub string_map: HashMap<String, String>,

    /// A map for storing variations of localized texts, for example multiple
    /// ways of saying "Help, I'm under attack". Used primarily for npc
    /// dialogue.
    pub vector_map: HashMap<String, Vec<String>>,
}

impl Language {
    /// Get a localized text from the given key
    ///
    /// If the key is not present in the localization object
    /// then the key is returned.
    pub fn get<'a>(&'a self, key: &'a str) -> Option<&str> {
        self.string_map.get(key).map(|s| s.as_str())
    }

    /// Get a variation of localized text from the given key
    ///
    /// `index` should be a random number from `0` to `u16::max()`
    ///
    /// If the key is not present in the localization object
    /// then the key is returned.
    pub fn get_variation<'a>(&'a self, key: &'a str, index: u16) -> Option<&str> {
        self.vector_map
            .get(key)
            .map(|v| {
                if !v.is_empty() {
                    Some(v[index as usize % v.len()].as_str())
                } else {
                    None
                }
            })
            .flatten()
    }
}

impl Default for Language {
    fn default() -> Self {
        Self {
            sub_directories: Vec::default(),
            string_map: HashMap::default(),
            vector_map: HashMap::default(),
            ..Default::default()
        }
    }
}

impl From<RawLocalization> for Language {
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

impl assets::Asset for RawLocalization {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}
impl assets::Asset for LocalizationFragment {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl assets::Compound for Language {
    fn load<S: assets::source::Source>(
        cache: &assets::AssetCache<S>,
        asset_key: &str,
    ) -> Result<Self, assets::Error> {
        let raw = cache
            .load::<RawLocalization>(&[asset_key, ".", LANG_MANIFEST_FILE].concat())?
            .cloned();
        let mut localization = Language::from(raw);

        // Walk through files in the folder, collecting localization fragment to merge
        // inside the asked_localization
        for localization_asset in cache.load_dir::<LocalizationFragment>(asset_key)?.iter() {
            localization
                .string_map
                .extend(localization_asset.read().string_map.clone());
            localization
                .vector_map
                .extend(localization_asset.read().vector_map.clone());
        }

        // Use the localization's subdirectory list to load fragments from there
        for sub_directory in localization.sub_directories.iter() {
            for localization_asset in cache
                .load_dir::<LocalizationFragment>(&[asset_key, ".", sub_directory].concat())?
                .iter()
            {
                localization
                    .string_map
                    .extend(localization_asset.read().string_map.clone());
                localization
                    .vector_map
                    .extend(localization_asset.read().vector_map.clone());
            }
        }

        // Update the text if UTF-8 to ASCII conversion is enabled
        if localization.convert_utf8_to_ascii {
            for value in localization.string_map.values_mut() {
                *value = deunicode(value);
            }

            for value in localization.vector_map.values_mut() {
                *value = value.iter().map(|s| deunicode(s)).collect();
            }
        }
        localization.metadata.language_name = deunicode(&localization.metadata.language_name);

        Ok(localization)
    }
}

/// the central data structure to handle localization in veloren
// inherit Copy+Clone from AssetHandle
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct LocalizationHandle {
    active: AssetHandle<Language>,
    fallback: Option<AssetHandle<Language>>,
    pub use_english_fallback: bool,
}

// RAII guard returned from Localization::read(), resembles AssetGuard
pub struct LocalizationGuard {
    active: AssetGuard<Language>,
    fallback: Option<AssetGuard<Language>>,
}

// arbitrary choice to minimize changing all of veloren
pub type Localization = LocalizationGuard;

impl LocalizationGuard {
    /// Get a localized text from the given key
    ///
    /// If the key is not present in the localization object
    /// then the key is returned.
    pub fn get<'a>(&'a self, key: &'a str) -> &str {
        self.active.get(key).unwrap_or_else(|| {
            self.fallback
                .as_ref()
                .map(|f| f.get(key))
                .flatten()
                .unwrap_or(key)
        })
    }

    /// Get a variation of localized text from the given key
    ///
    /// `index` should be a random number from `0` to `u16::max()`
    ///
    /// If the key is not present in the localization object
    /// then the key is returned.
    pub fn get_variation<'a>(&'a self, key: &'a str, index: u16) -> &str {
        self.active.get_variation(key, index).unwrap_or_else(|| {
            self.fallback
                .as_ref()
                .map(|f| f.get_variation(key, index))
                .flatten()
                .unwrap_or(key)
        })
    }

    /// Return the missing keys compared to the reference language
    fn list_missing_entries(&self) -> (HashSet<String>, HashSet<String>) {
        if let Some(ref_lang) = &self.fallback {
            let reference_string_keys: HashSet<_> = ref_lang.string_map.keys().cloned().collect();
            let string_keys: HashSet<_> = self.active.string_map.keys().cloned().collect();
            let strings = reference_string_keys
                .difference(&string_keys)
                .cloned()
                .collect();

            let reference_vector_keys: HashSet<_> = ref_lang.vector_map.keys().cloned().collect();
            let vector_keys: HashSet<_> = self.active.vector_map.keys().cloned().collect();
            let vectors = reference_vector_keys
                .difference(&vector_keys)
                .cloned()
                .collect();

            (strings, vectors)
        } else {
            (HashSet::default(), HashSet::default())
        }
    }

    /// Log missing entries (compared to the reference language) as warnings
    pub fn log_missing_entries(&self) {
        let (missing_strings, missing_vectors) = self.list_missing_entries();
        for missing_key in missing_strings {
            warn!(
                "[{:?}] Missing string key {:?}",
                self.metadata().language_identifier,
                missing_key
            );
        }
        for missing_key in missing_vectors {
            warn!(
                "[{:?}] Missing vector key {:?}",
                self.metadata().language_identifier,
                missing_key
            );
        }
    }

    pub fn fonts(&self) -> &Fonts { &self.active.fonts }

    pub fn metadata(&self) -> &LanguageMetadata { &self.active.metadata }
}

impl LocalizationHandle {
    pub fn set_english_fallback(&mut self, use_english_fallback: bool) {
        self.use_english_fallback = use_english_fallback;
    }

    pub fn read(&self) -> LocalizationGuard {
        LocalizationGuard {
            active: self.active.read(),
            fallback: if self.use_english_fallback {
                self.fallback.map(|f| f.read())
            } else {
                None
            },
        }
    }

    pub fn load(specifier: &str) -> Result<Self, common::assets::Error> {
        let default_key = i18n_asset_key(REFERENCE_LANG);
        let is_default = specifier == default_key;
        Ok(Self {
            active: Language::load(specifier)?,
            fallback: if is_default {
                None
            } else {
                Language::load(&default_key).ok()
            },
            use_english_fallback: false,
        })
    }

    pub fn load_expect(specifier: &str) -> Self {
        Self::load(specifier).expect("Can't load language files")
    }

    pub fn reloaded(&mut self) -> bool { self.active.reloaded() }
}

#[derive(Clone, Debug)]
struct LocalizationList(Vec<LanguageMetadata>);

impl assets::Compound for LocalizationList {
    fn load<S: assets::source::Source>(
        cache: &assets::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, assets::Error> {
        // List language directories
        let mut languages = vec![];

        let i18n_root = assets::path_of(specifier, "");
        for i18n_entry in (std::fs::read_dir(&i18n_root)?).flatten() {
            if let Some(i18n_key) = i18n_entry.file_name().to_str() {
                // load the root file of all the subdirectories
                if let Ok(localization) = cache.load::<RawLocalization>(
                    &[specifier, ".", i18n_key, ".", LANG_MANIFEST_FILE].concat(),
                ) {
                    languages.push(localization.read().metadata.clone());
                }
            }
        }

        Ok(LocalizationList(languages))
    }
}

/// Load all the available languages located in the voxygen asset directory
pub fn list_localizations() -> Vec<LanguageMetadata> {
    LocalizationList::load_expect_cloned("voxygen.i18n").0
}

/// Return the asset associated with the language_id
pub fn i18n_asset_key(language_id: &str) -> String { ["voxygen.i18n.", language_id].concat() }

#[cfg(test)]
mod tests {
    use super::{LocalizationFragment, RawLocalization, LANG_MANIFEST_FILE, REFERENCE_LANG};
    use git2::Repository;
    use hashbrown::{HashMap, HashSet};
    use ron::de::{from_bytes, from_reader};
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    /// List localization directories as a PathBuf vector
    fn i18n_directories(i18n_dir: &Path) -> Vec<PathBuf> {
        fs::read_dir(i18n_dir)
            .unwrap()
            .map(|res| res.map(|e| e.path()).unwrap())
            .filter(|e| e.is_dir())
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
                                match repo.graph_descendant_of(e.final_commit_id(), existing_commit)
                                {
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
        dir: &Path,
    ) {
        let root_dir = std::env::current_dir()
            .map(|p| p.parent().expect("").to_owned())
            .unwrap();
        //TODO: review unwraps in this file
        for i18n_file in root_dir.join(&dir).read_dir().unwrap().flatten() {
            if let Ok(file_type) = i18n_file.file_type() {
                if file_type.is_file() {
                    let full_path = i18n_file.path();
                    let path = full_path.strip_prefix(&root_dir).unwrap();
                    println!("-> {:?}", i18n_file.file_name());
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

    fn verify_localization_directory(directory_path: &Path) {
        let root_dir = std::env::current_dir()
            .map(|p| p.parent().expect("").to_owned())
            .unwrap();
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
    #[test]
    fn verify_all_localizations() {
        // Generate paths
        let i18n_asset_path = Path::new("assets/voxygen/i18n/");
        let ref_i18n_dir_path = i18n_asset_path.join(REFERENCE_LANG);
        let ref_i18n_path = ref_i18n_dir_path.join(LANG_MANIFEST_FILE.to_string() + ".ron");
        let root_dir = std::env::current_dir()
            .map(|p| p.parent().expect("").to_owned())
            .unwrap();
        assert!(
            root_dir.join(&ref_i18n_dir_path).is_dir(),
            "Reference language folder doesn't exist, something is wrong!"
        );
        assert!(
            root_dir.join(&ref_i18n_path).is_file(),
            "Reference language manifest file doesn't exist, something is wrong!"
        );
        let i18n_directories = i18n_directories(&root_dir.join(i18n_asset_path));
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
            verify_localization_directory(&i18n_directory);
            // Walk through each subdirectories and try to load files in them
            for sub_directory in raw_localization.sub_directories.iter() {
                let subdir_path = &i18n_directory.join(sub_directory);
                verify_localization_directory(&subdir_path);
            }
        }
    }

    // Test to verify all languages and print missing and faulty localisation
    #[test]
    #[ignore]
    #[allow(clippy::expect_fun_call)]
    fn test_all_localizations() {
        // Generate paths
        let i18n_asset_path = Path::new("assets/voxygen/i18n/");
        let ref_i18n_dir_path = i18n_asset_path.join(REFERENCE_LANG);
        let ref_i18n_path = ref_i18n_dir_path.join(LANG_MANIFEST_FILE.to_string() + ".ron");
        let root_dir = std::env::current_dir()
            .map(|p| p.parent().expect("").to_owned())
            .unwrap();
        let i18n_path = root_dir.join(i18n_asset_path);

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
        let repo = Repository::discover(&root_dir).expect(&format!(
            "Failed to open the Git repository at {:?}",
            &root_dir
        ));
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

        // read HEAD for the fragment files
        complete_key_versions(&repo, &head_ref, &mut i18n_references, &ref_i18n_dir_path);
        // read HEAD for the subfolders
        for sub_directory in loc.sub_directories.iter() {
            let subdir_path = &ref_i18n_dir_path.join(sub_directory);
            complete_key_versions(&repo, &head_ref, &mut i18n_references, &subdir_path);
        }

        // Compare to other reference files
        let i18n_directories = i18n_directories(&i18n_path);
        let mut i18n_entry_counts: HashMap<PathBuf, (usize, usize, usize, usize)> = HashMap::new();
        for file in &i18n_directories {
            let reldir = file.strip_prefix(&root_dir).unwrap();
            let relfile = reldir.join(&(LANG_MANIFEST_FILE.to_string() + ".ron"));
            if relfile == ref_i18n_path {
                continue;
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
                    continue;
                },
            };
            let mut current_i18n = generate_key_version(
                &repo,
                &LocalizationFragment::from(current_loc.clone()),
                &relfile,
                &current_blob,
            );
            // read HEAD for the fragment files
            complete_key_versions(&repo, &head_ref, &mut current_i18n, &reldir);
            // read HEAD for the subfolders
            for sub_directory in current_loc.sub_directories.iter() {
                let subdir_path = &reldir.join(sub_directory);
                complete_key_versions(&repo, &head_ref, &mut current_i18n, &subdir_path);
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
                ref_i18n_path.to_str().unwrap()
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
