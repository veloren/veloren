use super::cache::FigureKey;
use common::{
    assets::{self, watch::ReloadIndicator, Asset, AssetWith, Ron},
    comp::{
        biped_large::{self, BodyType as BLBodyType, Species as BLSpecies},
        bird_medium::{self, BodyType as BMBodyType, Species as BMSpecies},
        bird_small,
        critter::{self, BodyType as CBodyType, Species as CSpecies},
        dragon::{self, BodyType as DBodyType, Species as DSpecies},
        fish_medium, fish_small,
        golem::{self, BodyType as GBodyType, Species as GSpecies},
        humanoid::{self, Body, BodyType, EyeColor, Skin, Species},
        item::tool::ToolKind,
        object,
        quadruped_low::{self, BodyType as QLBodyType, Species as QLSpecies},
        quadruped_medium::{self, BodyType as QMBodyType, Species as QMSpecies},
        quadruped_small::{self, BodyType as QSBodyType, Species as QSSpecies},
    },
    figure::{DynaUnionizer, MatSegment, Material, Segment},
};
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use serde_derive::Deserialize;
use std::sync::Arc;
use tracing::{error, warn};
use vek::*;

pub type BoneMeshes = (Segment, Vec3<f32>);

fn load_segment(mesh_name: &str) -> Segment {
    let full_specifier: String = ["voxygen.voxel.", mesh_name].concat();
    Segment::from(DotVoxData::load_expect(full_specifier.as_str()).as_ref())
}
fn graceful_load_vox(mesh_name: &str) -> Arc<DotVoxData> {
    let full_specifier: String = ["voxygen.voxel.", mesh_name].concat();
    match DotVoxData::load(full_specifier.as_str()) {
        Ok(dot_vox) => dot_vox,
        Err(_) => {
            error!(?full_specifier, "Could not load vox file for figure");
            DotVoxData::load_expect("voxygen.voxel.not_found")
        },
    }
}
fn graceful_load_segment(mesh_name: &str) -> Segment {
    Segment::from(graceful_load_vox(mesh_name).as_ref())
}
fn graceful_load_segment_flipped(mesh_name: &str) -> Segment {
    Segment::from_vox(graceful_load_vox(mesh_name).as_ref(), true)
}
fn graceful_load_mat_segment(mesh_name: &str) -> MatSegment {
    MatSegment::from(graceful_load_vox(mesh_name).as_ref())
}
fn graceful_load_mat_segment_flipped(mesh_name: &str) -> MatSegment {
    MatSegment::from_vox(graceful_load_vox(mesh_name).as_ref(), true)
}

pub fn load_mesh(mesh_name: &str, position: Vec3<f32>) -> BoneMeshes {
    (load_segment(mesh_name), position)
}

fn recolor_grey(rgb: Rgb<u8>, color: Rgb<u8>) -> Rgb<u8> {
    use common::util::{linear_to_srgb, srgb_to_linear};

    const BASE_GREY: f32 = 178.0;
    if rgb.r == rgb.g && rgb.g == rgb.b {
        let c1 = srgb_to_linear(rgb.map(|e| e as f32 / BASE_GREY));
        let c2 = srgb_to_linear(color.map(|e| e as f32 / 255.0));

        linear_to_srgb(c1 * c2).map(|e| (e.min(1.0).max(0.0) * 255.0) as u8)
    } else {
        rgb
    }
}

/// A set of reloadable specifications for a Body.
pub trait BodySpec: Sized {
    type Spec;

    /// Initialize all the specifications for this Body and watch for changes.
    fn load_watched(indicator: &mut ReloadIndicator) -> Result<Self::Spec, assets::Error>;

    /// Reload all specifications for this Body (to be called if the reload
    /// indicator is set).
    fn reload(spec: &mut Self::Spec) -> Result<(), assets::Error>;

    /// Mesh bones using the given spec, character state, and mesh generation
    /// function.
    ///
    /// NOTE: We deliberately call this function with only the key into the
    /// cache, to enforce that the cached state only depends on the key.  We
    /// may end up using a mechanism different from this cache eventually,
    /// in which case this strategy might change.
    fn bone_meshes(
        key: &FigureKey<Self>,
        spec: &Self::Spec,
    ) -> [Option<BoneMeshes>; anim::MAX_BONE_COUNT];
}

macro_rules! make_vox_spec {
    (
        $body:ty,
        struct $Spec:ident { $( $(+)? $field:ident: $ty:ty = $asset_path:literal),* $(,)? },
        |$self_pat:pat, $spec_pat:pat| $bone_meshes:block $(,)?
    ) => {
        #[derive(Clone)]
        pub struct $Spec {
            $( $field: AssetWith<Ron<$ty>, $asset_path>, )*
        }

        impl BodySpec for $body {
            type Spec = $Spec;

            #[allow(unused_variables)]
            fn load_watched(indicator: &mut ReloadIndicator) -> Result<Self::Spec, assets::Error> {
                Ok(Self::Spec {
                    $( $field: AssetWith::load_watched(indicator)?, )*
                })
            }

            #[allow(unused_variables)]
            fn reload(spec: &mut Self::Spec) -> Result<(), assets::Error> {
                $( spec.$field.reload()?; )*
                Ok(())
            }

            fn bone_meshes(
                $self_pat: &FigureKey<Self>,
                $spec_pat: &Self::Spec,
            ) -> [Option<BoneMeshes>; anim::MAX_BONE_COUNT] {
                $bone_meshes
            }
        }
    }
}

// All offsets should be relative to an initial origin that doesn't change when
// combining segments
#[derive(Deserialize)]
struct VoxSpec<T>(String, [T; 3]);

#[derive(Deserialize)]
struct VoxSimple(String);

#[derive(Deserialize)]
struct ArmorVoxSpec {
    vox_spec: VoxSpec<f32>,
    color: Option<[u8; 3]>,
}

// For use by armor with a left and right component
#[derive(Deserialize)]
struct SidedArmorVoxSpec {
    left: ArmorVoxSpec,
    right: ArmorVoxSpec,
    /// FIXME: Either use this, or remove it.
    #[allow(dead_code)]
    color: Option<[u8; 3]>,
}

/// Color information not found in voxels, for humanoids.
#[derive(Deserialize)]
struct HumColorSpec {
    hair_colors: humanoid::species::PureCases<Vec<(u8, u8, u8)>>,
    eye_colors_light: humanoid::eye_color::PureCases<(u8, u8, u8)>,
    eye_colors_dark: humanoid::eye_color::PureCases<(u8, u8, u8)>,
    eye_white: (u8, u8, u8),
    skin_colors_plain: humanoid::skin::PureCases<(u8, u8, u8)>,
    skin_colors_light: humanoid::skin::PureCases<(u8, u8, u8)>,
    skin_colors_dark: humanoid::skin::PureCases<(u8, u8, u8)>,
}

impl HumColorSpec {
    fn hair_color(&self, species: Species, val: u8) -> (u8, u8, u8) {
        species
            .elim_case_pure(&self.hair_colors)
            .get(val as usize)
            .copied()
            .unwrap_or((0, 0, 0))
    }

    fn color_segment(
        &self,
        mat_segment: MatSegment,
        skin: Skin,
        hair_color: (u8, u8, u8),
        eye_color: EyeColor,
    ) -> Segment {
        // TODO move some of the colors to common
        mat_segment.to_segment(|mat| {
            match mat {
                Material::Skin => *skin.elim_case_pure(&self.skin_colors_plain),
                Material::SkinDark => *skin.elim_case_pure(&self.skin_colors_dark),
                Material::SkinLight => *skin.elim_case_pure(&self.skin_colors_light),
                Material::Hair => hair_color,
                // TODO add back multiple colors
                Material::EyeLight => *eye_color.elim_case_pure(&self.eye_colors_light),
                Material::EyeDark => *eye_color.elim_case_pure(&self.eye_colors_dark),
                Material::EyeWhite => self.eye_white,
            }
            .into()
        })
    }
}

// All reliant on humanoid::Species and humanoid::BodyType
#[derive(Deserialize)]
struct HumHeadSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    head: VoxSpec<i32>,
    eyes: Vec<Option<VoxSpec<i32>>>,
    hair: Vec<Option<VoxSpec<i32>>>,
    beard: Vec<Option<VoxSpec<i32>>>,
    accessory: Vec<Option<VoxSpec<i32>>>,
}
#[derive(Deserialize)]
struct HumHeadSpec(HashMap<(Species, BodyType), HumHeadSubSpec>);

