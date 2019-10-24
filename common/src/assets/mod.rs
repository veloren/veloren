//! Load assets (images or voxel data) from files
pub mod watch;

use dot_vox::DotVoxData;
use hashbrown::HashMap;
use image::DynamicImage;
use lazy_static::lazy_static;
use log::error;
use serde_json::Value;
use std::{
    any::Any,
    fs::{self, File, ReadDir},
    io::{BufReader, Read},
    path::PathBuf,
    sync::{Arc, RwLock},
};

/// The error returned by asset loading functions
#[derive(Debug, Clone)]
pub enum Error {
    /// An asset of a different type has already been loaded with this specifier.
    InvalidType,
    /// Asset does not exist.
    NotFound(String),
}

impl From<Arc<dyn Any + 'static + Sync + Send>> for Error {
    fn from(_: Arc<dyn Any + 'static + Sync + Send>) -> Self {
        Error::InvalidType
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::NotFound(format!("{:?}", err))
    }
}

lazy_static! {
    /// The HashMap where all loaded assets are stored in.
    static ref ASSETS: RwLock<HashMap<String, Arc<dyn Any + 'static + Sync + Send>>> =
        RwLock::new(HashMap::new());
}

// TODO: Remove this function. It's only used in world/ in a really ugly way.To do this properly
// assets should have all their necessary data in one file. A ron file could be used to combine
// voxel data with positioning data for example.
/// Function used to load assets from the filesystem or the cache. Permits manipulating the loaded asset with a mapping function.
/// Example usage:
/// ```no_run
/// use veloren_common::{assets, terrain::Structure};
/// use vek::*;
///
/// let my_tree_structure = assets::load_map(
///        "world.tree.oak_green.1",
///        |s: Structure| s.with_center(Vec3::new(15, 18, 14)),
///    ).unwrap();
/// ```
pub fn load_map<A: Asset + 'static, F: FnOnce(A) -> A>(
    specifier: &str,
    f: F,
) -> Result<Arc<A>, Error> {
    let mut assets_write = ASSETS.write().unwrap();
    match assets_write.get(specifier) {
        Some(asset) => Ok(Arc::clone(asset).downcast()?),
        None => {
            let asset = Arc::new(f(A::parse(load_file(specifier, A::ENDINGS)?)?));
            let clone = Arc::clone(&asset);
            assets_write.insert(specifier.to_owned(), clone);
            Ok(asset)
        }
    }
}

pub fn load_glob<A: Asset + 'static>(specifier: &str) -> Result<Arc<Vec<Arc<A>>>, Error> {
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
                    .filter_map(|name| load(&specifier.replace("*", &name)).ok())
                    .collect::<Vec<_>>(),
            );
            let clone = Arc::clone(&assets);

            let mut assets_write = ASSETS.write().unwrap();
            assets_write.insert(specifier.to_owned(), clone);
            Ok(assets)
        }
        Err(error) => Err(error),
    }
}

/// Function used to load assets from the filesystem or the cache.
/// Example usage:
/// ```no_run
/// use image::DynamicImage;
/// use veloren_common::assets;
///
/// let my_image = assets::load::<DynamicImage>("core.ui.backgrounds.city").unwrap();
/// ```
pub fn load<A: Asset + 'static>(specifier: &str) -> Result<Arc<A>, Error> {
    load_map(specifier, |x| x)
}

/// Function used to load assets from the filesystem or the cache and return a clone.
pub fn load_cloned<A: Asset + Clone + 'static>(specifier: &str) -> Result<A, Error> {
    let asset = load(specifier);
    asset.map(|asset: Arc<A>| (*asset).clone())
}

/// Function used to load essential assets from the filesystem or the cache. It will panic if the asset is not found.
/// Example usage:
/// ```no_run
/// use image::DynamicImage;
/// use veloren_common::assets;
///
/// let my_image = assets::load_expect::<DynamicImage>("core.ui.backgrounds.city");
/// ```
pub fn load_expect<A: Asset + 'static>(specifier: &str) -> Arc<A> {
    load(specifier).unwrap_or_else(|_| panic!("Failed loading essential asset: {}", specifier))
}

