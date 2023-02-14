use super::cache::{FigureKey, ToolKey};
use common::{
    assets::{self, AssetExt, AssetHandle, DotVoxAsset, ReloadWatcher, Ron},
    comp::{
        arthropod::{self, BodyType as ABodyType, Species as ASpecies},
        biped_large::{self, BodyType as BLBodyType, Species as BLSpecies},
        biped_small,
        bird_large::{self, BodyType as BLABodyType, Species as BLASpecies},
        bird_medium::{self, BodyType as BMBodyType, Species as BMSpecies},
        dragon::{self, BodyType as DBodyType, Species as DSpecies},
        fish_medium::{self, BodyType as FMBodyType, Species as FMSpecies},
        fish_small::{self, BodyType as FSBodyType, Species as FSSpecies},
        golem::{self, BodyType as GBodyType, Species as GSpecies},
        humanoid::{self, Body, BodyType, EyeColor, Skin, Species},
        item::item_key::ItemKey,
        item_drop, object,
        quadruped_low::{self, BodyType as QLBodyType, Species as QLSpecies},
        quadruped_medium::{self, BodyType as QMBodyType, Species as QMSpecies},
        quadruped_small::{self, BodyType as QSBodyType, Species as QSSpecies},
        ship::{
            self,
            figuredata::{ShipCentralSubSpec, ShipSpec},
        },
        theropod::{self, BodyType as TBodyType, Species as TSpecies},
    },
    figure::{Cell, DynaUnionizer, MatCell, MatSegment, Material, Segment},
    vol::{IntoFullPosIterator, ReadVol, Vox},
};
use hashbrown::HashMap;
use serde::Deserialize;
use std::{fmt, hash::Hash};
use tracing::{error, warn};
use vek::*;

pub type BoneMeshes = (Segment, Vec3<f32>);

fn load_segment(mesh_name: &str) -> Segment {
    let full_specifier: String = ["voxygen.voxel.", mesh_name].concat();
    Segment::from(&DotVoxAsset::load_expect(&full_specifier).read().0)
}
fn graceful_load_vox(mesh_name: &str) -> AssetHandle<DotVoxAsset> {
    let full_specifier: String = ["voxygen.voxel.", mesh_name].concat();
    graceful_load_vox_fullspec(&full_specifier)
}
fn graceful_load_vox_fullspec(full_specifier: &str) -> AssetHandle<DotVoxAsset> {
    match DotVoxAsset::load(full_specifier) {
        Ok(dot_vox) => dot_vox,
        Err(_) => {
            error!(?full_specifier, "Could not load vox file for figure");
            DotVoxAsset::load_expect("voxygen.voxel.not_found")
        },
    }
}
fn graceful_load_segment(mesh_name: &str) -> Segment {
    Segment::from(&graceful_load_vox(mesh_name).read().0)
}
fn graceful_load_segment_fullspec(full_specifier: &str) -> Segment {
    Segment::from(&graceful_load_vox_fullspec(full_specifier).read().0)
}
fn graceful_load_segment_flipped(mesh_name: &str, flipped: bool) -> Segment {
    Segment::from_vox(&graceful_load_vox(mesh_name).read().0, flipped)
}
fn graceful_load_mat_segment(mesh_name: &str) -> MatSegment {
    MatSegment::from(&graceful_load_vox(mesh_name).read().0)
}
fn graceful_load_mat_segment_flipped(mesh_name: &str) -> MatSegment {
    MatSegment::from_vox(&graceful_load_vox(mesh_name).read().0, true)
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

        linear_to_srgb(c1 * c2).map(|e| (e.clamp(0.0, 1.0) * 255.0) as u8)
    } else {
        rgb
    }
}

/// A set of reloadable specifications for a Body.
pub trait BodySpec: Sized {
    type Spec;
    /// Cloned on each cache invalidation. If this type is expensive to clone,
    /// place it behind an [`Arc`].
    type Manifests: Send + Sync + Clone;
    type Extra: Send + Sync;

    /// Initialize all the specifications for this Body.
    fn load_spec() -> Result<Self::Manifests, assets::Error>;

    /// Determine whether the cache's manifest was reloaded
    fn reload_watcher(manifests: &Self::Manifests) -> ReloadWatcher;

    /// Mesh bones using the given spec, character state, and mesh generation
    /// function.
    ///
    /// NOTE: We deliberately call this function with only the key into the
    /// cache, to enforce that the cached state only depends on the key.  We
    /// may end up using a mechanism different from this cache eventually,
    /// in which case this strategy might change.
    fn bone_meshes(
        key: &FigureKey<Self>,
        manifests: &Self::Manifests,
        extra: Self::Extra,
    ) -> [Option<BoneMeshes>; anim::MAX_BONE_COUNT];
}

