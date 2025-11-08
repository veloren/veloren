use serde::Deserialize;
use std::{fs, io, path::Path};
use tracing::warn;

/// Load settings from ron in a recoverable manner. Works best with
/// `#[serde(default)]`.
///
/// Lines with parse errors are deleted and parsing is attempted again; this is
/// repeated until either parsing succeeds or the string is empty.
/// If there was a parse error, the original file gets renamed to have the
/// extension `invalid.ron`. Otherwise the disk is not written to.
pub fn ron_from_path_recoverable<T: Default + for<'a> Deserialize<'a>>(path: &Path) -> T {
    if let Ok(file) = fs::File::open(path) {
        let deserialized = match io::read_to_string(file) {
            Ok(mut serialized) => match ron::from_str::<T>(&serialized) {
                Ok(s) => return s,
                Err(e) => {
                    warn!(
                        ?e,
                        ?path,
                        "Failed to parse configuration file! Attempting to recover valid data."
                    );
                    let mut span = e.span;
                    loop {
                        let start: usize = serialized
                            .split_inclusive('\n')
                            .take(span.start.line - 1)
                            .map(|s| s.len())
                            .sum();
                        let end: usize = serialized
                            .split_inclusive('\n')
                            .take(span.end.line)
                            .map(|s| s.len())
                            .sum();
                        drop(serialized.drain(start..end));

                        if serialized.is_empty() {
                            warn!(
                                ?path,
                                "Failed to recover anything from configuration file! Fallback to \
                                 default."
                            );
                            break T::default();
                        }

                        match ron::from_str::<T>(&serialized) {
                            Ok(s) => break s,
                            Err(e) => span = e.span,
                        }
                    }
                },
            },
            Err(e) => {
                warn!(
                    ?e,
                    ?path,
                    "Failed to read configuration file or convert it to string! Fallback to \
                     default."
                );
                T::default()
            },
        };

        // Rename the corrupted or outdated configuration file
        let new_path = path.with_extension("invalid.ron");
        if let Err(e) = fs::rename(path, &new_path) {
            warn!(?e, ?path, ?new_path, "Failed to rename configuration file.");
        }

        deserialized
    } else {
        // Presumably the file does not exist
        T::default()
    }
}
