use dot_vox::DotVoxData;
use image::DynamicImage;
use lazy_static::lazy_static;
use std::{
    any::Any,
    collections::HashMap,
    fs::File,
    io::Read,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
pub enum Error {
    /// An asset has already been loaded with this specifier but anot type
    InvalidType,
    /// Asset does not exist
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

/// Function used to load assets
/// loaded assets are cached in a global singleton hashmap
/// Example usage:
/// ```
/// use image::DynamicImage;
/// use common::assets;
/// 
/// let my_image = assets::load::<DynamicImage>("core.ui.backgrounds.city").unwrap();
/// ```
pub fn load<A: Asset + 'static>(specifier: &str) -> Result<Arc<A>, Error> {
    Ok(ASSETS
        .write().unwrap()
        .entry(specifier.to_string())
        .or_insert(Arc::new(A::load(specifier)?))
        .clone()
        .downcast()?)
}

/// Function used to load assets that will panic if the asset is not found
/// Use this to load essential assets
/// loaded assets are cached in a global singleton hashmap
/// Example usage:
/// ```
/// use image::DynamicImage;
/// use common::assets;
/// 
/// let my_image = assets::load_expect::<DynamicImage>("core.ui.backgrounds.city");
/// ```
pub fn load_expect<A: Asset + 'static>(specifier: &str) -> Arc<A> {
    load(specifier)
        .expect(&format!("Failed loading essential asset: {}", specifier))
}

/// Asset Trait
pub trait Asset: Send + Sync + Sized {
    fn load(specifier: &str) -> Result<Self, Error>;
}

impl Asset for DynamicImage {
    fn load(specifier: &str) -> Result<Self, Error> {
        Ok(image::load_from_memory(
                load_from_path(specifier)?.as_slice()
            )
            .unwrap()
        )
    }
}

impl Asset for DotVoxData {
    fn load(specifier: &str) -> Result<Self, Error> {
        Ok(dot_vox::load_bytes(
                load_from_path(specifier)?.as_slice()
            )
            .unwrap()
        )
    }
}

// TODO: System to load file from specifiers (eg "core.ui.backgrounds.city")
fn try_open_with_path(name: &str) -> Option<File> {
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
    .map(|bp| [bp, name].concat())
    .find_map(|ref filename| File::open(filename).ok())
}

pub fn load_from_path(name: &str) -> Result<Vec<u8>, Error> {
    match try_open_with_path(name) {
        Some(mut f) => {
            let mut content = Vec::<u8>::new();
            f.read_to_end(&mut content)?;
            Ok(content)
        },
        None => {
            Err(Error::NotFound(name.to_owned()))
        }
    }
}

/*
/// Translation Asset
pub struct Translations {
    pub translations: Value
}
impl Translations {
    pub fn get_lang(&self, lang: &str) -> &str {
        self.translations[lang].as_str().unwrap()
    }
}
impl Asset for Translations {
    type T=Self;
    fn load(path: &str) -> Result<Self, ()>{
        let file_out = read_from_path(path).unwrap();
        let content = from_utf8(file_out.as_slice()).unwrap();
        let value = content.parse::<Value>().unwrap();

        Ok(Translations{translations: value})
    }
}
*/