impl HumHeadSpec {
    fn mesh_head(&self, body: &Body, color_spec: &HumColorSpec) -> BoneMeshes {
        let spec = match self.0.get(&(body.species, body.body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    ?body.species,
                    ?body.body_type,
                    "No head specification exists for the combination of species and body"
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };

        let hair_color = color_spec.hair_color(body.species, body.hair_color);
        let hair_rgb = hair_color.into();
        let skin_rgb = body.species.skin_color(body.skin);
        let eye_rgb = body.species.eye_color(body.eye_color);

        // Load segment pieces
        let bare_head = graceful_load_mat_segment(&spec.head.0);

        let eyes = match spec.eyes.get(body.eyes as usize) {
            Some(Some(spec)) => Some((
                color_spec.color_segment(
                    graceful_load_mat_segment(&spec.0).map_rgb(|rgb| recolor_grey(rgb, hair_rgb)),
                    skin_rgb,
                    hair_color,
                    eye_rgb,
                ),
                Vec3::from(spec.1),
            )),
            Some(None) => None,
            None => {
                warn!("No specification for these eyes: {:?}", body.eyes);
                None
            },
        };
        let hair = match spec.hair.get(body.hair_style as usize) {
            Some(Some(spec)) => Some((
                graceful_load_segment(&spec.0).map_rgb(|rgb| recolor_grey(rgb, hair_rgb)),
                Vec3::from(spec.1),
            )),
            Some(None) => None,
            None => {
                warn!("No specification for hair style {}", body.hair_style);
                None
            },
        };
        let beard = match spec.beard.get(body.beard as usize) {
            Some(Some(spec)) => Some((
                graceful_load_segment(&spec.0).map_rgb(|rgb| recolor_grey(rgb, hair_rgb)),
                Vec3::from(spec.1),
            )),
            Some(None) => None,
            None => {
                warn!("No specification for this beard: {:?}", body.beard);
                None
            },
        };
        let accessory = match spec.accessory.get(body.accessory as usize) {
            Some(Some(spec)) => Some((graceful_load_segment(&spec.0), Vec3::from(spec.1))),
            Some(None) => None,
            None => {
                warn!("No specification for this accessory: {:?}", body.accessory);
                None
            },
        };

        let (head, origin_offset) = DynaUnionizer::new()
            .add(
                color_spec.color_segment(bare_head, skin_rgb, hair_color, eye_rgb),
                spec.head.1.into(),
            )
            .maybe_add(eyes)
            .maybe_add(hair)
            .maybe_add(beard)
            .maybe_add(accessory)
            .unify();

        (
            head,
            Vec3::from(spec.offset) + origin_offset.map(|e| e as f32 * -1.0),
        )
    }
}
// Armor aspects should be in the same order, top to bottom.
// These seem overly split up, but wanted to keep the armor seperated
// unlike head which is done above.
#[derive(Deserialize)]
struct ArmorVoxSpecMap<K, S>
where
    K: std::hash::Hash + std::cmp::Eq,
{
    default: S,
    map: HashMap<K, S>,
}
#[derive(Deserialize)]
struct HumArmorShoulderSpec(ArmorVoxSpecMap<String, SidedArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorChestSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorHandSpec(ArmorVoxSpecMap<String, SidedArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorBeltSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorBackSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorPantsSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorFootSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumMainWeaponSpec(HashMap<ToolKind, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorLanternSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorHeadSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorTabardSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);

make_vox_spec!(
    Body,
    struct HumSpec {
        color: HumColorSpec = "voxygen.voxel.humanoid_color_manifest",
        head: HumHeadSpec = "voxygen.voxel.humanoid_head_manifest",
        armor_shoulder: HumArmorShoulderSpec = "voxygen.voxel.humanoid_armor_shoulder_manifest",
        armor_chest: HumArmorChestSpec = "voxygen.voxel.humanoid_armor_chest_manifest",
        armor_hand: HumArmorHandSpec = "voxygen.voxel.humanoid_armor_hand_manifest",
        armor_belt: HumArmorBeltSpec = "voxygen.voxel.humanoid_armor_belt_manifest",
        armor_back: HumArmorBackSpec = "voxygen.voxel.humanoid_armor_back_manifest",
        armor_pants: HumArmorPantsSpec = "voxygen.voxel.humanoid_armor_pants_manifest",
        armor_foot: HumArmorFootSpec = "voxygen.voxel.humanoid_armor_foot_manifest",
        main_weapon: HumMainWeaponSpec = "voxygen.voxel.humanoid_main_weapon_manifest",
        armor_lantern: HumArmorLanternSpec = "voxygen.voxel.humanoid_lantern_manifest",
        // TODO: Add these.
        /* armor_head: HumArmorHeadSpec = "voxygen.voxel.humanoid_armor_head_manifest",
        tabard: HumArmorTabardSpec = "voxygen.voxel.humanoid_armor_tabard_manifest", */
    },
    |FigureKey { body, extra }, spec| {
        const DEFAULT_LOADOUT: super::cache::CharacterCacheKey = super::cache::CharacterCacheKey {
            third_person: None,
            tool: None,
            lantern: None,
            hand: None,
            foot: None,
        };

        // TODO: This is bad code, maybe this method should return Option<_>
        let loadout = extra.as_deref().unwrap_or(&DEFAULT_LOADOUT);
        let third_person = loadout.third_person.as_ref();
        let tool = loadout.tool.as_ref();
        let lantern = loadout.lantern.as_deref();
        let hand = loadout.hand.as_deref();
        let foot = loadout.foot.as_deref();

        [
            third_person.map(|_| {
                spec.head.asset.mesh_head(
                    body,
                    &spec.color.asset,
                )
            }),
            third_person.map(|loadout| {
                spec.armor_chest.asset.mesh_chest(
                    body,
                    &spec.color.asset,
                    loadout.chest.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_belt.asset.mesh_belt(
                    body,
                    &spec.color.asset,
                    loadout.belt.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_back.asset.mesh_back(
                    body,
                    &spec.color.asset,
                    loadout.back.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_pants.asset.mesh_pants(
                    body,
                    &spec.color.asset,
                    loadout.pants.as_deref(),
                )
            }),
            Some(spec.armor_hand.asset.mesh_left_hand(
                body,
                &spec.color.asset,
                hand,
            )),
            Some(spec.armor_hand.asset.mesh_right_hand(
                body,
                &spec.color.asset,
                hand,
            )),
            Some(spec.armor_foot.asset.mesh_left_foot(
                body,
                &spec.color.asset,
                foot,
            )),
            Some(spec.armor_foot.asset.mesh_right_foot(
                body,
                &spec.color.asset,
                foot,
            )),
            third_person.map(|loadout| {
                spec.armor_shoulder.asset.mesh_left_shoulder(
                    body,
                    &spec.color.asset,
                    loadout.shoulder.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_shoulder.asset.mesh_right_shoulder(
                    body,
                    &spec.color.asset,
                    loadout.shoulder.as_deref(),
                )
            }),
            Some(mesh_glider()),
            tool.and_then(|tool| tool.active.as_ref()).map(|tool| {
                spec.main_weapon.asset.mesh_main_weapon(
                    tool,
                    false,
                )
            }),
            tool.and_then(|tool| tool.second.as_ref()).map(|tool| {
                spec.main_weapon.asset.mesh_main_weapon(
                    tool,
                    true,
                )
            }),
            Some(spec.armor_lantern.asset.mesh_lantern(
                body,
                &spec.color.asset,
                lantern,
            )),
            Some(mesh_hold()),
        ]
    },
);

// Shoulder
impl HumArmorShoulderSpec {
    fn mesh_shoulder(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        shoulder: Option<&str>,
        flipped: bool,
    ) -> BoneMeshes {
        let spec = if let Some(shoulder) = shoulder {
            match self.0.map.get(shoulder) {
                Some(spec) => spec,
                None => {
                    error!(?shoulder, "No shoulder specification exists");
                    return load_mesh("not_found", Vec3::new(-3.0, -3.5, 0.1));
                },
            }
        } else {
            &self.0.default
        };

        let mut shoulder_segment = color_spec.color_segment(
            if flipped {
                graceful_load_mat_segment_flipped(&spec.left.vox_spec.0)
            } else {
                graceful_load_mat_segment(&spec.right.vox_spec.0)
            },
            body.species.skin_color(body.skin),
            color_spec.hair_color(body.species, body.hair_color),
            body.species.eye_color(body.eye_color),
        );

        // TODO: use this if we can
        /*let mut offset = spec.vox_spec.1;
        if flipped {
            offset[0] = -(shoulder_segment.size().x as f32) - offset[0];
        }*/
        let offset = if flipped {
            spec.left.vox_spec.1
        } else {
            spec.right.vox_spec.1
        };

        if let Some(color) = if flipped {
            spec.left.color
        } else {
            spec.right.color
        } {
            let shoulder_color = Vec3::from(color);
            shoulder_segment =
                shoulder_segment.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(shoulder_color)));
        }

        (shoulder_segment, Vec3::from(offset))
    }

    fn mesh_left_shoulder(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        shoulder: Option<&str>,
    ) -> BoneMeshes {
        self.mesh_shoulder(body, color_spec, shoulder, true)
    }

    fn mesh_right_shoulder(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        shoulder: Option<&str>,
    ) -> BoneMeshes {
        self.mesh_shoulder(body, color_spec, shoulder, false)
    }
}
// Chest
impl HumArmorChestSpec {
    fn mesh_chest(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        chest: Option<&str>,
    ) -> BoneMeshes {
        let spec = if let Some(chest) = chest {
            match self.0.map.get(chest) {
                Some(spec) => spec,
                None => {
                    error!(?chest, "No chest specification exists");
                    return load_mesh("not_found", Vec3::new(-7.0, -3.5, 2.0));
                },
            }
        } else {
            &self.0.default
        };

        let color = |mat_segment| {
            color_spec.color_segment(
                mat_segment,
                body.species.skin_color(body.skin),
                color_spec.hair_color(body.species, body.hair_color),
                body.species.eye_color(body.eye_color),
            )
        };

        let bare_chest = graceful_load_mat_segment("armor.empty");

        let mut chest_armor = graceful_load_mat_segment(&spec.vox_spec.0);

        if let Some(color) = spec.color {
            let chest_color = Vec3::from(color);
            chest_armor = chest_armor.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(chest_color)));
        }

        let chest = DynaUnionizer::new()
            .add(color(bare_chest), Vec3::new(0, 0, 0))
            .add(color(chest_armor), Vec3::new(0, 0, 0))
            .unify()
            .0;

        (chest, Vec3::from(spec.vox_spec.1))
    }
}
// Hand
impl HumArmorHandSpec {
    fn mesh_hand(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        hand: Option<&str>,
        flipped: bool,
    ) -> BoneMeshes {
        let spec = if let Some(hand) = hand {
            match self.0.map.get(hand) {
                Some(spec) => spec,
                None => {
                    error!(?hand, "No hand specification exists");
                    return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
                },
            }
        } else {
            &self.0.default
        };

        let mut hand_segment = color_spec.color_segment(
            if flipped {
                graceful_load_mat_segment_flipped(&spec.left.vox_spec.0)
            } else {
                graceful_load_mat_segment(&spec.right.vox_spec.0)
            },
            body.species.skin_color(body.skin),
            color_spec.hair_color(body.species, body.hair_color),
            body.species.eye_color(body.eye_color),
        );

        let offset = if flipped {
            spec.left.vox_spec.1
        } else {
            spec.right.vox_spec.1
        };

        if let Some(color) = if flipped {
            spec.left.color
        } else {
            spec.right.color
        } {
            let hand_color = Vec3::from(color);
            hand_segment = hand_segment.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(hand_color)));
        }

        (hand_segment, Vec3::from(offset))
    }

    fn mesh_left_hand(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        hand: Option<&str>,
    ) -> BoneMeshes {
        self.mesh_hand(body, color_spec, hand, true)
    }

    fn mesh_right_hand(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        hand: Option<&str>,
    ) -> BoneMeshes {
        self.mesh_hand(body, color_spec, hand, false)
    }
}
// Belt
impl HumArmorBeltSpec {
    fn mesh_belt(&self, body: &Body, color_spec: &HumColorSpec, belt: Option<&str>) -> BoneMeshes {
        let spec = if let Some(belt) = belt {
            match self.0.map.get(belt) {
                Some(spec) => spec,
                None => {
                    error!(?belt, "No belt specification exists");
                    return load_mesh("not_found", Vec3::new(-4.0, -3.5, 2.0));
                },
            }
        } else {
            &self.0.default
        };

        let mut belt_segment = color_spec.color_segment(
            graceful_load_mat_segment(&spec.vox_spec.0),
            body.species.skin_color(body.skin),
            color_spec.hair_color(body.species, body.hair_color),
            body.species.eye_color(body.eye_color),
        );

        if let Some(color) = spec.color {
            let belt_color = Vec3::from(color);
            belt_segment = belt_segment.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(belt_color)));
        }

