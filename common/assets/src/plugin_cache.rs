use std::{path::PathBuf, sync::RwLock};

use crate::Concatenate;

use super::{fs::FileSystem, tar_source::Tar};
use assets_manager::{
    source::{FileContent, Source},
    AnyCache, AssetCache, BoxedError,
};
use lazy_static::lazy_static;

struct PluginEntry {
    path: PathBuf,
    cache: AssetCache<Tar>,
}

lazy_static! {
    static ref PLUGIN_LIST: RwLock<Vec<PluginEntry>> = RwLock::new(Vec::new());
}

/// The source combining filesystem and plugins (typically used via
/// CombinedCache)
#[derive(Debug, Clone)]
pub struct CombinedSource {
    fs: FileSystem,
}

impl CombinedSource {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            fs: FileSystem::new()?,
        })
    }
}

impl CombinedSource {
    fn read_multiple(&self, id: &str, ext: &str) -> Vec<(Option<usize>, FileContent<'_>)> {
        let mut result = Vec::new();
        if let Ok(file_entry) = self.fs.read(id, ext) {
            result.push((None, file_entry));
        }
        if let Ok(guard) = PLUGIN_LIST.read() {
            for (n, p) in guard.iter().enumerate() {
                if let Ok(entry) = p.cache.raw_source().read(id, ext) {
                    result.push((Some(n), match entry {
                        FileContent::Slice(s) => FileContent::Buffer(Vec::from(s)),
                        FileContent::Buffer(b) => FileContent::Buffer(b),
                        FileContent::Owned(s) => {
                            FileContent::Buffer(Vec::from(s.as_ref().as_ref()))
                        },
                    }));
                }
            }
        }
        result
    }

    // we don't want to keep the lock, so we clone
    fn plugin_path(&self, index: Option<usize>) -> Option<PathBuf> {
        if let Some(index) = index {
            PLUGIN_LIST
                .read()
                .ok()
                .and_then(|p| p.get(index).map(|p| p.path.clone()))
        } else {
            None
        }
    }
}

impl Source for CombinedSource {
    fn read(&self, id: &str, ext: &str) -> std::io::Result<FileContent<'_>> {
        // we could shortcut on fs if we dont want to check for conflicts
        let mut entries = self.read_multiple(id, ext);
        if entries.is_empty() {
            Err(std::io::ErrorKind::NotFound.into())
        } else {
            if entries.len() > 1 {
                tracing::error!(
                    "Duplicate asset {id} in read, plugins {:?} + {:?}",
                    self.plugin_path(entries[0].0),
                    self.plugin_path(entries[1].0)
                );
            }
            Ok(entries.swap_remove(0).1)
        }
    }

    fn read_dir(
        &self,
        id: &str,
        f: &mut dyn FnMut(assets_manager::source::DirEntry),
    ) -> std::io::Result<()> {
        // TODO: we should combine the sources
        self.fs.read_dir(id, f)
    }

    fn exists(&self, entry: assets_manager::source::DirEntry) -> bool {
        self.fs.exists(entry)
            || PLUGIN_LIST
                .read()
                .map(|p| p.iter().any(|p| p.cache.raw_source().exists(entry)))
                .unwrap_or_default()
    }

    fn make_source(&self) -> Option<Box<dyn Source + Send>> { None }
}

/// A cache combining filesystem and plugin assets
pub struct CombinedCache(AssetCache<CombinedSource>);

impl CombinedCache {
    pub fn new() -> std::io::Result<Self> {
        CombinedSource::new().map(|s| Self(AssetCache::with_source(s)))
    }

    /// combine objects from filesystem and plugins
    pub fn combine<T: Concatenate>(
        &self,
        load_from: impl Fn(AnyCache) -> Result<T, BoxedError>,
    ) -> Result<T, BoxedError> {
        let mut result = load_from(self.as_any_cache());
        for i in PLUGIN_LIST.read().unwrap().iter() {
            if let Ok(b) = load_from(i.cache.as_any_cache()) {
                result = if let Ok(a) = result {
                    Ok(a.concatenate(b))
                } else {
                    Ok(b)
                };
            }
        }
        result
    }

    pub fn register_tar(&self, path: PathBuf) -> std::io::Result<()> {
        let tar_source = Tar::from_path(&path)?;
        println!("Tar {:?} {:?}", path, tar_source);
        let cache = AssetCache::with_source(tar_source);
        PLUGIN_LIST.write().unwrap().push(PluginEntry { path, cache });
        Ok(())
    }    
}

impl std::ops::Deref for CombinedCache {
    type Target = AssetCache<CombinedSource>;

    fn deref(&self) -> &Self::Target { &self.0 }
}
