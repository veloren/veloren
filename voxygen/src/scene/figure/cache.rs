use super::load::*;
use crate::{
    anim::SkeletonAttr,
    render::{FigurePipeline, Mesh, Model, Renderer},
    scene::camera::CameraMode,
};
use common::{
    assets::watch::ReloadIndicator,
    comp::{ActionState, Body, CharacterState, Equipment, MovementState},
};
use hashbrown::HashMap;
use std::mem::{discriminant, Discriminant};

#[derive(PartialEq, Eq, Hash, Clone)]
enum FigureKey {
    Simple(Body),
    Complex(
        Body,
        Option<Equipment>,
        CameraMode,
        Option<CharacterStateCacheKey>,
    ),
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct CharacterStateCacheKey {
    movement: Discriminant<MovementState>,
    action: Discriminant<ActionState>,
}

impl From<&CharacterState> for CharacterStateCacheKey {
    fn from(cs: &CharacterState) -> Self {
        Self {
            movement: discriminant(&cs.movement),
            action: discriminant(&cs.action),
        }
    }
}

pub struct FigureModelCache {
    models: HashMap<FigureKey, ((Model<FigurePipeline>, SkeletonAttr), u64)>,
    manifest_indicator: ReloadIndicator,
}

impl FigureModelCache {
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
        equipment: Option<&Equipment>,
        tick: u64,
        camera_mode: CameraMode,
        character_state: Option<&CharacterState>,
    ) -> &(Model<FigurePipeline>, SkeletonAttr) {
        let key = if equipment.is_some() {
            FigureKey::Complex(
                body,
                equipment.cloned(),
                camera_mode,
                character_state.map(|cs| CharacterStateCacheKey::from(cs)),
            )
        } else {
            FigureKey::Simple(body)
        };

        match self.models.get_mut(&key) {
            Some((_model, last_used)) => {
                *last_used = tick;
            }
            None => {
                self.models.insert(
                    key.clone(),
                    (
                        {
                            let humanoid_head_spec =
                                HumHeadSpec::load_watched(&mut self.manifest_indicator);
                            let humanoid_armor_shoulder_spec =
                                HumArmorShoulderSpec::load_watched(&mut self.manifest_indicator);
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

                            let bone_meshes = match body {
                                Body::Humanoid(body) => [
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
                                        }
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => {
                                            Some(humanoid_armor_chest_spec.mesh_chest(&body))
                                        }
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => {
                                            Some(humanoid_armor_belt_spec.mesh_belt(&body))
                                        }
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => {
                                            Some(humanoid_armor_pants_spec.mesh_pants(&body))
                                        }
                                        CameraMode::FirstPerson => None,
                                    },
                                    if camera_mode == CameraMode::FirstPerson
                                        && character_state
                                            .map(|cs| cs.movement.is_roll())
                                            .unwrap_or_default()
                                    {
                                        None
                                    } else {
                                        Some(humanoid_armor_hand_spec.mesh_left_hand(&body))
                                    },
                                    if character_state
                                        .map(|cs| cs.movement.is_roll())
                                        .unwrap_or_default()
                                    {
                                        None
                                    } else {
                                        Some(humanoid_armor_hand_spec.mesh_right_hand(&body))
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => {
                                            Some(humanoid_armor_foot_spec.mesh_left_foot(&body))
                                        }
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => {
                                            Some(humanoid_armor_foot_spec.mesh_right_foot(&body))
                                        }
                                        CameraMode::FirstPerson => None,
                                    },
                                    if camera_mode != CameraMode::FirstPerson
                                        || character_state
                                            .map(|cs| {
                                                cs.action.is_attack()
                                                    || cs.action.is_block()
                                                    || cs.action.is_wield()
                                            })
                                            .unwrap_or_default()
                                    {
                                        Some(mesh_main(equipment.and_then(|e| e.main.as_ref())))
                                    } else {
                                        None
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => Some(
                                            humanoid_armor_shoulder_spec.mesh_left_shoulder(&body),
                                        ),
                                        CameraMode::FirstPerson => None,
                                    },
                                    match camera_mode {
                                        CameraMode::ThirdPerson => Some(
                                            humanoid_armor_shoulder_spec.mesh_right_shoulder(&body),
                                        ),
                                        CameraMode::FirstPerson => None,
                                    },
                                    Some(mesh_draw()),
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
                                Body::Quadruped(body) => [
                                    Some(mesh_pig_head(body.head)),
                                    Some(mesh_pig_chest(body.chest)),
                                    Some(mesh_pig_leg_lf(body.leg_l)),
                                    Some(mesh_pig_leg_rf(body.leg_r)),
                                    Some(mesh_pig_leg_lb(body.leg_l)),
                                    Some(mesh_pig_leg_rb(body.leg_r)),
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
                                Body::QuadrupedMedium(body) => [
                                    Some(mesh_wolf_head_upper(body.head_upper)),
                                    Some(mesh_wolf_jaw(body.jaw)),
                                    Some(mesh_wolf_head_lower(body.head_lower)),
                                    Some(mesh_wolf_tail(body.tail)),
                                    Some(mesh_wolf_torso_back(body.torso_back)),
                                    Some(mesh_wolf_torso_mid(body.torso_mid)),
                                    Some(mesh_wolf_ears(body.ears)),
                                    Some(mesh_wolf_foot_lf(body.foot_lf)),
                                    Some(mesh_wolf_foot_rf(body.foot_rf)),
                                    Some(mesh_wolf_foot_lb(body.foot_lb)),
                                    Some(mesh_wolf_foot_rb(body.foot_rb)),
                                    None,
                                    None,
                                    None,
                                    None,
                                    None,
                                ],
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

                            let skeleton_attr = match body {
                                Body::Humanoid(body) => SkeletonAttr::from(&body),
                                _ => SkeletonAttr::default(),
                            };

                            let mut mesh = Mesh::new();
                            bone_meshes
                                .iter()
                                .enumerate()
                                .filter_map(|(i, bm)| bm.as_ref().map(|bm| (i, bm)))
                                .for_each(|(i, bone_mesh)| {
                                    mesh.push_mesh_map(bone_mesh, |vert| {
                                        vert.with_bone_idx(i as u8)
                                    })
                                });

                            (renderer.create_model(&mesh).unwrap(), skeleton_attr)
                        },
                        tick,
                    ),
                );
            }
        }

        &self.models[&key].0
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
