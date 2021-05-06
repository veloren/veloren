//! Load assets (images or voxel data) from files

use dot_vox::DotVoxData;
use image::DynamicImage;
use lazy_static::lazy_static;
use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

pub use assets_manager::{
    asset::Ron,
    loader::{
        self, BincodeLoader, BytesLoader, JsonLoader, LoadFrom, Loader, RonLoader, StringLoader,
    },
    source, Asset, AssetCache, BoxedError, Compound, Error,
};

lazy_static! {
    /// The HashMap where all loaded assets are stored in.
    static ref ASSETS: AssetCache = AssetCache::new(&*ASSETS_PATH).unwrap();
}

pub fn start_hot_reloading() { ASSETS.enhance_hot_reloading(); }

pub type AssetHandle<T> = assets_manager::Handle<'static, T>;
pub type AssetGuard<T> = assets_manager::AssetGuard<'static, T>;
pub type AssetDir<T> = assets_manager::DirReader<'static, T, source::FileSystem>;

/// The Asset trait, which is implemented by all structures that have their data
/// stored in the filesystem.
pub trait AssetExt: Sized + Send + Sync + 'static {
    /// Function used to load assets from the filesystem or the cache.
    /// Example usage:
    /// ```no_run
    /// use veloren_common::assets::{self, AssetExt};
    ///
    /// let my_image = assets::Image::load("core.ui.backgrounds.city").unwrap();
    /// ```
    fn load(specifier: &str) -> Result<AssetHandle<Self>, Error>;

    /// Function used to load assets from the filesystem or the cache and return
    /// a clone.
    fn load_cloned(specifier: &str) -> Result<Self, Error>
    where
        Self: Clone,
    {
        Self::load(specifier).map(AssetHandle::cloned)
    }

    /// Function used to load essential assets from the filesystem or the cache.
    /// It will panic if the asset is not found. Example usage:
    /// ```no_run
    /// use veloren_common::assets::{self, AssetExt};
    ///
    /// let my_image = assets::Image::load_expect("core.ui.backgrounds.city");
    /// ```
    #[track_caller]
    fn load_expect(specifier: &str) -> AssetHandle<Self> {
        Self::load(specifier).unwrap_or_else(|err| {
            panic!(
                "Failed loading essential asset: {} (error={:?})",
                specifier, err
            )
        })
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
}

pub fn load_dir<T: Asset>(specifier: &str) -> Result<AssetDir<T>, Error> {
    Ok(ASSETS.load_dir(specifier)?)
}

impl<T: Compound> AssetExt for T {
    fn load(specifier: &str) -> Result<AssetHandle<Self>, Error> { ASSETS.load(specifier) }

    fn load_owned(specifier: &str) -> Result<Self, Error> { ASSETS.load_owned(specifier) }
}

pub struct Image(pub Arc<DynamicImage>);

impl Image {
    pub fn to_image(&self) -> Arc<DynamicImage> { Arc::clone(&self.0) }
}

pub struct ImageLoader;
impl Loader<Image> for ImageLoader {
    fn load(content: Cow<[u8]>, ext: &str) -> Result<Image, BoxedError> {
        let format = match ext {
            "png" => image::ImageFormat::Png,
            "jpg" => image::ImageFormat::Jpeg,
            _ => return Err("unknown image format".into()),
        };
        let image = image::load_from_memory_with_format(&content, format)?;
        Ok(Image(Arc::new(image)))
    }
}

impl Asset for Image {
    type Loader = ImageLoader;

    const EXTENSIONS: &'static [&'static str] = &["png", "jpg"];
}

pub struct DotVoxAsset(pub DotVoxData);

pub struct DotVoxLoader;
impl Loader<DotVoxAsset> for DotVoxLoader {
    fn load(content: std::borrow::Cow<[u8]>, _: &str) -> Result<DotVoxAsset, BoxedError> {
        let data = dot_vox::load_bytes(&content).map_err(|err| err.to_owned())?;
        Ok(DotVoxAsset(data))
    }
}

impl Asset for DotVoxAsset {
    type Loader = DotVoxLoader;

