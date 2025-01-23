use std::{fs, path::Path};

use clap::Parser;
use vek::Vec2;

use common_assets::AssetExt;
use veloren_voxygen::{
    hud::item_imgs::{ImageSpec, ItemImagesSpec},
    ui::{Graphic, graphic::renderer::draw_vox},
};

#[derive(Parser)]
struct Cli {
    ///Optional width and height scaling
    #[clap(default_value_t = 20)]
    scale: u32,
}

pub fn main() {
    let args = Cli::parse();
    let manifest = ItemImagesSpec::load_expect("voxygen.item_image_manifest");
    let image_size = Vec2 {
        x: (10_u32 * args.scale) as u16,
        y: (10_u32 * args.scale) as u16,
    };
    for (_, spec) in manifest.read().0.iter() {
        let graphic = spec.create_graphic();
        let img = match graphic {
            Graphic::Voxel(segment, trans, sample_strat) => {
                draw_vox(&segment, image_size, trans, sample_strat)
            },
            _ => continue,
        };
        let specifier = match spec {
            ImageSpec::Vox(specifier, _) => specifier,
            ImageSpec::VoxTrans(specifier, _, _, _, _) => specifier,
            _ => continue,
        };
        let path = format!("img-export/{}.png", &specifier_to_path(specifier));
        let folder_path = path.rsplit_once('/').expect("Invalid path").0;
        let full_path = Path::new(&path);
        if let Err(e) = fs::create_dir_all(Path::new(folder_path)) {
            println!("{}", e);
            return;
        }

        img.save(full_path)
            .unwrap_or_else(|_| panic!("Can't save file {}", full_path.to_str().expect("")));
    }
}

fn specifier_to_path(specifier: &String) -> String {
    specifier
        .strip_prefix("voxel.")
        .unwrap_or_else(|| panic!("There was no prefix in {}", specifier))
        .replace('.', "/")
}
