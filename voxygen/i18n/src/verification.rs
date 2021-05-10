use ron::de::from_reader;
use std::{fs, path::Path};

use crate::data::{
    i18n_directories, LocalizationFragment, RawLocalization, LANG_MANIFEST_FILE, REFERENCE_LANG,
};

fn verify_localization_directory(root_dir: &Path, directory_path: &Path) {
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
                let _loc: LocalizationFragment = match from_reader(f) {
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

/// Test to verify all languages that they are VALID and loadable, without
/// need of git just on the local assets folder
/// `root_dir` - absolute path to main repo
/// `asset_path` - relative path to asset directory (right now it is
/// 'assets/voxygen/i18n')
pub fn verify_all_localizations(root_dir: &Path, asset_path: &Path) {
    let ref_i18n_dir_path = asset_path.join(REFERENCE_LANG);
    let ref_i18n_path = ref_i18n_dir_path.join(LANG_MANIFEST_FILE.to_string() + ".ron");
    assert!(
        root_dir.join(&ref_i18n_dir_path).is_dir(),
        "Reference language folder doesn't exist, something is wrong!"
    );
    assert!(
        root_dir.join(&ref_i18n_path).is_file(),
        "Reference language manifest file doesn't exist, something is wrong!"
    );
    let i18n_directories = i18n_directories(&root_dir.join(asset_path));
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
        verify_localization_directory(root_dir, &i18n_directory);
        // Walk through each subdirectories and try to load files in them
        for sub_directory in raw_localization.sub_directories.iter() {
            let subdir_path = &i18n_directory.join(sub_directory);
            verify_localization_directory(root_dir, &subdir_path);
        }
    }
}
