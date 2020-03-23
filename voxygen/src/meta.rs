use common::comp;
use directories::ProjectDirs;
use log::warn;
use serde_derive::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct CharacterData {
    pub name: String,
    pub body: comp::Body,
    pub tool: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
#[repr(C)]
pub struct Meta {
    pub characters: Vec<CharacterData>,
    pub selected_character: usize,
}

impl Meta {
    pub fn delete_character(&mut self, index: usize) {
        self.characters.remove(index);
        if index < self.selected_character {
            self.selected_character -= 1;
        }
    }

    pub fn add_character(&mut self, data: CharacterData) -> usize {
        self.characters.push(data);
        // return new character's index
        self.characters.len() - 1
    }

    pub fn load() -> Self {
        let path = Self::get_meta_path();

        if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader(file) {
                Ok(s) => return s,
                Err(e) => {
                    log::warn!("Failed to parse meta file! Fallback to default. {}", e);
                    // Rename the corrupted settings file
                    let mut new_path = path.to_owned();
                    new_path.pop();
                    new_path.push("meta.invalid.ron");
                    if let Err(err) = std::fs::rename(path, new_path) {
                        log::warn!("Failed to rename meta file. {}", err);
                    }
                },
            }
        }
        // This is reached if either:
        // - The file can't be opened (presumably it doesn't exist)
        // - Or there was an error parsing the file
        let default = Self::default();
        default.save_to_file_warn();
        default
    }

    pub fn save_to_file_warn(&self) {
        if let Err(err) = self.save_to_file() {
            warn!("Failed to save settings: {:?}", err);
        }
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = Self::get_meta_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut meta_file = fs::File::create(path)?;

        let s: &str = &ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        meta_file.write_all(s.as_bytes()).unwrap();
        Ok(())
    }

    pub fn get_meta_path() -> PathBuf {
        if let Some(val) = std::env::var_os("VOXYGEN_CONFIG") {
            let meta = PathBuf::from(val).join("meta.ron");
            if meta.exists() || meta.parent().map(|x| x.exists()).unwrap_or(false) {
                return meta;
            }
            log::warn!("VOXYGEN_CONFIG points to invalid path.");
        }

        let proj_dirs = ProjectDirs::from("net", "veloren", "voxygen")
            .expect("System's $HOME directory path not found!");
        proj_dirs.config_dir().join("meta").with_extension("ron")
    }
}
