use super::load::*;
use crate::{
    anim::{self, Skeleton},
    render::{FigurePipeline, Mesh, Model, Renderer},
    scene::camera::CameraMode,
};
use common::{
    assets::watch::ReloadIndicator,
    comp::{Body, CharacterState, ItemKind, Loadout},
};
use hashbrown::{hash_map::Entry, HashMap};
use std::{
    convert::TryInto,
    mem::{discriminant, Discriminant},
};

#[derive(PartialEq, Eq, Hash, Clone)]
enum FigureKey {
    Simple(Body),
    Complex(
        Body,
        Option<Loadout>,
        CameraMode,
        Option<CharacterStateCacheKey>,
    ),
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct CharacterStateCacheKey {
    state: Discriminant<CharacterState>, // TODO: Can this be simplified?
}

impl From<&CharacterState> for CharacterStateCacheKey {
    fn from(cs: &CharacterState) -> Self {
        Self {
            state: discriminant(&cs),
        }
    }
}

pub struct FigureModelCache<Skel = anim::character::CharacterSkeleton>
where
    Skel: Skeleton,
{
    models: HashMap<FigureKey, ((Model<FigurePipeline>, Skel::Attr), u64)>,
    manifest_indicator: ReloadIndicator,
}

impl<Skel: Skeleton> FigureModelCache<Skel> {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            manifest_indicator: ReloadIndicator::new(),
        }
    }

    pub fn get_or_create_model(
        &mut self,
        renderer: &mut Renderer,
        body: Body,
        loadout: Option<&Loadout>,
        tick: u64,
        camera_mode: CameraMode,
        character_state: Option<&CharacterState>,
    ) -> &(Model<FigurePipeline>, Skel::Attr)
    where
        for<'a> &'a common::comp::Body: std::convert::TryInto<Skel::Attr>,
        Skel::Attr: Default,
    {
        let key = if loadout.is_some() {
            FigureKey::Complex(
                body,
                loadout.cloned(),
                camera_mode,
                character_state.map(|cs| CharacterStateCacheKey::from(cs)),
            )
        } else {
            FigureKey::Simple(body)
        };

        match self.models.entry(key) {
            Entry::Occupied(o) => {
                let (model, last_used) = o.into_mut();
                *last_used = tick;
                model
            },
            Entry::Vacant(v) => {
                &v.insert((
                    {
                        let bone_meshes = match body {
                            Body::Humanoid(body) => {
                                let humanoid_head_spec =
                                    HumHeadSpec::load_watched(&mut self.manifest_indicator);
                                let humanoid_armor_shoulder_spec =
                                    HumArmorShoulderSpec::load_watched(
                                        &mut self.manifest_indicator,
                                    );
                                let humanoid_armor_chest_spec =
                                    HumArmorChestSpec::load_watched(&mut self.manifest_indicator);
                                let humanoid_armor_hand_spec =
                                    HumArmorHandSpec::load_watched(&mut self.manifest_indicator);
                                let humanoid_armor_belt_spec =
                                    HumArmorBeltSpec::load_watched(&mut self.manifest_indicator);
                                let humanoid_armor_pants_spec =
                                    HumArmorPantsSpec::load_watched(&mut self.manifest_indicator);
                                let humanoid_armor_foot_spec =
                                    HumArmorFootSpec::load_watched(&mut self.manifest_indicator);

                                // TODO: This is bad code, maybe this method should return Option<_>
                                let default_loadout = Loadout::default();
                                let loadout = loadout.unwrap_or(&default_loadout);

                                [
                                    match camera_mode {
                                        CameraMode::ThirdPerson => {
                                            Some(humanoid_head_spec.mesh_head(
                                                body.race,
                                                body.body_type,
                                                body.hair_color,
                                                body.hair_style,
                                                body.beard,
                                                body.eye_color,
                                                body.skin,
                                                body.eyebrows,
                                                body.accessory,
                                            ))
                                        },
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => Some(
                                            humanoid_armor_chest_spec.mesh_chest(&body, loadout),
                                        ),
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => {
                                            Some(humanoid_armor_belt_spec.mesh_belt(&body, loadout))
                                        },
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => Some(
                                            humanoid_armor_pants_spec.mesh_pants(&body, loadout),
                                        ),
                                        CameraMode::FirstPerson => None,
                                    },
                                    if camera_mode == CameraMode::FirstPerson
                                        && character_state
                                            .map(|cs| cs.is_dodge())
                                            .unwrap_or_default()
                                    {
                                        None
                                    } else {
                                        Some(
                                            humanoid_armor_hand_spec.mesh_left_hand(&body, loadout),
                                        )
                                    },
                                    if character_state.map(|cs| cs.is_dodge()).unwrap_or_default() {
                                        None
                                    } else {
                                        Some(
                                            humanoid_armor_hand_spec
                                                .mesh_right_hand(&body, loadout),
                                        )
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => Some(
                                            humanoid_armor_foot_spec.mesh_left_foot(&body, loadout),
                                        ),
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => Some(
                                            humanoid_armor_foot_spec
                                                .mesh_right_foot(&body, loadout),
                                        ),
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => Some(
                                            humanoid_armor_shoulder_spec
                                                .mesh_left_shoulder(&body, loadout),
                                        ),
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => Some(
                                            humanoid_armor_shoulder_spec
                                                .mesh_right_shoulder(&body, loadout),
                                        ),
                                        CameraMode::FirstPerson => None,
                                    },
                                    Some(mesh_glider()),
                                    if camera_mode != CameraMode::FirstPerson
                                        || character_state
                                            .map(|cs| {
                                                cs.is_attack() || cs.is_block() || cs.is_wield()
                                            })
                                            .unwrap_or_default()
                                    {
                                        Some(mesh_main(
                                            loadout.active_item.as_ref().map(|i| &i.item.kind),
                                        ))
                                    } else {
                                        None
                                    },
                                    Some(mesh_lantern()),
                                    None,
                                    None,
                                    None,
                                ]
                            },
                            Body::QuadrupedSmall(body) => {
                                let quadruped_small_central_spec =
                                    QuadrupedSmallCentralSpec::load_watched(
                                        &mut self.manifest_indicator,
                                    );
                                let quadruped_small_lateral_spec =
                                    QuadrupedSmallLateralSpec::load_watched(
                                        &mut self.manifest_indicator,
                                    );

                                [
                                    Some(
                                        quadruped_small_central_spec
                                            .mesh_head(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_small_central_spec
                                            .mesh_chest(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_small_lateral_spec
                                            .mesh_foot_lf(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_small_lateral_spec
                                            .mesh_foot_rf(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_small_lateral_spec
                                            .mesh_foot_lb(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_small_lateral_spec
                                            .mesh_foot_rb(body.species, body.body_type),
                                    ),
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
                            Body::QuadrupedMedium(body) => {
                                let quadruped_medium_central_spec =
                                    QuadrupedMediumCentralSpec::load_watched(
                                        &mut self.manifest_indicator,
                                    );
                                let quadruped_medium_lateral_spec =
                                    QuadrupedMediumLateralSpec::load_watched(
                                        &mut self.manifest_indicator,
                                    );

                                [
                                    Some(
                                        quadruped_medium_central_spec
                                            .mesh_head_upper(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_central_spec
                                            .mesh_head_lower(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_central_spec
                                            .mesh_jaw(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_central_spec
                                            .mesh_tail(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_central_spec
                                            .mesh_torso_f(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_central_spec
                                            .mesh_torso_b(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_central_spec
                                            .mesh_ears(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_lateral_spec
                                            .mesh_foot_lf(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_lateral_spec
                                            .mesh_foot_rf(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_lateral_spec
                                            .mesh_foot_lb(body.species, body.body_type),
                                    ),
                                    Some(
                                        quadruped_medium_lateral_spec
                                            .mesh_foot_rb(body.species, body.body_type),
                                    ),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                ]
                            },
                            Body::BirdMedium(body) => {
                                let bird_medium_center_spec = BirdMediumCenterSpec::load_watched(
                                    &mut self.manifest_indicator,
                                );
                                let bird_medium_lateral_spec = BirdMediumLateralSpec::load_watched(
                                    &mut self.manifest_indicator,
                                );

                                [
                                    Some(
                                        bird_medium_center_spec
                                            .mesh_head(body.species, body.body_type),
                                    ),
                                    Some(
                                        bird_medium_center_spec
                                            .mesh_torso(body.species, body.body_type),
                                    ),
                                    Some(
                                        bird_medium_center_spec
                                            .mesh_tail(body.species, body.body_type),
                                    ),
                                    Some(
                                        bird_medium_lateral_spec
                                            .mesh_wing_l(body.species, body.body_type),
                                    ),
                                    Some(
                                        bird_medium_lateral_spec
                                            .mesh_wing_r(body.species, body.body_type),
                                    ),
                                    Some(
                                        bird_medium_lateral_spec
                                            .mesh_foot_l(body.species, body.body_type),
                                    ),
                                    Some(
                                        bird_medium_lateral_spec
                                            .mesh_foot_r(body.species, body.body_type),
                                    ),
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
                            ],
                            Body::Dragon(body) => [
                                Some(mesh_dragon_head(body.head)),
                                Some(mesh_dragon_chest_front(body.chest_front)),
                                Some(mesh_dragon_chest_rear(body.chest_rear)),
                                Some(mesh_dragon_tail_front(body.tail_front)),
                                Some(mesh_dragon_tail_rear(body.tail_rear)),
                                Some(mesh_dragon_wing_in_l(body.wing_in_l)),
                                Some(mesh_dragon_wing_in_r(body.wing_in_r)),
                                Some(mesh_dragon_wing_out_l(body.wing_out_l)),
                                Some(mesh_dragon_wing_out_r(body.wing_out_r)),
                                Some(mesh_dragon_foot_fl(body.foot_fl)),
                                Some(mesh_dragon_foot_fr(body.foot_fr)),
                                Some(mesh_dragon_foot_bl(body.foot_bl)),
                                Some(mesh_dragon_foot_br(body.foot_br)),
                                None,
                                None,
                                None,
                            ],
                            Body::BirdSmall(body) => [
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
                            ],
                            Body::FishSmall(body) => [
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
                            ],
                            Body::BipedLarge(body) => {
                                let biped_large_center_spec = BipedLargeCenterSpec::load_watched(
                                    &mut self.manifest_indicator,
                                );
                                let biped_large_lateral_spec = BipedLargeLateralSpec::load_watched(
                                    &mut self.manifest_indicator,
                                );

                                [
                                    Some(
                                        biped_large_center_spec
                                            .mesh_head(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_center_spec
                                            .mesh_torso_upper(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_center_spec
                                            .mesh_torso_lower(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_lateral_spec
                                            .mesh_shoulder_l(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_lateral_spec
                                            .mesh_shoulder_r(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_lateral_spec
                                            .mesh_hand_l(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_lateral_spec
                                            .mesh_hand_r(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_lateral_spec
                                            .mesh_leg_l(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_lateral_spec
                                            .mesh_leg_r(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_lateral_spec
                                            .mesh_foot_l(body.species, body.body_type),
                                    ),
                                    Some(
                                        biped_large_lateral_spec
                                            .mesh_foot_r(body.species, body.body_type),
                                    ),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                ]
                            },
                            Body::Critter(body) => {
                                let critter_center_spec =
                                    CritterCenterSpec::load_watched(&mut self.manifest_indicator);

                                [
                                    Some(
                                        critter_center_spec.mesh_head(body.species, body.body_type),
                                    ),
                                    Some(
                                        critter_center_spec
                                            .mesh_chest(body.species, body.body_type),
                                    ),
                                    Some(
                                        critter_center_spec
                                            .mesh_feet_f(body.species, body.body_type),
                                    ),
                                    Some(
                                        critter_center_spec
                                            .mesh_feet_b(body.species, body.body_type),
                                    ),
                                    Some(
                                        critter_center_spec.mesh_tail(body.species, body.body_type),
                                    ),
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
                            Body::Object(object) => [
                                Some(mesh_object(object)),
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
                        };

                        let skeleton_attr = (&body)
                            .try_into()
                            .ok()
                            .unwrap_or_else(<Skel::Attr as Default>::default);

                        let mut mesh = Mesh::new();
                        bone_meshes
                            .iter()
                            .enumerate()
                            .filter_map(|(i, bm)| bm.as_ref().map(|bm| (i, bm)))
                            .for_each(|(i, bone_mesh)| {
                                mesh.push_mesh_map(bone_mesh, |vert| vert.with_bone_idx(i as u8))
                            });

                        (renderer.create_model(&mesh).unwrap(), skeleton_attr)
                    },
                    tick,
                ))
                .0
            },
        }
    }

    pub fn clean(&mut self, tick: u64) {
        // Check for reloaded manifests
        // TODO: maybe do this in a different function, maintain?
        if self.manifest_indicator.reloaded() {
            self.models.clear();
        }
        // TODO: Don't hard-code this.
        self.models
            .retain(|_, (_, last_used)| *last_used + 60 > tick);
    }
}
