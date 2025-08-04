use std::{fs, path::Path, sync::Arc};

use anim::{
    Animation, FigureBoneData, Skeleton, arthropod::ArthropodSkeleton,
    biped_large::BipedLargeSkeleton, biped_small::BipedSmallSkeleton,
    bird_large::BirdLargeSkeleton, bird_medium::BirdMediumSkeleton, character::CharacterSkeleton,
    crustacean::CrustaceanSkeleton, dragon::DragonSkeleton, fish_medium::FishMediumSkeleton,
    fish_small::FishSmallSkeleton, golem::GolemSkeleton, quadruped_low::QuadrupedLowSkeleton,
    quadruped_medium::QuadrupedMediumSkeleton, quadruped_small::QuadrupedSmallSkeleton,
    theropod::TheropodSkeleton,
};
use clap::Parser;
use common::{
    comp::{
        self, Body, CharacterState, Inventory,
        body::parts::HeadState,
        item::{ItemKind, armor::ArmorKind},
        slot::{ArmorSlot, EquipSlot},
    },
    figure::Segment,
    generation::{EntityInfo, try_all_entity_configs},
};
use image::RgbaImage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use vek::{Mat4, Quaternion, Vec2, Vec3};

use common::assets::{AssetExt, AssetHandle};
use veloren_voxygen::{
    hud::item_imgs::{ImageSpec, ItemImagesSpec},
    scene::{
        CameraMode,
        figure::{
            cache::{CharacterCacheKey, FigureKey},
            load::{
                ArthropodSpec, BipedLargeSpec, BipedSmallSpec, BirdLargeSpec, BirdMediumSpec,
                BodySpec, CrustaceanSpec, DragonSpec, FishMediumSpec, FishSmallSpec, GolemSpec,
                HumSpec, ObjectSpec, QuadrupedLowSpec, QuadrupedMediumSpec, QuadrupedSmallSpec,
                TheropodSpec,
            },
        },
    },
    ui::{
        Graphic,
        graphic::renderer::{draw_vox, draw_voxes},
    },
};

#[derive(Parser)]
struct Cli {
    ///Optional width and height scaling
    #[clap(default_value_t = 20)]
    scale: u32,

    #[clap(long = "all-items")]
    all_items: bool,

    #[clap(long = "all-npcs")]
    all_npcs: bool,

    #[clap(long)]
    filter: Option<String>,

    #[clap(long)]
    seed: Option<u128>,
}

pub fn main() {
    let args = Cli::parse();

    if !args.all_items && !args.all_npcs {
        println!("Nothing to do, to see arguments use `--help`.")
    } else {
        let image_size = Vec2 {
            x: (10_u32 * args.scale) as u16,
            y: (10_u32 * args.scale) as u16,
        };
        let mut img_count = 0;
        if args.all_items {
            let manifest = ItemImagesSpec::load_expect("voxygen.item_image_manifest");
            for (_, spec) in manifest.read().0.iter() {
                let specifier = match spec {
                    ImageSpec::Vox(specifier, _, _) => specifier,
                    ImageSpec::VoxTrans(specifier, _, _, _, _, _) => specifier,
                    _ => continue,
                };
                if args.filter.as_ref().is_some_and(|f| !specifier.contains(f)) {
                    continue;
                }
                let graphic = spec.create_graphic();
                let img = match graphic {
                    Graphic::Voxel(segment, trans, sample_strat) => {
                        draw_vox(&segment, image_size, trans, sample_strat)
                    },
                    _ => continue,
                };
                save_img(specifier, img);
                img_count += 1;
            }
        }
        if args.all_npcs {
            let manifests = Manifests::load().expect("This should work");
            for specifier in try_all_entity_configs().expect("Couldn't load npcs").iter() {
                if args.filter.as_ref().is_some_and(|f| !specifier.contains(f)) {
                    continue;
                }
                let mut rng = ChaChaRng::from_seed(
                    args.seed
                        .map(|s| {
                            let b = s.to_le_bytes();
                            std::array::from_fn(|i| b[i % b.len()])
                        })
                        .unwrap_or(rand::thread_rng().gen()),
                );
                // TODO: Could have args te specify calendar too.
                let info =
                    EntityInfo::at(Vec3::zero()).with_asset_expect(specifier, &mut rng, None);
                let bones = load_npc_bones(info, &manifests, Mat4::identity());
                if bones.is_empty() {
                    continue;
                };
                let bones: Vec<_> = bones.iter().map(|(t, s)| (*t, s)).collect();
                let img = draw_voxes(
                    &bones,
                    image_size,
                    veloren_voxygen::ui::Transform {
                        ori: Quaternion::rotation_x(-90.0 * std::f32::consts::PI / 180.0)
                            .rotated_y(180.0 * std::f32::consts::PI / 180.0)
                            .rotated_z(0.0 * std::f32::consts::PI / 180.0),
                        offset: Vec3::new(0.0, 0.0, 0.0),
                        zoom: 0.9,
                        orth: true,
                        stretch: false,
                    },
                    veloren_voxygen::ui::SampleStrat::None,
                    Vec3::new(0.0, 1.0, 1.0),
                );

                save_img(specifier, img);
                img_count += 1;
            }
        }

        println!("Exported {img_count} images!");
    }
}

