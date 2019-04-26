use dot_vox::DotVoxData;
use image::DynamicImage;
use lazy_static::lazy_static;
use std::{
    any::Any,
    collections::HashMap,
    fs::File,
    io::Read,
    sync::{Arc, PoisonError, RwLock},
};
use crate::figure::Segment;

#[derive(Debug, Clone)]
pub enum Error {
    InvalidType,
    NotFound,
    Poison,
}

impl From<Arc<dyn Any + 'static + Sync + Send>> for Error {
    fn from(_err: Arc<dyn Any + 'static + Sync + Send>) -> Self {
        Error::InvalidType
    }
}

impl From<std::io::Error> for Error {
    fn from(_err: std::io::Error) -> Self {
        Error::NotFound
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_err: PoisonError<T>) -> Self {
        Error::Poison
    }
}

lazy_static! {
    static ref ASSETS: RwLock<HashMap<String, Arc<dyn Any + 'static + Sync + Send>>> =
        RwLock::new(HashMap::new());
}

/// Function used to load assets
/// Example usage:
/// ```
/// use image::DynamicImage;
/// 
/// let my_image = common::asset::load::<DynamicImage>("core.ui.backgrounds.city").unwrap();
/// ```
// TODO: consider assets that we only need in one place or that don't need to be kept in memory?
pub fn load<A: Asset + 'static>(specifier: &str) -> Result<Arc<A>, Error> {
    Ok(ASSETS
        .write()?
        .entry(specifier.to_string())
        .or_insert(Arc::new(A::load(specifier)?))
        .clone()
        .downcast()?)
}

/// Asset Trait
pub trait Asset: Send + Sync + Sized {
    fn load(specifier: &str) -> Result<Self, Error>;
}

impl Asset for DynamicImage {
    fn load(specifier: &str) -> Result<Self, Error> {
        Ok(image::load_from_memory(load_from_path(specifier)?.as_slice()).unwrap())
    }
}

impl Asset for DotVoxData {
    fn load(specifier: &str) -> Result<Self, Error> {
        Ok(dot_vox::load_bytes(load_from_path(specifier)?.as_slice()).unwrap())
    }
}

impl Asset for Segment {
    fn load(specifier: &str) -> Result<Self, Error> {
        Ok(Segment::from(dot_vox::load_bytes(load_from_path(specifier)?.as_slice()).unwrap()))
    }
}

// TODO: System to load file from specifiers (eg "core.ui.backgrounds.city")
fn try_load_from_path(name: &str) -> Option<File> {
    let basepaths = [
        // if it's stupid and it works..,
        "assets".to_string(),
        "../../assets".to_string(),
        "../assets".to_string(), /* optimizations */
        [env!("CARGO_MANIFEST_DIR"), "/assets"].concat(),
        [env!("CARGO_MANIFEST_DIR"), "/../../assets"].concat(),
        [env!("CARGO_MANIFEST_DIR"), "/../assets"].concat(),
        "../../../assets".to_string(),
        [env!("CARGO_MANIFEST_DIR"), "/../../../assets"].concat(),
    ];
    for bp in &basepaths {
        let filename = [bp, name].concat();
        match File::open(&filename) {
            Ok(f) => {
                debug!("loading {} succedeed", filename);
                return Some(f);
            },
            Err(e) => {
                debug!("loading {} did not work with error: {}", filename, e);
            }
        };
    };
    None
}

pub fn load_from_path(name: &str) -> Result<Vec<u8>, Error> {
    match try_load_from_path(name) {
        Some(mut f) => {
            let mut content: Vec<u8> = vec!();
            f.read_to_end(&mut content);
            info!("loaded asset successful: {}", name);
            Ok(content)
        },
        None => {
            warn!("Loading asset failed, wanted to load {} but could not load it, check debug log!", name);
            Err(Error::NotFound)
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
