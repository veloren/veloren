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
    static ref ASSETS: AssetCache =
        AssetCache::new(&*ASSETS_PATH).unwrap();
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
        Self::load(specifier).map(AssetHandle::cloned)
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
        let format = image::ImageFormat::from_extension(ext)
            .ok_or_else(|| format!("Invalid file extension {}", ext))?;
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

/// Return path to repository root by searching 10 directories back
pub fn find_root() -> Option<PathBuf> {
    std::env::current_dir().map_or(None, |path| {
        // If we are in the root, push path
        if path.join(".git").exists() {
            return Some(path);
        }
        // Search .git directory in parent directries
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
    /// 4. Running through cargo (`assets` in workspace root but not always in cwd incase you `cd voxygen && cargo r`)
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

#[warn(clippy::pedantic)]
#[cfg(feature = "asset_tweak")]
pub mod asset_tweak {
    // Return path to repository by searching 10 directories back
    use super::{find_root, fs, Asset, AssetExt, Path, RonLoader};
    use ron::ser::{to_writer_pretty, PrettyConfig};
    use serde::{de::DeserializeOwned, Deserialize, Serialize};

    #[derive(Clone, Deserialize, Serialize)]
    struct AssetTweakWrapper<T>(T);

    impl<T> Asset for AssetTweakWrapper<T>
    where
        T: Clone + Sized + Send + Sync + 'static + DeserializeOwned,
    {
        type Loader = RonLoader;

        const EXTENSION: &'static str = "ron";
    }

    #[must_use]
    /// # Usage
    /// Create file with content which represent tweaked value
    ///
    /// Example if you want to tweak integer value
    /// ```no_run
    /// use veloren_common_assets::asset_tweak;
    /// let x: i32 = asset_tweak::tweak_expect("x");
    /// ```
    /// File needs to look like that
    /// ```text
    /// assets/tweak/x.ron
    /// (5)
    /// ```
    /// Note the parentheses.
    ///
    /// # Panics
    /// 1) If given `asset_specifier` does not exists
    /// 2) If asseet is broken
    pub fn tweak_expect<T>(specifier: &str) -> T
    where
        T: Clone + Sized + Send + Sync + 'static + DeserializeOwned,
    {
        let asset_specifier: &str = &format!("tweak.{}", specifier);
        let handle = <AssetTweakWrapper<T> as AssetExt>::load_expect(asset_specifier);
        let AssetTweakWrapper(value) = handle.read().clone();
        value
    }

    #[must_use]
    /// # Usage
    /// Will create file "assets/tweak/{specifier}.ron" if not exists
    /// and return passed `value`.
    /// If file exists will read a value from such file.
    ///
    /// In release builds (if `debug_assertions` == false) just returns passed
    /// `value`
    ///
    /// Example if you want to tweak integer value
    /// ```no_run
    /// use veloren_common_assets::asset_tweak;
    /// let x: i32 = asset_tweak::tweak_expect_or_create("x", 5);
    /// ```
    /// File needs to look like that
    /// ```text
    /// assets/tweak/x.ron
    /// (5)
    /// ```
    /// Note the parentheses.
    ///
    /// # Panics
    /// 1) If asset is broken
    /// 2) filesystem errors
    pub fn tweak_expect_or_create<T>(specifier: &str, value: T) -> T
    where
        T: Clone + Sized + Send + Sync + 'static + DeserializeOwned + Serialize,
    {
        if cfg!(not(debug_assertions)) {
            return value;
        }

        let root = find_root().expect("failed to discover repository_root");
        let tweak_dir = root.join("assets/tweak/");
        let filename = format!("{}.ron", specifier);

        if Path::new(&tweak_dir.join(&filename)).is_file() {
            let asset_specifier: &str = &format!("tweak.{}", specifier);
            let handle = <AssetTweakWrapper<T> as AssetExt>::load_expect(asset_specifier);
            let AssetTweakWrapper(new_value) = handle.read().clone();

            new_value
        } else {
            fs::create_dir_all(&tweak_dir).expect("failed to create directory for tweak files");
            let f = fs::File::create(tweak_dir.join(&filename)).unwrap_or_else(|err| {
                panic!("failed to create file {:?}. Error: {:?}", &filename, err)
            });
            to_writer_pretty(f, &AssetTweakWrapper(value.clone()), PrettyConfig::new())
                .unwrap_or_else(|err| {
                    panic!("failed to write to file {:?}. Error: {:?}", &filename, err)
                });

            value
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{find_root, tweak_expect, tweak_expect_or_create};
        use serial_test::serial;
        use std::{
            convert::AsRef,
            fmt::Debug,
            fs::{self, File},
            io::Write,
            path::Path,
        };

        struct DirectoryGuard<P>
        where
            P: AsRef<Path>,
        {
            dir: P,
        }

        impl<P> DirectoryGuard<P>
        where
            P: AsRef<Path>,
        {
            fn create(dir: P) -> Self {
                fs::create_dir_all(&dir).expect("failed to create directory");
                Self { dir }
            }
        }

        impl<P> Drop for DirectoryGuard<P>
        where
            P: AsRef<Path>,
        {
            fn drop(&mut self) { fs::remove_dir(&self.dir).expect("failed to remove directory"); }
        }

        struct FileGuard<P>
        where
            P: AsRef<Path> + Debug,
        {
            file: P,
        }

        impl<P> FileGuard<P>
        where
            P: AsRef<Path> + Debug,
        {
            fn create(file: P) -> (Self, File) {
                let f = File::create(&file)
                    .unwrap_or_else(|_| panic!("failed to create file {:?}", &file));
                (Self { file }, f)
            }

            fn hold(file: P) -> Self { Self { file } }
        }

        impl<P> Drop for FileGuard<P>
        where
            P: AsRef<Path> + Debug,
        {
            fn drop(&mut self) {
                fs::remove_file(&self.file)
                    .unwrap_or_else(|_| panic!("failed to create file {:?}", &self.file));
            }
        }

        #[test]
        #[serial]
        fn test_tweaked_string() {
            let root = find_root().expect("failed to discover repository_root");
            let tweak_dir = root.join("assets/tweak/");
            let _dir_guard = DirectoryGuard::create(tweak_dir.clone());

            // define test files
            let from_int = tweak_dir.join("__test_int_tweak.ron");
            let from_string = tweak_dir.join("__test_string_tweak.ron");
            let from_map = tweak_dir.join("__test_map_tweak.ron");

            // setup fs guards
            let (_file_guard1, mut file1) = FileGuard::create(from_int);
            let (_file_guard2, mut file2) = FileGuard::create(from_string);
            let (_file_guard3, mut file3) = FileGuard::create(from_map);

            // write to file and check result
            file1
                .write_all(b"(5)")
                .expect("failed to write to the file");
            let x = tweak_expect::<i32>("__test_int_tweak");
            assert_eq!(x, 5);

            // write to file and check result
            file2
                .write_all(br#"("Hello Zest")"#)
                .expect("failed to write to the file");
            let x = tweak_expect::<String>("__test_string_tweak");
            assert_eq!(x, "Hello Zest".to_owned());

            // write to file and check result
            file3
                .write_all(
                    br#"
        ({
            "wow": 4,
            "such": 5,
        })
        "#,
                )
                .expect("failed to write to the file");
            let x: std::collections::HashMap<String, i32> = tweak_expect("__test_map_tweak");
            let mut map = std::collections::HashMap::new();
            map.insert("wow".to_owned(), 4);
            map.insert("such".to_owned(), 5);
            assert_eq!(x, map);
        }

        #[test]
        #[serial]
        fn test_tweaked_create() {
            let root = find_root().expect("failed to discover repository_root");
            let tweak_dir = root.join("assets/tweak/");

            let test_path1 = tweak_dir.join("__test_int_create.ron");
            let _file_guard1 = FileGuard::hold(&test_path1);
            let x = tweak_expect_or_create("__test_int_create", 5);
            assert_eq!(x, 5);
            assert!(test_path1.is_file());
            // Recheck it loads back correctly
            let x = tweak_expect_or_create("__test_int_create", 5);
            assert_eq!(x, 5);

            let test_path2 = tweak_dir.join("__test_tuple_create.ron");
            let _file_guard2 = FileGuard::hold(&test_path2);
            let (x, y, z) = tweak_expect_or_create("__test_tuple_create", (5.0, 6.0, 7.0));
            assert_eq!((x, y, z), (5.0, 6.0, 7.0));
            // Recheck it loads back correctly
            let (x, y, z) = tweak_expect_or_create("__test_tuple_create", (5.0, 6.0, 7.0));
            assert_eq!((x, y, z), (5.0, 6.0, 7.0));

            // Test that file has stronger priority
            let test_path3 = tweak_dir.join("__test_priority.ron");
            let (_file_guard3, mut file) = FileGuard::create(&test_path3);
            file.write_all(b"(10)")
                .expect("failed to write to the file");
            let x = tweak_expect_or_create("__test_priority", 6);
            assert_eq!(x, 10);
        }
    }
}
