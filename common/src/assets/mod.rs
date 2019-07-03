use dot_vox::DotVoxData;
use image::DynamicImage;
use lazy_static::lazy_static;
use serde_json::Value;
use std::{
    any::Any,
    collections::HashMap,
    env,
    fs::{read_dir, read_link, File, ReadDir},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

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
    static ref ASSETS: RwLock<HashMap<String, Arc<dyn Any + 'static + Sync + Send>>> =
        RwLock::new(HashMap::new());
}

/// Function used to load assets. Permits manipulating the loaded asset with a mapping function.
/// Loaded assets are cached in a global singleton hashmap.
/// Example usage:
/// ```no_run=
/// use veloren_common::{assets, terrain::Structure};
/// use vek::*;
///
/// let my_tree_structure = assets::load_map(
///        "world/tree/oak_green/1.vox",
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
            let asset = Arc::new(f(A::load(load_from_path(specifier)?)?));
            let clone = Arc::clone(&asset);
            assets_write.insert(specifier.to_owned(), clone);
            Ok(asset)
        }
    }
}

/// Function used to load assets.
/// Loaded assets are cached in a global singleton hashmap.
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

/// Function used to load assets that will panic if the asset is not found.
/// Use this to load essential assets.
/// Loaded assets are cached in a global singleton hashmap.
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

/// Asset Trait
pub trait Asset: Send + Sync + Sized {
    fn load(buf_reader: BufReader<impl Read>) -> Result<Self, Error>;
}

impl Asset for DynamicImage {
    fn load(mut buf_reader: BufReader<impl Read>) -> Result<Self, Error> {
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        Ok(image::load_from_memory(&buf).unwrap())
    }
}

impl Asset for DotVoxData {
    fn load(mut buf_reader: BufReader<impl Read>) -> Result<Self, Error> {
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        Ok(dot_vox::load_bytes(&buf).unwrap())
    }
}

impl Asset for Value {
    fn load(buf_reader: BufReader<impl Read>) -> Result<Self, Error> {
        Ok(serde_json::from_reader(buf_reader).unwrap())
    }
}

// TODO: System to load file from specifiers (e.g.: "core.ui.backgrounds.city").
fn assets_folder() -> PathBuf {
    let mut paths = Vec::new();

    // VELOREN_ASSETS environment variable
    if let Ok(var) = std::env::var("VELOREN_ASSETS") {
        paths.push(var.to_string().into());
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
            a += path.to_str().unwrap_or("<invalid>");
            a += "\n";
            a
        }),
    );
}

// TODO: System to load file from specifiers (e.g.: "core.ui.backgrounds.city").
pub fn load_from_path(name: &str) -> Result<BufReader<File>, Error> {
    debug!("Trying to access \"{}\"", name);

    let mut path = assets_folder();
    path.push(name);

    match File::open(path) {
        Ok(file) => Ok(BufReader::new(file)),
        Err(_) => Err(Error::NotFound(name.to_owned())),
    }
}

/// Read directory from `veloren/assets/*`
pub fn read_from_assets(dir_name: &str) -> Result<ReadDir, Error> {
    let mut entry = assets_folder();
    entry.push("../assets/");
    entry.push(dir_name);
    match Path::new(&entry).exists() {
        true => Ok(read_dir(entry).expect("`read_dir` failed.")),
        false => Err(Error::NotFound(entry.to_str().unwrap().to_owned())),
    }
}

/// Returns the cargo manifest directory when running the executable with cargo
/// or the directory in which the executable resides otherwise,
/// traversing symlinks if necessary.
pub fn application_root_dir() -> String {
    match env::var("PROFILE") {
        Ok(_) => String::from(env!("CARGO_MANIFEST_DIR")),
        Err(_) => {
            let mut path = env::current_exe().expect("Failed to find executable path.");
            while let Ok(target) = read_link(path.clone()) {
                path = target;
            }
            String::from(
                path.parent()
                    .expect("Failed to get parent directory of the executable.")
                    .to_str()
                    .unwrap(),
            )
        }
    }
}
