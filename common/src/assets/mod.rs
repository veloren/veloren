use dot_vox::DotVoxData;
use image::DynamicImage;
use lazy_static::lazy_static;
use serde_json::Value;
use std::{
    any::Any,
    collections::HashMap,
    fs::File,
    io::BufReader,
    io::Read,
    path::PathBuf,
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
fn try_open_with_path(name: &str) -> Option<File> {
    debug!("Trying to access \"{}\"", name);
    let abs_path = std::env::current_dir().expect("No current directory?");
    // TODO: don't do this?
    // if it's stupid and it works..,
    [
        "assets".to_string(),
        "../assets".to_string(), /* optimizations */
        "../../assets".to_string(),
        [env!("CARGO_MANIFEST_DIR"), "/../assets"].concat(),
        [env!("CARGO_MANIFEST_DIR"), "/assets"].concat(),
        [env!("CARGO_MANIFEST_DIR"), "/../../assets"].concat(),
        "../../../assets".to_string(),
        [env!("CARGO_MANIFEST_DIR"), "/../../../assets"].concat(),
    ]
    .into_iter()
    .map(|bp| {
        let mut p = abs_path.clone();
        p.push(bp);
        p.push(name);
        p
    })
    .find_map(|ref filename| File::open(filename).ok())
}

pub fn load_from_path(name: &str) -> Result<BufReader<File>, Error> {
    match try_open_with_path(name) {
        Some(mut f) => Ok(BufReader::new(f)),
        None => Err(Error::NotFound(name.to_owned())),
    }
}
