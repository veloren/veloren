use super::{load::*, FigureModelEntry};
use crate::{
    mesh::{greedy::GreedyMesh, Meshable},
    render::{BoneMeshes, FigureModel, FigurePipeline, Mesh, Renderer, TerrainPipeline},
    scene::camera::CameraMode,
};
use anim::Skeleton;
use common::{
    assets::watch::ReloadIndicator,
    comp::{
        item::{
            armor::{Armor, ArmorKind},
            tool::ToolKind,
            ItemKind,
        },
        Body, CharacterState, Loadout,
    },
    figure::Segment,
    vol::BaseVol,
};
use core::convert::TryInto;
use hashbrown::{hash_map::Entry, HashMap};
use vek::*;

pub type FigureModelEntryLod = FigureModelEntry<3>;

#[derive(Eq, Hash, PartialEq)]
struct FigureKey {
    /// Body pointed to by this key.
    body: Body,
    /// Extra state.
    extra: Option<Box<CharacterCacheKey>>,
}

/// Character data that should be visible when tools are visible (i.e. in third
/// person or when the character is in a tool-using state).
#[derive(Eq, Hash, PartialEq)]
pub struct CharacterToolKey {
    active: Option<ToolKind>,
    second: Option<ToolKind>,
}

/// Character data that exists in third person only.
#[derive(Eq, Hash, PartialEq)]
struct CharacterThirdPersonKey {
    shoulder: Option<String>,
    chest: Option<String>,
    belt: Option<String>,
    back: Option<String>,
    pants: Option<String>,
}

#[derive(Eq, Hash, PartialEq)]
/// NOTE: To avoid spamming the character cache with player models, we try to
/// store only the minimum information required to correctly update the model.
///
/// TODO: Memoize, etc.
struct CharacterCacheKey {
    /// Character state that is only visible in third person.
    third_person: Option<CharacterThirdPersonKey>,
    /// Tool state should be present when a character is either in third person,
    /// or is in first person and the character state is tool-using.
    ///
    /// NOTE: This representation could be tightened in various ways to
    /// eliminate incorrect states, e.g. setting active_tool to None when no
    /// tools are equipped, but currently we are more focused on the big
    /// performance impact of recreating the whole model whenever the character
    /// state changes, so for now we don't bother with this.
    tool: Option<CharacterToolKey>,
    lantern: Option<String>,
    hand: Option<String>,
    foot: Option<String>,
}

impl CharacterCacheKey {
    fn from(cs: Option<&CharacterState>, camera_mode: CameraMode, loadout: &Loadout) -> Self {
        let is_first_person = match camera_mode {
            CameraMode::FirstPerson => true,
            CameraMode::ThirdPerson | CameraMode::Freefly => false,
        };

        // Third person tools are only modeled when the camera is either not first
        // person, or the camera is first person and we are in a tool-using
        // state.
        let are_tools_visible = !is_first_person
            || cs
            .map(|cs| cs.is_attack() || cs.is_block() || cs.is_wield())
            // If there's no provided character state but we're still somehow in first person,
            // We currently assume there's no need to visually model tools.
            //
            // TODO: Figure out what to do here, and/or refactor how this works.
            .unwrap_or(false);

        Self {
            // Third person armor is only modeled when the camera mode is not first person.
            third_person: if is_first_person {
                None
            } else {
                Some(CharacterThirdPersonKey {
                    shoulder: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Shoulder(armor),
                        ..
                    })) = loadout.shoulder.as_ref().map(|i| &i.kind)
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                    chest: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Chest(armor),
                        ..
                    })) = loadout.chest.as_ref().map(|i| &i.kind)
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                    belt: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Belt(armor),
                        ..
                    })) = loadout.belt.as_ref().map(|i| &i.kind)
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                    back: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Back(armor),
                        ..
                    })) = loadout.back.as_ref().map(|i| &i.kind)
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                    pants: if let Some(ItemKind::Armor(Armor {
                        kind: ArmorKind::Pants(armor),
                        ..
                    })) = loadout.pants.as_ref().map(|i| &i.kind)
                    {
                        Some(armor.clone())
                    } else {
                        None
                    },
                })
            },
            tool: if are_tools_visible {
                Some(CharacterToolKey {
                    active: if let Some(ItemKind::Tool(tool)) =
                        loadout.active_item.as_ref().map(|i| &i.item.kind)
                    {
                        Some(tool.kind.clone())
                    } else {
                        None
                    },
                    second: if let Some(ItemKind::Tool(tool)) =
                        loadout.second_item.as_ref().map(|i| &i.item.kind)
                    {
                        Some(tool.kind.clone())
                    } else {
                        None
                    },
                })
            } else {
                None
            },
            lantern: if let Some(ItemKind::Lantern(lantern)) =
                loadout.lantern.as_ref().map(|i| &i.kind)
            {
                Some(lantern.kind.clone())
            } else {
                None
            },
            hand: if let Some(ItemKind::Armor(Armor {
                kind: ArmorKind::Hand(armor),
                ..
            })) = loadout.hand.as_ref().map(|i| &i.kind)
            {
                Some(armor.clone())
            } else {
                None
            },
            foot: if let Some(ItemKind::Armor(Armor {
                kind: ArmorKind::Foot(armor),
                ..
            })) = loadout.foot.as_ref().map(|i| &i.kind)
            {
                Some(armor.clone())
            } else {
                None
            },
        }
    }
}

