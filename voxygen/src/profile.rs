use crate::hud;
use directories::ProjectDirs;
use hashbrown::HashMap;
use log::warn;
use serde_derive::{Deserialize, Serialize};
use std::{fs, io::prelude::*, path::PathBuf};

/// Represents a character in the profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CharacterProfile {
    /// Array representing a character's hotbar.
    pub hotbar_slots: [Option<hud::HotbarSlotContents>; 10],
}

impl Default for CharacterProfile {
    fn default() -> Self {
        CharacterProfile {
            hotbar_slots: [None; 10],
        }
    }
}

/// Represents a server in the profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerProfile {
    /// A map of character's by id to their CharacterProfile.
    pub characters: HashMap<i32, CharacterProfile>,
}

impl Default for ServerProfile {
    fn default() -> Self {
        ServerProfile {
            characters: HashMap::new(),
        }
    }
}

/// `Profile` contains everything that can be configured in the profile.ron
///
/// Initially it is just for persisting things that don't belong in
/// setttings.ron - like the state of hotbar and any other character level
/// configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub servers: HashMap<String, ServerProfile>,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            servers: HashMap::new(),
        }
    }
}

impl Profile {
    /// Load the profile.ron file from the standard path or create it.
    pub fn load() -> Self {
        let path = Profile::get_path();

        if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader(file) {
                Ok(profile) => return profile,
                Err(e) => {
                    log::warn!(
                        "Failed to parse profile file! Falling back to default. {}",
                        e
                    );
                    // Rename the corrupted profile file.
                    let new_path = path.with_extension("invalid.ron");
                    if let Err(err) = std::fs::rename(path, new_path) {
                        log::warn!("Failed to rename profile file. {}", err);
                    }
                },
            }
        }
        // This is reached if either:
        // - The file can't be opened (presumably it doesn't exist)
        // - Or there was an error parsing the file
        let default_profile = Self::default();
        default_profile.save_to_file_warn();
        default_profile
    }

    /// Save the current profile to disk, warn on failure.
    pub fn save_to_file_warn(&self) {
        if let Err(err) = self.save_to_file() {
            warn!("Failed to save profile: {:?}", err);
        }
    }

    /// Get the hotbar_slots for the requested character_id.
    ///
    /// if the server or character does not exist then the appropriate fields
    /// will be initialised and default hotbar_slots (empty) returned.
    ///
    /// # Arguments
    ///
    /// * server - current server the character is on.
    /// * character_id - id of the character.
    pub fn get_hotbar_slots(
        &mut self,
        server: &str,
        character_id: i32,
    ) -> [Option<hud::HotbarSlotContents>; 10] {
        self.servers
            .entry(server.to_string())
            .or_insert(ServerProfile::default())
            // Get or update the CharacterProfile.
            .characters
            .entry(character_id)
            .or_insert(CharacterProfile::default())
            .hotbar_slots
    }

    /// Set the hotbar_slots for the requested character_id.
    ///
    /// If the server or character does not exist then the appropriate fields
    /// will be initialised and the slots added.
    ///
    /// # Arguments
    ///
    /// * server - current server the character is on.
    /// * character_id - id of the character.
    /// * slots - array of hotbar_slots to save.
    pub fn set_hotbar_slots(
        &mut self,
        server: &str,
        character_id: i32,
        slots: [Option<hud::HotbarSlotContents>; 10],
    ) {
        self.servers
            .entry(server.to_string())
            .or_insert(ServerProfile::default())
            // Get or update the CharacterProfile.
            .characters
            .entry(character_id)
            .or_insert(CharacterProfile::default())
            .hotbar_slots = slots;
    }

    /// Save the current profile to disk.
    fn save_to_file(&self) -> std::io::Result<()> {
        let path = Profile::get_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut config_file = fs::File::create(path)?;

        let s: &str = &ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).unwrap();
        config_file.write_all(s.as_bytes()).unwrap();
        Ok(())
    }

    fn get_path() -> PathBuf {
        if let Some(val) = std::env::var_os("VOXYGEN_CONFIG") {
            let profile = PathBuf::from(val).join("profile.ron");
            if profile.exists() || profile.parent().map(|x| x.exists()).unwrap_or(false) {
                return profile;
            }
            log::warn!("VOXYGEN_CONFIG points to invalid path.");
        }

        let proj_dirs = ProjectDirs::from("net", "veloren", "voxygen")
            .expect("System's $HOME directory path not found!");

        proj_dirs
            .config_dir()
            .join("profile.ron")
            .with_extension("ron")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_slots_with_empty_profile() {
        let mut profile = Profile::default();
        let slots = profile.get_hotbar_slots("TestServer", 12345);
        assert_eq!(slots, [None; 10])
    }

    #[test]
    fn test_set_slots_with_empty_profile() {
        let mut profile = Profile::default();
        let slots = [None; 10];
        profile.set_hotbar_slots("TestServer", 12345, slots);
    }
}
