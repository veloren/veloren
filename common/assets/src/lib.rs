//#![warn(clippy::pedantic)]
//! Load assets (images or voxel data) from files

use dot_vox::DotVoxData;
use image::DynamicImage;
use lazy_static::lazy_static;
use std::{borrow::Cow, path::PathBuf, sync::Arc};

pub use assets_manager::{
    asset::{DirLoadable, Ron},
    loader::{
        self, BincodeLoader, BytesLoader, JsonLoader, LoadFrom, Loader, RonLoader, StringLoader,
    },
    source::{self, Source},
    AnyCache, Asset, AssetCache, BoxedError, Compound, Error, SharedString,
};

mod fs;
mod walk;
pub use walk::{walk_tree, Walk};

lazy_static! {
    /// The HashMap where all loaded assets are stored in.
    static ref ASSETS: AssetCache<fs::FileSystem> =
        AssetCache::with_source(fs::FileSystem::new().unwrap());
}

#[cfg(feature = "hot-reloading")]
pub fn start_hot_reloading() { ASSETS.enhance_hot_reloading(); }

pub type AssetHandle<T> = assets_manager::Handle<'static, T>;
pub type AssetGuard<T> = assets_manager::AssetGuard<'static, T>;
pub type AssetDirHandle<T> = assets_manager::DirHandle<'static, T>;
pub type ReloadWatcher = assets_manager::ReloadWatcher<'static>;

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

    fn load_or_insert_with(
        specifier: &str,
        default: impl FnOnce(Error) -> Self,
    ) -> AssetHandle<Self> {
        Self::load(specifier).unwrap_or_else(|err| Self::get_or_insert(specifier, default(err)))
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
        #[track_caller]
        #[cold]
        fn expect_failed(err: Error) -> ! {
            panic!(
                "Failed loading essential asset: {} (error={:?})",
                err.id(),
                err.reason()
            )
        }

        // Avoid using `unwrap_or_else` to avoid breaking `#[track_caller]`
        match Self::load(specifier) {
            Ok(handle) => handle,
            Err(err) => expect_failed(err),
        }
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

    fn get_or_insert(specifier: &str, default: Self) -> AssetHandle<Self>;
}

/// Loads directory and all files in it
///
/// # Errors
/// An error is returned if the given id does not match a valid readable
/// directory.
///
/// When loading a directory recursively, directories that can't be read are
/// ignored.
pub fn load_dir<T: DirLoadable>(
    specifier: &str,
    recursive: bool,
) -> Result<AssetDirHandle<T>, Error> {
    let specifier = specifier.strip_suffix(".*").unwrap_or(specifier);
    ASSETS.load_dir(specifier, recursive)
}

/// Loads directory and all files in it
///
/// # Panics
/// 1) If can't load directory (filesystem errors)
/// 2) If file can't be loaded (parsing problem)
#[track_caller]
pub fn read_expect_dir<T: DirLoadable + Compound>(
    specifier: &str,
    recursive: bool,
) -> impl Iterator<Item = AssetGuard<T>> {
    #[track_caller]
    #[cold]
    fn expect_failed(err: Error) -> ! {
        panic!(
            "Failed loading directory: {} (error={:?})",
            err.id(),
            err.reason()
        )
    }

    // Avoid using `unwrap_or_else` to avoid breaking `#[track_caller]`
    match load_dir::<T>(specifier, recursive) {
        Ok(dir) => dir.ids().map(|entry| T::load_expect(entry).read()),
        Err(err) => expect_failed(err),
    }
}

impl<T: Compound> AssetExt for T {
    fn load(specifier: &str) -> Result<AssetHandle<Self>, Error> { ASSETS.load(specifier) }

    fn load_owned(specifier: &str) -> Result<Self, Error> { ASSETS.load_owned(specifier) }

    fn get_or_insert(specifier: &str, default: Self) -> AssetHandle<Self> {
        ASSETS.get_or_insert(specifier, default)
    }
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
    fn load(content: Cow<[u8]>, _: &str) -> Result<DotVoxAsset, BoxedError> {
        let data = dot_vox::load_bytes(&content).map_err(|err| err.to_owned())?;
        Ok(DotVoxAsset(data))
    }
}