#[allow(clippy::type_complexity)] // TODO: Pending review in #587
pub struct FigureModelCache<Skel = anim::character::CharacterSkeleton>
where
    Skel: Skeleton,
{
    models: HashMap<FigureKey, ((FigureModelEntryLod, Skel::Attr), u64)>,
    manifest_indicator: ReloadIndicator,
}

impl<Skel: Skeleton> FigureModelCache<Skel> {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            manifest_indicator: ReloadIndicator::new(),
        }
    }

    /// NOTE: We deliberately call this function with only the key into the
    /// cache, to enforce that the cached state only depends on the key.  We
    /// may end up using a mechanism different from this cache eventually,
    /// in which case this strategy might change.
    fn bone_meshes(
        FigureKey { body, extra }: &FigureKey,
        manifest_indicator: &mut ReloadIndicator,
        mut generate_mesh: impl FnMut(Segment, Vec3<f32>) -> BoneMeshes,
    ) -> [Option<BoneMeshes>; 16] {
        match body {
            Body::Humanoid(body) => {
                let humanoid_color_spec = HumColorSpec::load_watched(manifest_indicator);
                let humanoid_head_spec = HumHeadSpec::load_watched(manifest_indicator);
                let humanoid_armor_shoulder_spec =
                    HumArmorShoulderSpec::load_watched(manifest_indicator);
                let humanoid_armor_chest_spec = HumArmorChestSpec::load_watched(manifest_indicator);
                let humanoid_armor_hand_spec = HumArmorHandSpec::load_watched(manifest_indicator);
                let humanoid_armor_belt_spec = HumArmorBeltSpec::load_watched(manifest_indicator);
                let humanoid_armor_back_spec = HumArmorBackSpec::load_watched(manifest_indicator);
                let humanoid_armor_lantern_spec =
                    HumArmorLanternSpec::load_watched(manifest_indicator);
                let humanoid_armor_pants_spec = HumArmorPantsSpec::load_watched(manifest_indicator);
                let humanoid_armor_foot_spec = HumArmorFootSpec::load_watched(manifest_indicator);
                let humanoid_main_weapon_spec = HumMainWeaponSpec::load_watched(manifest_indicator);

                const DEFAULT_LOADOUT: CharacterCacheKey = CharacterCacheKey {
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
                        humanoid_head_spec.mesh_head(
                            body,
                            &humanoid_color_spec,
                            |segment, offset| generate_mesh(segment, offset),
                        )
                    }),
                    third_person.map(|loadout| {
                        humanoid_armor_chest_spec.mesh_chest(
                            body,
                            &humanoid_color_spec,
                            loadout.chest.as_deref(),
                            |segment, offset| generate_mesh(segment, offset),
                        )
                    }),
                    third_person.map(|loadout| {
                        humanoid_armor_belt_spec.mesh_belt(
                            body,
                            &humanoid_color_spec,
                            loadout.belt.as_deref(),
                            |segment, offset| generate_mesh(segment, offset),
                        )
                    }),
                    third_person.map(|loadout| {
                        humanoid_armor_back_spec.mesh_back(
                            body,
                            &humanoid_color_spec,
                            loadout.back.as_deref(),
                            |segment, offset| generate_mesh(segment, offset),
                        )
                    }),
                    third_person.map(|loadout| {
                        humanoid_armor_pants_spec.mesh_pants(
                            body,
                            &humanoid_color_spec,
                            loadout.pants.as_deref(),
                            |segment, offset| generate_mesh(segment, offset),
                        )
                    }),
                    Some(humanoid_armor_hand_spec.mesh_left_hand(
                        body,
                        &humanoid_color_spec,
                        hand,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(humanoid_armor_hand_spec.mesh_right_hand(
                        body,
                        &humanoid_color_spec,
                        hand,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(humanoid_armor_foot_spec.mesh_left_foot(
                        body,
                        &humanoid_color_spec,
                        foot,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(humanoid_armor_foot_spec.mesh_right_foot(
                        body,
                        &humanoid_color_spec,
                        foot,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    third_person.map(|loadout| {
                        humanoid_armor_shoulder_spec.mesh_left_shoulder(
                            body,
                            &humanoid_color_spec,
                            loadout.shoulder.as_deref(),
                            |segment, offset| generate_mesh(segment, offset),
                        )
                    }),
                    third_person.map(|loadout| {
                        humanoid_armor_shoulder_spec.mesh_right_shoulder(
                            body,
                            &humanoid_color_spec,
                            loadout.shoulder.as_deref(),
                            |segment, offset| generate_mesh(segment, offset),
                        )
                    }),
                    Some(mesh_glider(|segment, offset| {
                        generate_mesh(segment, offset)
                    })),
                    tool.map(|tool| {
                        humanoid_main_weapon_spec.mesh_main_weapon(
                            tool.active.as_ref(),
                            false,
                            |segment, offset| generate_mesh(segment, offset),
                        )
                    }),
                    tool.map(|tool| {
                        humanoid_main_weapon_spec.mesh_main_weapon(
                            tool.second.as_ref(),
                            true,
                            |segment, offset| generate_mesh(segment, offset),
                        )
                    }),
                    Some(humanoid_armor_lantern_spec.mesh_lantern(
                        body,
                        &humanoid_color_spec,
                        lantern,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(mesh_hold(|segment, offset| generate_mesh(segment, offset))),
                ]
            },
            Body::QuadrupedSmall(body) => {
                let quadruped_small_central_spec =
                    QuadrupedSmallCentralSpec::load_watched(manifest_indicator);
                let quadruped_small_lateral_spec =
                    QuadrupedSmallLateralSpec::load_watched(manifest_indicator);

                [
                    Some(quadruped_small_central_spec.mesh_head(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_small_central_spec.mesh_chest(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_small_lateral_spec.mesh_foot_fl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_small_lateral_spec.mesh_foot_fr(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_small_lateral_spec.mesh_foot_bl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_small_lateral_spec.mesh_foot_br(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_small_central_spec.mesh_tail(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
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
            Body::QuadrupedMedium(body) => {
                let quadruped_medium_central_spec =
                    QuadrupedMediumCentralSpec::load_watched(manifest_indicator);
                let quadruped_medium_lateral_spec =
                    QuadrupedMediumLateralSpec::load_watched(manifest_indicator);

                [
                    Some(quadruped_medium_central_spec.mesh_head_upper(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_central_spec.mesh_head_lower(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_central_spec.mesh_jaw(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_central_spec.mesh_tail(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_central_spec.mesh_torso_front(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_central_spec.mesh_torso_back(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_central_spec.mesh_ears(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_leg_fl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_leg_fr(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_leg_bl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_leg_br(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_foot_fl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_foot_fr(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_foot_bl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_foot_br(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    None,
                ]
            },
            Body::BirdMedium(body) => {
                let bird_medium_center_spec =
                    BirdMediumCenterSpec::load_watched(manifest_indicator);
                let bird_medium_lateral_spec =
                    BirdMediumLateralSpec::load_watched(manifest_indicator);

                [
                    Some(bird_medium_center_spec.mesh_head(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(bird_medium_center_spec.mesh_torso(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(bird_medium_center_spec.mesh_tail(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(bird_medium_lateral_spec.mesh_wing_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(bird_medium_lateral_spec.mesh_wing_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(bird_medium_lateral_spec.mesh_foot_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(bird_medium_lateral_spec.mesh_foot_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
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
            Body::FishMedium(body) => [
                Some(mesh_fish_medium_head(body.head, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
                Some(mesh_fish_medium_torso(body.torso, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
                Some(mesh_fish_medium_rear(body.rear, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
                Some(mesh_fish_medium_tail(body.tail, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
                Some(mesh_fish_medium_fin_l(body.fin_l, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
                Some(mesh_fish_medium_fin_r(body.fin_r, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
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
            ],
            Body::Dragon(body) => {
                let dragon_center_spec = DragonCenterSpec::load_watched(manifest_indicator);
                let dragon_lateral_spec = DragonLateralSpec::load_watched(manifest_indicator);

                [
                    Some(dragon_center_spec.mesh_head_upper(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_center_spec.mesh_head_lower(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_center_spec.mesh_jaw(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_center_spec.mesh_chest_front(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_center_spec.mesh_chest_rear(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_center_spec.mesh_tail_front(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_center_spec.mesh_tail_rear(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_lateral_spec.mesh_wing_in_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_lateral_spec.mesh_wing_in_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_lateral_spec.mesh_wing_out_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_lateral_spec.mesh_wing_out_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_lateral_spec.mesh_foot_fl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_lateral_spec.mesh_foot_fr(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_lateral_spec.mesh_foot_bl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(dragon_lateral_spec.mesh_foot_br(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    None,
                ]
            },
            Body::BirdSmall(body) => [
                Some(mesh_bird_small_head(body.head, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
                Some(mesh_bird_small_torso(body.torso, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
                Some(mesh_bird_small_wing_l(body.wing_l, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
                Some(mesh_bird_small_wing_r(body.wing_r, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
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
            ],
            Body::FishSmall(body) => [
                Some(mesh_fish_small_torso(body.torso, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
                Some(mesh_fish_small_tail(body.tail, |segment, offset| {
                    generate_mesh(segment, offset)
                })),
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
            ],
            Body::BipedLarge(body) => {
                let biped_large_center_spec =
                    BipedLargeCenterSpec::load_watched(manifest_indicator);
                let biped_large_lateral_spec =
                    BipedLargeLateralSpec::load_watched(manifest_indicator);

                [
                    Some(biped_large_center_spec.mesh_head(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_center_spec.mesh_jaw(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_center_spec.mesh_torso_upper(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_center_spec.mesh_torso_lower(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_center_spec.mesh_tail(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_center_spec.mesh_main(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_center_spec.mesh_second(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_lateral_spec.mesh_shoulder_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_lateral_spec.mesh_shoulder_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_lateral_spec.mesh_hand_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_lateral_spec.mesh_hand_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_lateral_spec.mesh_leg_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_lateral_spec.mesh_leg_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_lateral_spec.mesh_foot_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(biped_large_lateral_spec.mesh_foot_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    None,
                ]
            },
            Body::Golem(body) => {
                let golem_center_spec = GolemCenterSpec::load_watched(manifest_indicator);
                let golem_lateral_spec = GolemLateralSpec::load_watched(manifest_indicator);

                [
                    Some(golem_center_spec.mesh_head(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(golem_center_spec.mesh_torso_upper(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(golem_lateral_spec.mesh_shoulder_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(golem_lateral_spec.mesh_shoulder_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(golem_lateral_spec.mesh_hand_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(golem_lateral_spec.mesh_hand_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(golem_lateral_spec.mesh_leg_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(golem_lateral_spec.mesh_leg_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(golem_lateral_spec.mesh_foot_l(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(golem_lateral_spec.mesh_foot_r(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ]
            },
            Body::Critter(body) => {
                let critter_center_spec = CritterCenterSpec::load_watched(manifest_indicator);

                [
                    Some(critter_center_spec.mesh_head(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(critter_center_spec.mesh_chest(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(critter_center_spec.mesh_feet_f(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(critter_center_spec.mesh_feet_b(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(critter_center_spec.mesh_tail(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
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
            Body::QuadrupedLow(body) => {
                let quadruped_low_central_spec =
                    QuadrupedLowCentralSpec::load_watched(manifest_indicator);
                let quadruped_low_lateral_spec =
                    QuadrupedLowLateralSpec::load_watched(manifest_indicator);

                [
                    Some(quadruped_low_central_spec.mesh_head_upper(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_low_central_spec.mesh_head_lower(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_low_central_spec.mesh_jaw(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_low_central_spec.mesh_chest(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_low_central_spec.mesh_tail_front(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_low_central_spec.mesh_tail_rear(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_low_lateral_spec.mesh_foot_fl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_low_lateral_spec.mesh_foot_fr(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_low_lateral_spec.mesh_foot_bl(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    Some(quadruped_low_lateral_spec.mesh_foot_br(
                        body.species,
                        body.body_type,
                        |segment, offset| generate_mesh(segment, offset),
                    )),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ]
            },
            Body::Object(object) => [
                Some(mesh_object(object, generate_mesh)),
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
            ],
        }
    }

    pub fn get_or_create_model(
        &mut self,
        renderer: &mut Renderer,
        col_lights: &mut super::FigureColLights,
        body: Body,
        loadout: Option<&Loadout>,
        tick: u64,
        camera_mode: CameraMode,
        character_state: Option<&CharacterState>,
    ) -> &(FigureModelEntryLod, Skel::Attr)
    where
        for<'a> &'a common::comp::Body: std::convert::TryInto<Skel::Attr>,
        Skel::Attr: Default,
    {
        let key = FigureKey {
            body,
            extra: loadout.map(|loadout| {
                Box::new(CharacterCacheKey::from(
                    character_state,
                    camera_mode,
                    loadout,
                ))
            }),
        };

        match self.models.entry(key) {
            Entry::Occupied(o) => {
                let (model, last_used) = o.into_mut();
                *last_used = tick;
                model
            },
            Entry::Vacant(v) => {
                let key = v.key();
                let model = {
                    let skeleton_attr = (&body)
                        .try_into()
                        .ok()
                        .unwrap_or_else(<Skel::Attr as Default>::default);

                    let manifest_indicator = &mut self.manifest_indicator;
                    let mut greedy = FigureModel::make_greedy();
                    let mut opaque = Mesh::<TerrainPipeline>::new();
                    // Choose the most conservative bounds for any LOD model.
                    let mut figure_bounds = anim::vek::Aabb {
                        min: anim::vek::Vec3::zero(),
                        max: anim::vek::Vec3::zero(),
                    };
                    // Meshes all bone models for this figure using the given mesh generation
                    // function, attaching it to the current greedy mesher and opaque vertex
                    // list.  Returns the vertex bounds of the meshed model within the opaque
                    // mesh.
                    let mut make_model = |generate_mesh: for<'a, 'b> fn(
                        &mut GreedyMesh<'a>,
                        &'b mut _,
                        _,
                        _,
                    )
                        -> _| {
                        let vertex_start = opaque.vertices().len();
                        let meshes =
                            Self::bone_meshes(key, manifest_indicator, |segment, offset| {
                                generate_mesh(&mut greedy, &mut opaque, segment, offset)
                            });
                        meshes
                            .iter()
                            .enumerate()
                            // NOTE: Cast to u8 is safe because i <= 16.
                            .filter_map(|(i, bm)| bm.as_ref().map(|bm| (i as u8, bm.clone())))
                            .for_each(|(i, (_opaque_mesh, (bounds, vertex_range)))| {
                                // Update the bone index for all vertices that belong to this
                                // model.
                                opaque
                                    .iter_mut(vertex_range)
                                    .for_each(|vert| {
                                        vert.set_bone_idx(i);
                                    });

                                // Update the figure bounds to the largest granularity seen so far
                                // (NOTE: this is more than a little imperfect).
                                //
                                // FIXME: Maybe use the default bone position in the idle animation
                                // to figure this out instead?
                                figure_bounds.expand_to_contain(bounds);
                            });
                        // NOTE: vertex_start and vertex_end *should* fit in a u32, by the
                        // following logic:
                        //
                        // Our new figure maximum is constrained to at most 2^8 × 2^8 × 2^8.
                        // This uses at most 24 bits to store every vertex exactly once.
                        // Greedy meshing can store each vertex in up to 3 quads, we have 3
                        // greedy models, and we store 1.5x the vertex count, so the maximum
                        // total space a model can take up is 3 * 3 * 1.5 * 2^24; rounding
                        // up to 4 * 4 * 2^24 gets us to 2^28, which clearly still fits in a
                        // u32.
                        //
                        // (We could also, though we prefer not to, reason backwards from the
                        // maximum figure texture size of 2^15 × 2^15, also fits in a u32; we
                        // can also see that, since we can have at most one texture entry per
                        // vertex, any texture atlas of size 2^14 × 2^14 or higher should be
                        // able to store data for any figure.  So the only reason we would fail
                        // here would be if the user's computer could not store a texture large
                        // enough to fit all the LOD models for the figure, not for fundamental
                        // reasons related to fitting in a u32).
                        //
                        // Therefore, these casts are safe.
                        vertex_start as u32..opaque.vertices().len() as u32
                    };

                    fn generate_mesh<'a>(
                        greedy: &mut GreedyMesh<'a>,
                        opaque_mesh: &mut Mesh<TerrainPipeline>,
                        segment: Segment,
                        offset: Vec3<f32>,
                    ) -> BoneMeshes {
                        let (opaque, _, _, bounds) =
                            Meshable::<FigurePipeline, &mut GreedyMesh>::generate_mesh(
                                segment,
                                (greedy, opaque_mesh, offset, Vec3::one()),
                            );
                        (opaque, bounds)
                    }

                    fn generate_mesh_lod_mid<'a>(
                        greedy: &mut GreedyMesh<'a>,
                        opaque_mesh: &mut Mesh<TerrainPipeline>,
                        segment: Segment,
                        offset: Vec3<f32>,
                    ) -> BoneMeshes {
                        let lod_scale = 0.6;
                        let (opaque, _, _, bounds) =
                            Meshable::<FigurePipeline, &mut GreedyMesh>::generate_mesh(
                                segment.scaled_by(Vec3::broadcast(lod_scale)),
                                (
                                    greedy,
                                    opaque_mesh,
                                    offset * lod_scale,
                                    Vec3::one() / lod_scale,
                                ),
                            );
                        (opaque, bounds)
                    }

                    fn generate_mesh_lod_low<'a>(
                        greedy: &mut GreedyMesh<'a>,
                        opaque_mesh: &mut Mesh<TerrainPipeline>,
                        segment: Segment,
                        offset: Vec3<f32>,
                    ) -> BoneMeshes {
                        let lod_scale = 0.3;
                        let (opaque, _, _, bounds) =
                            Meshable::<FigurePipeline, &mut GreedyMesh>::generate_mesh(
                                segment.scaled_by(Vec3::broadcast(lod_scale)),
                                (
                                    greedy,
                                    opaque_mesh,
                                    offset * lod_scale,
                                    Vec3::one() / lod_scale,
                                ),
                            );
                        (opaque, bounds)
                    }

                    let models = [
                        make_model(generate_mesh),
                        make_model(generate_mesh_lod_mid),
                        make_model(generate_mesh_lod_low),
                    ];
                    (
                        col_lights
                            .create_figure(renderer, greedy, (opaque, figure_bounds), models)
                            .expect("Failed to upload figure data to the GPU!"),
                        skeleton_attr,
                    )
                };
                &v.insert((model, tick)).0
            },
        }
    }

    pub fn clean(&mut self, col_lights: &mut super::FigureColLights, tick: u64) {
        // Check for reloaded manifests
        // TODO: maybe do this in a different function, maintain?
        if self.manifest_indicator.reloaded() {
            col_lights.atlas.clear();
            self.models.clear();
        }
        // TODO: Don't hard-code this.
        if tick % 60 == 0 {
            self.models.retain(|_, ((model_entry, _), last_used)| {
                // Wait about a minute at 60 fps before invalidating old models.
                let delta = 60 * 60;
                let alive = *last_used + delta > tick;
                if !alive {
                    col_lights.atlas.deallocate(model_entry.allocation.id);
                }
                alive
            });
        }
    }
}