        (belt_segment, Vec3::from(spec.vox_spec.1))
    }
}
// Cape
impl HumArmorBackSpec {
    fn mesh_back(&self, body: &Body, color_spec: &HumColorSpec, back: Option<&str>) -> BoneMeshes {
        let spec = if let Some(back) = back {
            match self.0.map.get(back) {
                Some(spec) => spec,
                None => {
                    error!(?back, "No back specification exists");
                    return load_mesh("not_found", Vec3::new(-4.0, -3.5, 2.0));
                },
            }
        } else {
            &self.0.default
        };

        let mut back_segment = color_spec.color_segment(
            graceful_load_mat_segment(&spec.vox_spec.0),
            body.species.skin_color(body.skin),
            color_spec.hair_color(body.species, body.hair_color),
            body.species.eye_color(body.eye_color),
        );
        if let Some(color) = spec.color {
            let back_color = Vec3::from(color);
            back_segment = back_segment.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(back_color)));
        }

        (back_segment, Vec3::from(spec.vox_spec.1))
    }
}
// Legs
impl HumArmorPantsSpec {
    fn mesh_pants(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        pants: Option<&str>,
    ) -> BoneMeshes {
        let spec = if let Some(pants) = pants {
            match self.0.map.get(pants) {
                Some(spec) => spec,
                None => {
                    error!(?pants, "No pants specification exists");
                    return load_mesh("not_found", Vec3::new(-5.0, -3.5, 1.0));
                },
            }
        } else {
            &self.0.default
        };

        let color = |mat_segment| {
            color_spec.color_segment(
                mat_segment,
                body.species.skin_color(body.skin),
                color_spec.hair_color(body.species, body.hair_color),
                body.species.eye_color(body.eye_color),
            )
        };

        let bare_pants = graceful_load_mat_segment("armor.empty");

        let mut pants_armor = graceful_load_mat_segment(&spec.vox_spec.0);

        if let Some(color) = spec.color {
            let pants_color = Vec3::from(color);
            pants_armor = pants_armor.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(pants_color)));
        }

        let pants = DynaUnionizer::new()
            .add(color(bare_pants), Vec3::new(0, 0, 0))
            .add(color(pants_armor), Vec3::new(0, 0, 0))
            .unify()
            .0;

        (pants, Vec3::from(spec.vox_spec.1))
    }
}
// Foot
impl HumArmorFootSpec {
    fn mesh_foot(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        foot: Option<&str>,
        flipped: bool,
    ) -> BoneMeshes {
        let spec = if let Some(foot) = foot {
            match self.0.map.get(foot) {
                Some(spec) => spec,
                None => {
                    error!(?foot, "No foot specification exists");
                    return load_mesh("not_found", Vec3::new(-2.5, -3.5, -9.0));
                },
            }
        } else {
            &self.0.default
        };

        let mut foot_segment = color_spec.color_segment(
            if flipped {
                graceful_load_mat_segment_flipped(&spec.vox_spec.0)
            } else {
                graceful_load_mat_segment(&spec.vox_spec.0)
            },
            body.species.skin_color(body.skin),
            color_spec.hair_color(body.species, body.hair_color),
            body.species.eye_color(body.eye_color),
        );

        if let Some(color) = spec.color {
            let foot_color = Vec3::from(color);
            foot_segment = foot_segment.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(foot_color)));
        }

        (foot_segment, Vec3::from(spec.vox_spec.1))
    }

    fn mesh_left_foot(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        foot: Option<&str>,
    ) -> BoneMeshes {
        self.mesh_foot(body, color_spec, foot, true)
    }

    fn mesh_right_foot(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        foot: Option<&str>,
    ) -> BoneMeshes {
        self.mesh_foot(body, color_spec, foot, false)
    }
}

