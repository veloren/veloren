use clap::Parser;
use common::figure::Segment;
use common_assets::{AssetExt, DotVoxAsset};
use std::{fs, path::Path};
use vek::{Mat4, Quaternion, Vec2, Vec3, Vec4};
use veloren_voxygen::{
    hud::item_imgs::{ImageSpec, ItemImagesSpec},
    ui::graphic::renderer::{draw_vox, SampleStrat, Transform},
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
    for (_, spec) in manifest.read().0.iter() {
        match spec {
            ImageSpec::Vox(specifier) => voxel_to_png(&specifier, Transform::default(), args.scale),
            ImageSpec::VoxTrans(specifier, offset, [rot_x, rot_y, rot_z], zoom) => voxel_to_png(
                &specifier,
                Transform {
                    ori: Quaternion::rotation_x(rot_x * std::f32::consts::PI / 180.0)
                        .rotated_y(rot_y * std::f32::consts::PI / 180.0)
                        .rotated_z(rot_z * std::f32::consts::PI / 180.0),
                    offset: Vec3::from(*offset),
                    /* FIXME: This is a dirty workaround to not cut off the edges of some objects
                     * like ./img-export/weapon/component/axe/poleaxe/bronze.vox
                     * more details here: https://gitlab.com/veloren/veloren/-/merge_requests/3494#note_1205030803 */
                    zoom: *zoom * 0.8,
                    orth: true,
                    stretch: false,
                },
                args.scale,
            ),
            ImageSpec::Png(specifier) => {
                println!("Skip png image {}", specifier);
                continue;
            },
        }
    }
}

fn voxel_to_png(specifier: &String, transform: Transform, scale: u32) {
    let voxel = match DotVoxAsset::load(&format!("voxygen.{}", specifier)) {
        Ok(dot_vox) => dot_vox,
        Err(err) => {
            println!("Coudn't load voxel: {}", err);
            return;
        },
    };
    let dot_vox_data = &voxel.read().0;
    let model_size = dot_vox_data
        .models
        .get(0)
        .expect("Error getting model from voxel")
        .size;
    let ori_mat = Mat4::from(transform.ori);
    let aabb_size = Vec3::new(model_size.x, model_size.y, model_size.z);
    //TODO: skip dims transformation if transform is default(), instead use
    // model_size
    let rotated_size = calc_rotated_size(&ori_mat, &aabb_size);
    let projection_size = Vec2 {
        x: ((rotated_size.y as u32) * scale) as u16,
        y: ((rotated_size.z as u32) * scale) as u16,
    };
    let segment = Segment::from_vox(dot_vox_data, false);
    let path = format!("img-export/{}.png", &specifier_to_path(specifier));
    let folder_path = path.rsplit_once('/').expect("Invalid path").0;
    let full_path = Path::new(&path);
    if let Err(e) = fs::create_dir_all(Path::new(folder_path)) {
        println!("{}", e);
        return;
    }

    draw_vox(&segment, projection_size, transform, SampleStrat::None)
        .save(full_path)
        .unwrap_or_else(|_| panic!("Can't save file {}", full_path.to_str().expect("")));
}

fn calc_rotated_size(ori_mat: &Mat4<f32>, aabb_size: &Vec3<u32>) -> Vec3<f32> {
    let aabb_min = Vec3 {
        x: 0f32,
        y: 0f32,
        z: 0f32,
    };
    let aabb_max = Vec3 {
        x: aabb_size.y as f32,
        y: aabb_size.z as f32,
        z: aabb_size.x as f32,
    };
    let aabb_vertices: [Vec3<f32>; 8] = [
        Vec3::new(aabb_min.x, aabb_min.y, aabb_min.z),
        Vec3::new(aabb_max.x, aabb_min.y, aabb_min.z),
        Vec3::new(aabb_max.x, aabb_max.y, aabb_min.z),
        Vec3::new(aabb_min.x, aabb_max.y, aabb_min.z),
        Vec3::new(aabb_min.x, aabb_min.y, aabb_max.z),
        Vec3::new(aabb_max.x, aabb_min.y, aabb_max.z),
        Vec3::new(aabb_max.x, aabb_max.y, aabb_max.z),
        Vec3::new(aabb_min.x, aabb_max.y, aabb_max.z),
    ];
    let rotated_vertices = aabb_vertices.map(|c| (Vec4::<f32>::from(c) * *ori_mat).xyz());
    let max_xyz = rotated_vertices
        .iter()
        .copied()
        .reduce(|acc, corner| Vec3::<f32>::partial_max(acc, corner))
        .expect("Failed find maximum");
    let min_xyz = rotated_vertices
        .iter()
        .copied()
        .reduce(|acc, vertex| Vec3::<f32>::partial_min(acc, vertex))
        .expect("Failed find minimum");
    Vec3 {
        x: max_xyz.x - min_xyz.x,
        y: max_xyz.y - min_xyz.y,
        z: max_xyz.z - min_xyz.z,
    }
}

fn specifier_to_path(specifier: &String) -> String {
    specifier
        .strip_prefix("voxel.")
        .unwrap_or_else(|| panic!("There was no prefix in {}", specifier))
        .replace('.', "/")
}
