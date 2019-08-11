//! Load assets (images or voxel data) from files

use dot_vox::DotVoxData;
use hashbrown::HashMap;
use image::DynamicImage;
use lazy_static::lazy_static;
use serde_json::Value;
use std::{
    any::Any,
    env,
    fs::{self, read_link, File, ReadDir},
    io::{BufReader, Read},
    path::{Path, PathBuf},
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
    match assets_write.get(&(specifier.to_owned() + A::ENDINGS[0])) {
        Some(asset) => Ok(Arc::clone(asset).downcast()?),
        None => {
            let asset = Arc::new(f(A::parse(load_file(specifier, A::ENDINGS)?)?));
            let clone = Arc::clone(&asset);
            assets_write.insert(specifier.to_owned(), clone);
            Ok(asset)
        }
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

/// Function to find where the asset/ directory is.
fn assets_dir() -> PathBuf {
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
}

/// Converts a specifier like "core.backgrounds.city" to ".../veloren/assets/core/backgrounds/city".
fn unpack_specifier(specifier: &str) -> PathBuf {
    let mut path = assets_dir();
    path.push(specifier.replace(".", "/"));
    path
}

/// Loads a file based on the specifier and possible extensions
pub fn load_file(specifier: &str, endings: &[&str]) -> Result<BufReader<File>, Error> {
    let mut path = unpack_specifier(specifier);
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
