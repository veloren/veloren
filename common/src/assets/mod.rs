//! Load assets (images or voxel data) from files
pub mod watch;

use core::{any::Any, fmt, marker::PhantomData};
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use image::DynamicImage;
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::Value;
use std::{
    fs::{self, File, ReadDir},
    io::{BufReader, Read},
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tracing::{error, trace};

/// The error returned by asset loading functions
#[derive(Debug, Clone)]
pub enum Error {
    /// Parsing error occurred.
    ParseError(Arc<dyn std::fmt::Debug>),
    /// An asset of a different type has already been loaded with this
    /// specifier.
    InvalidType,
    /// Asset does not exist.
    NotFound(String),
}

impl Error {
    pub fn parse_error<E: std::fmt::Debug + 'static>(err: E) -> Self {
        Self::ParseError(Arc::new(err))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ParseError(err) => write!(f, "{:?}", err),
            Error::InvalidType => write!(
                f,
                "an asset of a different type has already been loaded with this specifier."
            ),
            Error::NotFound(s) => write!(f, "{}", s),
        }
    }
}

impl From<Arc<dyn Any + 'static + Sync + Send>> for Error {
    fn from(_: Arc<dyn Any + 'static + Sync + Send>) -> Self { Error::InvalidType }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self { Error::NotFound(format!("{}", err)) }
}

lazy_static! {
    /// The HashMap where all loaded assets are stored in.
    static ref ASSETS: RwLock<HashMap<String, Arc<dyn Any + 'static + Sync + Send>>> =
        RwLock::new(HashMap::new());
}

fn reload<A: Asset>(specifier: &str) -> Result<(), Error>
where
    A::Output: Send + Sync + 'static,
{
    let asset = Arc::new(A::parse(load_file(specifier, A::ENDINGS)?)?);
    let mut assets_write = ASSETS.write().unwrap();
    match assets_write.get_mut(specifier) {
        Some(a) => *a = asset,
        None => {
            assets_write.insert(specifier.to_owned(), asset);
        },
    }

    Ok(())
}

/// The Asset trait, which is implemented by all structures that have their data
/// stored in the filesystem.
pub trait Asset: Sized {
    type Output = Self;

