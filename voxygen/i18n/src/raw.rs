//! handle the loading of a `Language`
use hashbrown::hash_map::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;
use ron::de::from_reader;
use deunicode::deunicode;
use crate::{Fonts, LanguageMetadata, LANG_MANIFEST_FILE, LANG_EXTENSION};
use crate::Language;

/// Raw localization metadata from LANG_MANIFEST_FILE file
/// See `Language` for more info on each attributes
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub(crate) struct RawManifest {
    pub(crate) convert_utf8_to_ascii: bool,
    pub(crate) fonts: Fonts,
    pub(crate) metadata: LanguageMetadata,
}

/// Raw localization data from one specific file
/// These structs are meant to be merged into a Language
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub(crate) struct RawFragment {
    pub(crate) string_map: HashMap<String, String>,
    pub(crate) vector_map: HashMap<String, Vec<String>>,
}

pub(crate) struct RawLanguage {
    pub(crate) manifest: RawManifest,
    pub(crate) fragments: HashMap<PathBuf, RawFragment>,
}

#[derive(Debug)]
pub(crate) enum RawError {
    RonError(ron::Error),
}

/// `i18n_root_path` - absolute path to i18n path which contains `en`, `de_DE`, `fr_FR` folders
pub(crate) fn load_manifest(i18n_root_path: &Path, language_identifier: &str) -> Result<RawManifest, common_assets::Error> {
    let manifest_file = i18n_root_path.join(language_identifier).join(format!("{}.{}", LANG_MANIFEST_FILE, LANG_EXTENSION));
    println!("file , {:?}", manifest_file);
    let f = fs::File::open(&manifest_file)?;
    Ok(from_reader(f).map_err(RawError::RonError)?)
}

/// `i18n_root_path` - absolute path to i18n path which contains `en`, `de_DE`, `fr_FR` files
pub(crate) fn load_raw_language(i18n_root_path: &Path, manifest: RawManifest) -> Result<RawLanguage, common_assets::Error> {
    // Walk through each file in the directory
    let mut fragments = HashMap::new();
    let language_identifier = &manifest.metadata.language_identifier;
    let language_dir = i18n_root_path.join(language_identifier);
    for fragment_file in language_dir.read_dir().unwrap().flatten() {
        let file_type = fragment_file.file_type()?;
        if file_type.is_dir() {
            // TODO: recursive
            continue;
        }
        if file_type.is_file() {
            let full_path = fragment_file.path();
            let relative_path = full_path.strip_prefix(&i18n_root_path).unwrap();
            let f = fs::File::open(&full_path)?;
            let fragment = from_reader(f).map_err(RawError::RonError)?;
            fragments.insert(relative_path.to_path_buf(), fragment);
        }
    }
    Ok(RawLanguage{
        manifest,
        fragments,
    })
}

impl From<RawLanguage> for Language {
    fn from(raw: RawLanguage) -> Self {

        let mut string_map = HashMap::new();
        let mut vector_map = HashMap::new();

        for (_, fragment) in raw.fragments {
            string_map.extend(fragment.string_map);
            vector_map.extend(fragment.vector_map);
        }

        let convert_utf8_to_ascii = raw.manifest.convert_utf8_to_ascii;

        // Update the text if UTF-8 to ASCII conversion is enabled
        if convert_utf8_to_ascii {
            for value in string_map.values_mut() {
                *value = deunicode(value);
            }

            for value in vector_map.values_mut() {
                *value = value.iter().map(|s| deunicode(s)).collect();
            }
        }
        let mut metadata = raw.manifest.metadata;
        metadata.language_name = deunicode(&metadata.language_name);

        Self {
            string_map,
            vector_map,
            convert_utf8_to_ascii,
            fonts: raw.manifest.fonts,
            metadata: metadata,
        }
    }
}

impl core::fmt::Display for RawError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RawError::RonError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for RawError {}


impl From<RawError> for common_assets::Error {
    fn from(e: RawError) -> Self {
        Self::Conversion(Box::new(e))
    }
}


impl common_assets::Asset for RawManifest {
    type Loader = common_assets::RonLoader;

    const EXTENSION: &'static str = LANG_EXTENSION;
}

impl common_assets::Asset for RawFragment {
    type Loader = common_assets::RonLoader;

    const EXTENSION: &'static str = LANG_EXTENSION;
}