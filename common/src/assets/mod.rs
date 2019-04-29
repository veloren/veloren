use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;

fn try_load(name: &str) -> Option<File> {
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
            }
            Err(e) => {
                debug!("loading {} did not work with error: {}", filename, e);
            }
        };
    }
    return None;
}

pub fn load(name: &str) -> Result<Vec<u8>, ()> {
    return match try_load(name) {
        Some(mut f) => {
            let mut content: Vec<u8> = vec![];
            f.read_to_end(&mut content);
            info!("loaded asset successful: {}", name);
            Ok(content)
        }
        None => {
            warn!(
                "Loading asset failed, wanted to load {} but could not load it, check debug log!",
                name
            );
            Err(())
        }
    };
}
