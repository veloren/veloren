use crate::ui::{Graphic, SampleStrat, Transform, Ui};
use common::{
    assets::{self, watch::ReloadIndicator, Asset},
    comp::item::{
        armor::{Armor, ArmorKind},
        Glider, ItemDesc, ItemKind, Lantern, Throwable, Utility,
    },
    figure::Segment,
};
use conrod_core::image::Id;
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, sync::Arc};
use tracing::{error, warn};
use vek::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKey {
    Tool(String),
    Lantern(String),
    Glider(String),
    Armor(ArmorKind),
    Utility(Utility),
    Consumable(String),
    Throwable(Throwable),
    Ingredient(String),
    Empty,
}

impl<T: ItemDesc> From<&T> for ItemKey {
    fn from(item_desc: &T) -> Self {
        let item_kind = item_desc.kind();
        let item_definition_id = item_desc.item_definition_id();

        match item_kind {
            ItemKind::Tool(_) => ItemKey::Tool(item_definition_id.to_owned()),
            ItemKind::Lantern(Lantern { kind, .. }) => ItemKey::Lantern(kind.clone()),
            ItemKind::Glider(Glider { kind, .. }) => ItemKey::Glider(kind.clone()),
            ItemKind::Armor(Armor { kind, .. }) => ItemKey::Armor(kind.clone()),
            ItemKind::Utility { kind, .. } => ItemKey::Utility(*kind),
            ItemKind::Consumable { kind, .. } => ItemKey::Consumable(kind.clone()),
            ItemKind::Throwable { kind, .. } => ItemKey::Throwable(*kind),
            ItemKind::Ingredient { kind, .. } => ItemKey::Ingredient(kind.clone()),
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
            ImageSpec::Png(specifier) => Graphic::Image(graceful_load_img(&specifier), None),
            ImageSpec::Vox(specifier) => Graphic::Voxel(
                graceful_load_segment_no_skin(&specifier),
                Transform {
                    stretch: false,
                    ..Default::default()
                },
                SampleStrat::None,
            ),
            ImageSpec::VoxTrans(specifier, offset, [rot_x, rot_y, rot_z], zoom) => Graphic::Voxel(
                graceful_load_segment_no_skin(&specifier),
                Transform {
                    ori: Quaternion::rotation_x(rot_x * std::f32::consts::PI / 180.0)
                        .rotated_y(rot_y * std::f32::consts::PI / 180.0)
                        .rotated_z(rot_z * std::f32::consts::PI / 180.0),
                    offset: Vec3::from(*offset),
                    zoom: *zoom,
                    orth: true, // TODO: Is this what we want here? @Pfau
                    stretch: false,
                },
                SampleStrat::None,
            ),
        }
    }
}
#[derive(Serialize, Deserialize)]
struct ItemImagesSpec(HashMap<ItemKey, ImageSpec>);
impl Asset for ItemImagesSpec {
    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>, _specifier: &str) -> Result<Self, assets::Error> {
        ron::de::from_reader(buf_reader).map_err(assets::Error::parse_error)
    }
}

// TODO: when there are more images don't load them all into memory
pub struct ItemImgs {
    map: HashMap<ItemKey, Id>,
    indicator: ReloadIndicator,
    not_found: Id,
}
impl ItemImgs {
    pub fn new(ui: &mut Ui, not_found: Id) -> Self {
        let mut indicator = ReloadIndicator::new();
        Self {
            map: ItemImagesSpec::load_watched(
                "voxygen.item_image_manifest",
                &mut indicator,
            )
            .expect("Unable to load item image manifest")
            .0
            .iter()
            // TODO: what if multiple kinds map to the same image, it would be nice to use the same
            // image id for both, although this does interfere with the current hot-reloading
            // strategy
            .map(|(kind, spec)| (kind.clone(), ui.add_graphic(spec.create_graphic())))
            .collect(),
            indicator,
            not_found,
        }
    }

    /// Checks if the manifest has been changed and reloads the images if so
    /// Reuses img ids
    pub fn reload_if_changed(&mut self, ui: &mut Ui) {
        if self.indicator.reloaded() {
            for (kind, spec) in ItemImagesSpec::load("voxygen.item_image_manifest")
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
                    },
                }
            }
        }
    }

    pub fn img_id(&self, item_key: ItemKey) -> Option<Id> {
        match self.map.get(&item_key) {
            Some(id) => Some(*id),
            // There was no specification in the ron
            None => {
                warn!(
                    ?item_key,
                    "missing specified image file (note: hot-reloading won't work here)",
                );
                None
            },
        }
    }

    pub fn img_id_or_not_found_img(&self, item_key: ItemKey) -> Id {
        self.img_id(item_key).unwrap_or(self.not_found)
    }
}

// Copied from figure/load.rs
// TODO: remove code dup?
fn graceful_load_vox(specifier: &str) -> Arc<DotVoxData> {
    let full_specifier: String = ["voxygen.", specifier].concat();
    match DotVoxData::load(full_specifier.as_str()) {
        Ok(dot_vox) => dot_vox,
        Err(_) => {
            error!(?full_specifier, "Could not load vox file for item images",);
            DotVoxData::load_expect("voxygen.voxel.not_found")
        },
    }
}
fn graceful_load_img(specifier: &str) -> Arc<DynamicImage> {
    let full_specifier: String = ["voxygen.", specifier].concat();
    match DynamicImage::load(full_specifier.as_str()) {
        Ok(img) => img,
        Err(_) => {
            error!(?full_specifier, "Could not load image file for item images");
            DynamicImage::load_expect("voxygen.element.not_found")
        },
    }
}

fn graceful_load_segment_no_skin(specifier: &str) -> Arc<Segment> {
    use common::figure::{mat_cell::MatCell, MatSegment};
    let mat_seg = MatSegment::from(&*graceful_load_vox(specifier));
    let seg = mat_seg
        .map(|mat_cell| match mat_cell {
            MatCell::None => None,
            MatCell::Mat(_) => Some(MatCell::None),
            MatCell::Normal(_) => None,
        })
        .to_segment(|_| Rgb::broadcast(255));
    Arc::new(seg)
}
