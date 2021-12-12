//! handle the loading of a `Language`
use crate::{
    path::{LangPath, LANG_EXTENSION, LANG_MANIFEST_FILE},
    Fonts, Language, LanguageMetadata,
};
use deunicode::deunicode;
use hashbrown::hash_map::HashMap;
use ron::de::from_reader;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

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
pub(crate) struct RawFragment<T> {
    pub(crate) string_map: HashMap<String, T>,
    pub(crate) vector_map: HashMap<String, Vec<T>>,
}

pub(crate) struct RawLanguage<T> {
    pub(crate) manifest: RawManifest,
    pub(crate) fragments: HashMap</* relative to i18n_path */ PathBuf, RawFragment<T>>,
}

pub(crate) fn load_manifest(path: &LangPath) -> Result<RawManifest, common_assets::BoxedError> {
    let manifest_file = path.file(LANG_MANIFEST_FILE);
    tracing::debug!(?manifest_file, "manifest loading");
    let f = fs::File::open(&manifest_file)?;
    let manifest: RawManifest = from_reader(f)?;
    // verify that the folder name `de_DE` matches the value inside the metadata!
    assert_eq!(
        manifest.metadata.language_identifier,
        path.language_identifier()
    );
    Ok(manifest)
}

pub(crate) fn load_raw_language(
    path: &LangPath,
    manifest: RawManifest,
) -> Result<RawLanguage<String>, common_assets::BoxedError> {
    //get List of files
    let files = path.fragments()?;

    // Walk through each file in the directory
    let mut fragments = HashMap::new();
    for sub_path in files {
        let f = fs::File::open(path.sub_path(&sub_path))?;
        let fragment = from_reader(f)?;
        fragments.insert(sub_path, fragment);
    }

    Ok(RawLanguage {
        manifest,
        fragments,
    })
}

impl From<RawLanguage<String>> for Language {
    fn from(raw: RawLanguage<String>) -> Self {
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
            metadata,
        }
    }
}

impl common_assets::Asset for RawManifest {
    type Loader = common_assets::RonLoader;

    const EXTENSION: &'static str = LANG_EXTENSION;
}

impl common_assets::Asset for RawFragment<String> {
    type Loader = common_assets::RonLoader;

    const EXTENSION: &'static str = LANG_EXTENSION;
}
