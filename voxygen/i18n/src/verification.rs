use std::path::Path;

use crate::{i18n_directories, raw, LANG_MANIFEST_FILE, REFERENCE_LANG};

/// Test to verify all languages that they are VALID and loadable, without
/// need of git just on the local assets folder
/// `root_path` - absolute path to main repo
/// `relative_i18n_root_path` - relative path to asset directory (right now it
/// is 'assets/voxygen/i18n')
pub fn verify_all_localizations(root_path: &Path, relative_i18n_root_path: &Path) {
    let i18n_root_path = root_path.join(relative_i18n_root_path);
    let ref_i18n_path = i18n_root_path.join(REFERENCE_LANG);
    let ref_i18n_manifest_path =
        ref_i18n_path.join(LANG_MANIFEST_FILE.to_string() + "." + crate::LANG_EXTENSION);
    assert!(
        root_path.join(&ref_i18n_path).is_dir(),
        "Reference language folder doesn't exist, something is wrong!"
    );
    assert!(
        root_path.join(&ref_i18n_manifest_path).is_file(),
        "Reference language manifest file doesn't exist, something is wrong!"
    );
    let i18n_directories = i18n_directories(&i18n_root_path);
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
        let display_language_identifier = i18n_directory
            .strip_prefix(&root_path)
            .unwrap()
            .to_str()
            .unwrap();
        let language_identifier = i18n_directory
            .strip_prefix(&i18n_root_path)
            .unwrap()
            .to_str()
            .unwrap();
        println!("verifying {:?}", display_language_identifier);
        // Walk through each files and try to load them
        verify_localization_directory(root_path, relative_i18n_root_path, language_identifier);
    }
}

fn verify_localization_directory(
    root_path: &Path,
    relative_i18n_root_path: &Path,
    language_identifier: &str,
) {
    let i18n_path = root_path.join(relative_i18n_root_path);
    let manifest =
        raw::load_manifest(&i18n_path, language_identifier).expect("error accessing manifest file");
    raw::load_raw_language(&i18n_path, manifest).expect("error accessing fragment file");
}