fn save_img(specifier: &str, img: RgbaImage) {
    let path = specifier_to_path(specifier);
    let folder_path = path.rsplit_once('/').expect("Invalid path").0;
    let full_path = Path::new(&path);
    if let Err(e) = fs::create_dir_all(Path::new(folder_path)) {
        println!("{}", e);
        return;
    }

    img.save(full_path)
        .unwrap_or_else(|_| panic!("Can't save file {}", full_path.to_str().expect("")));
}

fn specifier_to_path(specifier: &str) -> String {
    let inner = specifier.replace('.', "/");

    format!("img-export/{inner}.png")
}

struct Manifests {
    humanoid: AssetHandle<HumSpec>,
    quadruped_small: AssetHandle<QuadrupedSmallSpec>,
    quadruped_medium: AssetHandle<QuadrupedMediumSpec>,
    bird_medium: AssetHandle<BirdMediumSpec>,
    fish_medium: AssetHandle<FishMediumSpec>,
    dragon: AssetHandle<DragonSpec>,
    bird_large: AssetHandle<BirdLargeSpec>,
    fish_small: AssetHandle<FishSmallSpec>,
    biped_large: AssetHandle<BipedLargeSpec>,
    biped_small: AssetHandle<BipedSmallSpec>,
    object: AssetHandle<ObjectSpec>,
    golem: AssetHandle<GolemSpec>,
    theropod: AssetHandle<TheropodSpec>,
    quadruped_low: AssetHandle<QuadrupedLowSpec>,
    arthropod: AssetHandle<ArthropodSpec>,
    crustacean: AssetHandle<CrustaceanSpec>,
}

impl Manifests {
    fn load() -> Result<Self, common::assets::BoxedError> {
        Ok(Self {
            humanoid: AssetExt::load("")?,
            quadruped_small: AssetExt::load("")?,
            quadruped_medium: AssetExt::load("")?,
            bird_medium: AssetExt::load("")?,
            fish_medium: AssetExt::load("")?,
            dragon: AssetExt::load("")?,
            bird_large: AssetExt::load("")?,
            fish_small: AssetExt::load("")?,
            biped_large: AssetExt::load("")?,
            biped_small: AssetExt::load("")?,
            object: AssetExt::load("")?,
            golem: AssetExt::load("")?,
            theropod: AssetExt::load("")?,
            quadruped_low: AssetExt::load("")?,
            arthropod: AssetExt::load("")?,
            crustacean: AssetExt::load("")?,
        })
    }
}

