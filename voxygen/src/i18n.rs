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
            log::warn!(
                "[{:?}] Missing string key {:?}",
                self.metadata.language_identifier,
                missing_key
            );
        }
        for missing_key in missing_vectors {
            log::warn!(
                "[{:?}] Missing vector key {:?}",
                self.metadata.language_identifier,
                missing_key
            );
        }
    }
}

impl Asset for VoxygenLocalization {
    const ENDINGS: &'static [&'static str] = &["ron"];

    /// Load the translations located in the input buffer and convert them
    /// into a `VoxygenLocalization` object.
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
