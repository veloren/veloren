use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tracing::{error, warn};

pub trait EditableSetting: Serialize + DeserializeOwned + Default {
    const FILENAME: &'static str;

    fn load(data_dir: &Path) -> Self {
        let path = Self::get_path(data_dir);

        if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader(file) {
                Ok(setting) => setting,
                Err(e) => {
                    warn!(
                        ?e,
                        "Failed to parse setting file! Falling back to default and moving \
                         existing file to a .invalid"
                    );

                    // Rename existing file to .invalid.ron
                    let mut new_path = path.with_extension("invalid.ron");

                    // If invalid path already exists append number
                    for i in 1.. {
                        if !new_path.exists() {
                            break;
                        }

                        warn!(
                            ?new_path,
                            "Path to move invalid settings exists, appending number"
                        );
                        new_path = path.with_extension(format!("invalid{}.ron", i));
                    }

                    warn!("Renaming invalid settings file to: {}", new_path.display());
                    if let Err(e) = fs::rename(&path, &new_path) {
                        warn!(?e, ?path, ?new_path, "Failed to rename settings file.");
                    }

                    create_and_save_default(&path)
                },
            }
        } else {
            create_and_save_default(&path)
        }
    }

    fn edit<R>(&mut self, data_dir: &Path, f: impl FnOnce(&mut Self) -> R) -> R {
        let path = Self::get_path(data_dir);

        let r = f(self);
        save_to_file(&*self, &path)
            .unwrap_or_else(|err| warn!("Failed to save setting: {:?}", err));
        r
    }

    fn get_path(data_dir: &Path) -> PathBuf {
        let mut path = super::with_config_dir(data_dir);
        path.push(Self::FILENAME);
        path
    }
}

fn save_to_file<S: Serialize>(setting: &S, path: &Path) -> std::io::Result<()> {
    // Create dir if it doesn't exist
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let ron = ron::ser::to_string_pretty(setting, ron::ser::PrettyConfig::default())
        .expect("Failed serialize setting.");

    fs::write(path, ron.as_bytes())?;

    Ok(())
}

fn create_and_save_default<S: EditableSetting>(path: &Path) -> S {
    let default = S::default();
    if let Err(e) = save_to_file(&default, path) {
        error!(?e, "Failed to create default setting file!");
    }
    default
}