impl HumMainWeaponSpec {
    fn mesh_main_weapon(&self, tool_kind: &ToolKind, flipped: bool) -> BoneMeshes {
        let spec = match self.0.get(tool_kind) {
            Some(spec) => spec,
            None => {
                error!(?tool_kind, "No tool/weapon specification exists");
                return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
            },
        };

        let tool_kind_segment = if flipped {
            graceful_load_segment_flipped(&spec.vox_spec.0)
        } else {
            graceful_load_segment(&spec.vox_spec.0)
        };

        let offset = Vec3::new(
            if flipped {
                //log::warn!("tool kind segment {:?}", );
                //tool_kind_segment.;
                0.0 - spec.vox_spec.1[0] - (tool_kind_segment.sz.x as f32)
            } else {
                spec.vox_spec.1[0]
            },
            spec.vox_spec.1[1],
            spec.vox_spec.1[2],
        );

        (tool_kind_segment, offset)
    }
}

// Lantern
impl HumArmorLanternSpec {
    fn mesh_lantern(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        lantern: Option<&str>,
    ) -> BoneMeshes {
        let spec = if let Some(kind) = lantern {
            match self.0.map.get(kind) {
                Some(spec) => spec,
                None => {
                    error!(?kind, "No lantern specification exists");
                    return load_mesh("not_found", Vec3::new(-4.0, -3.5, 2.0));
                },
            }
        } else {
            &self.0.default
        };

        let mut lantern_segment = color_spec.color_segment(
            graceful_load_mat_segment(&spec.vox_spec.0),
            body.species.skin_color(body.skin),
            color_spec.hair_color(body.species, body.hair_color),
            body.species.eye_color(body.eye_color),
        );
        if let Some(color) = spec.color {
            let lantern_color = Vec3::from(color);
            lantern_segment =
                lantern_segment.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(lantern_color)));
        }

        (lantern_segment, Vec3::from(spec.vox_spec.1))
    }
}
impl HumArmorHeadSpec {
    /// FIXME: Either use this, or remove it.
    #[allow(dead_code)]
    fn mesh_head(&self, body: &Body, color_spec: &HumColorSpec, head: Option<&str>) -> BoneMeshes {
        let spec = if let Some(head) = head {
            match self.0.map.get(head) {
                Some(spec) => spec,
                None => {
                    error!(?head, "No head specification exists");
                    return load_mesh("not_found", Vec3::new(-5.0, -3.5, 1.0));
                },
            }
        } else {
            &self.0.default
        };

        let color = |mat_segment| {
            color_spec.color_segment(
                mat_segment,
                body.species.skin_color(body.skin),
                color_spec.hair_color(body.species, body.hair_color),
                body.species.eye_color(body.eye_color),
            )
        };

        let bare_head = graceful_load_mat_segment("armor.empty");

        let mut head_armor = graceful_load_mat_segment(&spec.vox_spec.0);

        if let Some(color) = spec.color {
            let head_color = Vec3::from(color);
            head_armor = head_armor.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(head_color)));
        }

        let head = DynaUnionizer::new()
            .add(color(bare_head), Vec3::new(0, 0, 0))
            .add(color(head_armor), Vec3::new(0, 0, 0))
            .unify()
            .0;

        (head, Vec3::from(spec.vox_spec.1))
    }
}
impl HumArmorTabardSpec {
    /// FIXME: Either use this, or remove it.
    #[allow(dead_code)]
    fn mesh_tabard(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        tabard: Option<&str>,
    ) -> BoneMeshes {
        let spec = if let Some(tabard) = tabard {
            match self.0.map.get(tabard) {
                Some(spec) => spec,
                None => {
                    error!(?tabard, "No tabard specification exists");
                    return load_mesh("not_found", Vec3::new(-5.0, -3.5, 1.0));
                },
            }
        } else {
            &self.0.default
        };

        let color = |mat_segment| {
            color_spec.color_segment(
                mat_segment,
                body.species.skin_color(body.skin),
                color_spec.hair_color(body.species, body.hair_color),
                body.species.eye_color(body.eye_color),
            )
        };

        let bare_tabard = graceful_load_mat_segment("armor.empty");

        let mut tabard_armor = graceful_load_mat_segment(&spec.vox_spec.0);

        if let Some(color) = spec.color {
            let tabard_color = Vec3::from(color);
            tabard_armor = tabard_armor.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(tabard_color)));
        }

        let tabard = DynaUnionizer::new()
            .add(color(bare_tabard), Vec3::new(0, 0, 0))
            .add(color(tabard_armor), Vec3::new(0, 0, 0))
            .unify()
            .0;

        (tabard, Vec3::from(spec.vox_spec.1))
    }
}
// TODO: Inventory
fn mesh_glider() -> BoneMeshes { load_mesh("object.glider", Vec3::new(-26.0, -26.0, -5.0)) }

fn mesh_hold() -> BoneMeshes {
    load_mesh(
        "weapon.projectile.simple-arrow",
        Vec3::new(-0.5, -6.0, -1.5),
    )
}

/////////
#[derive(Deserialize)]
struct QuadrupedSmallCentralSpec(HashMap<(QSSpecies, QSBodyType), SidedQSCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedQSCentralVoxSpec {
    head: QuadrupedSmallCentralSubSpec,
    chest: QuadrupedSmallCentralSubSpec,
    tail: QuadrupedSmallCentralSubSpec,
}
#[derive(Deserialize)]
struct QuadrupedSmallCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}

#[derive(Deserialize)]
struct QuadrupedSmallLateralSpec(HashMap<(QSSpecies, QSBodyType), SidedQSLateralVoxSpec>);

#[derive(Deserialize)]
struct SidedQSLateralVoxSpec {
    left_front: QuadrupedSmallLateralSubSpec,
    right_front: QuadrupedSmallLateralSubSpec,
    left_back: QuadrupedSmallLateralSubSpec,
    right_back: QuadrupedSmallLateralSubSpec,
}
#[derive(Deserialize)]
struct QuadrupedSmallLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    quadruped_small::Body,
    struct QuadrupedSmallSpec {
        central: QuadrupedSmallCentralSpec = "voxygen.voxel.quadruped_small_central_manifest",
        lateral: QuadrupedSmallLateralSpec = "voxygen.voxel.quadruped_small_lateral_manifest",
    },
    |FigureKey { body, .. }, spec| {
        [
            Some(spec.central.asset.mesh_head(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_chest(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_br(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_tail(
                body.species,
                body.body_type,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    },
);

impl QuadrupedSmallCentralSpec {
    fn mesh_head(&self, species: QSSpecies, body_type: QSBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.head.central.0);

        (central, Vec3::from(spec.head.offset))
    }

    fn mesh_chest(&self, species: QSSpecies, body_type: QSBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No chest specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.chest.central.0);

        (central, Vec3::from(spec.chest.offset))
    }

    fn mesh_tail(&self, species: QSSpecies, body_type: QSBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No tail specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.tail.central.0);

        (central, Vec3::from(spec.tail.offset))
    }
}

impl QuadrupedSmallLateralSpec {
    fn mesh_foot_fl(&self, species: QSSpecies, body_type: QSBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.left_front.lateral.0);

        (lateral, Vec3::from(spec.left_front.offset))
    }

    fn mesh_foot_fr(&self, species: QSSpecies, body_type: QSBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.right_front.lateral.0);

        (lateral, Vec3::from(spec.right_front.offset))
    }

    fn mesh_foot_bl(&self, species: QSSpecies, body_type: QSBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.left_back.lateral.0);

        (lateral, Vec3::from(spec.left_back.offset))
    }

    fn mesh_foot_br(&self, species: QSSpecies, body_type: QSBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.right_back.lateral.0);

        (lateral, Vec3::from(spec.right_back.offset))
    }
}

//////
#[derive(Deserialize)]
struct QuadrupedMediumCentralSpec(HashMap<(QMSpecies, QMBodyType), SidedQMCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedQMCentralVoxSpec {
    upper: QuadrupedMediumCentralSubSpec,
    lower: QuadrupedMediumCentralSubSpec,
    jaw: QuadrupedMediumCentralSubSpec,
    ears: QuadrupedMediumCentralSubSpec,
    torso_front: QuadrupedMediumCentralSubSpec,
    torso_back: QuadrupedMediumCentralSubSpec,
    tail: QuadrupedMediumCentralSubSpec,
}
#[derive(Deserialize)]
struct QuadrupedMediumCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}