fn load_npc_bones(
    info: EntityInfo,
    manifests: &Manifests,
    mut base_mat: Mat4<f32>,
) -> Vec<(Mat4<f32>, Segment)> {
    base_mat *= Mat4::scaling_3d(info.scale);
    let loadout = info.loadout.build();

    let inventory = Inventory::with_loadout(loadout, info.body);

    let state = CharacterState::Idle(Default::default());

    let extra = Some(Arc::new(CharacterCacheKey::from(
        Some(&state),
        CameraMode::ThirdPerson,
        &inventory,
    )));

    let bone_segments = match info.body {
        Body::Humanoid(body) => comp::humanoid::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.humanoid,
            (),
        ),
        Body::QuadrupedSmall(body) => comp::quadruped_small::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.quadruped_small,
            (),
        ),
        Body::QuadrupedMedium(body) => comp::quadruped_medium::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.quadruped_medium,
            (),
        ),
        Body::BirdMedium(body) => comp::bird_medium::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.bird_medium,
            (),
        ),
        Body::FishMedium(body) => comp::fish_medium::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.fish_medium,
            (),
        ),
        Body::Dragon(body) => comp::dragon::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.dragon,
            (),
        ),
        Body::BirdLarge(body) => comp::bird_large::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.bird_large,
            (),
        ),
        Body::FishSmall(body) => comp::fish_small::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.fish_small,
            (),
        ),
        Body::BipedLarge(body) => comp::biped_large::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.biped_large,
            (),
        ),
        Body::BipedSmall(body) => comp::biped_small::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.biped_small,
            (),
        ),
        Body::Object(body) => comp::object::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.object,
            (),
        ),
        Body::Golem(body) => comp::golem::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.golem,
            (),
        ),
        Body::Theropod(body) => comp::theropod::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.theropod,
            (),
        ),
        Body::QuadrupedLow(body) => comp::quadruped_low::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.quadruped_low,
            (),
        ),
        Body::Arthropod(body) => comp::arthropod::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.arthropod,
            (),
        ),
        Body::Crustacean(body) => comp::crustacean::Body::bone_meshes(
            &FigureKey {
                body,
                item_key: None,
                extra,
            },
            &manifests.crustacean,
            (),
        ),
        Body::Item(_) => panic!("Item bodies aren't supported"),
        Body::Ship(_) => panic!("Ship bodies aren't supported"),
        Body::Plugin(_) => panic!("Plugin bodies aren't supported"),
    };

    let tool_info = |equip_slot| {
        inventory
            .equipped(equip_slot)
            .map(|i| {
                if let ItemKind::Tool(tool) = &*i.kind() {
                    (Some(tool.kind), Some(tool.hands), i.ability_spec())
                } else {
                    (None, None, None)
                }
            })
            .unwrap_or((None, None, None))
    };

    let (active_tool_kind, active_tool_hand, active_tool_spec) =
        tool_info(EquipSlot::ActiveMainhand);
    let _active_tool_spec = active_tool_spec.as_deref();
    let (second_tool_kind, second_tool_hand, second_tool_spec) =
        tool_info(EquipSlot::ActiveOffhand);
    let _second_tool_spec = second_tool_spec.as_deref();
    let hands = (active_tool_hand, second_tool_hand);
    let time = 0.0;

    let mut buf = [FigureBoneData::default(); 16];
    let mount_mat = match info.body {
        Body::Humanoid(body) => {
            let back_carry_offset = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Back))
                .and_then(|i| {
                    if let ItemKind::Armor(armor) = i.kind().as_ref() {
                        match &armor.kind {
                            ArmorKind::Backpack => Some(4.0),
                            ArmorKind::Back => Some(1.5),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);

            let skel = anim::character::StandAnimation::update_skeleton(
                &CharacterSkeleton::new(false, back_carry_offset),
                (
                    active_tool_kind,
                    second_tool_kind,
                    hands,
                    anim::vek::Vec3::<f32>::unit_y(),
                    anim::vek::Vec3::<f32>::unit_y(),
                    time,
                    Vec3::zero(),
                ),
                0.0,
                &mut 1.0,
                &anim::character::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::character::mount_mat(&computed_skel, &skel).0
        },
        Body::QuadrupedSmall(body) => {
            let skel = anim::quadruped_small::IdleAnimation::update_skeleton(
                &QuadrupedSmallSkeleton::default(),
                time,
                0.0,
                &mut 1.0,
                &anim::quadruped_small::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::quadruped_small::mount_mat(&computed_skel, &skel).0
        },
        Body::QuadrupedMedium(body) => {
            let skel = anim::quadruped_medium::IdleAnimation::update_skeleton(
                &QuadrupedMediumSkeleton::default(),
                time,
                0.0,
                &mut 1.0,
                &anim::quadruped_medium::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::quadruped_medium::mount_mat(&body, &computed_skel, &skel).0
        },
        Body::BirdMedium(body) => {
            let skel = anim::bird_medium::IdleAnimation::update_skeleton(
                &BirdMediumSkeleton::default(),
                time,
                0.0,
                &mut 1.0,
                &anim::bird_medium::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::bird_medium::mount_mat(&computed_skel, &skel).0
        },
        Body::FishMedium(body) => {
            let skel = anim::fish_medium::IdleAnimation::update_skeleton(
                &FishMediumSkeleton::default(),
                (
                    Vec3::zero(),
                    anim::vek::Vec3::<f32>::unit_y(),
                    anim::vek::Vec3::<f32>::unit_y(),
                    time,
                    Vec3::zero(),
                ),
                0.0,
                &mut 1.0,
                &anim::fish_medium::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::fish_medium::mount_mat(&computed_skel, &skel).0
        },
        Body::Dragon(body) => {
            let skel = anim::dragon::IdleAnimation::update_skeleton(
                &DragonSkeleton::default(),
                time,
                0.0,
                &mut 1.0,
                &anim::dragon::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::dragon::mount_mat(&computed_skel, &skel).0
        },
        Body::BirdLarge(body) => {
            let skel = anim::bird_large::IdleAnimation::update_skeleton(
                &BirdLargeSkeleton::default(),
                time,
                0.0,
                &mut 1.0,
                &anim::bird_large::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::bird_large::mount_mat(&body, &computed_skel, &skel).0
        },
        Body::FishSmall(body) => {
            let skel = anim::fish_small::IdleAnimation::update_skeleton(
                &FishSmallSkeleton::default(),
                (
                    Vec3::zero(),
                    anim::vek::Vec3::<f32>::unit_y(),
                    anim::vek::Vec3::<f32>::unit_y(),
                    time,
                    Vec3::zero(),
                ),
                0.0,
                &mut 1.0,
                &anim::fish_small::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::fish_small::mount_mat(&computed_skel, &skel).0
        },
        Body::BipedLarge(body) => {
            let skel = anim::biped_large::IdleAnimation::update_skeleton(
                &BipedLargeSkeleton::default(),
                (active_tool_kind, second_tool_kind, time),
                0.0,
                &mut 1.0,
                &anim::biped_large::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::biped_large::mount_mat(&body, &computed_skel, &skel).0
        },
        Body::BipedSmall(body) => {
            let skel = anim::biped_small::IdleAnimation::update_skeleton(
                &BipedSmallSkeleton::default(),
                (
                    Vec3::zero(),
                    anim::vek::Vec3::<f32>::unit_y(),
                    anim::vek::Vec3::<f32>::unit_y(),
                    time,
                    Vec3::zero(),
                ),
                0.0,
                &mut 1.0,
                &anim::biped_small::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::biped_small::mount_mat(&body, &computed_skel, &skel).0
        },
        Body::Object(_) => Mat4::identity(), // Objects do not support mounting
        Body::Golem(body) => {
            let skel = anim::golem::IdleAnimation::update_skeleton(
                &GolemSkeleton::default(),
                time,
                0.0,
                &mut 1.0,
                &anim::golem::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::golem::mount_mat(&computed_skel, &skel).0
        },
        Body::Theropod(body) => {
            let skel = anim::theropod::IdleAnimation::update_skeleton(
                &TheropodSkeleton::default(),
                time,
                0.0,
                &mut 1.0,
                &anim::theropod::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::theropod::mount_mat(&body, &computed_skel, &skel).0
        },
        Body::QuadrupedLow(body) => {
            let skel = anim::quadruped_low::IdleAnimation::update_skeleton(
                &QuadrupedLowSkeleton::default(),
                (time, [
                    HeadState::Attached,
                    HeadState::Attached,
                    HeadState::Attached,
                ]),
                0.0,
                &mut 1.0,
                &anim::quadruped_low::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::quadruped_low::mount_mat(&body, &computed_skel, &skel).0
        },
        Body::Arthropod(body) => {
            let skel = anim::arthropod::IdleAnimation::update_skeleton(
                &ArthropodSkeleton::default(),
                time,
                0.0,
                &mut 1.0,
                &anim::arthropod::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::arthropod::mount_mat(&body, &computed_skel, &skel).0
        },
        Body::Crustacean(body) => {
            let skel = anim::crustacean::IdleAnimation::update_skeleton(
                &CrustaceanSkeleton::default(),
                time,
                0.0,
                &mut 1.0,
                &anim::crustacean::SkeletonAttr::from(&body),
            );
            let computed_skel = skel.compute_matrices(base_mat, &mut buf, body);

            anim::crustacean::mount_mat(&computed_skel, &skel).0
        },
        Body::Item(_) => panic!("Item bodies aren't supported"),
        Body::Ship(_) => panic!("Ship bodies aren't supported"),
        Body::Plugin(_) => panic!("Plugin bodies aren't supported"),
    };

    let mut bones: Vec<_> = bone_segments
        .into_iter()
        .zip(buf)
        .filter_map(|(segment, bone)| {
            let (segment, offset) = segment?;

            Some((
                Mat4::from_col_arrays(bone.0) * Mat4::translation_3d(offset),
                segment,
            ))
        })
        .collect();

    if let Some(rider) = info.rider {
        bones.extend(load_npc_bones(*rider, manifests, mount_mat));
    }

    bones
}