    const ENDINGS: &'static [&'static str];
    /// Parse the input file and return the correct Asset.
    fn parse(buf_reader: BufReader<File>) -> Result<Self::Output, Error>;

    // TODO: Remove this function. It's only used in world/ in a really ugly way.To
    // do this properly assets should have all their necessary data in one file. A
    // ron file could be used to combine voxel data with positioning data for
    // example.
    /// Function used to load assets from the filesystem or the cache. Permits
    /// manipulating the loaded asset with a mapping function. Example usage:
    /// ```no_run
    /// use vek::*;
    /// use veloren_common::{assets::Asset, terrain::Structure};
    ///
    /// let my_tree_structure = Structure::load_map("world.tree.oak_green.1", |s: Structure| {
    ///     s.with_center(Vec3::new(15, 18, 14))
    /// })
    /// .unwrap();
    /// ```
    fn load_map<F: FnOnce(Self::Output) -> Self::Output>(
        specifier: &str,
        f: F,
    ) -> Result<Arc<Self::Output>, Error>
    where
        Self::Output: Send + Sync + 'static,
    {
        let assets_write = ASSETS.read().unwrap();
        match assets_write.get(specifier) {
            Some(asset) => Ok(Arc::clone(asset).downcast()?),
            None => {
                drop(assets_write); // Drop the asset hashmap to permit recursive loading
                let asset = Arc::new(f(Self::parse(load_file(specifier, Self::ENDINGS)?)?));
                let clone = Arc::clone(&asset);
                ASSETS.write().unwrap().insert(specifier.to_owned(), clone);
                Ok(asset)
            },
        }
    }

    fn load_glob(specifier: &str) -> Result<Arc<Vec<Arc<Self::Output>>>, Error>
    where
        Self::Output: Send + Sync + 'static,
    {
        if let Some(assets) = ASSETS.read().unwrap().get(specifier) {
            return Ok(Arc::clone(assets).downcast()?);
        }

        // Get glob matches
        let glob_matches = read_dir(specifier.trim_end_matches(".*")).map(|dir| {
            dir.filter_map(|direntry| {
                direntry.ok().and_then(|file| {
                    file.file_name()
                        .to_string_lossy()
                        .rsplitn(2, '.')
                        .last()
                        .map(|s| s.to_owned())
                })
            })
            .collect::<Vec<_>>()
        });

        match glob_matches {
            Ok(glob_matches) => {
                let assets = Arc::new(
                    glob_matches
                        .into_iter()
                        .filter_map(|name| {
                            Self::load(&specifier.replace("*", &name))
                                .map_err(|e| {
                                    error!(
                                        ?e,
                                        "Failed to load \"{}\" as part of glob \"{}\"",
                                        name,
                                        specifier
                                    )
                                })
                                .ok()
                        })
                        .collect::<Vec<_>>(),
                );
                let clone = Arc::clone(&assets);

                let mut assets_write = ASSETS.write().unwrap();
                assets_write.insert(specifier.to_owned(), clone);
                Ok(assets)
            },
            Err(error) => Err(error),
        }
    }

    /// Function used to load assets from the filesystem or the cache.
    /// Example usage:
    /// ```no_run
    /// use image::DynamicImage;
    /// use veloren_common::assets::Asset;
    ///
    /// let my_image = DynamicImage::load("core.ui.backgrounds.city").unwrap();
    /// ```
    fn load(specifier: &str) -> Result<Arc<Self::Output>, Error>
    where
        Self::Output: Send + Sync + 'static,
    {
        Self::load_map(specifier, |x| x)
    }

    /// Function used to load assets from the filesystem or the cache and return
    /// a clone.
    fn load_cloned(specifier: &str) -> Result<Self::Output, Error>
    where
        Self::Output: Clone + Send + Sync + 'static,
    {
        Self::load(specifier).map(|asset| (*asset).clone())
    }

    /// Function used to load essential assets from the filesystem or the cache.
    /// It will panic if the asset is not found. Example usage:
    /// ```no_run
    /// use image::DynamicImage;
    /// use veloren_common::assets::Asset;
    ///
    /// let my_image = DynamicImage::load_expect("core.ui.backgrounds.city");
    /// ```
    fn load_expect(specifier: &str) -> Arc<Self::Output>
    where
        Self::Output: Send + Sync + 'static,
    {
        Self::load(specifier).unwrap_or_else(|err| {
            panic!(
                "Failed loading essential asset: {} (error={:?})",
                specifier, err
            )
        })
    }

    /// Function used to load essential assets from the filesystem or the cache
    /// and return a clone. It will panic if the asset is not found.
    fn load_expect_cloned(specifier: &str) -> Self::Output
    where
        Self::Output: Clone + Send + Sync + 'static,
    {
        Self::load_expect(specifier).as_ref().clone()
    }

    /// Load an asset while registering it to be watched and reloaded when it
    /// changes
    fn load_watched(
        specifier: &str,
        indicator: &mut watch::ReloadIndicator,
    ) -> Result<Arc<Self::Output>, Error>
    where
        Self::Output: Send + Sync + 'static,
    {
        let asset = Self::load(specifier)?;

        // Determine path to watch
        let path = unpack_specifier(specifier);
        let mut path_with_extension = None;
        for ending in Self::ENDINGS {
            let mut path = path.clone();
            path.set_extension(ending);

            if path.exists() {
                path_with_extension = Some(path);
                break;
            }
        }

        let owned_specifier = specifier.to_string();
        indicator.add(
            path_with_extension
                .ok_or_else(|| Error::NotFound(path.to_string_lossy().into_owned()))?,
            move || {
                if let Err(e) = reload::<Self>(&owned_specifier) {
                    error!(?e, ?owned_specifier, "Error reloading owned_specifier");
                }
            },
        );

        Ok(asset)
    }
}

impl Asset for DynamicImage {
    const ENDINGS: &'static [&'static str] = &["png", "jpg"];

    fn parse(mut buf_reader: BufReader<File>) -> Result<Self, Error> {
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        image::load_from_memory(&buf).map_err(Error::parse_error)
    }
}

impl Asset for DotVoxData {
    const ENDINGS: &'static [&'static str] = &["vox"];

    fn parse(mut buf_reader: BufReader<File>) -> Result<Self, Error> {
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        dot_vox::load_bytes(&buf).map_err(Error::parse_error)
    }
}

// Read a JSON file
impl Asset for Value {
    const ENDINGS: &'static [&'static str] = &["json"];

    fn parse(buf_reader: BufReader<File>) -> Result<Self, Error> {
        serde_json::from_reader(buf_reader).map_err(Error::parse_error)
    }
}

