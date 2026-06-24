//#![warn(clippy::pedantic)]
//! Load assets (images or voxel data) from files

use image::DynamicImage;
use lazy_static::lazy_static;
use std::{
    borrow::Cow,
    collections::HashMap,
    hash::{BuildHasher, Hash},
    path::PathBuf,
    sync::Arc,
};

pub use assets_manager::{
    Asset, AssetCache, BoxedError, Error, FileAsset, SharedString,
    asset::{DirLoadable, Ron, load_bincode_legacy, load_ron},
    source::{self, Source},
};

mod fs;
#[cfg(feature = "plugins")] mod plugin_cache;
mod walk;
pub use walk::{Walk, walk_tree};

#[cfg(feature = "plugins")]
lazy_static! {
    /// The HashMap where all loaded assets are stored in.
    static ref ASSETS: plugin_cache::CombinedCache = plugin_cache::CombinedCache::new().unwrap();
}
#[cfg(not(feature = "plugins"))]
lazy_static! {
    /// The HashMap where all loaded assets are stored in.
    static ref ASSETS: AssetCache =
            AssetCache::with_source(fs::FileSystem::new().unwrap());
}

// register a new plugin
#[cfg(feature = "plugins")]
pub fn register_tar(path: PathBuf) -> std::io::Result<()> { ASSETS.register_tar(path) }

pub type AssetHandle<T> = &'static assets_manager::Handle<T>;
pub type AssetReadGuard<T> = assets_manager::AssetReadGuard<'static, T>;
pub type AssetDirHandle<T> = AssetHandle<assets_manager::RecursiveDirectory<T>>;
pub type ReloadWatcher = assets_manager::ReloadWatcher<'static>;

/// The Asset trait, which is implemented by all structures that have their data
/// stored in the filesystem.
pub trait AssetExt: Sized + Send + Sync + 'static {
    /// Function used to load assets from the filesystem or the cache.
    /// Example usage:
    /// ```no_run
    /// use veloren_common_assets::{AssetExt, Image};
    ///
    /// let my_image = Image::load("core.ui.backgrounds.city").unwrap();
    /// ```
    fn load(specifier: &str) -> Result<AssetHandle<Self>, Error>;

    /// Function used to load assets from the filesystem or the cache and return
    /// a clone.
    fn load_cloned(specifier: &str) -> Result<Self, Error>
    where
        Self: Clone,
    {
        Self::load(specifier).map(|h| h.cloned())
    }

    fn load_or_insert_with(
        specifier: &str,
        default: impl FnOnce(Error) -> Self,
    ) -> AssetHandle<Self> {
        Self::load(specifier).unwrap_or_else(|err| Self::get_or_insert(specifier, default(err)))
    }

    /// Function used to load essential assets from the filesystem or the cache.
    /// It will panic if the asset is not found. Example usage:
    /// ```no_run
    /// use veloren_common_assets::{AssetExt, Image};
    ///
    /// let my_image = Image::load_expect("core.ui.backgrounds.city");
    /// ```
    #[track_caller]
    fn load_expect(specifier: &str) -> AssetHandle<Self> {
        #[track_caller]
        #[cold]
        fn expect_failed(err: Error) -> ! {
            panic!(
                "Failed loading essential asset: {} (error={:?})",
                err.id(),
                err.reason()
            )
        }

        // Avoid using `unwrap_or_else` to avoid breaking `#[track_caller]`
        match Self::load(specifier) {
            Ok(handle) => handle,
            Err(err) => expect_failed(err),
        }
    }

    /// Function used to load essential assets from the filesystem or the cache
    /// and return a clone. It will panic if the asset is not found.
    #[track_caller]
    fn load_expect_cloned(specifier: &str) -> Self
    where
        Self: Clone,
    {
        Self::load_expect(specifier).cloned()
    }

    fn load_owned(specifier: &str) -> Result<Self, Error>;

    fn get_or_insert(specifier: &str, default: Self) -> AssetHandle<Self>;
}

impl<T: Asset> AssetExt for T {
    fn load(specifier: &str) -> Result<AssetHandle<Self>, Error> { ASSETS.load(specifier) }

    fn load_owned(specifier: &str) -> Result<Self, Error> { ASSETS.load_owned(specifier) }

    fn get_or_insert(specifier: &str, default: Self) -> AssetHandle<Self> {
        ASSETS.get_or_insert(specifier, default)
    }
}

/// Extension to AssetExt to combine Ron files from filesystem and plugins
pub trait AssetCombined: AssetExt {
    fn load_and_combine(
        cache: &'static AssetCache,
        specifier: &str,
    ) -> Result<AssetHandle<Self>, Error>;

