use std::fs::{self, File};

use wasmer_runtime::{
    imports,
    instantiate,
    Instance,
};

pub struct ModLoader {
    // How do we store a list of mods that could change length?
    // see Chapter 7 in the book, it's about collections of things!
    mods: Vec<Instance>// what type goes here?
}

impl ModLoader {
    // this lets you make a new ModLoader.
    // ModLoaders start out empty, so making a new one is very simple.
    pub fn new() -> Self {
        Self {
            mods: Vec::new(),// how do we make an empty list of mods that could change length?
        }
    }

    // it's important to call this every time you want to see if there are any new mods.
    // don't call it too often though, because processing all of that WASM data could be
    // very slow!
    // This takes a mutable reference to self, since it's changing what's inside its list of mods.
    pub fn load_mods(&mut self) {
        use std::io::Read;
		self.mods.clear();

        let paths = fs::read_dir("./mods/").unwrap();

        for path in paths {
            let mut path_to_mod = path.unwrap().path();
            let mod_name: String = path_to_mod
                .iter()
                .last()
                .unwrap()
                .to_owned()
                .into_string()
                .unwrap();

            path_to_mod.push("target");
            path_to_mod.push("wasm32-unknown-unknown");
            path_to_mod.push("debug");
            path_to_mod.push(mod_name.clone() + ".wasm");
            println!("{}", path_to_mod.display());
			let mut wasm_file = File::open(path_to_mod)
                .expect(&format!("Couldn't open {}.wasm file!", mod_name));
            let mut wasm_bytes = Vec::new();
            wasm_file
                .read_to_end(&mut wasm_bytes)
                .expect("error reading WASM bytes");

            let new_mod = instantiate(&wasm_bytes, &imports!{})
                .expect("Invalid WASM provided!");

            self.mods.push(new_mod);
        }
    }
}

// some example usage code of a ModLoader
pub fn test_mod() {
    // this looks for folders in `/veloren/mods/`, and looks for WASM in each of those folders.
    let mut mod_loader = ModLoader::new();

    mod_loader.load_mods();

    for wasm_mod in mod_loader.mods.iter() {
        // see if mod has "add" function
        if let Ok(add) = wasm_mod.func::<(i32, i32), i32>("add") {
            // call it if it does!
            let output = add.call(1, 2).expect("add WASM mod fail");
            println!("add output: {}", output);
        }
    }
}