#[derive(Deserialize)]
struct QuadrupedMediumLateralSpec(HashMap<(QMSpecies, QMBodyType), SidedQMLateralVoxSpec>);
#[derive(Deserialize)]
struct SidedQMLateralVoxSpec {
    leg_fl: QuadrupedMediumLateralSubSpec,
    leg_fr: QuadrupedMediumLateralSubSpec,
    leg_bl: QuadrupedMediumLateralSubSpec,
    leg_br: QuadrupedMediumLateralSubSpec,
    foot_fl: QuadrupedMediumLateralSubSpec,
    foot_fr: QuadrupedMediumLateralSubSpec,
    foot_bl: QuadrupedMediumLateralSubSpec,
    foot_br: QuadrupedMediumLateralSubSpec,
}
#[derive(Deserialize)]
struct QuadrupedMediumLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    quadruped_medium::Body,
    struct QuadrupedMediumSpec {
        central: QuadrupedMediumCentralSpec = "voxygen.voxel.quadruped_medium_central_manifest",
        lateral: QuadrupedMediumLateralSpec = "voxygen.voxel.quadruped_medium_lateral_manifest",
    },
    |FigureKey { body, .. }, spec| {
        [
            Some(spec.central.asset.mesh_head_upper(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_head_lower(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_jaw(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_tail(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_torso_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_torso_back(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_ears(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_leg_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_leg_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_leg_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_leg_br(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_br(
                body.species,
                body.body_type,
            )),
            None,
        ]
    }
);

impl QuadrupedMediumCentralSpec {
    fn mesh_head_upper(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No upper head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.upper.central.0);

        (central, Vec3::from(spec.upper.offset))
    }

    fn mesh_head_lower(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No lower head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.lower.central.0);

        (central, Vec3::from(spec.lower.offset))
    }

    fn mesh_jaw(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No jaw specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.jaw.central.0);

        (central, Vec3::from(spec.jaw.offset))
    }

    fn mesh_ears(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No ears specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.ears.central.0);

        (central, Vec3::from(spec.ears.offset))
    }

    fn mesh_torso_front(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No torso specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.torso_front.central.0);

        (central, Vec3::from(spec.torso_front.offset))
    }

    fn mesh_torso_back(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No torso specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.torso_back.central.0);

        (central, Vec3::from(spec.torso_back.offset))
    }

    fn mesh_tail(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No tail specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.tail.central.0);

        (central, Vec3::from(spec.tail.offset))
    }
}

impl QuadrupedMediumLateralSpec {
    fn mesh_leg_fl(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_fl.lateral.0);

        (lateral, Vec3::from(spec.leg_fl.offset))
    }

    fn mesh_leg_fr(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_fr.lateral.0);

        (lateral, Vec3::from(spec.leg_fr.offset))
    }

    fn mesh_leg_bl(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_bl.lateral.0);

        (lateral, Vec3::from(spec.leg_bl.offset))
    }

    fn mesh_leg_br(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_br.lateral.0);

        (lateral, Vec3::from(spec.leg_br.offset))
    }

    fn mesh_foot_fl(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_fl.lateral.0);

        (lateral, Vec3::from(spec.foot_fl.offset))
    }

    fn mesh_foot_fr(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_fr.lateral.0);

        (lateral, Vec3::from(spec.foot_fr.offset))
    }

    fn mesh_foot_bl(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_bl.lateral.0);

        (lateral, Vec3::from(spec.foot_bl.offset))
    }

    fn mesh_foot_br(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_br.lateral.0);

        (lateral, Vec3::from(spec.foot_br.offset))
    }
}

////
#[derive(Deserialize)]
struct BirdMediumCenterSpec(HashMap<(BMSpecies, BMBodyType), SidedBMCenterVoxSpec>);

#[derive(Deserialize)]
struct SidedBMCenterVoxSpec {
    head: BirdMediumCenterSubSpec,
    torso: BirdMediumCenterSubSpec,
    tail: BirdMediumCenterSubSpec,
}
#[derive(Deserialize)]
struct BirdMediumCenterSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    center: VoxSimple,
}

#[derive(Deserialize)]
struct BirdMediumLateralSpec(HashMap<(BMSpecies, BMBodyType), SidedBMLateralVoxSpec>);

#[derive(Deserialize)]
struct SidedBMLateralVoxSpec {
    wing_l: BirdMediumLateralSubSpec,
    wing_r: BirdMediumLateralSubSpec,
    foot_l: BirdMediumLateralSubSpec,
    foot_r: BirdMediumLateralSubSpec,
}
#[derive(Deserialize)]
struct BirdMediumLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    bird_medium::Body,
    struct BirdMediumSpec {
        center: BirdMediumCenterSpec = "voxygen.voxel.bird_medium_center_manifest",
        lateral: BirdMediumLateralSpec = "voxygen.voxel.bird_medium_lateral_manifest",
    },
    |FigureKey { body, .. }, spec| {
        [
            Some(spec.center.asset.mesh_head(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_torso(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_tail(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_wing_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_wing_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_r(
                body.species,
                body.body_type,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    },
);

impl BirdMediumCenterSpec {
    fn mesh_head(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.head.center.0);

        (center, Vec3::from(spec.head.offset))
    }

    fn mesh_torso(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No torso specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.torso.center.0);

        (center, Vec3::from(spec.torso.offset))
    }

    fn mesh_tail(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No tail specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.tail.center.0);

        (center, Vec3::from(spec.tail.offset))
    }
}
impl BirdMediumLateralSpec {
    fn mesh_wing_l(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_l.lateral.0);

        (lateral, Vec3::from(spec.wing_l.offset))
    }

    fn mesh_wing_r(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_r.lateral.0);

        (lateral, Vec3::from(spec.wing_r.offset))
    }

    fn mesh_foot_l(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_l.lateral.0);

        (lateral, Vec3::from(spec.foot_l.offset))
    }

    fn mesh_foot_r(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_r.lateral.0);

        (lateral, Vec3::from(spec.foot_r.offset))
    }
}
////
#[derive(Deserialize)]
struct CritterCenterSpec(HashMap<(CSpecies, CBodyType), SidedCCenterVoxSpec>);

#[derive(Deserialize)]
struct SidedCCenterVoxSpec {
    head: CritterCenterSubSpec,
    chest: CritterCenterSubSpec,
    feet_f: CritterCenterSubSpec,
    feet_b: CritterCenterSubSpec,
    tail: CritterCenterSubSpec,
}
#[derive(Deserialize)]
struct CritterCenterSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    center: VoxSimple,
}