    /// Load combined table without hot-reload support
    fn load_and_combine_static(specifier: &str) -> Result<AssetHandle<Self>, Error> {
        #[cfg(feature = "plugins")]
        {
            ASSETS.no_record(|| Self::load_and_combine(ASSETS.as_cache(), specifier))
        }
        #[cfg(not(feature = "plugins"))]
        {
            Self::load(specifier)
        }
    }

    #[track_caller]
    fn load_expect_combined(cache: &'static AssetCache, specifier: &str) -> AssetHandle<Self> {
        // Avoid using `unwrap_or_else` to avoid breaking `#[track_caller]`
        match Self::load_and_combine(cache, specifier) {
            Ok(handle) => handle,
            Err(err) => {
                panic!("Failed loading essential combined asset: {specifier} (error={err:?})")
            },
        }
    }

    /// Load combined table without hot-reload support, panic on error
    #[track_caller]
    fn load_expect_combined_static(specifier: &str) -> AssetHandle<Self> {
        #[cfg(feature = "plugins")]
        {
            ASSETS.no_record(|| Self::load_expect_combined(ASSETS.as_cache(), specifier))
        }
        #[cfg(not(feature = "plugins"))]
        {
            Self::load_expect(specifier)
        }
    }
}

impl<T: Asset + Concatenate> AssetCombined for T {
    fn load_and_combine(
        cache: &'static AssetCache,
        specifier: &str,
    ) -> Result<AssetHandle<Self>, Error> {
        cache.load_and_combine(specifier)
    }
}

/// Extension to AssetCache to combine Ron files from filesystem and plugins
pub trait CacheCombined {
    fn load_and_combine<A: Asset + Concatenate>(
        &self,
        id: &str,
    ) -> Result<&assets_manager::Handle<A>, Error>;
}

impl CacheCombined for AssetCache {
    fn load_and_combine<A: Asset + Concatenate>(
        &self,
        specifier: &str,
    ) -> Result<&assets_manager::Handle<A>, Error> {
        #[cfg(feature = "plugins")]
        {
            tracing::info!("combine {specifier}");
            let data: Result<A, _> = ASSETS.combine(self, |cache| cache.load_owned::<A>(specifier));
            data.map(|data| self.get_or_insert(specifier, data))
        }
        #[cfg(not(feature = "plugins"))]
        {
            self.load(specifier)
        }
    }
}

/// Loads directory and all files in it.
///
/// "rec" stands for "recursively"
///
/// Note, this only gets the ids of assets, they are not actually loaded. The
/// returned handle can be used to iterate over the IDs or to iterate over
/// assets trying to load them.
///
/// # Errors
/// An error is returned if the given id does not match a valid readable
/// directory.
///
/// When loading a directory recursively, directories that can't be read are
/// ignored.
pub fn load_rec_dir<T: DirLoadable + Asset>(specifier: &str) -> Result<AssetDirHandle<T>, Error> {
    let specifier = specifier.strip_suffix(".*").unwrap_or(specifier);
    ASSETS.load_rec_dir(specifier)
}

pub struct Image(pub Arc<DynamicImage>);

impl Image {
    pub fn to_image(&self) -> Arc<DynamicImage> { Arc::clone(&self.0) }
}

impl FileAsset for Image {
    const EXTENSIONS: &'static [&'static str] = &["png", "jpg"];

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        let image = image::load_from_memory(&bytes)?;
        Ok(Image(Arc::new(image)))
    }
}

pub struct DotVox(pub dot_vox::DotVoxData);

impl FileAsset for DotVox {
    const EXTENSION: &'static str = "vox";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        let data = dot_vox::load_bytes(&bytes).map_err(|err| err.to_owned())?;
        Ok(DotVox(data))
    }
}

pub struct Obj(pub wavefront::Obj);

impl FileAsset for Obj {
    const EXTENSION: &'static str = "obj";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        let data = wavefront::Obj::from_reader(&*bytes)?;
        Ok(Obj(data))
    }
}

pub trait Concatenate {
    fn concatenate(self, b: Self) -> Self;
}

impl<K: Eq + Hash, V, S: BuildHasher> Concatenate for HashMap<K, V, S> {
    fn concatenate(mut self, b: Self) -> Self {
        self.extend(b);
        self
    }
}

impl<V> Concatenate for Vec<V> {
    fn concatenate(mut self, b: Self) -> Self {
        self.extend(b);
        self
    }
}

impl<K: Eq + Hash, V, S: BuildHasher> Concatenate for hashbrown::HashMap<K, V, S> {
    fn concatenate(mut self, b: Self) -> Self {
        self.extend(b);
        self
    }
}

impl<T: Concatenate> Concatenate for Ron<T> {
    fn concatenate(self, b: Self) -> Self { Self(self.into_inner().concatenate(b.into_inner())) }
}