/// Load fron an arbitrary RON file.
pub struct Ron<T>(pub PhantomData<T>);

impl<T: Send + Sync + for<'de> Deserialize<'de>> Asset for Ron<T> {
    type Output = T;

    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>) -> Result<T, Error> {
        ron::de::from_reader(buf_reader).map_err(Error::parse_error)
    }
}

/// Load from a specific asset path.
pub struct AssetWith<T: Asset, const ASSET_PATH: &'static str> {
    pub asset: Arc<T::Output>,
}

impl<T: Asset, const ASSET_PATH: &'static str> Clone for AssetWith<T, ASSET_PATH> {
    fn clone(&self) -> Self {
        Self {
            asset: Arc::clone(&self.asset),
        }
    }
}

impl<T: Asset, const ASSET_PATH: &'static str> AssetWith<T, ASSET_PATH>
where
    T::Output: Send + Sync + 'static,
{
    #[inline]
    pub fn load_watched(indicator: &mut watch::ReloadIndicator) -> Result<Self, Error> {
        T::load_watched(ASSET_PATH, indicator).map(|asset| Self { asset })
    }

    #[inline]
    pub fn reload(&mut self) -> Result<(), Error> {
        self.asset = T::load(ASSET_PATH)?;
        Ok(())
    }
}

lazy_static! {
    /// Lazy static to find and cache where the asset directory is.
    /// Cases we need to account for:
    /// 1. Running through airshipper (`assets` next to binary)
    /// 2. Install with package manager and run (assets probably in `/usr/share/veloren/assets` while binary in `/usr/bin/`)
    /// 3. Download & hopefully extract zip (`assets` next to binary)
    /// 4. Running through cargo (`assets` in workspace root but not always in cwd incase you `cd voxygen && cargo r`)
    /// 5. Running executable in the target dir (`assets` in workspace)
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

        // 3. Working path
        if let Ok(path) = std::env::current_dir() {
            paths.push(path);
        }

        // 4. Cargo Workspace (e.g. local development)
        // https://github.com/rust-lang/cargo/issues/3946#issuecomment-359619839
        if let Ok(Ok(path)) = std::env::var("CARGO_MANIFEST_DIR").map(|s| s.parse::<PathBuf>()) {
            paths.push(path.parent().unwrap().to_path_buf());
            paths.push(path);
        }

        // 5. System paths
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

        for path in paths.clone() {
            match find_folder::check_dir("assets", &path) {
                Ok(assets_path) => {
                    tracing::info!("Assets found path={}", assets_path.display());
                    return assets_path;
                },
                Err(_) => continue,
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

/// Converts a specifier like "core.backgrounds.city" to
/// ".../veloren/assets/core/backgrounds/city".
fn unpack_specifier(specifier: &str) -> PathBuf {
    let mut path = ASSETS_PATH.clone();
    path.push(specifier.replace(".", "/"));
    path
}

/// Loads a file based on the specifier and possible extensions
pub fn load_file(specifier: &str, endings: &[&str]) -> Result<BufReader<File>, Error> {
    let path = unpack_specifier(specifier);
    for ending in endings {
        let mut path = path.clone();
        path.set_extension(ending);

        trace!(?path, "Trying to access");
        if let Ok(file) = File::open(path) {
            return Ok(BufReader::new(file));
        }
    }

    Err(Error::NotFound(path.to_string_lossy().into_owned()))
}

/// Loads a file based on the specifier and possible extensions
pub fn load_file_glob(specifier: &str, endings: &[&str]) -> Result<BufReader<File>, Error> {
    let path = unpack_specifier(specifier);
    for ending in endings {
        let mut path = path.clone();
        path.set_extension(ending);

        trace!(?path, "Trying to access");
        if let Ok(file) = File::open(path) {
            return Ok(BufReader::new(file));
        }
    }

    Err(Error::NotFound(path.to_string_lossy().into_owned()))
}

/// Read directory from `veloren/assets/*`
pub fn read_dir(specifier: &str) -> Result<ReadDir, Error> {
    let dir_name = unpack_specifier(specifier);
    if dir_name.exists() {
        Ok(fs::read_dir(dir_name).expect("`read_dir` failed."))
    } else {
        Err(Error::NotFound(dir_name.to_string_lossy().into_owned()))
    }
}
