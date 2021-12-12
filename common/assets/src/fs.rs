use std::{borrow::Cow, io, path::PathBuf};

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

        Ok(Self {
            default,
            override_dir,
        })
    }

    pub fn path_of(&self, specifier: &str, ext: &str) -> PathBuf {
        self.default.path_of(DirEntry::File(specifier, ext))
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
                        tracing::warn!("Error reading \"{}\": {}", path.display(), err);
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
                        tracing::warn!("Error reading \"{}\": {}", path.display(), err);
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