macro_rules! make_vox_spec {
    (
        $body:ty,
        struct $Spec:ident { $( $(+)? $field:ident: $ty:ty = $asset_path:literal),* $(,)? },
        |$self_pat:pat, $spec_pat:pat_param| $bone_meshes:block $(,)?
    ) => {
        #[derive(Clone)]
        pub struct $Spec {
            $( $field: AssetHandle<Ron<$ty>>, )*
        }

        impl assets::Compound for $Spec {
            fn load(_: assets::AnyCache, _: &assets::SharedString) -> Result<Self, assets::BoxedError> {
                Ok($Spec {
                    $( $field: AssetExt::load($asset_path)?, )*
                })
            }
        }

        impl BodySpec for $body {
            type Spec = $Spec;
            type Manifests = AssetHandle<Self::Spec>;
            type Extra = ();

            fn load_spec() -> Result<Self::Manifests, assets::Error> {
                Self::Spec::load("")
            }

            fn reload_watcher(manifests: &Self::Manifests) -> ReloadWatcher { manifests.reload_watcher() }

            fn bone_meshes(
                $self_pat: &FigureKey<Self>,
                manifests: &Self::Manifests,
                _: Self::Extra,
            ) -> [Option<BoneMeshes>; anim::MAX_BONE_COUNT] {
                let $spec_pat = &*manifests.read();
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
struct VoxMirror(String, bool);

#[derive(Deserialize)]
struct ArmorVoxSpec {
    vox_spec: VoxSpec<f32>,
    color: Option<[u8; 3]>,
}

#[derive(Deserialize)]
enum ModularComponentSpec {
    /// item id, offset from origin to mount point
    Damage((String, [i32; 3])),
    /// item id, offset from origin to hand, offset from origin to mount point
    Held((String, [f32; 3], [i32; 3])),
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
    fn mesh_head(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        helmet: Option<(Segment, Vec3<i32>)>,
    ) -> BoneMeshes {
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
            .maybe_add(helmet)
            .unify_with(|v| if v.is_hollow() { Cell::empty() } else { v });
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
    K: Hash + Eq,
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
struct HumMainWeaponSpec(HashMap<ToolKey, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumModularComponentSpec(HashMap<String, ModularComponentSpec>);
#[derive(Deserialize)]
struct HumArmorLanternSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorGliderSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct HumArmorHeadSpec(ArmorVoxSpecMap<(Species, BodyType, String), ArmorVoxSpec>);
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
        main_weapon: HumMainWeaponSpec = "voxygen.voxel.biped_weapon_manifest",
        armor_lantern: HumArmorLanternSpec = "voxygen.voxel.humanoid_lantern_manifest",
        armor_glider: HumArmorGliderSpec = "voxygen.voxel.humanoid_glider_manifest",
        armor_head: HumArmorHeadSpec = "voxygen.voxel.humanoid_armor_head_manifest",
        // TODO: Add these.
        /* tabard: HumArmorTabardSpec = "voxygen.voxel.humanoid_armor_tabard_manifest", */
    },
    |FigureKey { body, item_key: _, extra }, spec| {
        const DEFAULT_LOADOUT: super::cache::CharacterCacheKey = super::cache::CharacterCacheKey {
            third_person: None,
            tool: None,
            lantern: None,
            glider: None,
            hand: None,
            foot: None,
            head: None,
        };

        // TODO: This is bad code, maybe this method should return Option<_>
        let loadout = extra.as_deref().unwrap_or(&DEFAULT_LOADOUT);
        let third_person = loadout.third_person.as_ref();
        let tool = loadout.tool.as_ref();
        let lantern = loadout.lantern.as_deref();
        let glider = loadout.glider.as_deref();
        let hand = loadout.hand.as_deref();
        let foot = loadout.foot.as_deref();

        let color = &spec.color.read().0;

        [
            third_person.map(|_| {
                spec.head.read().0.mesh_head(
                    body,
                    color,
                    spec.armor_head.read().0.load_head(
                        body,
                        loadout.head.as_deref()
                    ),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_chest.read().0.mesh_chest(
                    body,
                    color,
                    loadout.chest.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_belt.read().0.mesh_belt(
                    body,
                    color,
                    loadout.belt.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_back.read().0.mesh_back(
                    body,
                    color,
                    loadout.back.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_pants.read().0.mesh_pants(
                    body,
                    color,
                    loadout.pants.as_deref(),
                )
            }),
            Some(spec.armor_hand.read().0.mesh_left_hand(
                body,
                color,
                hand,
            )),
            Some(spec.armor_hand.read().0.mesh_right_hand(
                body,
                color,
                hand,
            )),
            Some(spec.armor_foot.read().0.mesh_left_foot(
                body,
                color,
                foot,
            )),
            Some(spec.armor_foot.read().0.mesh_right_foot(
                body,
                color,
                foot,
            )),
            third_person.map(|loadout| {
                spec.armor_shoulder.read().0.mesh_left_shoulder(
                    body,
                    color,
                    loadout.shoulder.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_shoulder.read().0.mesh_right_shoulder(
                    body,
                    color,
                    loadout.shoulder.as_deref(),
                )
            }),
            Some(spec.armor_glider.read().0.mesh_glider(
                body,
                color,
                glider,
            )),
            tool.and_then(|tool| tool.active.as_ref()).map(|tool| {
                spec.main_weapon.read().0.mesh_main_weapon(
                    tool,
                    false,
                )
            }),
            tool.and_then(|tool| tool.second.as_ref()).map(|tool| {
                spec.main_weapon.read().0.mesh_main_weapon(
                    tool,
                    true,
                )
            }),
            Some(spec.armor_lantern.read().0.mesh_lantern(
                body,
                color,
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
    fn mesh_main_weapon(&self, tool: &ToolKey, flipped: bool) -> BoneMeshes {
        let not_found = |tool: &ToolKey| {
            error!(?tool, "No tool/weapon specification exists");
            load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0))
        };

        let spec = match self.0.get(tool) {
            Some(spec) => spec,
            None => return not_found(tool),
        };

        let tool_kind_segment = graceful_load_segment_flipped(&spec.vox_spec.0, flipped);
        let mut offset = Vec3::from(spec.vox_spec.1);

        if flipped {
            offset.x = 0.0 - offset.x - (tool_kind_segment.sz.x as f32);
        }

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
    fn load_head(&self, body: &Body, head: Option<&str>) -> Option<(Segment, Vec3<i32>)> {
        match self
            .0
            .map
            .get(&(body.species, body.body_type, head?.to_string()))
        {
            Some(spec) => Some((
                graceful_load_segment(&spec.vox_spec.0),
                Vec3::<f32>::from(spec.vox_spec.1).as_(),
            )),
            None => {
                warn!("No specification for this head: {:?}", head);
                None
            },
        }
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
impl HumArmorGliderSpec {
    fn mesh_glider(
        &self,
        body: &Body,
        color_spec: &HumColorSpec,
        glider: Option<&str>,
    ) -> BoneMeshes {
        let spec = if let Some(kind) = glider {
            match self.0.map.get(kind) {
                Some(spec) => spec,
                None => {
                    error!(?kind, "No glider specification exists");
                    return load_mesh("not_found", Vec3::new(-4.0, -3.5, 2.0));
                },
            }
        } else {
            &self.0.default
        };

        let mut glider_segment = color_spec.color_segment(
            graceful_load_mat_segment(&spec.vox_spec.0),
            body.species.skin_color(body.skin),
            color_spec.hair_color(body.species, body.hair_color),
            body.species.eye_color(body.eye_color),
        );
        if let Some(color) = spec.color {
            let glider_color = Vec3::from(color);
            glider_segment =
                glider_segment.map_rgb(|rgb| recolor_grey(rgb, Rgb::from(glider_color)));
        }

        (glider_segment, Vec3::from(spec.vox_spec.1))
    }
}

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
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_chest(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_br(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail(
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
        let lateral = graceful_load_segment_flipped(&spec.left_front.lateral.0, true);

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
        let lateral = graceful_load_segment_flipped(&spec.left_back.lateral.0, true);

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
    head: QuadrupedMediumCentralSubSpec,
    neck: QuadrupedMediumCentralSubSpec,
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
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.central.read().0.mesh_neck(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.central.read().0.mesh_jaw(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_tail(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_torso_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_torso_back(
                body.species,
                body.body_type,
            )),
            third_person.map(|_| {
                spec.central.read().0.mesh_ears(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.lateral.read().0.mesh_leg_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_br(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_br(
                body.species,
                body.body_type,
            )),
            None,
        ]
    }
);

impl QuadrupedMediumCentralSpec {
    fn mesh_head(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
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

    fn mesh_neck(&self, species: QMSpecies, body_type: QMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No neck specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.neck.central.0);

        (central, Vec3::from(spec.neck.offset))
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
        let lateral = graceful_load_segment_flipped(&spec.leg_fl.lateral.0, true);

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
        let lateral = graceful_load_segment_flipped(&spec.leg_bl.lateral.0, true);

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
        let lateral = graceful_load_segment_flipped(&spec.foot_fl.lateral.0, true);

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
        let lateral = graceful_load_segment_flipped(&spec.foot_bl.lateral.0, true);

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
struct BirdMediumCentralSpec(HashMap<(BMSpecies, BMBodyType), SidedBMCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedBMCentralVoxSpec {
    head: BirdMediumCentralSubSpec,
    chest: BirdMediumCentralSubSpec,
    tail: BirdMediumCentralSubSpec,
}
#[derive(Deserialize)]
struct BirdMediumCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}

#[derive(Deserialize)]
struct BirdMediumLateralSpec(HashMap<(BMSpecies, BMBodyType), SidedBMLateralVoxSpec>);

#[derive(Deserialize)]
struct SidedBMLateralVoxSpec {
    wing_in_l: BirdMediumLateralSubSpec,
    wing_in_r: BirdMediumLateralSubSpec,
    wing_out_l: BirdMediumLateralSubSpec,
    wing_out_r: BirdMediumLateralSubSpec,
    leg_l: BirdMediumLateralSubSpec,
    leg_r: BirdMediumLateralSubSpec,
}
#[derive(Deserialize)]
struct BirdMediumLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    bird_medium::Body,
    struct BirdMediumSpec {
        central: BirdMediumCentralSpec = "voxygen.voxel.bird_medium_central_manifest",
        lateral: BirdMediumLateralSpec = "voxygen.voxel.bird_medium_lateral_manifest",
    },
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_chest(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_in_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_in_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_out_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_out_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_r(
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
        ]
    },
);

impl BirdMediumCentralSpec {
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
        let central = graceful_load_segment(&spec.head.central.0);

        (central, Vec3::from(spec.head.offset))
    }

    fn mesh_chest(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
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
        let central = graceful_load_segment(&spec.tail.central.0);

        (central, Vec3::from(spec.tail.offset))
    }
}
impl BirdMediumLateralSpec {
    fn mesh_wing_in_l(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing in in left specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.wing_in_l.lateral.0, true);

        (lateral, Vec3::from(spec.wing_in_l.offset))
    }

    fn mesh_wing_in_r(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing in right specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_in_r.lateral.0);

        (lateral, Vec3::from(spec.wing_in_r.offset))
    }

    fn mesh_wing_out_l(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing out specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.wing_out_l.lateral.0, true);

        (lateral, Vec3::from(spec.wing_out_l.offset))
    }

    fn mesh_wing_out_r(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing out specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_out_r.lateral.0);

        (lateral, Vec3::from(spec.wing_out_r.offset))
    }

    fn mesh_leg_l(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
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
        let lateral = graceful_load_segment_flipped(&spec.leg_l.lateral.0, true);

        (lateral, Vec3::from(spec.leg_l.offset))
    }

    fn mesh_leg_r(&self, species: BMSpecies, body_type: BMBodyType) -> BoneMeshes {
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
}

////
#[derive(Deserialize)]
struct TheropodCentralSpec(HashMap<(TSpecies, TBodyType), SidedTCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedTCentralVoxSpec {
    head: TheropodCentralSubSpec,
    jaw: TheropodCentralSubSpec,
    neck: TheropodCentralSubSpec,
    chest_front: TheropodCentralSubSpec,
    chest_back: TheropodCentralSubSpec,
    tail_front: TheropodCentralSubSpec,
    tail_back: TheropodCentralSubSpec,
}
#[derive(Deserialize)]
struct TheropodCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}
#[derive(Deserialize)]
struct TheropodLateralSpec(HashMap<(TSpecies, TBodyType), SidedTLateralVoxSpec>);

#[derive(Deserialize)]
struct SidedTLateralVoxSpec {
    hand_l: TheropodLateralSubSpec,
    hand_r: TheropodLateralSubSpec,
    leg_l: TheropodLateralSubSpec,
    leg_r: TheropodLateralSubSpec,
    foot_l: TheropodLateralSubSpec,
    foot_r: TheropodLateralSubSpec,
}
#[derive(Deserialize)]
struct TheropodLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}
make_vox_spec!(
    theropod::Body,
    struct TheropodSpec {
        central: TheropodCentralSpec = "voxygen.voxel.theropod_central_manifest",
        lateral: TheropodLateralSpec = "voxygen.voxel.theropod_lateral_manifest",
    },
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.central.read().0.mesh_jaw(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_neck(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_chest_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_chest_back(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail_back(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_hand_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_hand_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_r(
                body.species,
                body.body_type,
            )),
            None,
            None,
            None,
        ]
    },
);

impl TheropodCentralSpec {
    fn mesh_head(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
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

    fn mesh_jaw(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
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

    fn mesh_neck(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
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
        let central = graceful_load_segment(&spec.neck.central.0);

        (central, Vec3::from(spec.neck.offset))
    }

    fn mesh_chest_front(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No front chest specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.chest_front.central.0);

        (central, Vec3::from(spec.chest_front.offset))
    }

    fn mesh_chest_back(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No back chest specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.chest_back.central.0);

        (central, Vec3::from(spec.chest_back.offset))
    }

    fn mesh_tail_front(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No front tail specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.tail_front.central.0);

        (central, Vec3::from(spec.tail_front.offset))
    }

    fn mesh_tail_back(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No back tail specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.tail_back.central.0);

        (central, Vec3::from(spec.tail_back.offset))
    }
}
impl TheropodLateralSpec {
    fn mesh_hand_l(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No left hand specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.hand_l.lateral.0, true);

        (lateral, Vec3::from(spec.hand_l.offset))
    }

    fn mesh_hand_r(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No right hand specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.hand_r.lateral.0);

        (lateral, Vec3::from(spec.hand_r.offset))
    }

    fn mesh_leg_l(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No left leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.leg_l.lateral.0, true);

        (lateral, Vec3::from(spec.leg_l.offset))
    }

    fn mesh_leg_r(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No right leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_r.lateral.0);

        (lateral, Vec3::from(spec.leg_r.offset))
    }

    fn mesh_foot_l(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No left foot specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.foot_l.lateral.0, true);

        (lateral, Vec3::from(spec.foot_l.offset))
    }

    fn mesh_foot_r(&self, species: TSpecies, body_type: TBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No right foot specification exists for the combination of {:?} and {:?}",
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
struct ArthropodCentralSpec(HashMap<(ASpecies, ABodyType), SidedACentralVoxSpec>);

#[derive(Deserialize)]
struct SidedACentralVoxSpec {
    head: ArthropodCentralSubSpec,
    chest: ArthropodCentralSubSpec,
}
#[derive(Deserialize)]
struct ArthropodCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}
#[derive(Deserialize)]
struct ArthropodLateralSpec(HashMap<(ASpecies, ABodyType), SidedALateralVoxSpec>);

#[derive(Deserialize)]
struct SidedALateralVoxSpec {
    mandible_l: ArthropodLateralSubSpec,
    mandible_r: ArthropodLateralSubSpec,
    wing_fl: ArthropodLateralSubSpec,
    wing_fr: ArthropodLateralSubSpec,
    wing_bl: ArthropodLateralSubSpec,
    wing_br: ArthropodLateralSubSpec,
    leg_fl: ArthropodLateralSubSpec,
    leg_fr: ArthropodLateralSubSpec,
    leg_fcl: ArthropodLateralSubSpec,
    leg_fcr: ArthropodLateralSubSpec,
    leg_bcl: ArthropodLateralSubSpec,
    leg_bcr: ArthropodLateralSubSpec,
    leg_bl: ArthropodLateralSubSpec,
    leg_br: ArthropodLateralSubSpec,
}
#[derive(Deserialize)]
struct ArthropodLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}
make_vox_spec!(
    arthropod::Body,
    struct ArthropodSpec {
        central: ArthropodCentralSpec = "voxygen.voxel.arthropod_central_manifest",
        lateral: ArthropodLateralSpec = "voxygen.voxel.arthropod_lateral_manifest",
    },
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_chest(
                body.species,
                body.body_type,
            )),
            third_person.map(|_| {
                spec.lateral.read().0.mesh_mandible_l(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.lateral.read().0.mesh_mandible_r(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.lateral.read().0.mesh_wing_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_br(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_fcl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_fcr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_bcl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_bcr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_br(
                body.species,
                body.body_type,
            )),
        ]
    },
);

impl ArthropodCentralSpec {
    fn mesh_head(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
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

    fn mesh_chest(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
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
}
impl ArthropodLateralSpec {
    fn mesh_mandible_l(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No left mandible specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.mandible_l.lateral.0, true);

        (lateral, Vec3::from(spec.mandible_l.offset))
    }

    fn mesh_mandible_r(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No right mandible specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.mandible_r.lateral.0);

        (lateral, Vec3::from(spec.mandible_r.offset))
    }

    fn mesh_wing_fl(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No front left wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.wing_fl.lateral.0, true);

        (lateral, Vec3::from(spec.wing_fl.offset))
    }

    fn mesh_wing_fr(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No front right wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_fr.lateral.0);

        (lateral, Vec3::from(spec.wing_fr.offset))
    }

    fn mesh_wing_bl(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No back left wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.wing_bl.lateral.0, true);

        (lateral, Vec3::from(spec.wing_bl.offset))
    }

    fn mesh_wing_br(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No back right wing specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_br.lateral.0);

        (lateral, Vec3::from(spec.wing_br.offset))
    }

    fn mesh_leg_fl(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No front left leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.leg_fl.lateral.0, true);

        (lateral, Vec3::from(spec.leg_fl.offset))
    }

    fn mesh_leg_fr(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No front right leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_fr.lateral.0);

        (lateral, Vec3::from(spec.leg_fr.offset))
    }

    fn mesh_leg_fcl(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No front center left leg specification exists for the combination of {:?} \
                     and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.leg_fcl.lateral.0, true);

        (lateral, Vec3::from(spec.leg_fcl.offset))
    }

    fn mesh_leg_fcr(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No front center right leg specification exists for the combination of {:?} \
                     and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_fcr.lateral.0);

        (lateral, Vec3::from(spec.leg_fcr.offset))
    }

    fn mesh_leg_bcl(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No back center left leg specification exists for the combination of {:?} and \
                     {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.leg_bcl.lateral.0, true);

        (lateral, Vec3::from(spec.leg_bcl.offset))
    }

    fn mesh_leg_bcr(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No back center right leg specification exists for the combination of {:?} \
                     and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_bcr.lateral.0);

        (lateral, Vec3::from(spec.leg_bcr.offset))
    }

    fn mesh_leg_bl(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No back left leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.leg_bl.lateral.0, true);

        (lateral, Vec3::from(spec.leg_bl.offset))
    }

    fn mesh_leg_br(&self, species: ASpecies, body_type: ABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No back right leg specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.leg_br.lateral.0);

        (lateral, Vec3::from(spec.leg_br.offset))
    }
}
////
#[derive(Deserialize)]
struct FishMediumCentralSpec(HashMap<(FMSpecies, FMBodyType), SidedFMCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedFMCentralVoxSpec {
    head: FishMediumCentralSubSpec,
    jaw: FishMediumCentralSubSpec,
    chest_front: FishMediumCentralSubSpec,
    chest_back: FishMediumCentralSubSpec,
    tail: FishMediumCentralSubSpec,
}
#[derive(Deserialize)]
struct FishMediumCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}
#[derive(Deserialize)]
struct FishMediumLateralSpec(HashMap<(FMSpecies, FMBodyType), SidedFMLateralVoxSpec>);
#[derive(Deserialize)]
struct SidedFMLateralVoxSpec {
    fin_l: FishMediumLateralSubSpec,
    fin_r: FishMediumLateralSubSpec,
}
#[derive(Deserialize)]
struct FishMediumLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    fish_medium::Body,
    struct FishMediumSpec {
        central: FishMediumCentralSpec = "voxygen.voxel.fish_medium_central_manifest",
        lateral: FishMediumLateralSpec = "voxygen.voxel.fish_medium_lateral_manifest",
    },
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_jaw(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_chest_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_chest_back(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_fin_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_fin_r(
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

impl FishMediumCentralSpec {
    fn mesh_head(&self, species: FMSpecies, body_type: FMBodyType) -> BoneMeshes {
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

    fn mesh_jaw(&self, species: FMSpecies, body_type: FMBodyType) -> BoneMeshes {
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

    fn mesh_chest_front(&self, species: FMSpecies, body_type: FMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No front chest specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.chest_front.central.0);

        (central, Vec3::from(spec.chest_front.offset))
    }

    fn mesh_chest_back(&self, species: FMSpecies, body_type: FMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No back chest specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.chest_back.central.0);

        (central, Vec3::from(spec.chest_back.offset))
    }

    fn mesh_tail(&self, species: FMSpecies, body_type: FMBodyType) -> BoneMeshes {
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

impl FishMediumLateralSpec {
    fn mesh_fin_l(&self, species: FMSpecies, body_type: FMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No fin specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.fin_l.lateral.0, true);

        (lateral, Vec3::from(spec.fin_l.offset))
    }

    fn mesh_fin_r(&self, species: FMSpecies, body_type: FMBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No fin specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.fin_r.lateral.0);

        (lateral, Vec3::from(spec.fin_r.offset))
    }
}

////
#[derive(Deserialize)]
struct FishSmallCentralSpec(HashMap<(FSSpecies, FSBodyType), SidedFSCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedFSCentralVoxSpec {
    chest: FishSmallCentralSubSpec,
    tail: FishSmallCentralSubSpec,
}
#[derive(Deserialize)]
struct FishSmallCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}
#[derive(Deserialize)]
struct FishSmallLateralSpec(HashMap<(FSSpecies, FSBodyType), SidedFSLateralVoxSpec>);
#[derive(Deserialize)]
struct SidedFSLateralVoxSpec {
    fin_l: FishSmallLateralSubSpec,
    fin_r: FishSmallLateralSubSpec,
}
#[derive(Deserialize)]
struct FishSmallLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    fish_small::Body,
    struct FishSmallSpec {
        central: FishSmallCentralSpec = "voxygen.voxel.fish_small_central_manifest",
        lateral: FishSmallLateralSpec = "voxygen.voxel.fish_small_lateral_manifest",
    },
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_chest(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_tail(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_fin_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_fin_r(
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
            None,
        ]
    },
);

impl FishSmallCentralSpec {
    fn mesh_chest(&self, species: FSSpecies, body_type: FSBodyType) -> BoneMeshes {
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

    fn mesh_tail(&self, species: FSSpecies, body_type: FSBodyType) -> BoneMeshes {
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

impl FishSmallLateralSpec {
    fn mesh_fin_l(&self, species: FSSpecies, body_type: FSBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No fin specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.fin_l.lateral.0, true);

        (lateral, Vec3::from(spec.fin_l.offset))
    }

    fn mesh_fin_r(&self, species: FSSpecies, body_type: FSBodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No fin specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.fin_r.lateral.0);

        (lateral, Vec3::from(spec.fin_r.offset))
    }
}

////

#[derive(Deserialize)]
struct BipedSmallWeaponSpec(HashMap<ToolKey, ArmorVoxSpec>);
#[derive(Deserialize)]
struct BipedSmallArmorHeadSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct BipedSmallArmorHandSpec(ArmorVoxSpecMap<String, SidedArmorVoxSpec>);
#[derive(Deserialize)]
struct BipedSmallArmorFootSpec(ArmorVoxSpecMap<String, SidedArmorVoxSpec>);
#[derive(Deserialize)]
struct BipedSmallArmorChestSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct BipedSmallArmorPantsSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
#[derive(Deserialize)]
struct BipedSmallArmorTailSpec(ArmorVoxSpecMap<String, ArmorVoxSpec>);
make_vox_spec!(
    biped_small::Body,
    struct BipedSmallSpec {
        armor_foot: BipedSmallArmorFootSpec = "voxygen.voxel.biped_small_armor_foot_manifest",
        weapon: BipedSmallWeaponSpec = "voxygen.voxel.biped_weapon_manifest",
        armor_hand: BipedSmallArmorHandSpec = "voxygen.voxel.biped_small_armor_hand_manifest",
        armor_chest: BipedSmallArmorChestSpec = "voxygen.voxel.biped_small_armor_chest_manifest",
        armor_pants: BipedSmallArmorPantsSpec = "voxygen.voxel.biped_small_armor_pants_manifest",
        armor_head: BipedSmallArmorHeadSpec = "voxygen.voxel.biped_small_armor_head_manifest",
        armor_tail: BipedSmallArmorTailSpec = "voxygen.voxel.biped_small_armor_tail_manifest",

    },
    |FigureKey { body: _, item_key: _, extra }, spec| {
        const DEFAULT_LOADOUT: super::cache::CharacterCacheKey = super::cache::CharacterCacheKey {
            third_person: None,
            tool: None,
            lantern: None,
            glider: None,
            hand: None,
            foot: None,
            head: None,
        };

        // TODO: This is bad code, maybe this method should return Option<_>
        let loadout = extra.as_deref().unwrap_or(&DEFAULT_LOADOUT);
        let third_person = loadout.third_person.as_ref();
        let tool = loadout.tool.as_ref();
        let hand = loadout.hand.as_deref();
        let foot = loadout.foot.as_deref();


        [
            third_person.map(|loadout| {
                spec.armor_head.read().0.mesh_head(
                    loadout.head.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_chest.read().0.mesh_chest(
                    loadout.chest.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_pants.read().0.mesh_pants(
                    loadout.pants.as_deref(),
                )
            }),
            third_person.map(|loadout| {
                spec.armor_tail.read().0.mesh_tail(
                    loadout.belt.as_deref(),
                )
            }),
            tool.and_then(|tool| tool.active.as_ref()).map(|tool| {
                spec.weapon.read().0.mesh_main(
                    tool,
                    false,
                )
            }),
            Some(spec.armor_hand.read().0.mesh_left_hand(
                hand,
            )),
            Some(spec.armor_hand.read().0.mesh_right_hand(
                hand,
            )),
            Some(spec.armor_foot.read().0.mesh_left_foot(
                foot,
            )),
            Some(spec.armor_foot.read().0.mesh_right_foot(
                foot,
            )),
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

impl BipedSmallArmorHeadSpec {
    fn mesh_head(&self, head: Option<&str>) -> BoneMeshes {
        let spec = if let Some(head) = head {
            match self.0.map.get(head) {
                Some(spec) => spec,
                None => {
                    error!(?head, "No head specification exists");
                    return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
                },
            }
        } else {
            &self.0.default
        };

        let head_segment = graceful_load_segment(&spec.vox_spec.0);

        let offset = Vec3::new(spec.vox_spec.1[0], spec.vox_spec.1[1], spec.vox_spec.1[2]);

        (head_segment, offset)
    }
}
impl BipedSmallArmorChestSpec {
    fn mesh_chest(&self, chest: Option<&str>) -> BoneMeshes {
        let spec = if let Some(chest) = chest {
            match self.0.map.get(chest) {
                Some(spec) => spec,
                None => {
                    error!(?chest, "No chest specification exists");
                    return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
                },
            }
        } else {
            &self.0.default
        };

        let chest_segment = graceful_load_segment(&spec.vox_spec.0);

        let offset = Vec3::new(spec.vox_spec.1[0], spec.vox_spec.1[1], spec.vox_spec.1[2]);

        (chest_segment, offset)
    }
}
impl BipedSmallArmorTailSpec {
    fn mesh_tail(&self, tail: Option<&str>) -> BoneMeshes {
        let spec = if let Some(tail) = tail {
            match self.0.map.get(tail) {
                Some(spec) => spec,
                None => {
                    error!(?tail, "No tail specification exists");
                    return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
                },
            }
        } else {
            &self.0.default
        };

        let tail_segment = graceful_load_segment(&spec.vox_spec.0);

        let offset = Vec3::new(spec.vox_spec.1[0], spec.vox_spec.1[1], spec.vox_spec.1[2]);

        (tail_segment, offset)
    }
}
impl BipedSmallArmorPantsSpec {
    fn mesh_pants(&self, pants: Option<&str>) -> BoneMeshes {
        let spec = if let Some(pants) = pants {
            match self.0.map.get(pants) {
                Some(spec) => spec,
                None => {
                    error!(?pants, "No pants specification exists");
                    return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
                },
            }
        } else {
            &self.0.default
        };

        let pants_segment = graceful_load_segment(&spec.vox_spec.0);

        let offset = Vec3::new(spec.vox_spec.1[0], spec.vox_spec.1[1], spec.vox_spec.1[2]);

        (pants_segment, offset)
    }
}
impl BipedSmallArmorHandSpec {
    fn mesh_hand(&self, hand: Option<&str>, flipped: bool) -> BoneMeshes {
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

        let hand_segment = if flipped {
            graceful_load_segment_flipped(&spec.left.vox_spec.0, true)
        } else {
            graceful_load_segment(&spec.right.vox_spec.0)
        };
        let offset = if flipped {
            spec.left.vox_spec.1
        } else {
            spec.right.vox_spec.1
        };

        (hand_segment, Vec3::from(offset))
    }

    fn mesh_left_hand(&self, hand: Option<&str>) -> BoneMeshes { self.mesh_hand(hand, true) }

    fn mesh_right_hand(&self, hand: Option<&str>) -> BoneMeshes { self.mesh_hand(hand, false) }
}
impl BipedSmallArmorFootSpec {
    fn mesh_foot(&self, foot: Option<&str>, flipped: bool) -> BoneMeshes {
        let spec = if let Some(foot) = foot {
            match self.0.map.get(foot) {
                Some(spec) => spec,
                None => {
                    error!(?foot, "No foot specification exists");
                    return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
                },
            }
        } else {
            &self.0.default
        };

        let foot_segment = if flipped {
            graceful_load_segment_flipped(&spec.left.vox_spec.0, true)
        } else {
            graceful_load_segment(&spec.right.vox_spec.0)
        };
        let offset = if flipped {
            spec.left.vox_spec.1
        } else {
            spec.right.vox_spec.1
        };

        (foot_segment, Vec3::from(offset))
    }

    fn mesh_left_foot(&self, foot: Option<&str>) -> BoneMeshes { self.mesh_foot(foot, true) }

    fn mesh_right_foot(&self, foot: Option<&str>) -> BoneMeshes { self.mesh_foot(foot, false) }
}

impl BipedSmallWeaponSpec {
    fn mesh_main(&self, tool: &ToolKey, flipped: bool) -> BoneMeshes {
        let spec = match self.0.get(tool) {
            Some(spec) => spec,
            None => {
                error!(?tool, "No tool/weapon specification exists");
                return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
            },
        };

        let tool_kind_segment = if flipped {
            graceful_load_segment_flipped(&spec.vox_spec.0, true)
        } else {
            graceful_load_segment(&spec.vox_spec.0)
        };

        let offset = Vec3::new(
            if flipped {
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
////

#[derive(Deserialize)]
struct DragonCentralSpec(HashMap<(DSpecies, DBodyType), SidedDCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedDCentralVoxSpec {
    upper: DragonCentralSubSpec,
    lower: DragonCentralSubSpec,
    jaw: DragonCentralSubSpec,
    chest_front: DragonCentralSubSpec,
    chest_rear: DragonCentralSubSpec,
    tail_front: DragonCentralSubSpec,
    tail_rear: DragonCentralSubSpec,
}
#[derive(Deserialize)]
struct DragonCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
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
        central: DragonCentralSpec = "voxygen.voxel.dragon_central_manifest",
        lateral: DragonLateralSpec = "voxygen.voxel.dragon_lateral_manifest",
    },
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head_upper(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_head_lower(
                body.species,
                body.body_type,
            )),
            third_person.map(|_| {
                spec.central.read().0.mesh_jaw(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_chest_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_chest_rear(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail_rear(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_in_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_in_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_out_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_out_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_br(
                body.species,
                body.body_type,
            )),
            None,
        ]
    },
);

impl DragonCentralSpec {
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
        let central = graceful_load_segment(&spec.upper.central.0);

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
        let central = graceful_load_segment(&spec.lower.central.0);

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
        let central = graceful_load_segment(&spec.jaw.central.0);

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
        let central = graceful_load_segment(&spec.chest_front.central.0);

        (central, Vec3::from(spec.chest_front.offset))
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
        let central = graceful_load_segment(&spec.chest_rear.central.0);

        (central, Vec3::from(spec.chest_rear.offset))
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
        let central = graceful_load_segment(&spec.tail_front.central.0);

        (central, Vec3::from(spec.tail_front.offset))
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
        let central = graceful_load_segment(&spec.tail_rear.central.0);

        (central, Vec3::from(spec.tail_rear.offset))
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
#[derive(Deserialize)]
struct BirdLargeCentralSpec(HashMap<(BLASpecies, BLABodyType), SidedBLACentralVoxSpec>);

#[derive(Deserialize)]
struct SidedBLACentralVoxSpec {
    head: BirdLargeCentralSubSpec,
    beak: BirdLargeCentralSubSpec,
    neck: BirdLargeCentralSubSpec,
    chest: BirdLargeCentralSubSpec,
    tail_front: BirdLargeCentralSubSpec,
    tail_rear: BirdLargeCentralSubSpec,
}
#[derive(Deserialize)]
struct BirdLargeCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}

#[derive(Deserialize)]
struct BirdLargeLateralSpec(HashMap<(BLASpecies, BLABodyType), SidedBLALateralVoxSpec>);

#[derive(Deserialize)]
struct SidedBLALateralVoxSpec {
    wing_in_l: BirdLargeLateralSubSpec,
    wing_in_r: BirdLargeLateralSubSpec,
    wing_mid_l: BirdLargeLateralSubSpec,
    wing_mid_r: BirdLargeLateralSubSpec,
    wing_out_l: BirdLargeLateralSubSpec,
    wing_out_r: BirdLargeLateralSubSpec,
    leg_l: BirdLargeLateralSubSpec,
    leg_r: BirdLargeLateralSubSpec,
    foot_l: BirdLargeLateralSubSpec,
    foot_r: BirdLargeLateralSubSpec,
}
#[derive(Deserialize)]
struct BirdLargeLateralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    lateral: VoxSimple,
}

make_vox_spec!(
    bird_large::Body,
    struct BirdLargeSpec {
        central: BirdLargeCentralSpec = "voxygen.voxel.bird_large_central_manifest",
        lateral: BirdLargeLateralSpec = "voxygen.voxel.bird_large_lateral_manifest",
    },
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.central.read().0.mesh_beak(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_neck(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_chest(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail_rear(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_in_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_in_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_mid_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_mid_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_out_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_wing_out_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_r(
                body.species,
                body.body_type,
            )),
        ]
    },
);

impl BirdLargeCentralSpec {
    fn mesh_head(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
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

    fn mesh_beak(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No beak specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.beak.central.0);

        (central, Vec3::from(spec.beak.offset))
    }

    fn mesh_neck(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No neck specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.neck.central.0);

        (central, Vec3::from(spec.neck.offset))
    }

    fn mesh_chest(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
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

    fn mesh_tail_front(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
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
        let central = graceful_load_segment(&spec.tail_front.central.0);

        (central, Vec3::from(spec.tail_front.offset))
    }

    fn mesh_tail_rear(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
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
        let central = graceful_load_segment(&spec.tail_rear.central.0);

        (central, Vec3::from(spec.tail_rear.offset))
    }
}
impl BirdLargeLateralSpec {
    fn mesh_wing_in_l(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing in in left specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.wing_in_l.lateral.0, true);

        (lateral, Vec3::from(spec.wing_in_l.offset))
    }

    fn mesh_wing_in_r(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing in right specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_in_r.lateral.0);

        (lateral, Vec3::from(spec.wing_in_r.offset))
    }

    fn mesh_wing_mid_l(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing mid specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.wing_mid_l.lateral.0, true);

        (lateral, Vec3::from(spec.wing_mid_l.offset))
    }

    fn mesh_wing_mid_r(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing mid specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_mid_r.lateral.0);

        (lateral, Vec3::from(spec.wing_mid_r.offset))
    }

    fn mesh_wing_out_l(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing out specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment_flipped(&spec.wing_out_l.lateral.0, true);

        (lateral, Vec3::from(spec.wing_out_l.offset))
    }

    fn mesh_wing_out_r(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
        let spec = match self.0.get(&(species, body_type)) {
            Some(spec) => spec,
            None => {
                error!(
                    "No wing out specification exists for the combination of {:?} and {:?}",
                    species, body_type
                );
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let lateral = graceful_load_segment(&spec.wing_out_r.lateral.0);

        (lateral, Vec3::from(spec.wing_out_r.offset))
    }

    fn mesh_leg_l(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
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
        let lateral = graceful_load_segment_flipped(&spec.leg_l.lateral.0, true);

        (lateral, Vec3::from(spec.leg_l.offset))
    }

    fn mesh_leg_r(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
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

    fn mesh_foot_l(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
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
        let lateral = graceful_load_segment_flipped(&spec.foot_l.lateral.0, true);

        (lateral, Vec3::from(spec.foot_l.offset))
    }

    fn mesh_foot_r(&self, species: BLASpecies, body_type: BLABodyType) -> BoneMeshes {
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
struct BipedLargeCentralSpec(HashMap<(BLSpecies, BLBodyType), SidedBLCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedBLCentralVoxSpec {
    head: BipedLargeCentralSubSpec,
    jaw: BipedLargeCentralSubSpec,
    torso_upper: BipedLargeCentralSubSpec,
    torso_lower: BipedLargeCentralSubSpec,
    tail: BipedLargeCentralSubSpec,
}
#[derive(Deserialize)]
struct BipedLargeCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
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
#[derive(Deserialize)]
struct BipedLargeMainSpec(HashMap<ToolKey, ArmorVoxSpec>);
#[derive(Deserialize)]
struct BipedLargeSecondSpec(HashMap<ToolKey, ArmorVoxSpec>);
make_vox_spec!(
    biped_large::Body,
    struct BipedLargeSpec {
        central: BipedLargeCentralSpec = "voxygen.voxel.biped_large_central_manifest",
        lateral: BipedLargeLateralSpec = "voxygen.voxel.biped_large_lateral_manifest",
        main: BipedLargeMainSpec = "voxygen.voxel.biped_weapon_manifest",
        second: BipedLargeSecondSpec = "voxygen.voxel.biped_weapon_manifest",
    },
    |FigureKey { body, item_key: _, extra }, spec| {
        const DEFAULT_LOADOUT: super::cache::CharacterCacheKey = super::cache::CharacterCacheKey {
            third_person: None,
            tool: None,
            lantern: None,
            glider: None,
            hand: None,
            foot: None,
            head: None,
        };

        // TODO: This is bad code, maybe this method should return Option<_>
        let loadout = extra.as_deref().unwrap_or(&DEFAULT_LOADOUT);
        let third_person = loadout.third_person.as_ref();

        //let third_person = loadout.third_person.as_ref();
        let tool = loadout.tool.as_ref();
        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.central.read().0.mesh_jaw(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.central.read().0.mesh_torso_upper(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_torso_lower(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail(
                body.species,
                body.body_type,
            )),
            tool.and_then(|tool| tool.active.as_ref()).map(|tool| {
                spec.main.read().0.mesh_main(
                    tool,
                    false,
                )
            }),
            tool.and_then(|tool| tool.active.as_ref()).map(|tool| {
                spec.second.read().0.mesh_second(
                    tool,
                    false,
                )
            }),
            Some(spec.lateral.read().0.mesh_shoulder_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_shoulder_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_hand_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_hand_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_r(
                body.species,
                body.body_type,
            )),
            Some(mesh_hold()),
        ]
    },
);

impl BipedLargeCentralSpec {
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
        let central = graceful_load_segment(&spec.head.central.0);

        (central, Vec3::from(spec.head.offset))
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
        let central = graceful_load_segment(&spec.jaw.central.0);

        (central, Vec3::from(spec.jaw.offset))
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
        let central = graceful_load_segment(&spec.torso_upper.central.0);

        (central, Vec3::from(spec.torso_upper.offset))
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
        let central = graceful_load_segment(&spec.torso_lower.central.0);

        (central, Vec3::from(spec.torso_lower.offset))
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
        let central = graceful_load_segment(&spec.tail.central.0);

        (central, Vec3::from(spec.tail.offset))
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
impl BipedLargeMainSpec {
    fn mesh_main(&self, tool: &ToolKey, flipped: bool) -> BoneMeshes {
        let spec = match self.0.get(tool) {
            Some(spec) => spec,
            None => {
                error!(?tool, "No tool/weapon specification exists");
                return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
            },
        };

        let tool_kind_segment = if flipped {
            graceful_load_segment_flipped(&spec.vox_spec.0, true)
        } else {
            graceful_load_segment(&spec.vox_spec.0)
        };

        let offset = Vec3::new(
            if flipped {
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
impl BipedLargeSecondSpec {
    fn mesh_second(&self, tool: &ToolKey, flipped: bool) -> BoneMeshes {
        let spec = match self.0.get(tool) {
            Some(spec) => spec,
            None => {
                error!(?tool, "No tool/weapon specification exists");
                return load_mesh("not_found", Vec3::new(-1.5, -1.5, -7.0));
            },
        };

        let tool_kind_segment = if flipped {
            graceful_load_segment_flipped(&spec.vox_spec.0, true)
        } else {
            graceful_load_segment(&spec.vox_spec.0)
        };

        let offset = Vec3::new(
            if flipped {
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
////
#[derive(Deserialize)]
struct GolemCentralSpec(HashMap<(GSpecies, GBodyType), SidedGCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedGCentralVoxSpec {
    head: GolemCentralSubSpec,
    jaw: GolemCentralSubSpec,
    torso_upper: GolemCentralSubSpec,
    torso_lower: GolemCentralSubSpec,
}
#[derive(Deserialize)]
struct GolemCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
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
        central: GolemCentralSpec = "voxygen.voxel.golem_central_manifest",
        lateral: GolemLateralSpec = "voxygen.voxel.golem_lateral_manifest",
    },
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.central.read().0.mesh_jaw(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_torso_upper(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_torso_lower(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_shoulder_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_shoulder_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_hand_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_hand_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_leg_r(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_l(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_r(
                body.species,
                body.body_type,
            )),
            None,
            None,
            None,
            None,
        ]
    },
);

impl GolemCentralSpec {
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
        let central = graceful_load_segment(&spec.head.central.0);

        (central, Vec3::from(spec.head.offset))
    }

    fn mesh_jaw(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
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
        let central = graceful_load_segment(&spec.torso_upper.central.0);

        (central, Vec3::from(spec.torso_upper.offset))
    }

    pub fn mesh_torso_lower(&self, species: GSpecies, body_type: GBodyType) -> BoneMeshes {
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
        let central = graceful_load_segment(&spec.torso_lower.central.0);

        (central, Vec3::from(spec.torso_lower.offset))
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
    lateral: VoxMirror,
}

make_vox_spec!(
    quadruped_low::Body,
    struct QuadrupedLowSpec {
        central: QuadrupedLowCentralSpec = "voxygen.voxel.quadruped_low_central_manifest",
        lateral: QuadrupedLowLateralSpec = "voxygen.voxel.quadruped_low_lateral_manifest",
    },
    |FigureKey { body, extra, .. }, spec| {
        let third_person = extra.as_ref().and_then(|loadout| loadout.third_person.as_ref());

        [
            third_person.map(|_| {
                spec.central.read().0.mesh_head_upper(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.central.read().0.mesh_head_lower(
                    body.species,
                    body.body_type,
                )
            }),
            third_person.map(|_| {
                spec.central.read().0.mesh_jaw(
                    body.species,
                    body.body_type,
                )
            }),
            Some(spec.central.read().0.mesh_chest(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail_front(
                body.species,
                body.body_type,
            )),
            Some(spec.central.read().0.mesh_tail_rear(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_fl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_fr(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_bl(
                body.species,
                body.body_type,
            )),
            Some(spec.lateral.read().0.mesh_foot_br(
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
        let latspec = &spec.front_left.lateral;
        let lateral = graceful_load_segment_flipped(&latspec.0, !latspec.1);

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
        let latspec = &spec.front_right.lateral;
        let lateral = graceful_load_segment_flipped(&latspec.0, latspec.1);

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
        let latspec = &spec.back_left.lateral;
        let lateral = graceful_load_segment_flipped(&latspec.0, !latspec.1);

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
        let latspec = &spec.back_right.lateral;
        let lateral = graceful_load_segment_flipped(&latspec.0, latspec.1);

        (lateral, Vec3::from(spec.back_right.offset))
    }
}

////

#[derive(Deserialize)]
struct ObjectCentralSpec(HashMap<object::Body, SidedObjectCentralVoxSpec>);

#[derive(Deserialize)]
struct SidedObjectCentralVoxSpec {
    bone0: ObjectCentralSubSpec,
    bone1: ObjectCentralSubSpec,
}
#[derive(Deserialize)]
struct ObjectCentralSubSpec {
    offset: [f32; 3], // Should be relative to initial origin
    central: VoxSimple,
}

make_vox_spec!(
    object::Body,
    struct ObjectSpec {
        central: ObjectCentralSpec = "voxygen.voxel.object_manifest",
    },
    |FigureKey { body, .. }, spec| {
        [
            Some(spec.central.read().0.mesh_bone0(
                body,
            )),
            Some(spec.central.read().0.mesh_bone1(
                body,
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
            None,
            None,
            None,
        ]
    },
);

impl ObjectCentralSpec {
    fn mesh_bone0(&self, obj: &object::Body) -> BoneMeshes {
        let spec = match self.0.get(obj) {
            Some(spec) => spec,
            None => {
                error!("No specification exists for {:?}", obj);
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.bone0.central.0);

        (central, Vec3::from(spec.bone0.offset))
    }

    fn mesh_bone1(&self, obj: &object::Body) -> BoneMeshes {
        let spec = match self.0.get(obj) {
            Some(spec) => spec,
            None => {
                error!("No specification exists for {:?}", obj);
                return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
            },
        };
        let central = graceful_load_segment(&spec.bone1.central.0);

        (central, Vec3::from(spec.bone1.offset))
    }
}

#[derive(Deserialize)]
struct ItemDropCentralSpec(HashMap<ItemKey, String>);

make_vox_spec!(
    item_drop::Body,
    struct ItemDropSpec {
        central: ItemDropCentralSpec = "voxygen.voxel.item_drop_manifest",
    },
    | FigureKey { body, item_key, .. }, spec| {
        [
            Some(spec.central.read().0.mesh_bone0(body, item_key.as_deref().unwrap_or(&ItemKey::Empty))),
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

impl ItemDropCentralSpec {
    fn mesh_bone0(&self, item_drop: &item_drop::Body, item_key: &ItemKey) -> BoneMeshes {
        let coin_pouch = "voxel.object.pouch".to_string();

        if let Some(spec) = match item_drop {
            item_drop::Body::CoinPouch => Some(&coin_pouch),
            _ => self.0.get(item_key),
        } {
            let full_spec: String = ["voxygen.", spec.as_str()].concat();
            let segment = match item_drop {
                item_drop::Body::Armor(_) => {
                    MatSegment::from(&graceful_load_vox_fullspec(&full_spec).read().0)
                        .map(|mat_cell| match mat_cell {
                            MatCell::None => None,
                            MatCell::Mat(_) => Some(MatCell::None),
                            MatCell::Normal(data) => data.is_hollow().then_some(MatCell::None),
                        })
                        .to_segment(|_| Default::default())
                },
                _ => graceful_load_segment_fullspec(&full_spec),
            };
            let offset = segment_center(&segment).unwrap_or_default();
            (segment, match item_drop {
                // TODO: apply non-random rotations to items here
                item_drop::Body::Tool(_) => Vec3::new(offset.x - 2.0, offset.y, offset.z),
                item_drop::Body::Armor(kind) => match kind {
                    item_drop::ItemDropArmorKind::Neck
                    | item_drop::ItemDropArmorKind::Back
                    | item_drop::ItemDropArmorKind::Tabard => {
                        Vec3::new(offset.x, offset.y - 2.0, offset.z)
                    },
                    _ => offset * Vec3::new(1.0, 1.0, 0.0),
                },
                _ => offset * Vec3::new(1.0, 1.0, 0.0),
            })
        } else {
            error!(
                "No specification exists for {:?}, {:?}",
                item_drop, item_key
            );
            load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5))
        }
    }
}

fn segment_center(segment: &Segment) -> Option<Vec3<f32>> {
    let (mut x_min, mut x_max, mut y_min, mut y_max, mut z_min, mut z_max) =
        (i32::MAX, 0, i32::MAX, 0, i32::MAX, 0);
    for pos in segment.full_pos_iter() {
        if let Ok(Cell::Filled(data)) = segment.get(pos) {
            if !data.is_hollow() {
                if pos.x < x_min {
                    x_min = pos.x;
                } else if pos.x > x_max {
                    x_max = pos.x;
                }
                if pos.y < y_min {
                    y_min = pos.y;
                } else if pos.y > y_max {
                    y_max = pos.y;
                }
                if pos.z < z_min {
                    z_min = pos.z;
                } else if pos.z > z_max {
                    z_max = pos.z;
                }
            }
        }
    }

    if (x_min, x_max, y_min, y_max, z_min, z_max) == (i32::MAX, 0, i32::MAX, 0, i32::MAX, 0) {
        None
    } else {
        Some(Vec3::new(x_min + x_max, y_min + y_max, z_min + z_max).map(|n| n as f32 / -2.0))
    }
}

fn mesh_ship_bone<K: fmt::Debug + Eq + Hash, V, F: Fn(&V) -> &ShipCentralSubSpec>(
    map: &HashMap<K, V>,
    obj: &K,
    f: F,
) -> BoneMeshes {
    let spec = match map.get(obj) {
        Some(spec) => spec,
        None => {
            error!("No specification exists for {:?}", obj);
            return load_mesh("not_found", Vec3::new(-5.0, -5.0, -2.5));
        },
    };
    let bone = f(spec);
    let central = graceful_load_segment_fullspec(&["common.voxel.", &bone.central.0].concat());

    (central, Vec3::from(bone.offset))
}

impl BodySpec for ship::Body {
    type Extra = ();
    type Manifests = AssetHandle<Self::Spec>;
    type Spec = ShipSpec;

    fn load_spec() -> Result<Self::Manifests, assets::Error> { Self::Spec::load("") }

    fn reload_watcher(manifests: &Self::Manifests) -> ReloadWatcher { manifests.reload_watcher() }

    fn bone_meshes(
        FigureKey { body, .. }: &FigureKey<Self>,
        manifests: &Self::Manifests,
        _: Self::Extra,
    ) -> [Option<BoneMeshes>; anim::MAX_BONE_COUNT] {
        let spec = &*manifests.read();
        let map = &(spec.central.read().0).0;
        [
            Some(mesh_ship_bone(map, body, |spec| &spec.bone0)),
            Some(mesh_ship_bone(map, body, |spec| &spec.bone1)),
            Some(mesh_ship_bone(map, body, |spec| &spec.bone2)),
            Some(mesh_ship_bone(map, body, |spec| &spec.bone3)),
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
    }
}
