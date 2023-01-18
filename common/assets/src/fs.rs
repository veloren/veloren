use std::{borrow::Cow, fs, io};

use assets_manager::{
    hot_reloading::{DynUpdateSender, EventSender, FsWatcherBuilder},
    source::{DirEntry, FileSystem as RawFs, Source},
    BoxedError,
};

/// Loads assets from the default path or `VELOREN_ASSETS_OVERRIDE` env if it is
/// set.
#[derive(Debug, Clone)]
pub struct FileSystem {
    default: RawFs,
    override_dir: Option<RawFs>,
}

impl FileSystem {
    pub fn new() -> io::Result<Self> {
        let default = RawFs::new(&*super::ASSETS_PATH)?;
        let override_dir = std::env::var_os("VELOREN_ASSETS_OVERRIDE").and_then(|path| {
            RawFs::new(path)
                .map_err(|err| tracing::error!("Error setting override assets directory: {}", err))
                .ok()
        });

        let canary = fs::read_to_string(super::ASSETS_PATH.join("common").join("canary.canary"))
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("failed to load canary asset: {}", e),
                )
            })?;

        if !canary.starts_with("VELOREN_CANARY_MAGIC") {
            panic!("Canary asset `canary.canary` was present but did not contain the expected data. This *heavily* implies that you've not correctly set up Git LFS (Large File Storage). Visit `https://book.veloren.net/contributors/development-tools.html#git-lfs` for more information about setting up Git LFS.");
        }

        Ok(Self {
            default,
            override_dir,
        })
    }
}

impl Source for FileSystem {
    fn read(&self, id: &str, ext: &str) -> io::Result<Cow<[u8]>> {
        if let Some(dir) = &self.override_dir {
            match dir.read(id, ext) {
                Ok(content) => return Ok(content),
                Err(err) => {
                    if err.kind() != io::ErrorKind::NotFound {
                        let path = dir.path_of(DirEntry::File(id, ext));
                        tracing::warn!(
                            "Error reading \"{}\": {}. Falling back to default",
                            path.display(),
                            err
                        );
                    }
                },
            }
        }

        // If not found in override path, try load from main asset path
        self.default.read(id, ext)
    }

    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        if let Some(dir) = &self.override_dir {
            match dir.read_dir(id, f) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    if err.kind() != io::ErrorKind::NotFound {
                        let path = dir.path_of(DirEntry::Directory(id));
                        tracing::warn!(
                            "Error reading \"{}\": {}. Falling back to default",
                            path.display(),
                            err
                        );
                    }
                },
            }
        }

        // If not found in override path, try load from main asset path
        self.default.read_dir(id, f)
    }

    fn exists(&self, entry: DirEntry) -> bool {
        self.override_dir
            .as_ref()
            .map_or(false, |dir| dir.exists(entry))
            || self.default.exists(entry)
    }

    fn make_source(&self) -> Option<Box<dyn Source + Send>> { Some(Box::new(self.clone())) }

    fn configure_hot_reloading(&self, events: EventSender) -> Result<DynUpdateSender, BoxedError> {
        let mut builder = FsWatcherBuilder::new()?;

        if let Some(dir) = &self.override_dir {
            builder.watch(dir.root().to_owned())?;
        }
        builder.watch(self.default.root().to_owned())?;

        Ok(builder.build(events))
    }
}