make_vox_spec!(
    critter::Body,
    struct CritterSpec {
        center: CritterCenterSpec = "voxygen.voxel.critter_center_manifest",
    },
    |FigureKey { body, .. }, spec| {
        [
            Some(spec.center.asset.mesh_head(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_chest(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_feet_f(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_feet_b(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_tail(
                body.species,
                body.body_type,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    },
);

impl CritterCenterSpec {
    fn mesh_head(&self, species: CSpecies, body_type: CBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.head.center.0);

        (center, Vec3::from(spec.head.offset))
    }

    fn mesh_chest(&self, species: CSpecies, body_type: CBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No chest specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.chest.center.0);

        (center, Vec3::from(spec.chest.offset))
    }

    fn mesh_feet_f(&self, species: CSpecies, body_type: CBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No feet specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.feet_f.center.0);

        (center, Vec3::from(spec.feet_f.offset))
    }

    fn mesh_feet_b(&self, species: CSpecies, body_type: CBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No feet specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.feet_b.center.0);

        (center, Vec3::from(spec.feet_b.offset))
    }

    fn mesh_tail(&self, species: CSpecies, body_type: CBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No tail specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.tail.center.0);

        (center, Vec3::from(spec.tail.offset))
    }
}
////
make_vox_spec!(
    fish_medium::Body,
    struct FishMediumSpec {},
    |FigureKey { body, .. }, _spec| {
        [
            Some(mesh_fish_medium_head(body.head)),
            Some(mesh_fish_medium_torso(body.torso)),
            Some(mesh_fish_medium_rear(body.rear)),
            Some(mesh_fish_medium_tail(body.tail)),
            Some(mesh_fish_medium_fin_l(body.fin_l)),
            Some(mesh_fish_medium_fin_r(body.fin_r)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    },
);

fn mesh_fish_medium_head(head: fish_medium::Head) -> BoneMeshes {
    load_mesh(
        match head {
            fish_medium::Head::Default => "npc.marlin.head",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn mesh_fish_medium_torso(torso: fish_medium::Torso) -> BoneMeshes {
    load_mesh(
        match torso {
            fish_medium::Torso::Default => "npc.marlin.torso",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn mesh_fish_medium_rear(rear: fish_medium::Rear) -> BoneMeshes {
    load_mesh(
        match rear {
            fish_medium::Rear::Default => "npc.marlin.rear",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn mesh_fish_medium_tail(tail: fish_medium::Tail) -> BoneMeshes {
    load_mesh(
        match tail {
            fish_medium::Tail::Default => "npc.marlin.tail",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn mesh_fish_medium_fin_l(fin_l: fish_medium::FinL) -> BoneMeshes {
    load_mesh(
        match fin_l {
            fish_medium::FinL::Default => "npc.marlin.fin_l",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn mesh_fish_medium_fin_r(fin_r: fish_medium::FinR) -> BoneMeshes {
    load_mesh(
        match fin_r {
            fish_medium::FinR::Default => "npc.marlin.fin_r",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

////

#[derive(Deserialize)]
struct DragonCenterSpec(HashMap<(DSpecies, DBodyType), SidedDCenterVoxSpec>);

#[derive(Deserialize)]
struct SidedDCenterVoxSpec {
    upper: DragonCenterSubSpec,
    lower: DragonCenterSubSpec,
    jaw: DragonCenterSubSpec,
    chest_front: DragonCenterSubSpec,
    chest_rear: DragonCenterSubSpec,
    tail_front: DragonCenterSubSpec,
    tail_rear: DragonCenterSubSpec,
}
#[derive(Deserialize)]
struct DragonCenterSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    center: VoxSimple,
}

#[derive(Deserialize)]
struct DragonLateralSpec(HashMap<(DSpecies, DBodyType), SidedDLateralVoxSpec>);

#[derive(Deserialize)]
struct SidedDLateralVoxSpec {
    wing_in_l: DragonLateralSubSpec,
    wing_in_r: DragonLateralSubSpec,
    wing_out_l: DragonLateralSubSpec,
    wing_out_r: DragonLateralSubSpec,
    foot_fl: DragonLateralSubSpec,
    foot_fr: DragonLateralSubSpec,
    foot_bl: DragonLateralSubSpec,
    foot_br: DragonLateralSubSpec,
}
#[derive(Deserialize)]
struct DragonLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    dragon::Body,
    struct DragonSpec {
        center: DragonCenterSpec = "voxygen.voxel.dragon_center_manifest",
        lateral: DragonLateralSpec = "voxygen.voxel.dragon_lateral_manifest",
    },
    |FigureKey { body, .. }, spec| {
        [
            Some(spec.center.asset.mesh_head_upper(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_head_lower(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_jaw(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_chest_front(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_chest_rear(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_tail_front(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_tail_rear(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_wing_in_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_wing_in_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_wing_out_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_wing_out_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_br(
                body.species,
                body.body_type,
            )),
            None,
        ]
    },
);

impl DragonCenterSpec {
    fn mesh_head_upper(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No upper head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.upper.center.0);

        (central, Vec3::from(spec.upper.offset))
    }

    fn mesh_head_lower(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No lower head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.lower.center.0);

        (central, Vec3::from(spec.lower.offset))
    }

    fn mesh_jaw(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No jaw specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.jaw.center.0);

        (central, Vec3::from(spec.jaw.offset))
    }

    fn mesh_chest_front(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No chest front specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.chest_front.center.0);

        (center, Vec3::from(spec.chest_front.offset))
    }

    fn mesh_chest_rear(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No chest rear specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.chest_rear.center.0);

        (center, Vec3::from(spec.chest_rear.offset))
    }

    fn mesh_tail_front(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No tail front specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.tail_front.center.0);

        (center, Vec3::from(spec.tail_front.offset))
    }

    fn mesh_tail_rear(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No tail rear specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.tail_rear.center.0);

        (center, Vec3::from(spec.tail_rear.offset))
    }
}
impl DragonLateralSpec {
    fn mesh_wing_in_l(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_in_l.lateral.0);

        (lateral, Vec3::from(spec.wing_in_l.offset))
    }

    fn mesh_wing_in_r(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_in_r.lateral.0);

        (lateral, Vec3::from(spec.wing_in_r.offset))
    }

    fn mesh_wing_out_l(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_out_l.lateral.0);

        (lateral, Vec3::from(spec.wing_out_l.offset))
    }

    fn mesh_wing_out_r(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_out_r.lateral.0);

        (lateral, Vec3::from(spec.wing_out_r.offset))
    }

    fn mesh_foot_fl(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_fl.lateral.0);

        (lateral, Vec3::from(spec.foot_fl.offset))
    }

    fn mesh_foot_fr(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_fr.lateral.0);

        (lateral, Vec3::from(spec.foot_fr.offset))
    }

    fn mesh_foot_bl(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_bl.lateral.0);

        (lateral, Vec3::from(spec.foot_bl.offset))
    }

    fn mesh_foot_br(&self, species: DSpecies, body_type: DBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_br.lateral.0);

        (lateral, Vec3::from(spec.foot_br.offset))
    }
}

////
make_vox_spec!(
    bird_small::Body,
    struct BirdSmallSpec {},
    |FigureKey { body, .. }, _spec| {
        [
            Some(mesh_bird_small_head(body.head)),
            Some(mesh_bird_small_torso(body.torso)),
            Some(mesh_bird_small_wing_l(body.wing_l)),
            Some(mesh_bird_small_wing_r(body.wing_r)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    },
);

fn mesh_bird_small_head(head: bird_small::Head) -> BoneMeshes {
    load_mesh(
        match head {
            bird_small::Head::Default => "npc.crow.head",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn mesh_bird_small_torso(torso: bird_small::Torso) -> BoneMeshes {
    load_mesh(
        match torso {
            bird_small::Torso::Default => "npc.crow.torso",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn mesh_bird_small_wing_l(wing_l: bird_small::WingL) -> BoneMeshes {
    load_mesh(
        match wing_l {
            bird_small::WingL::Default => "npc.crow.wing_l",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn mesh_bird_small_wing_r(wing_r: bird_small::WingR) -> BoneMeshes {
    load_mesh(
        match wing_r {
            bird_small::WingR::Default => "npc.crow.wing_r",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}
////
make_vox_spec!(
    fish_small::Body,
    struct FishSmallSpec {},
    |FigureKey { body, .. }, _spec| {
        [
            Some(mesh_fish_small_torso(body.torso)),
            Some(mesh_fish_small_tail(body.tail)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    },
);

fn mesh_fish_small_torso(torso: fish_small::Torso) -> BoneMeshes {
    load_mesh(
        match torso {
            fish_small::Torso::Default => "npc.cardinalfish.torso",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}

fn mesh_fish_small_tail(tail: fish_small::Tail) -> BoneMeshes {
    load_mesh(
        match tail {
            fish_small::Tail::Default => "npc.cardinalfish.tail",
        },
        Vec3::new(-7.0, -6.0, -6.0),
    )
}
////
#[derive(Deserialize)]
struct BipedLargeCenterSpec(HashMap<(BLSpecies, BLBodyType), SidedBLCenterVoxSpec>);

#[derive(Deserialize)]
struct SidedBLCenterVoxSpec {
    head: BipedLargeCenterSubSpec,
    jaw: BipedLargeCenterSubSpec,
    torso_upper: BipedLargeCenterSubSpec,
    torso_lower: BipedLargeCenterSubSpec,
    tail: BipedLargeCenterSubSpec,
    main: BipedLargeCenterSubSpec,
    second: BipedLargeCenterSubSpec,
}
#[derive(Deserialize)]
struct BipedLargeCenterSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    center: VoxSimple,
}

#[derive(Deserialize)]
struct BipedLargeLateralSpec(HashMap<(BLSpecies, BLBodyType), SidedBLLateralVoxSpec>);

#[derive(Deserialize)]
struct SidedBLLateralVoxSpec {
    shoulder_l: BipedLargeLateralSubSpec,
    shoulder_r: BipedLargeLateralSubSpec,
    hand_l: BipedLargeLateralSubSpec,
    hand_r: BipedLargeLateralSubSpec,
    leg_l: BipedLargeLateralSubSpec,
    leg_r: BipedLargeLateralSubSpec,
    foot_l: BipedLargeLateralSubSpec,
    foot_r: BipedLargeLateralSubSpec,
}
#[derive(Deserialize)]
struct BipedLargeLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    biped_large::Body,
    struct BipedLargeSpec {
        center: BipedLargeCenterSpec = "voxygen.voxel.biped_large_center_manifest",
        lateral: BipedLargeLateralSpec = "voxygen.voxel.biped_large_lateral_manifest",
    },
    |FigureKey { body, .. }, spec| {
        [
            Some(spec.center.asset.mesh_head(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_jaw(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_torso_upper(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_torso_lower(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_tail(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_main(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_second(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_shoulder_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_shoulder_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_hand_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_hand_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_leg_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_leg_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_r(
                body.species,
                body.body_type,
            )),
            None,
        ]
    },
);

impl BipedLargeCenterSpec {
    fn mesh_head(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.head.center.0);

        (center, Vec3::from(spec.head.offset))
    }

    fn mesh_jaw(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No jaw specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.jaw.center.0);

        (center, Vec3::from(spec.jaw.offset))
    }

    fn mesh_torso_upper(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No torso upper specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.torso_upper.center.0);

        (center, Vec3::from(spec.torso_upper.offset))
    }

    fn mesh_torso_lower(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No torso lower specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.torso_lower.center.0);

        (center, Vec3::from(spec.torso_lower.offset))
    }

    fn mesh_tail(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No tail specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.tail.center.0);

        (center, Vec3::from(spec.tail.offset))
    }

    fn mesh_main(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No main weapon specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.main.center.0);

        (center, Vec3::from(spec.main.offset))
    }

    fn mesh_second(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No second weapon specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.second.center.0);

        (center, Vec3::from(spec.second.offset))
    }
}
impl BipedLargeLateralSpec {
    fn mesh_shoulder_l(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No shoulder specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.shoulder_l.lateral.0);

        (lateral, Vec3::from(spec.shoulder_l.offset))
    }

    fn mesh_shoulder_r(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No shoulder specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.shoulder_r.lateral.0);

        (lateral, Vec3::from(spec.shoulder_r.offset))
    }

    fn mesh_hand_l(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No hand specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.hand_l.lateral.0);

        (lateral, Vec3::from(spec.hand_l.offset))
    }

    fn mesh_hand_r(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No hand specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.hand_r.lateral.0);

        (lateral, Vec3::from(spec.hand_r.offset))
    }

    fn mesh_leg_l(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_l.lateral.0);

        (lateral, Vec3::from(spec.leg_l.offset))
    }

    fn mesh_leg_r(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_r.lateral.0);

        (lateral, Vec3::from(spec.leg_r.offset))
    }

    fn mesh_foot_l(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_l.lateral.0);

        (lateral, Vec3::from(spec.foot_l.offset))
    }

    fn mesh_foot_r(&self, species: BLSpecies, body_type: BLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_r.lateral.0);

        (lateral, Vec3::from(spec.foot_r.offset))
    }
}
////
#[derive(Deserialize)]
struct GolemCenterSpec(HashMap<(GSpecies, GBodyType), SidedGCenterVoxSpec>);

#[derive(Deserialize)]
struct SidedGCenterVoxSpec {
    head: GolemCenterSubSpec,
    torso_upper: GolemCenterSubSpec,
}
#[derive(Deserialize)]
struct GolemCenterSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    center: VoxSimple,
}

#[derive(Deserialize)]
struct GolemLateralSpec(HashMap<(GSpecies, GBodyType), SidedGLateralVoxSpec>);

#[derive(Deserialize)]
struct SidedGLateralVoxSpec {
    shoulder_l: GolemLateralSubSpec,
    shoulder_r: GolemLateralSubSpec,
    hand_l: GolemLateralSubSpec,
    hand_r: GolemLateralSubSpec,
    leg_l: GolemLateralSubSpec,
    leg_r: GolemLateralSubSpec,
    foot_l: GolemLateralSubSpec,
    foot_r: GolemLateralSubSpec,
}
#[derive(Deserialize)]
struct GolemLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    golem::Body,
    struct GolemSpec {
        center: GolemCenterSpec = "voxygen.voxel.golem_center_manifest",
        lateral: GolemLateralSpec = "voxygen.voxel.golem_lateral_manifest",
    },
    |FigureKey { body, .. }, spec| {
        [
            Some(spec.center.asset.mesh_head(
                body.species,
                body.body_type,
            )),
            Some(spec.center.asset.mesh_torso_upper(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_shoulder_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_shoulder_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_hand_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_hand_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_leg_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_leg_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_r(
                body.species,
                body.body_type,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    },
);

impl GolemCenterSpec {
    fn mesh_head(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.head.center.0);

        (center, Vec3::from(spec.head.offset))
    }

    fn mesh_torso_upper(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No torso upper specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let center = graceful_load_segment(&spec.torso_upper.center.0);

        (center, Vec3::from(spec.torso_upper.offset))
    }
}
impl GolemLateralSpec {
    fn mesh_shoulder_l(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No shoulder specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.shoulder_l.lateral.0);

        (lateral, Vec3::from(spec.shoulder_l.offset))
    }

    fn mesh_shoulder_r(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No shoulder specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.shoulder_r.lateral.0);

        (lateral, Vec3::from(spec.shoulder_r.offset))
    }

    fn mesh_hand_l(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No hand specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.hand_l.lateral.0);

        (lateral, Vec3::from(spec.hand_l.offset))
    }

    fn mesh_hand_r(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No hand specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.hand_r.lateral.0);

        (lateral, Vec3::from(spec.hand_r.offset))
    }

    fn mesh_leg_l(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_l.lateral.0);

        (lateral, Vec3::from(spec.leg_l.offset))
    }

    fn mesh_leg_r(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_r.lateral.0);

        (lateral, Vec3::from(spec.leg_r.offset))
    }

    fn mesh_foot_l(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_l.lateral.0);

        (lateral, Vec3::from(spec.foot_l.offset))
    }

    fn mesh_foot_r(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.foot_r.lateral.0);

        (lateral, Vec3::from(spec.foot_r.offset))
    }
}

/////

#[derive(Deserialize)]
struct QuadrupedLowCentralSpec(HashMap<(QLSpecies, QLBodyType), SidedQLCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedQLCentralVoxSpec {
    upper: QuadrupedLowCentralSubSpec,
    lower: QuadrupedLowCentralSubSpec,
    jaw: QuadrupedLowCentralSubSpec,
    chest: QuadrupedLowCentralSubSpec,
    tail_front: QuadrupedLowCentralSubSpec,
    tail_rear: QuadrupedLowCentralSubSpec,
}
#[derive(Deserialize)]
struct QuadrupedLowCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}

#[derive(Deserialize)]
struct QuadrupedLowLateralSpec(HashMap<(QLSpecies, QLBodyType), SidedQLLateralVoxSpec>);
#[derive(Deserialize)]
struct SidedQLLateralVoxSpec {
    front_left: QuadrupedLowLateralSubSpec,
    front_right: QuadrupedLowLateralSubSpec,
    back_left: QuadrupedLowLateralSubSpec,
    back_right: QuadrupedLowLateralSubSpec,
}
#[derive(Deserialize)]
struct QuadrupedLowLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    quadruped_low::Body,
    struct QuadrupedLowSpec {
        central: QuadrupedLowCentralSpec = "voxygen.voxel.quadruped_low_central_manifest",
        lateral: QuadrupedLowLateralSpec = "voxygen.voxel.quadruped_low_lateral_manifest",
    },
    |FigureKey { body, .. }, spec| {
        [
            Some(spec.central.asset.mesh_head_upper(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_head_lower(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_jaw(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_chest(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_tail_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.asset.mesh_tail_rear(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.asset.mesh_foot_br(
                body.species,
                body.body_type,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    },
);

impl QuadrupedLowCentralSpec {
    fn mesh_head_upper(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No upper head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.upper.central.0);

        (central, Vec3::from(spec.upper.offset))
    }

    fn mesh_head_lower(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No lower head specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.lower.central.0);

        (central, Vec3::from(spec.lower.offset))
    }

    fn mesh_jaw(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No jaw specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.jaw.central.0);

        (central, Vec3::from(spec.jaw.offset))
    }

    fn mesh_chest(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No chest specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.chest.central.0);

        (central, Vec3::from(spec.chest.offset))
    }

    fn mesh_tail_rear(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No tail_rear specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.tail_rear.central.0);

        (central, Vec3::from(spec.tail_rear.offset))
    }

    fn mesh_tail_front(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No tail_front specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.tail_front.central.0);

        (central, Vec3::from(spec.tail_front.offset))
    }
}

impl QuadrupedLowLateralSpec {
    fn mesh_foot_fl(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.front_left.lateral.0);

        (lateral, Vec3::from(spec.front_left.offset))
    }

    fn mesh_foot_fr(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.front_right.lateral.0);

        (lateral, Vec3::from(spec.front_right.offset))
    }

    fn mesh_foot_bl(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.back_left.lateral.0);

        (lateral, Vec3::from(spec.back_left.offset))
    }

    fn mesh_foot_br(&self, species: QLSpecies, body_type: QLBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.back_right.lateral.0);

        (lateral, Vec3::from(spec.back_right.offset))
    }
}

////
make_vox_spec!(
    object::Body,
    struct ObjectSpec {},
    |FigureKey { body, .. }, _spec| {
        [
            Some(mesh_object(body)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    },
);

fn mesh_object(obj: &object::Body) -> BoneMeshes {
    use object::Body;

    let (name, offset) = match obj {
        Body::Arrow => (
            "weapon.projectile.simple-arrow",
            Vec3::new(-0.5, -6.0, -1.5),
        ),
        Body::Bomb => ("object.bomb", Vec3::new(-5.5, -5.5, 0.0)),
        Body::FireworkBlue => (
            "weapon.projectile.fireworks_blue-0",
            Vec3::new(0.0, 0.0, 0.0),
        ),
        Body::FireworkGreen => (
            "weapon.projectile.fireworks_green-0",
            Vec3::new(0.0, 0.0, 0.0),
        ),
        Body::FireworkPurple => (
            "weapon.projectile.fireworks_purple-0",
            Vec3::new(0.0, 0.0, 0.0),
        ),
        Body::FireworkRed => (
            "weapon.projectile.fireworks_red-0",
            Vec3::new(0.0, 0.0, 0.0),
        ),
        Body::FireworkYellow => (
            "weapon.projectile.fireworks_yellow-0",
            Vec3::new(0.0, 0.0, 0.0),
        ),
        Body::Scarecrow => ("object.scarecrow", Vec3::new(-9.5, -4.0, 0.0)),
        Body::Cauldron => ("object.cauldron", Vec3::new(-10.0, -10.0, 0.0)),
        Body::ChestVines => ("object.chest_vines", Vec3::new(-7.5, -6.0, 0.0)),
        Body::Chest => ("object.chest", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestDark => ("object.chest_dark", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestDemon => ("object.chest_demon", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestGold => ("object.chest_gold", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestLight => ("object.chest_light", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestOpen => ("object.chest_open", Vec3::new(-7.5, -6.0, 0.0)),
        Body::ChestSkull => ("object.chest_skull", Vec3::new(-7.5, -6.0, 0.0)),
        Body::Pumpkin => ("object.pumpkin", Vec3::new(-5.5, -4.0, 0.0)),
        Body::Pumpkin2 => ("object.pumpkin_2", Vec3::new(-5.0, -4.0, 0.0)),
        Body::Pumpkin3 => ("object.pumpkin_3", Vec3::new(-5.0, -4.0, 0.0)),
        Body::Pumpkin4 => ("object.pumpkin_4", Vec3::new(-5.0, -4.0, 0.0)),
        Body::Pumpkin5 => ("object.pumpkin_5", Vec3::new(-4.0, -5.0, 0.0)),
        Body::Campfire => ("object.campfire", Vec3::new(-9.0, -10.0, 0.0)),
        Body::CampfireLit => ("object.campfire_lit", Vec3::new(-9.0, -10.0, 0.0)),
        Body::LanternGround => ("object.lantern_ground", Vec3::new(-3.5, -3.5, 0.0)),
        Body::LanternGroundOpen => ("object.lantern_ground_open", Vec3::new(-3.5, -3.5, 0.0)),
        Body::LanternStanding => ("object.lantern_standing", Vec3::new(-7.5, -3.5, 0.0)),
        Body::LanternStanding2 => ("object.lantern_standing_2", Vec3::new(-11.5, -3.5, 0.0)),
        Body::PotionRed => ("object.potion_red", Vec3::new(-2.0, -2.0, 0.0)),
        Body::PotionBlue => ("object.potion_blue", Vec3::new(-2.0, -2.0, 0.0)),
        Body::PotionGreen => ("object.potion_green", Vec3::new(-2.0, -2.0, 0.0)),
        Body::Crate => ("object.crate", Vec3::new(-7.0, -7.0, 0.0)),
        Body::Tent => ("object.tent", Vec3::new(-18.5, -19.5, 0.0)),
        Body::WindowSpooky => ("object.window_spooky", Vec3::new(-15.0, -1.5, -1.0)),
        Body::DoorSpooky => ("object.door_spooky", Vec3::new(-15.0, -4.5, 0.0)),
        Body::Table => ("object.table", Vec3::new(-12.0, -8.0, 0.0)),
        Body::Table2 => ("object.table_2", Vec3::new(-8.0, -8.0, 0.0)),
        Body::Table3 => ("object.table_3", Vec3::new(-10.0, -10.0, 0.0)),
        Body::Drawer => ("object.drawer", Vec3::new(-11.0, -7.5, 0.0)),
        Body::BedBlue => ("object.bed_human_blue", Vec3::new(-11.0, -15.0, 0.0)),
        Body::Anvil => ("object.anvil", Vec3::new(-3.0, -7.0, 0.0)),
        Body::Gravestone => ("object.gravestone", Vec3::new(-5.0, -2.0, 0.0)),
        Body::Gravestone2 => ("object.gravestone_2", Vec3::new(-8.5, -3.0, 0.0)),
        Body::Chair => ("object.chair", Vec3::new(-5.0, -4.5, 0.0)),
        Body::Chair2 => ("object.chair_2", Vec3::new(-5.0, -4.5, 0.0)),
        Body::Chair3 => ("object.chair_3", Vec3::new(-5.0, -4.5, 0.0)),
        Body::Bench => ("object.bench", Vec3::new(-8.8, -5.0, 0.0)),
        Body::Carpet => ("object.carpet", Vec3::new(-14.0, -14.0, -0.5)),
        Body::Bedroll => ("object.bedroll", Vec3::new(-11.0, -19.5, -0.5)),
        Body::CarpetHumanRound => ("object.carpet_human_round", Vec3::new(-14.0, -14.0, -0.5)),
        Body::CarpetHumanSquare => ("object.carpet_human_square", Vec3::new(-13.5, -14.0, -0.5)),
        Body::CarpetHumanSquare2 => (
            "object.carpet_human_square_2",
            Vec3::new(-13.5, -14.0, -0.5),
        ),
        Body::CarpetHumanSquircle => (
            "object.carpet_human_squircle",
            Vec3::new(-21.0, -21.0, -0.5),
        ),
        Body::Pouch => ("object.pouch", Vec3::new(-5.5, -4.5, 0.0)),
        Body::CraftingBench => ("object.crafting_bench", Vec3::new(-9.5, -7.0, 0.0)),
        Body::ArrowSnake => ("weapon.projectile.snake-arrow", Vec3::new(-1.5, -6.5, 0.0)),
        Body::BoltFire => ("weapon.projectile.fire-bolt-0", Vec3::new(-3.0, -5.5, -3.0)),
        Body::BoltFireBig => ("weapon.projectile.fire-bolt-1", Vec3::new(-6.0, -6.0, -6.0)),
        Body::TrainingDummy => ("object.training_dummy", Vec3::new(-7.0, -5.0, 0.0)),
        Body::MultiArrow => ("weapon.projectile.multi-arrow", Vec3::new(-4.0, -9.5, -5.0)),
    };
    load_mesh(name, offset)
}
