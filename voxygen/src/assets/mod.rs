pub mod ids;

use conrod_core::widget::image::Image;

use crate::ui::Ui;
use crate::ui::Graphic;

use dot_vox::DotVoxData;
use image::DynamicImage;

use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::fs::File;

fn read_from_path(path: &str) -> Result<Vec<u8>, std::io::Error> {
    let path_slash = path.replace(".", "/");
    let path_slash_toml = [env!("CARGO_MANIFEST_DIR"), &path_slash, ".toml"].concat();
    println!("{}", path_slash_toml);

    let mut content = Vec::new();
    File::open(path_slash_toml)?.read_to_end(&mut content);
    Ok(content)
}

pub trait Asset where Self: std::marker::Sized {
    fn load(path: &str) -> Result<Self, std::io::Error>;
}

impl Asset for DynamicImage {
    fn load(path: &str) -> Result<Self, std::io::Error> {
        let image = image::load_from_memory(
            &read_from_path(path)?
        ).unwrap();

        Ok(image)
    }
}

impl Asset for DotVoxData {
    fn load(path: &str) -> Result<Self, std::io::Error> {
        let dot_vox = dot_vox::load_bytes(
            &read_from_path(path)?
        ).unwrap();

        Ok(dot_vox)
    }
}

