use std::{fs, io};

use assets_manager::{
    BoxedError,
    hot_reloading::{EventSender, FsWatcherBuilder},
    source::{DirEntry, FileContent, FileSystem as RawFs, Source},
};
use hashbrown::HashSet;

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
            .map_err(|e| io::Error::other(format!("failed to load canary asset: {}", e)))?;

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
    fn read(&self, id: &str, ext: &str) -> io::Result<FileContent<'_>> {
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
        // It's easy to get wrong, so here's the algorithm:
        //
        // 1) Read default assets directory first, gather directories it has.
        // 2) Read override assets directory second, gather directories *it* has.
        // 3) Call callback on each new directory (or file).
        //
        // This should route to src.read() above, which does read override
        // first, so even if we search for default directories first, we're
        // still overriding files proper.
        //
        // The rest is just properly routing errors.
        let mut collected = HashSet::new();

        let mut f = |dir_entry: DirEntry| {
            let cache_id = match dir_entry {
                DirEntry::File(path, ext) => (path.to_owned(), Some(ext.to_owned())),
                DirEntry::Directory(path) => (path.to_owned(), None),
            };

            // on first hit, call the callback
            if collected.insert(cache_id) {
                f(dir_entry)
            }
        };

        let default_res = self.default.read_dir(id, &mut f);
        let Some(dir) = &self.override_dir else {
            // If no override, return right there.
            return default_res;
        };

        let override_res = match dir.read_dir(id, &mut f) {
            Ok(()) => Ok(()),
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    let path = dir.path_of(DirEntry::Directory(id));
                    tracing::warn!(
                        "Error reading \"{}\": {}. Falling back to default",
                        path.display(),
                        err
                    );
                }
                Err(err)
            },
        };

        // Error juggling
        match (default_res, override_res) {
            // If failed from the start, error.
            //
            // Technically not necessary, but better be safe then sorry?
            (Err(err1), _) if err1.kind() != io::ErrorKind::NotFound => Err(err1),
            // If override succed, cool, celebrate.
            (_, Ok(())) => Ok(()),
            // If override failed, but default succeded, who cares.
            //
            // We could be strict here, but overrides are brittle by design,
            // and may fail with new version, so ...
            //
            // We log the warning there, that's it.
            (Ok(()), Err(_)) => Ok(()),
            // If If both failed, return last error.
            (Err(_), Err(err2)) => Err(err2),
        }
    }

    fn exists(&self, entry: DirEntry) -> bool {
        self.override_dir
            .as_ref()
            .is_some_and(|dir| dir.exists(entry))
            || self.default.exists(entry)
    }

    fn configure_hot_reloading(&self, events: EventSender) -> Result<(), BoxedError> {
        let mut builder = FsWatcherBuilder::new()?;

        if let Some(dir) = &self.override_dir {
            builder.watch(dir.root().to_owned())?;
        }
        builder.watch(self.default.root().to_owned())?;

        builder.build(events);
        Ok(())
    }
}
