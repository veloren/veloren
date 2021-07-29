use crate::path::{BasePath, LangPath, LANG_MANIFEST_FILE};

use crate::{raw, REFERENCE_LANG};

/// Test to verify all languages that they are VALID and loadable, without
/// need of git just on the local assets folder
pub fn verify_all_localizations(path: &BasePath) {
    let ref_i18n_path = path.i18n_path(REFERENCE_LANG);
    let ref_i18n_manifest_path = ref_i18n_path.file(LANG_MANIFEST_FILE);
    assert!(
        ref_i18n_manifest_path.is_file(),
        "Reference language manifest file doesn't exist, something is wrong!"
    );
    let i18n_directories = path.i18n_directories();
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
        println!("verifying {:?}", i18n_directory);
        // Walk through each files and try to load them
        verify_localization_directory(&i18n_directory);
    }
}

fn verify_localization_directory(path: &LangPath) {
    let manifest = raw::load_manifest(path).expect("error accessing manifest file");
    raw::load_raw_language(path, manifest).expect("error accessing fragment file");
}
