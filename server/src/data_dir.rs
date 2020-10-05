use std::path::{Path, PathBuf};

/// Used so that different server frontends can share the same server saves,
/// etc.
pub const DEFAULT_DATA_DIR_NAME: &'static str = "server";

/// Indicates where maps, saves, and server_config folders are to be stored
pub struct DataDir {
    pub path: PathBuf,
}
impl<T: Into<PathBuf>> From<T> for DataDir {
    fn from(t: T) -> Self { Self { path: t.into() } }
}
impl AsRef<Path> for DataDir {
    fn as_ref(&self) -> &Path { &self.path }
}