/// This wrapper combines several RON files from multiple sources
#[cfg(feature = "plugins")]
#[derive(Clone)]
pub struct MultiRon<T>(pub T);

#[cfg(feature = "plugins")]
impl<T> Asset for MultiRon<T>
where
    T: for<'de> serde::Deserialize<'de> + Send + Sync + 'static + Concatenate,
{
    // the passed cache registers with hot reloading
    fn load(cache: &AssetCache, id: &SharedString) -> Result<Self, BoxedError> {
        ASSETS
            .combine(cache, |cache| {
                cache.load_owned::<Ron<T>>(id).map(|ron| ron.into_inner())
            })
            .map(MultiRon)
            .map_err(Into::<BoxedError>::into)
    }
}

// fallback
#[cfg(not(feature = "plugins"))]
pub use assets_manager::asset::Ron as MultiRon;

/// Return path to repository root by searching 10 directories back
pub fn find_root() -> Option<PathBuf> {
    std::env::current_dir().map_or(None, |path| {
        // If we are in the root, push path
        if path.join(".git").exists() {
            return Some(path);
        }
        // Search .git directory in parent directories
        for ancestor in path.ancestors().take(10) {
            if ancestor.join(".git").exists() {
                return Some(ancestor.to_path_buf());
            }
        }
        None
    })
}

lazy_static! {
    /// Lazy static to find and cache where the asset directory is.
    /// Cases we need to account for:
    /// 1. Running through airshipper (`assets` next to binary)
    /// 2. Install with package manager and run (assets probably in `/usr/share/veloren/assets` while binary in `/usr/bin/`)
    /// 3. Download & hopefully extract zip (`assets` next to binary)
    /// 4. Running through cargo (`assets` in workspace root but not always in cwd in case you `cd voxygen && cargo r`)
    /// 5. Running executable in the target dir (`assets` in workspace)
    /// 6. Running tests (`assets` in workspace root)
    pub static ref ASSETS_PATH: PathBuf = {
        let mut paths = Vec::new();

        // Note: Ordering matters here!

        // 1. VELOREN_ASSETS environment variable
        if let Ok(var) = std::env::var("VELOREN_ASSETS") {
            paths.push(var.into());
        }

        // 2. Executable path
        if let Ok(mut path) = std::env::current_exe() {
            path.pop();
            paths.push(path);
        }

        // 3. Root of the repository
        if let Some(path) = find_root() {
            paths.push(path);
        }

        // 4. System paths
        #[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android")))]
        {
            if let Ok(result) = std::env::var("XDG_DATA_HOME") {
                paths.push(format!("{}/veloren/", result).into());
            } else if let Ok(result) = std::env::var("HOME") {
                paths.push(format!("{}/.local/share/veloren/", result).into());
            }

            if let Ok(result) = std::env::var("XDG_DATA_DIRS") {
                result.split(':').for_each(|x| paths.push(format!("{}/veloren/", x).into()));
            } else {
                // Fallback
                let fallback_paths = vec!["/usr/local/share", "/usr/share"];
                for fallback_path in fallback_paths {
                    paths.push(format!("{}/veloren/", fallback_path).into());
                }
            }
        }

        tracing::trace!("Possible asset locations paths={:?}", paths);

        for mut path in paths.clone() {
            if !path.ends_with("assets") {
                path = path.join("assets");
            }

            if path.is_dir() {
                tracing::info!("Assets found path={}", path.display());
                return path;
            }
        }

        panic!(
            "Asset directory not found. In attempting to find it, we searched:\n{})",
            paths.iter().fold(String::new(), |mut a, path| {
                a += &path.to_string_lossy();
                a += "\n";
                a
            }),
        );
    };
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, fs::File};
    use walkdir::WalkDir;

    #[test]
    fn load_canary() {
        // Loading the asset cache will automatically cause the canary to load
        let _ = *super::ASSETS;
    }

    /// Fail unless all `.ron` asset files successfully parse to `ron::Value`.
    #[test]
    fn parse_all_ron_files_to_value() {
        let ext = OsStr::new("ron");
        WalkDir::new(crate::ASSETS_PATH.as_path())
            .into_iter()
            .map(|ent| {
                ent.expect("Failed to walk over asset directory")
                    .into_path()
            })
            .filter(|path| path.is_file())
            .filter(|path| {
                path.extension()
                    .is_some_and(|e| ext == e.to_ascii_lowercase())
            })
            .for_each(|path| {
                let file = File::open(&path).expect("Failed to open the file");
                if let Err(err) = ron::de::from_reader::<_, ron::Value>(file) {
                    println!("{:?}", path);
                    println!("{:#?}", err);
                    panic!("Parse failed");
                }
            });
    }
}
