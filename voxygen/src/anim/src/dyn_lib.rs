use lazy_static::lazy_static;
use libloading::Library;
use notify::{
    event::{AccessKind, AccessMode},
    immediate_watcher, EventKind, RecursiveMode, Watcher,
};
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
            .arg("--release")
            .arg("--package")
            .arg("veloren-voxygen-anim")
            .output()
            .unwrap();

        Self::load()
    }

    fn load() -> Self {
        #[cfg(target_os = "windows")]
        let lib = Library::new("../target/release/libvoxygen_anim.dll").unwrap();
        #[cfg(not(target_os = "windows"))]
        let lib = Library::new("../target/release/libvoxygen_anim.so").unwrap();

        Self { lib }
    }
}

// Starts up watcher test test2 test3 test4 test5
pub fn init() {
    // Start watcher
    let mut watcher = immediate_watcher(event_fn).unwrap();
    watcher.watch("src/anim", RecursiveMode::Recursive).unwrap();

    // Let the watcher live forever
    std::mem::forget(watcher);
}

// Recompiles and hotreloads the lib if the source is changed
// Note: designed with voxygen dir as working dir, could be made more flexible
fn event_fn(res: notify::Result<notify::Event>) {
    match res {
        Ok(event) => match event.kind {
            EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
                if event
                    .paths
                    .iter()
                    .any(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
                {
                    println!(
                        "Hot reloading animations because these files were modified:\n{:?}",
                        event.paths
                    );
                    reload();
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
        .arg("--release")
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