/// Function used to load essential assets from the filesystem or the cache and return a clone. It will panic if the asset is not found.
pub fn load_expect_cloned<A: Asset + Clone + 'static>(specifier: &str) -> A {
    let asset: Arc<A> = load_expect(specifier);
    (*asset).clone()
}

/// Load an asset while registering it to be watched and reloaded when it changes
pub fn load_watched<A: Asset + 'static>(
    specifier: &str,
    indicator: &mut watch::ReloadIndicator,
) -> Result<Arc<A>, Error> {
    let asset = load(specifier)?;

    // Determine path to watch
    let path = unpack_specifier(specifier);
    let mut path_with_extension = None;
    for ending in A::ENDINGS {
        let mut path = path.clone();
        path.set_extension(ending);

        if path.exists() {
            path_with_extension = Some(path);
            break;
        }
    }

    let owned_specifier = specifier.to_string();
    indicator.add(
        path_with_extension.ok_or_else(|| Error::NotFound(path.to_string_lossy().into_owned()))?,
        move || {
            if let Err(err) = reload::<A>(&owned_specifier) {
                error!("Error reloading {}: {:#?}", &owned_specifier, err);
            }
        },
    );

    Ok(asset)
}

fn reload<A: Asset + 'static>(specifier: &str) -> Result<(), Error> {
    let asset = Arc::new(A::parse(load_file(specifier, A::ENDINGS)?)?);
    let clone = Arc::clone(&asset);
    let mut assets_write = ASSETS.write().unwrap();
    match assets_write.get_mut(specifier) {
        Some(a) => *a = clone,
        None => {
            assets_write.insert(specifier.to_owned(), clone);
        }
    }

    Ok(())
}

/// The Asset trait, which is implemented by all structures that have their data stored in the
/// filesystem.
pub trait Asset: Send + Sync + Sized {
    const ENDINGS: &'static [&'static str];
    /// Parse the input file and return the correct Asset.
    fn parse(buf_reader: BufReader<File>) -> Result<Self, Error>;
}

impl Asset for DynamicImage {
    const ENDINGS: &'static [&'static str] = &["png", "jpg"];
    fn parse(mut buf_reader: BufReader<File>) -> Result<Self, Error> {
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        Ok(image::load_from_memory(&buf).unwrap())
    }
}

impl Asset for DotVoxData {
    const ENDINGS: &'static [&'static str] = &["vox"];
    fn parse(mut buf_reader: BufReader<File>) -> Result<Self, Error> {
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        Ok(dot_vox::load_bytes(&buf).unwrap())
    }
}

// Read a JSON file
impl Asset for Value {
    const ENDINGS: &'static [&'static str] = &["json"];
    fn parse(buf_reader: BufReader<File>) -> Result<Self, Error> {
        Ok(serde_json::from_reader(buf_reader).unwrap())
    }
}

impl Asset for String {
    const ENDINGS: &'static [&'static str] = &["glsl"];
    fn parse(mut buf_reader: BufReader<File>) -> Result<Self, Error> {
        let mut string = String::new();
        buf_reader.read_to_string(&mut string)?;
        Ok(string)
    }
}

lazy_static! {
    /// Lazy static to find and cache where the asset directory is.
    static ref ASSETS_PATH: PathBuf = {
        let mut paths = Vec::new();

        // VELOREN_ASSETS environment variable
        if let Ok(var) = std::env::var("VELOREN_ASSETS") {
            paths.push(var.to_owned().into());
        }

        // Executable path
        if let Ok(mut path) = std::env::current_exe() {
            path.pop();
            paths.push(path);
        }

        // Working path
        if let Ok(path) = std::env::current_dir() {
            paths.push(path);
        }

        // System paths
        #[cfg(target_os = "linux")]
        paths.push("/usr/share/veloren/assets".into());

        for path in paths.clone() {
            match find_folder::Search::ParentsThenKids(3, 1)
                .of(path)
                .for_folder("assets")
            {
                Ok(assets_path) => return assets_path,
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

/// Converts a specifier like "core.backgrounds.city" to ".../veloren/assets/core/backgrounds/city".
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

        debug!("Trying to access \"{:?}\"", path);
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

        debug!("Trying to access \"{:?}\"", path);
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
