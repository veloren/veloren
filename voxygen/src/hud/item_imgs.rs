use crate::ui::{Graphic, Transform, Ui};
use common::{
    assets::{self, watch::ReloadIndicator, Asset},
    comp::item::{Armor, Consumable, Ingredient, Item, ItemKind, ToolData, ToolKind},
};
use conrod_core::image::Id;
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use image::DynamicImage;
use log::{error, warn};
use serde_derive::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, sync::Arc};
use vek::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKey {
    Tool(ToolKind),
    Armor(Armor),
    Consumable(Consumable),
    Ingredient(Ingredient),
}
impl From<&Item> for ItemKey {
    fn from(item: &Item) -> Self {
        match &item.kind {
            ItemKind::Tool(ToolData { kind, .. }) => ItemKey::Tool(kind.clone()),
            ItemKind::Armor { kind, .. } => ItemKey::Armor(kind.clone()),
            ItemKind::Consumable { kind, .. } => ItemKey::Consumable(kind.clone()),
            ItemKind::Ingredient(kind) => ItemKey::Ingredient(kind.clone()),
        }
    }
}

#[derive(Serialize, Deserialize)]
enum ImageSpec {
    Png(String),
    Vox(String),
    // (specifier, offset, (axis, 2 * angle / pi), zoom)
    VoxTrans(String, [f32; 3], [f32; 3], f32),
}
impl ImageSpec {
    fn create_graphic(&self) -> Graphic {
        match self {
            ImageSpec::Png(specifier) => Graphic::Image(graceful_load_img(&specifier)),
            ImageSpec::Vox(specifier) => Graphic::Voxel(
                graceful_load_vox(&specifier),
                Transform {
                    stretch: false,
                    ..Default::default()
                },
                None,
            ),
            ImageSpec::VoxTrans(specifier, offset, [rot_x, rot_y, rot_z], zoom) => Graphic::Voxel(
                graceful_load_vox(&specifier),
                Transform {
                    ori: Quaternion::rotation_x(rot_x * std::f32::consts::PI / 180.0)
                        .rotated_y(rot_y * std::f32::consts::PI / 180.0)
                        .rotated_z(rot_z * std::f32::consts::PI / 180.0),
                    offset: Vec3::from(*offset),
                    zoom: *zoom,
                    orth: true, // TODO: Is this what we want here? @Pfau
                    stretch: false,
                },
                None,
            ),
        }
    }
}
#[derive(Serialize, Deserialize)]
struct ItemImagesSpec(HashMap<ItemKey, ImageSpec>);
impl Asset for ItemImagesSpec {
    const ENDINGS: &'static [&'static str] = &["ron"];
    fn parse(buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        Ok(ron::de::from_reader(buf_reader).expect("Error parsing item images spec"))
    }
}

pub struct ItemImgs {
    map: HashMap<ItemKey, Id>,
    indicator: ReloadIndicator,
}
impl ItemImgs {
    pub fn new(ui: &mut Ui) -> Self {
        let mut indicator = ReloadIndicator::new();
        Self {
            map: assets::load_watched::<ItemImagesSpec>(
                "voxygen.item_image_manifest",
                &mut indicator,
            )
            .expect("Unable to load item image manifest")
            .0
            .iter()
            .map(|(kind, spec)| (kind.clone(), ui.add_graphic(spec.create_graphic())))
            .collect(),
            indicator,
        }
    }
    /// Checks if the manifest has been changed and reloads the images if so
    /// Reuses img ids
    pub fn reload_if_changed(&mut self, ui: &mut Ui) {
        if self.indicator.reloaded() {
            for (kind, spec) in assets::load::<ItemImagesSpec>("voxygen.item_image_manifest")
                .expect("Unable to load item image manifest")
                .0
                .iter()
            {
                // Load new graphic
                let graphic = spec.create_graphic();
                // See if we already have an id we can use
                match self.map.get(&kind) {
                    Some(id) => ui.replace_graphic(*id, graphic),
                    // Otherwise, generate new id and insert it into our Id -> ItemKey map
                    None => {
                        self.map.insert(kind.clone(), ui.add_graphic(graphic));
                    }
                }
            }
        }
    }
    pub fn img_id(&self, item_kind: ItemKey) -> Option<Id> {
        match self.map.get(&item_kind) {
            Some(id) => Some(*id),
            // There was no specification in the ron
            None => {
                warn!(
                    "{:?} has no specified image file (note: hot-reloading won't work here)",
                    item_kind
                );
                None
            }
        }
    }
}

// Copied from figure/load.rs
// TODO: remove code dup?
fn graceful_load_vox(specifier: &str) -> Arc<DotVoxData> {
    let full_specifier: String = ["voxygen.", specifier].concat();
    match assets::load::<DotVoxData>(full_specifier.as_str()) {
        Ok(dot_vox) => dot_vox,
        Err(_) => {
            error!(
                "Could not load vox file for item images: {}",
                full_specifier
            );
            assets::load_expect::<DotVoxData>("voxygen.voxel.not_found")
        }
    }
}
fn graceful_load_img(specifier: &str) -> Arc<DynamicImage> {
    let full_specifier: String = ["voxygen.", specifier].concat();
    match assets::load::<DynamicImage>(full_specifier.as_str()) {
        Ok(img) => img,
        Err(_) => {
            error!(
                "Could not load image file for item images: {}",
                full_specifier
            );
            assets::load_expect::<DynamicImage>("voxygen.element.not_found")
        }
    }
}
