use lazy_static::lazy_static;
use libloading::Library;
use notify::{immediate_watcher, EventKind, RecursiveMode, Watcher};
use std::{
    process::{Command, Stdio},
    sync::{mpsc, Mutex},
    thread,
    time::Duration,
};

lazy_static! {
    pub static ref LIB: Mutex<Option<LoadedLib>> = Mutex::new(Some(LoadedLib::compile_load()));
}

pub struct LoadedLib {
    pub lib: Library,
}

impl LoadedLib {
    fn compile_load() -> Self {
        // Compile
        compile();

        #[cfg(target_os = "windows")]
        copy();

        Self::load()
    }

    fn load() -> Self {
        #[cfg(target_os = "windows")]
        let lib = Library::new("../target/debug/voxygen_anim_active.dll").unwrap();
        #[cfg(not(target_os = "windows"))]
        let lib = Library::new("../target/debug/libvoxygen_anim.so").unwrap();

        Self { lib }
    }
}

// Starts up watcher
pub fn init() {
    // Make sure first compile is done
    drop(LIB.lock());

    // TODO: use crossbeam
    let (reload_send, reload_recv) = mpsc::channel();

    // Start watcher
    let mut watcher = immediate_watcher(move |res| event_fn(res, &reload_send)).unwrap();
    watcher.watch("src/anim", RecursiveMode::Recursive).unwrap();

    // Start reloader that watcher signals
    // "Debounces" events since I can't find the option to do this in the latest
    // `notify`
    thread::spawn(move || {
        let mut modified_paths = std::collections::HashSet::new();

        while let Ok(path) = reload_recv.recv() {
            modified_paths.insert(path);
            // Wait for any additional modify events before reloading
            while let Ok(path) = reload_recv.recv_timeout(Duration::from_millis(300)) {
                modified_paths.insert(path);
            }

            warn!(
                ?modified_paths,
                "Hot reloading animations because these files were modified"
            );

            // Reload
            reload();
        }
    });

    // Let the watcher live forever
    std::mem::forget(watcher);
}

// Recompiles and hotreloads the lib if the source has been changed
// Note: designed with voxygen dir as working dir, could be made more flexible
fn event_fn(res: notify::Result<notify::Event>, sender: &mpsc::Sender<String>) {
    match res {
        Ok(event) => match event.kind {
            EventKind::Modify(_) => {
                event
                    .paths
                    .iter()
                    .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
                    .map(|p| p.to_string_lossy().into_owned())
                    // Signal reloader
                    .for_each(|p| { let _ = sender.send(p); });
            },
            _ => {},
        },
        Err(e) => error!("Animation hotreload watcher error: {:?}", e),
    }
}

fn reload() {
    // Stop if recompile failed
    if !compile() {
        return;
    }

    let mut lock = LIB.lock().unwrap();

    // Close lib
    lock.take().unwrap().lib.close().unwrap();

    // Rename lib file on windows
    // Called after closing lib so file will be unlocked
    #[cfg(target_os = "windows")]
    copy();

    // Open new lib
    *lock = Some(LoadedLib::load());

    warn!("Updated animations");
}

// Returns false if compile failed
fn compile() -> bool {
    let output = Command::new("cargo")
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .arg("build")
        .arg("--package")
        .arg("veloren-voxygen-anim")
        .output()
        .unwrap();

    // If compile failed
    if !output.status.success() {
        error!("Failed to compile anim crate");
        false
    } else {
        warn!("Animation recompile success!!");
        true
    }
}

// Copy lib file if on windows since loading the lib locks the file blocking
// future compilation
#[cfg(target_os = "windows")]
fn copy() {
    std::fs::copy(
        "../target/debug/voxygen_anim.dll",
        "../target/debug/voxygen_anim_active.dll",
    )
    .expect("Failed to rename animations dll");
}