impl Asset for DotVoxAsset {
    type Loader = DotVoxLoader;

    const EXTENSION: &'static str = "vox";
}

pub struct ObjAsset(pub wavefront::Obj);

impl Asset for ObjAsset {
    type Loader = ObjAssetLoader;

    const EXTENSION: &'static str = "obj";
}

pub struct ObjAssetLoader;
impl Loader<ObjAsset> for ObjAssetLoader {
    fn load(content: Cow<[u8]>, _: &str) -> Result<ObjAsset, BoxedError> {
        let data = wavefront::Obj::from_reader(&*content)?;
        Ok(ObjAsset(data))
    }
}

/// Return path to repository root by searching 10 directories back
pub fn find_root() -> Option<PathBuf> {
    std::env::current_dir().map_or(None, |path| {
        // If we are in the root, push path
        if path.join(".git").exists() {
            return Some(path);
        }
        // Search .git directory in parent directories
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
    /// 4. Running through cargo (`assets` in workspace root but not always in cwd in case you `cd voxygen && cargo r`)
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

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, fs::File};
    use walkdir::WalkDir;

    #[test]
    fn load_canary() {
        // Loading the asset cache will automatically cause the canary to load
        let _ = *super::ASSETS;
    }

    /// Fail unless all `.ron` asset files successfully parse to `ron::Value`.
    #[test]
    fn parse_all_ron_files_to_value() {
        let ext = OsStr::new("ron");
        WalkDir::new(crate::ASSETS_PATH.as_path())
            .into_iter()
            .map(|ent| {
                ent.expect("Failed to walk over asset directory")
                    .into_path()
            })
            .filter(|path| path.is_file())
            .filter(|path| {
                path.extension()
                    .map_or(false, |e| ext == e.to_ascii_lowercase())
            })
            .for_each(|path| {
                let file = File::open(&path).expect("Failed to open the file");
                if let Err(err) = ron::de::from_reader::<_, ron::Value>(file) {
                    println!("{:?}", path);
                    println!("{:#?}", err);
                    panic!("Parse failed");
                }
            });
    }
}

#[cfg(feature = "asset_tweak")]
pub mod asset_tweak {
    //! Set of functions and macros for easy tweaking values
    //! using our asset cache machinery.
    //!
    //! Because of how macros works, you will not find
    //! [tweak] and [tweak_from] macros in this module,
    //! import it from [assets](super) crate directly.
    //!
    //! Will hot-reload (if corresponded feature is enabled).
    // TODO: don't use the same ASSETS_PATH as game uses?
    use super::{Asset, AssetExt, RonLoader, ASSETS_PATH};
    use ron::ser::{to_writer_pretty, PrettyConfig};
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use std::{fs, path::Path};

    /// Specifier to use with tweak functions in this module
    ///
    /// `Tweak("test")` will be interpreted as `<assets_dir>/tweak/test.ron`.
    ///
    /// `Asset(&["path", "to", "file"])` will be interpreted as
    /// `<assets_dir>/path/to/file.ron`
    pub enum Specifier<'a> {
        Tweak(&'a str),
        Asset(&'a [&'a str]),
    }

    #[derive(Clone, Deserialize, Serialize)]
    struct AssetTweakWrapper<T>(T);

    impl<T> Asset for AssetTweakWrapper<T>
    where
        T: Clone + Sized + Send + Sync + 'static + DeserializeOwned,
    {
        type Loader = RonLoader;

        const EXTENSION: &'static str = "ron";
    }

    /// Read value from file, will panic if file doesn't exist.
    ///
    /// If you don't have a file or its content is invalid,
    /// this function will panic.
    /// If you want to have some default content,
    /// read documentation for [tweak_expect_or_create] for more.
    ///
    /// # Examples:
    /// How not to use.
    /// ```should_panic
    /// use veloren_common_assets::asset_tweak::{tweak_expect, Specifier};
    ///
    /// // will panic if you don't have a file
    /// let specifier = Specifier::Asset(&["no_way_we_have_this_directory", "x"]);
    /// let x: i32 = tweak_expect(specifier);
    /// ```
    ///
    /// How to use.
    /// ```
    /// use std::fs;
    /// use veloren_common_assets::{
    ///     asset_tweak::{tweak_expect, Specifier},
    ///     ASSETS_PATH,
    /// };
    ///
    /// // you need to create file first
    /// let tweak_path = ASSETS_PATH.join("tweak/year.ron");
    /// // note parentheses
    /// fs::write(&tweak_path, b"(10)");
    ///
    /// let y: i32 = tweak_expect(Specifier::Tweak("year"));
    /// assert_eq!(y, 10);
    ///
    /// // Specifier::Tweak is just a shorthand
    /// // for Specifier::Asset(&["tweak", ..])
    /// let y1: i32 = tweak_expect(Specifier::Asset(&["tweak", "year"]));
    /// assert_eq!(y1, 10);
    ///
    /// // you may want to remove this file later
    /// fs::remove_file(tweak_path);
    /// ```
    pub fn tweak_expect<T>(specifier: Specifier) -> T
    where
        T: Clone + Sized + Send + Sync + 'static + DeserializeOwned,
    {
        let asset_specifier = match specifier {
            Specifier::Tweak(specifier) => format!("tweak.{}", specifier),
            Specifier::Asset(path) => path.join("."),
        };
        let handle = <AssetTweakWrapper<T> as AssetExt>::load_expect(&asset_specifier);
        let AssetTweakWrapper(value) = handle.cloned();

        value
    }

    // Helper function to create new file to tweak.
    //
    // The file will be filled with passed value
    // returns passed value.
    fn create_new<T>(tweak_dir: &Path, filename: &str, value: T) -> T
    where
        T: Sized + Send + Sync + 'static + DeserializeOwned + Serialize,
    {
        fs::create_dir_all(tweak_dir).expect("failed to create directory for tweak files");
        let f = fs::File::create(tweak_dir.join(filename)).unwrap_or_else(|error| {
            panic!("failed to create file {:?}. Error: {:?}", filename, error)
        });
        let tweaker = AssetTweakWrapper(&value);
        if let Err(e) = to_writer_pretty(f, &tweaker, PrettyConfig::new()) {
            panic!("failed to write to file {:?}. Error: {:?}", filename, e);
        }

        value
    }

    // Helper function to get directory and file from asset list.
    //
    // Converts ["path", "to", "file"] to (String("path/to"), "file")
    fn directory_and_name<'a>(path: &'a [&'a str]) -> (String, &'a str) {
        let (file, path) = path.split_last().expect("empty asset list");
        let directory = path.join("/");

        (directory, file)
    }

    /// Read a value from asset, creating file if not exists.
    ///
    /// If file exists will read a value from such file
    /// using [tweak_expect].
    ///
    /// File should look like that (note the parentheses).
    /// ```text
    /// assets/tweak/x.ron
    /// (5)
    /// ```
    ///
    /// # Example:
    /// Tweaking integer value
    /// ```
    /// use veloren_common_assets::{
    ///     asset_tweak::{tweak_expect_or_create, Specifier},
    ///     ASSETS_PATH,
    /// };
    ///
    /// // first time it will create the file
    /// let x: i32 = tweak_expect_or_create(Specifier::Tweak("stars"), 5);
    /// let file_path = ASSETS_PATH.join("tweak/stars.ron");
    /// assert!(file_path.is_file());
    /// assert_eq!(x, 5);
    ///
    /// // next time it will read value from file
    /// // whatever you will pass as default
    /// let x1: i32 = tweak_expect_or_create(Specifier::Tweak("stars"), 42);
    /// assert_eq!(x1, 5);
    ///
    /// // you may want to remove this file later
    /// std::fs::remove_file(file_path);
    /// ```
    pub fn tweak_expect_or_create<T>(specifier: Specifier, value: T) -> T
    where
        T: Clone + Sized + Send + Sync + 'static + DeserializeOwned + Serialize,
    {
        let (dir, filename) = match specifier {
            Specifier::Tweak(name) => (ASSETS_PATH.join("tweak"), format!("{}.ron", name)),
            Specifier::Asset(list) => {
                let (directory, name) = directory_and_name(list);
                (ASSETS_PATH.join(directory), format!("{}.ron", name))
            },
        };

        if Path::new(&dir.join(&filename)).is_file() {
            tweak_expect(specifier)
        } else {
            create_new(&dir, &filename, value)
        }
    }

    /// Convenient macro to quickly tweak value.
    ///
    /// Will use [Specifier]`::Tweak` specifier and call
    /// [tweak_expect] if passed only name
    /// or [tweak_expect_or_create] if default is passed.
    ///
    /// # Examples:
    /// ```
    /// // note that you need to export it from `assets` crate,
    /// // not from `assets::asset_tweak`
    /// use veloren_common_assets::{tweak, ASSETS_PATH};
    ///
    /// // you need to create file first
    /// let own_path = ASSETS_PATH.join("tweak/grizelda.ron");
    /// // note parentheses
    /// std::fs::write(&own_path, b"(10)");
    ///
    /// let z: i32 = tweak!("grizelda");
    /// assert_eq!(z, 10);
    ///
    /// // voila, you don't need to care about creating file first
    /// let p: i32 = tweak!("peter", 8);
    ///
    /// let created_path = ASSETS_PATH.join("tweak/peter.ron");
    /// assert!(created_path.is_file());
    /// assert_eq!(p, 8);
    ///
    /// // will use default value only first time
    /// // if file exists, will load from this file
    /// let p: i32 = tweak!("peter", 50);
    /// assert_eq!(p, 8);
    ///
    /// // you may want to remove this file later
    /// std::fs::remove_file(own_path);
    /// std::fs::remove_file(created_path);
    /// ```
    #[macro_export]
    macro_rules! tweak {
        ($name:literal) => {{
            use $crate::asset_tweak::{tweak_expect, Specifier::Tweak};

            tweak_expect(Tweak($name))
        }};

        ($name:literal, $default:expr) => {{
            use $crate::asset_tweak::{tweak_expect_or_create, Specifier::Tweak};

            tweak_expect_or_create(Tweak($name), $default)
        }};
    }

    /// Convenient macro to quickly tweak value from some existing path.
    ///
    /// Will use [Specifier]`::Asset` specifier and call
    /// [tweak_expect] if passed only name
    /// or [tweak_expect_or_create] if default is passed.
    ///
    /// The main use case is when you have some object
    /// which needs constant tuning of values, but you can't afford
    /// loading a file.
    /// So you can use tweak_from! and then just copy values from asset
    /// to your object.
    ///
    /// # Examples:
    /// ```no_run
    /// // note that you need to export it from `assets` crate,
    /// // not from `assets::asset_tweak`
    /// use serde::{Deserialize, Serialize};
    /// use veloren_common_assets::{tweak_from, ASSETS_PATH};
    ///
    /// #[derive(Clone, PartialEq, Deserialize, Serialize)]
    /// struct Data {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    /// let default = Data { x: 5, y: 7 };
    /// let data: Data = tweak_from!(&["common", "body", "dimensions"], default);
    /// ```
    #[macro_export]
    macro_rules! tweak_from {
        ($path:expr) => {{
            use $crate::asset_tweak::{tweak_expect, Specifier::Asset};

            tweak_expect(Asset($path))
        }};

        ($path:expr, $default:expr) => {{
            use $crate::asset_tweak::{tweak_expect_or_create, Specifier::Asset};

            tweak_expect_or_create(Asset($path), $default)
        }};
    }

    #[cfg(test)]
    mod tests {
        use super::*;
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
                fs::remove_file(&self.file).unwrap_or_else(|e| {
                    panic!("failed to remove file {:?}. Error: {:?}", &self.file, e)
                });
            }
        }

        // helper function to create environment with needed directory and file
        // and responsible for cleaning
        fn run_with_file(tweak_path: &[&str], test: impl Fn(&mut File)) {
            let (tweak_dir, tweak_name) = directory_and_name(tweak_path);
            let tweak_folder = ASSETS_PATH.join(tweak_dir);
            let tweak_file = tweak_folder.join(format!("{}.ron", tweak_name));

            let _dir_guard = DirectoryGuard::create(tweak_folder);
            let (_file_guard, mut file) = FileGuard::create(tweak_file);

            test(&mut file);
        }

        #[test]
        fn test_tweaked_int() {
            let tweak_path = &["tweak_test_int", "tweak"];

            run_with_file(tweak_path, |file| {
                file.write_all(b"(5)").expect("failed to write to the file");
                let x: i32 = tweak_expect(Specifier::Asset(tweak_path));
                assert_eq!(x, 5);
            });
        }

        #[test]
        fn test_tweaked_string() {
            let tweak_path = &["tweak_test_string", "tweak"];

            run_with_file(tweak_path, |file| {
                file.write_all(br#"("Hello Zest")"#)
                    .expect("failed to write to the file");

                let x: String = tweak_expect(Specifier::Asset(tweak_path));
                assert_eq!(x, "Hello Zest".to_owned());
            });
        }

        #[test]
        fn test_tweaked_hashmap() {
            type Map = std::collections::HashMap<String, i32>;

            let tweak_path = &["tweak_test_map", "tweak"];

            run_with_file(tweak_path, |file| {
                file.write_all(
                    br#"
                    ({
                        "wow": 4,
                        "such": 5,
                    })
                    "#,
                )
                .expect("failed to write to the file");

                let x: Map = tweak_expect(Specifier::Asset(tweak_path));

                let mut map = Map::new();
                map.insert("wow".to_owned(), 4);
                map.insert("such".to_owned(), 5);
                assert_eq!(x, map);
            });
        }

        #[test]
        fn test_tweaked_with_macro_struct() {
            // partial eq and debug because of assert_eq in this test
            #[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
            struct Wow {
                such: i32,
                field: f32,
            }

            let tweak_path = &["tweak_test_struct", "tweak"];

            run_with_file(tweak_path, |file| {
                file.write_all(
                    br#"
                    ((
                        such: 5,
                        field: 35.752346,
                    ))
                    "#,
                )
                .expect("failed to write to the file");

                let x: Wow = crate::tweak_from!(tweak_path);
                let expected = Wow {
                    such: 5,
                    field: 35.752_346,
                };
                assert_eq!(x, expected);
            });
        }

        fn run_with_path(tweak_path: &[&str], test: impl Fn(&Path)) {
            let (tweak_dir, tweak_name) = directory_and_name(tweak_path);

            let tweak_folder = ASSETS_PATH.join(tweak_dir);
            let test_path = tweak_folder.join(format!("{}.ron", tweak_name));

            let _file_guard = FileGuard::hold(&test_path);

            test(&test_path);
        }

        #[test]
        fn test_create_tweak() {
            let tweak_path = &["tweak_create_test", "tweak"];

            run_with_path(tweak_path, |test_path| {
                let x = tweak_expect_or_create(Specifier::Asset(tweak_path), 5);
                assert_eq!(x, 5);
                assert!(test_path.is_file());
                // Recheck it loads back correctly
                let x = tweak_expect_or_create(Specifier::Asset(tweak_path), 5);
                assert_eq!(x, 5);
            });
        }

        #[test]
        fn test_create_tweak_deep() {
            let tweak_path = &["so_much", "deep_test", "tweak_create_test", "tweak"];

            run_with_path(tweak_path, |test_path| {
                let x = tweak_expect_or_create(Specifier::Asset(tweak_path), 5);
                assert_eq!(x, 5);
                assert!(test_path.is_file());
                // Recheck it loads back correctly
                let x = tweak_expect_or_create(Specifier::Asset(tweak_path), 5);
                assert_eq!(x, 5);
            });
        }

        #[test]
        fn test_create_but_prioritize_loaded() {
            let tweak_path = &["tweak_create_and_prioritize_test", "tweak"];

            run_with_path(tweak_path, |test_path| {
                let x = tweak_expect_or_create(Specifier::Asset(tweak_path), 5);
                assert_eq!(x, 5);
                assert!(test_path.is_file());

                // Recheck it loads back
                // with content as priority
                fs::write(test_path, b"(10)").expect("failed to write to the file");
                let x = tweak_expect_or_create(Specifier::Asset(tweak_path), 5);
                assert_eq!(x, 10);
            });
        }
    }
}
