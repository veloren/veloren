use lazy_static::lazy_static;
use libloading::Library;
use notify::{immediate_watcher, EventKind, RecursiveMode, Watcher};
use std::{
    process::{Command, Stdio},
    sync::Mutex,
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
        let _output = Command::new("cargo")
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .arg("build")
            .arg("--package")
            .arg("veloren-voxygen-anim")
            .output()
            .unwrap();

        Self::load()
    }

    fn load() -> Self {
        #[cfg(target_os = "windows")]
        let lib = Library::new("../target/debug/voxygen_anim.dll").unwrap();
        #[cfg(not(target_os = "windows"))]
        let lib = Library::new("../target/debug/libvoxygen_anim.so").unwrap();

        Self { lib }
    }
}

// Starts up watcher
pub fn init() {
    // TODO: use crossbeam
    let (reload_send, reload_recv) = std::sync::mpsc::channel();

    // Start watcher
    let mut watcher = immediate_watcher(move |res| event_fn(res, &reload_send)).unwrap();
    watcher.watch("src/anim", RecursiveMode::Recursive).unwrap();

    // Start reloader that watcher signals
    // "Debounces" events since I can't find the option to do this in the latest
    // `notify`
    std::thread::spawn(move || {
        while let Ok(()) = reload_recv.recv() {
            // Wait for another modify event before reloading
            while let Ok(()) = reload_recv.recv_timeout(std::time::Duration::from_millis(300)) {}

            // Reload
            reload();
        }
    });

    // Let the watcher live forever
    std::mem::forget(watcher);
}

// Recompiles and hotreloads the lib if the source is changed
// Note: designed with voxygen dir as working dir, could be made more flexible
fn event_fn(res: notify::Result<notify::Event>, sender: &std::sync::mpsc::Sender<()>) {
    match res {
        Ok(event) => match event.kind {
            EventKind::Modify(_) => {
                if event
                    .paths
                    .iter()
                    .any(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
                {
                    println!(
                        "Hot reloading animations because these files were modified:\n{:?}",
                        event.paths
                    );

                    // Signal reloader
                    let _ = sender.send(());
                }
            },
            _ => {},
        },
        Err(e) => println!("watch error: {:?}", e),
    }
}

fn reload() {
    // Compile
    let output = Command::new("cargo")
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .arg("build")
        .arg("--package")
        .arg("veloren-voxygen-anim")
        .output()
        .unwrap();

    // Stop if recompile failed
    if !output.status.success() {
        println!("Failed to compile anim crate");
        return;
    }

    println!("Compile Success!!");

    let mut lock = LIB.lock().unwrap();

    // Close lib
    lock.take().unwrap().lib.close().unwrap();

    // Open new lib
    *lock = Some(LoadedLib::load());

    println!("Updated");
}
