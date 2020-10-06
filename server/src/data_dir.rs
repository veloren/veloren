use std::path::{Path, PathBuf};

/// Used so that different server frontends can share the same server saves,
/// etc.
pub const DEFAULT_DATA_DIR_NAME: &str = "server";

/// Indicates where maps, saves, and server_config folders are to be stored
pub struct DataDir {
    pub path: PathBuf,
}
impl AsRef<Path> for DataDir {
    fn as_ref(&self) -> &Path { &self.path }
}
