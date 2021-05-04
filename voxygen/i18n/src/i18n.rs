use crate::assets::{self, AssetExt, AssetGuard, AssetHandle};
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
    /// First lookup is done in the active language, second in
    /// the fallback (if present).
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

    pub fn load(specifier: &str) -> Result<Self, crate::assets::Error> {
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
    use crate::analysis;
    use std::path::Path;

    // Test to verify all languages that they are VALID and loadable, without
    // need of git just on the local assets folder
    #[test]
    fn verify_all_localizations() {
        // Generate paths
        let i18n_asset_path = Path::new("assets/voxygen/i18n/");
        let curr_dir = std::env::current_dir().unwrap();
        let root_dir = curr_dir.parent().unwrap().parent().unwrap();
        analysis::verify_all_localizations(&root_dir, &i18n_asset_path);
    }

    // Test to verify all languages and print missing and faulty localisation
    #[test]
    #[ignore]
    fn test_all_localizations() {
        // Generate paths
        let i18n_asset_path = Path::new("assets/voxygen/i18n/");
        let curr_dir = std::env::current_dir().unwrap();
        let root_dir = curr_dir.parent().unwrap().parent().unwrap();
        analysis::test_all_localizations(&root_dir, &i18n_asset_path);
    }
}