    const EXTENSION: &'static str = "vox";
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

/// Returns the actual path of the specifier with the extension.
///
/// For directories, give `""` as extension.
pub fn path_of(specifier: &str, ext: &str) -> PathBuf { ASSETS.source().path_of(specifier, ext) }

fn get_dir_files(files: &mut Vec<String>, path: &Path, specifier: &str) -> io::Result<()> {
    for entry in (fs::read_dir(path)?).flatten() {
        let path = entry.path();
        let maybe_stem = path.file_stem().and_then(|stem| stem.to_str());

        if let Some(stem) = maybe_stem {
            let specifier = format!("{}.{}", specifier, stem);

            if path.is_dir() {
                get_dir_files(files, &path, &specifier)?;
            } else {
                files.push(specifier);
            }
        }
    }

    Ok(())
}

pub struct Directory(Vec<String>);

impl Directory {
    pub fn iter(&self) -> impl Iterator<Item = &String> { self.0.iter() }
}

impl Compound for Directory {
    fn load<S: source::Source>(_: &AssetCache<S>, specifier: &str) -> Result<Self, Error> {
        let specifier = specifier.strip_suffix(".*").unwrap_or(specifier);
        let root = ASSETS.source().path_of(specifier, "");
        let mut files = Vec::new();

        get_dir_files(&mut files, &root, specifier)?;

        Ok(Directory(files))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_assets_items() {
        // TODO: Figure out how to get file name in error so only a single glob is
        // needed

        // Separated out into subsections so that error more descriptive
        crate::comp::item::Item::new_from_asset_glob("common.items.armor.*")
            .expect("Failed to iterate over armors.");

        crate::comp::item::Item::new_from_asset_glob("common.items.boss_drops.*")
            .expect("Failed to iterate over boss drops.");

        crate::comp::item::Item::new_from_asset_glob("common.items.consumable.*")
            .expect("Failed to iterate over consumables.");

        crate::comp::item::Item::new_from_asset_glob("common.items.crafting_ing.*")
            .expect("Failed to iterate over crafting ingredients.");

        crate::comp::item::Item::new_from_asset_glob("common.items.crafting_tools.*")
            .expect("Failed to iterate over crafting tools.");

        crate::comp::item::Item::new_from_asset_glob("common.items.debug.*")
            .expect("Failed to iterate over debug items.");

        crate::comp::item::Item::new_from_asset_glob("common.items.flowers.*")
            .expect("Failed to iterate over flower items.");

        crate::comp::item::Item::new_from_asset_glob("common.items.food.*")
            .expect("Failed to iterate over food items.");

        crate::comp::item::Item::new_from_asset_glob("common.items.glider.*")
            .expect("Failed to iterate over gliders.");

        crate::comp::item::Item::new_from_asset_glob("common.items.grasses.*")
            .expect("Failed to iterate over grasses.");

        crate::comp::item::Item::new_from_asset_glob("common.items.lantern.*")
            .expect("Failed to iterate over lanterns.");

        crate::comp::item::Item::new_from_asset_glob("common.items.npc_armor.*")
            .expect("Failed to iterate over npc armors.");

        crate::comp::item::Item::new_from_asset_glob("common.items.npc_weapons.*")
            .expect("Failed to iterate over npc weapons.");

        crate::comp::item::Item::new_from_asset_glob("common.items.ore.*")
            .expect("Failed to iterate over ores.");

        crate::comp::item::Item::new_from_asset_glob("common.items.tag_examples.*")
            .expect("Failed to iterate over tag examples.");

        crate::comp::item::Item::new_from_asset_glob("common.items.testing.*")
            .expect("Failed to iterate over testing items.");

        crate::comp::item::Item::new_from_asset_glob("common.items.utility.*")
            .expect("Failed to iterate over utility items.");

        // Checks each weapon type to allow errors to be located more easily
        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.axe.*")
            .expect("Failed to iterate over axes.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.axe_1h.*")
            .expect("Failed to iterate over 1h axes.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.bow.*")
            .expect("Failed to iterate over bows.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.dagger.*")
            .expect("Failed to iterate over daggers.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.empty.*")
            .expect("Failed to iterate over empty.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.hammer.*")
            .expect("Failed to iterate over hammers.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.hammer_1h.*")
            .expect("Failed to iterate over 1h hammers.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.sceptre.*")
            .expect("Failed to iterate over sceptres.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.shield.*")
            .expect("Failed to iterate over shields.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.staff.*")
            .expect("Failed to iterate over staffs.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.sword.*")
            .expect("Failed to iterate over swords.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.sword_1h.*")
            .expect("Failed to iterate over 1h swords.");

        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.tool.*")
            .expect("Failed to iterate over tools.");

        // Checks all weapons should more weapons be added later
        crate::comp::item::Item::new_from_asset_glob("common.items.weapons.*")
            .expect("Failed to iterate over weapons.");

        // Final at the end to account for a new folder being added
        crate::comp::item::Item::new_from_asset_glob("common.items.*")
            .expect("Failed to iterate over item folders.");
    }
}
