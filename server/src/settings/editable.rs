use atomicwrites::{AtomicFile, Error as AtomicError, OverwriteBehavior};
use core::{convert::TryInto, fmt};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs,
    io::{Seek, Write},
    path::{Path, PathBuf},
};
use tracing::{error, info, warn};

#[derive(Debug)]
/// Errors that can occur during edits to a settings file.
pub enum Error<S: EditableSetting> {
    /// An error occurred validating the settings file.
    Integrity(S::Error),
    /// An IO error occurred when writing to the settings file.
    Io(std::io::Error),
}

#[derive(Debug)]
/// Same as Error, but carries the validated settings in the Io case.
enum ErrorInternal<S: EditableSetting> {
    Integrity(S::Error),
    Io(std::io::Error, S),
}

pub enum Version {
    /// This was an old version of the settings file, so overwrite with the
    /// modern config.
    Old,
    /// Latest version of the settings file.
    Latest,
}

pub trait EditableSetting: Clone + Default {
    const FILENAME: &'static str;

    /// Please use this error sparingly, since we ideally want to preserve
    /// forwards compatibility for all migrations.  In particular, this
    /// error should be used to fail validation *of the original settings
    /// file* that cannot be caught with ordinary parsing, rather than used
    /// to signal errors that occurred during migrations.
    ///
    /// The best error type is Infallible.
    type Error: fmt::Debug;

    /// Into<Setting> is expected to migrate directly to the latest version,
    /// which can be implemented using "chaining".  The use of `Into` here
    /// rather than TryInto is intended (together with the expected use of
    /// chaining) to prevent migrations from invalidating old save files
    /// without warning; there should always be a non-failing migration path
    /// from the oldest to latest format (if the migration path fails, we can
    /// panic).
    type Legacy: Serialize + DeserializeOwned + Into<Self>;

    /// TryInto<(Version, Self)> is expected to migrate to the latest version
    /// from any older version, using "chaining" (see [super::banlist] for
    /// examples).
    ///
    /// From<Self> is intended to construct the latest version of the
    /// configuratino file from Self, which we use to save the config file
    /// on migration or modification.  Note that it should always be the
    /// case that if x is constructed from any of Self::clone, Self::default, or
    /// Setting::try_into, then Setting::try_from(Self::into(x)).is_ok() must be
    /// true!
    ///
    /// The error should be used to fail validation *of the original settings
    /// file* that cannot be caught with parsing.  If we can possibly avoid
    /// it, we should not create errors in valid settings files during
    /// migration, to ensure forwards compatibility.
    type Setting: Serialize
        + DeserializeOwned
        + TryInto<(Version, Self), Error = Self::Error>
        + From<Self>;

