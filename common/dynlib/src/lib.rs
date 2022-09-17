use libloading::Library;
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use std::{
    process::{Command, Stdio},
    sync::{mpsc, Mutex},
    time::Duration,
};

use find_folder::Search;
use std::{
    env,
    env::consts::{DLL_PREFIX, DLL_SUFFIX},
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::{debug, error, info};

// Re-exports
pub use libloading::Symbol;

/// LoadedLib holds a loaded dynamic library and the location of library file
/// with the appropriate OS specific name and extension i.e.
/// `libvoxygen_anim_dyn_active.dylib`, `voxygen_anim_dyn_active.dll`.
///
/// # NOTE
/// DOES NOT WORK ON MACOS, due to some limitations with hot-reloading the
/// `.dylib`.
pub struct LoadedLib {
    /// Loaded library.
    pub lib: Library,
    /// Path to the library.
    pub lib_path: PathBuf,
}

impl LoadedLib {
    /// Compile and load the dynamic library
    ///
    /// This is necessary because the very first time you use hot reloading you
    /// wont have the library, so you can't load it until you have compiled it!
    fn compile_load(dyn_package: &str) -> Self {
        #[cfg(target_os = "macos")]
        error!("The hot reloading feature does not work on macos.");

        // Compile
        if !compile(dyn_package) {
            panic!("{} compile failed.", dyn_package);
        } else {
            info!("{} compile succeeded.", dyn_package);
        }

        copy(&LoadedLib::determine_path(dyn_package), dyn_package);

        Self::load(dyn_package)
    }

    /// Load a library from disk.
    ///
    /// Currently this is pretty fragile, it gets the path of where it thinks
    /// the dynamic library should be and tries to load it. It will panic if it
    /// is missing.
    fn load(dyn_package: &str) -> Self {
        let lib_path = LoadedLib::determine_path(dyn_package);

        // Try to load the library.
        let lib = match unsafe { Library::new(lib_path.clone()) } {
            Ok(lib) => lib,
            Err(e) => panic!(
                "Tried to load dynamic library from {:?}, but it could not be found. A potential \
                 reason is we may require a special case for your OS so we can find it. {:?}",
                lib_path, e
            ),
        };

        Self { lib, lib_path }
    }

    /// Determine the path to the dynamic library based on the path of the
    /// current executable.
    fn determine_path(dyn_package: &str) -> PathBuf {
        let current_exe = env::current_exe();

        // If we got the current_exe, we need to go up a level and then down
        // in to debug (in case we were in release or another build dir).
        let mut lib_path = match current_exe {
            Ok(mut path) => {
                // Remove the filename to get the directory.
                path.pop();

                // Search for the debug directory.
                let dir = Search::ParentsThenKids(1, 1)
                    .of(path)
                    .for_folder("debug")
                    .expect(
                        "Could not find the debug build directory relative to the current \
                         executable.",
                    );

                debug!(?dir, "Found the debug build directory.");
                dir
            },
            Err(e) => {
                panic!(
                    "Could not determine the path of the current executable, this is needed to \
                     hot-reload the dynamic library. {:?}",
                    e
                );
            },
        };

        // Determine the platform specific path and push it onto our already
        // established target/debug dir.
        lib_path.push(active_file(dyn_package));

        lib_path
    }
}

/// Initialise a watcher.
///
/// This will search for the directory named `package_source_dir` and watch the
/// files within it for any changes.
pub fn init(
    package: &'static str,
    package_source_dir: &'static str,
) -> Arc<Mutex<Option<LoadedLib>>> {
    let lib_storage = Arc::new(Mutex::new(Some(LoadedLib::compile_load(package))));

    // TODO: use crossbeam
    let (reload_send, reload_recv) = mpsc::channel();

    // Start watcher
    let mut watcher = recommended_watcher(move |res| event_fn(res, &reload_send)).unwrap();

    // Search for the source directory of the package being hot-reloaded.
    let watch_dir = Search::Kids(1)
        .for_folder(package_source_dir)
        .unwrap_or_else(|_| {
            panic!(
                "Could not find the {} crate directory relative to the current directory",
                package_source_dir
            )
        });

    watcher.watch(&watch_dir, RecursiveMode::Recursive).unwrap();

    // Start reloader that watcher signals
    // "Debounces" events since I can't find the option to do this in the latest
    // `notify`
    let lib_storage_clone = Arc::clone(&lib_storage);
    std::thread::Builder::new()
        .name(format!("{}_hotreload_watcher", package))
        .spawn(move || {
            let mut modified_paths = std::collections::HashSet::new();
            while let Ok(path) = reload_recv.recv() {
                modified_paths.insert(path);
                // Wait for any additional modify events before reloading
                while let Ok(path) = reload_recv.recv_timeout(Duration::from_millis(300)) {
                    modified_paths.insert(path);
                }

                info!(
                    ?modified_paths,
                    "Hot reloading {} because files in `{}` modified.", package, package_source_dir
                );

                hotreload(package, &lib_storage_clone);
            }
        })
        .unwrap();

    // Let the watcher live forever
    std::mem::forget(watcher);

    lib_storage
}

fn compiled_file(dyn_package: &str) -> String { dyn_lib_file(dyn_package, false) }

fn active_file(dyn_package: &str) -> String { dyn_lib_file(dyn_package, true) }

fn dyn_lib_file(dyn_package: &str, active: bool) -> String {
    format!(
        "{}{}{}{}",
        DLL_PREFIX,
        dyn_package.replace('-', "_"),
        if active { "_active" } else { "" },
        DLL_SUFFIX
    )
}

/// Event function to hotreload the dynamic library
///
/// This is called by the watcher to filter for modify events on `.rs` files
/// before sending them back.
fn event_fn(res: notify::Result<notify::Event>, sender: &mpsc::Sender<String>) {
    match res {
        Ok(event) => {
            if let EventKind::Modify(_) = event.kind {
                event
                    .paths
                    .iter()
                    .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
                    .map(|p| p.to_string_lossy().into_owned())
                    // Signal reloader
                    .for_each(|p| { let _ = sender.send(p); });
            }
        },
        Err(e) => error!(?e, "hotreload watcher error."),
    }
}

/// Hotreload the dynamic library
///
/// This will reload the dynamic library by first internally calling compile
/// and then reloading the library.
fn hotreload(dyn_package: &str, loaded_lib: &Mutex<Option<LoadedLib>>) {
    // Do nothing if recompile failed.
    if compile(dyn_package) {
        let mut lock = loaded_lib.lock().unwrap();

        // Close lib.
        let loaded_lib = lock.take().unwrap();
        loaded_lib.lib.close().unwrap();
        copy(&loaded_lib.lib_path, dyn_package);

        // Open new lib.
        *lock = Some(LoadedLib::load(dyn_package));

        info!("Updated {}.", dyn_package);
    }
}

/// Recompile the dyn package
///
/// Returns `false` if the compile failed.
fn compile(dyn_package: &str) -> bool {
    let output = Command::new("cargo")
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .arg("rustc")
        .arg("--package")
        .arg(dyn_package)
        .arg("--features")
        .arg(format!("{}/be-dyn-lib", dyn_package))
        .arg("-Z")
        .arg("unstable-options")
        .arg("--crate-type")
        .arg("dylib")
        .output()
        .unwrap();

    output.status.success()
}

/// Copy the lib file, so we have an `_active` copy.
///
/// We do this for all OS's although it is only strictly necessary for windows.
/// The reason we do this is to make the code easier to understand and debug.
fn copy(lib_path: &Path, dyn_package: &str) {
    // Use the platform specific names.
    let lib_compiled_path = lib_path.with_file_name(compiled_file(dyn_package));
    let lib_output_path = lib_path.with_file_name(active_file(dyn_package));

    // Get the path to where the lib was compiled to.
    debug!(?lib_compiled_path, ?lib_output_path, "Moving.");

    // Copy the library file from where it is output, to where we are going to
    // load it from i.e. lib_path.
    std::fs::copy(&lib_compiled_path, &lib_output_path).unwrap_or_else(|err| {
        panic!(
            "Failed to rename dynamic library from {:?} to {:?}. {:?}",
            lib_compiled_path, lib_output_path, err
        )
    });
}
