use dot_vox::DotVoxData;
use image::DynamicImage;
use lazy_static::lazy_static;
use serde_json::Value;
use std::{
    any::Any,
    collections::HashMap,
    env,
    fs::{read_dir, read_link, File, ReadDir},
    io::BufReader,
    io::Read,
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
/// ```no_run
/// use image::DynamicImage;
/// use veloren_common::assets;
///
/// let my_image = assets::load::<DynamicImage>("core.ui.backgrounds.city").unwrap();
/// ```
pub fn load_map<A: Asset + 'static, F: FnOnce(A) -> A>(
    specifier: &str,
    f: F,
) -> Result<Arc<A>, Error> {
    Ok(ASSETS
        .write()
        .unwrap()
        .entry(specifier.to_string())
        .or_insert(Arc::new(f(A::load(specifier)?)))
        .clone()
        .downcast()?)
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
    load(specifier).expect(&format!("Failed loading essential asset: {}", specifier))
}

/// Asset Trait
pub trait Asset: Send + Sync + Sized {
    fn load(specifier: &str) -> Result<Self, Error>;
}

impl Asset for DynamicImage {
    fn load(specifier: &str) -> Result<Self, Error> {
        let mut buf = Vec::new();
        load_from_path(specifier)?.read_to_end(&mut buf)?;
        Ok(image::load_from_memory(&buf).unwrap())
    }
}

impl Asset for DotVoxData {
    fn load(specifier: &str) -> Result<Self, Error> {
        let mut buf = Vec::new();
        load_from_path(specifier)?.read_to_end(&mut buf)?;
        Ok(dot_vox::load_bytes(&buf).unwrap())
    }
}

impl Asset for Value {
    fn load(specifier: &str) -> Result<Self, Error> {
        Ok(serde_json::from_reader(load_from_path(specifier)?).unwrap())
    }
}

// TODO: System to load file from specifiers (e.g.: "core.ui.backgrounds.city").
fn assets_folder() -> PathBuf {
    match std::env::current_exe() {
        Ok(mut exe_path) => {
            exe_path.pop();
            find_folder::Search::Parents(3)
                .of(exe_path)
                .for_folder("assets")
        }
        Err(_) => find_folder::Search::Parents(3).for_folder("assets"),
    }
    .expect("Could not find assets folder")
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