    fn load(data_dir: &Path) -> Self {
        let path = Self::get_path(data_dir);

        if let Ok(mut file) = fs::File::open(&path) {
            match ron::de::from_reader(&mut file)
                .map(|setting: Self::Setting| setting.try_into())
                .or_else(|orig_err| {
                    file.rewind()?;
                    ron::de::from_reader(file)
                         .map(|legacy| Ok((Version::Old, Self::Legacy::into(legacy))))
                         // When both legacy and non-legacy have parse errors, prioritize the
                         // non-legacy one, since we can't tell which one is "right" and legacy
                         // formats are simple, early, and uncommon enough that we expect
                         // few parse errors in those.
                         .or(Err(orig_err))
                })
                .map_err(|e| {
                    warn!(
                        ?e,
                        "Failed to parse setting file! Falling back to default and moving \
                         existing file to a .invalid"
                    );
                })
                .and_then(|inner| {
                    inner.map_err(|e| {
                        warn!(
                            ?e,
                            "Failed to parse setting file! Falling back to default and moving \
                             existing file to a .invalid"
                        );
                    })
                }) {
                Ok((version, mut settings)) => {
                    if matches!(version, Version::Old) {
                        // Old version, which means we either performed a migration or there was
                        // some needed update to the file.  If this is the case, we preemptively
                        // overwrite the settings file (not strictly needed, but useful for
                        // people who do manual editing).
                        info!("Settings were changed on load, updating file...");
                        // We don't care if we encountered an error on saving updates to a
                        // settings file that we just loaded (it's already logged and migrated).
                        // However, we should crash if it reported an integrity failure, since we
                        // supposedly just validated it.
                        if let Err(Error::Integrity(err)) = settings
                            .edit(data_dir, |_| Some(()))
                            .expect("Some always returns Some")
                            .1
                        {
                            panic!(
                                "The identity conversion from a validated settings file must
                                    always be valid, but we found an integrity error: {:?}",
                                err
                            );
                        }
                    }
                    settings
                },
                Err(()) => {
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

    /// If the result of calling f is None,we return None (this constitutes an
    /// early return and lets us abandon the in-progress edit).  For
    /// example, this can be used to avoid adding a new ban entry if someone
    /// is already banned and the user didn't explicitly specify that they
    /// wanted to add a new ban record, even though it would be completely
    /// valid to attach one.
    ///
    /// Otherwise (the result of calling f was Some(r)), we always return
    /// Some((r, res)), where:
    ///
    /// If res is Ok(()), validation succeeded for the edited, and changes made
    /// inside the closure are applied both in memory (to self) and
    /// atomically on disk.
    ///
    /// Otherwise (res is Err(e)), some step in the edit process failed.
    /// Specifically:
    ///
    /// * If e is Integrity, validation failed and the settings were not
    ///   updated.
    /// * If e is Io, validation succeeded and the settings were updated in
    ///   memory, but they
    /// could not be saved to storage (and a warning was logged).  The reason we
    /// return an error even though the operation was partially successful
    /// is so we can alert the player who ran the command about the failure,
    /// as they will often be an administrator who can usefully act upon that
    /// information.
    #[must_use]
    fn edit<R>(
        &mut self,
        data_dir: &Path,
        f: impl FnOnce(&mut Self) -> Option<R>,
    ) -> Option<(R, Result<(), Error<Self>>)> {
        let path = Self::get_path(data_dir);

        // First, edit a copy.
        let mut copy = self.clone();
        let r = f(&mut copy)?;
        // Validate integrity of the raw data before saving (by making sure that
        // converting to and from the Settings format still produces a valid
        // file).
        Some((r, match save_to_file(copy, &path) {
            Ok(new_settings) => {
                *self = new_settings;
                Ok(())
            },
            Err(ErrorInternal::Io(err, new_settings)) => {
                warn!("Failed to save setting: {:?}", err);
                *self = new_settings;
                Err(Error::Io(err))
            },
            Err(ErrorInternal::Integrity(err)) => Err(Error::Integrity(err)),
        }))
    }

    fn get_path(data_dir: &Path) -> PathBuf {
        let mut path = super::with_config_dir(data_dir);
        path.push(Self::FILENAME);
        path
    }
}

fn save_to_file<S: EditableSetting>(setting: S, path: &Path) -> Result<S, ErrorInternal<S>> {
    let raw: <S as EditableSetting>::Setting = setting.into();
    let ron = ron::ser::to_string_pretty(&raw, ron::ser::PrettyConfig::default())
        .expect("RON does not throw any parse errors during serialization to string.");
    // This has the side effect of validating the copy, meaning it's safe to save
    // the file.
    let (_, settings): (Version, S) = raw.try_into().map_err(ErrorInternal::Integrity)?;
    // Create dir if it doesn't exist
    if let Some(dir) = path.parent() {
        if let Err(err) = fs::create_dir_all(dir) {
            return Err(ErrorInternal::Io(err, settings));
        }
    }
    // Atomically write the validated string to the settings file.
    let atomic_file = AtomicFile::new(path, OverwriteBehavior::AllowOverwrite);
    match atomic_file.write(|file| file.write_all(ron.as_bytes())) {
        Ok(()) => Ok(settings),
        Err(AtomicError::Internal(err)) | Err(AtomicError::User(err)) => {
            Err(ErrorInternal::Io(err, settings))
        },
    }
}

fn create_and_save_default<S: EditableSetting>(path: &Path) -> S {
    let default = S::default();
    match save_to_file(default, path) {
        Ok(settings) => settings,
        Err(ErrorInternal::Io(e, settings)) => {
            error!(?e, "Failed to create default setting file!");
            settings
        },
        Err(ErrorInternal::Integrity(err)) => {
            panic!(
                "The default settings file must always be valid, but we found an integrity error: \
                 {:?}",
                err
            );
        },
    }
}
