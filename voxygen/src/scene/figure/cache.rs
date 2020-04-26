use super::load::*;
use crate::{
    anim::{self, Skeleton},
    mesh::Meshable,
    render::{FigurePipeline, Mesh, Model, Renderer},
    scene::camera::CameraMode,
};
use common::{
    assets::watch::ReloadIndicator,
    comp::{
        item::{tool::ToolKind, ItemKind},
        Body, CharacterState, Item, Loadout,
    },
    figure::Segment,
    vol::BaseVol,
};
use hashbrown::{hash_map::Entry, HashMap};
use std::{
    convert::TryInto,
    mem::{discriminant, Discriminant},
};
use vek::*;

#[derive(PartialEq, Eq, Hash, Clone)]
enum FigureKey {
    Simple(Body),
    Complex(Body, CameraMode, CharacterCacheKey),
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct CharacterCacheKey {
    state: Option<Discriminant<CharacterState>>, // TODO: Can this be simplified?
    active_tool: Option<ToolKind>,
    shoulder: Option<Item>,
    chest: Option<Item>,
    belt: Option<Item>,
    back: Option<Item>,
    lantern: Option<Item>,
    hand: Option<Item>,
    pants: Option<Item>,
    foot: Option<Item>,
}

impl CharacterCacheKey {
    fn from(cs: Option<&CharacterState>, loadout: &Loadout) -> Self {
        Self {
            state: cs.map(|cs| discriminant(cs)),
            active_tool: if let Some(ItemKind::Tool(tool)) =
                loadout.active_item.as_ref().map(|i| &i.item.kind)
            {
                Some(tool.kind)
            } else {
                None
            },
            shoulder: loadout.shoulder.clone(),
            chest: loadout.chest.clone(),
            belt: loadout.belt.clone(),
            back: loadout.back.clone(),
            lantern: loadout.lantern.clone(),
            hand: loadout.hand.clone(),
            pants: loadout.pants.clone(),
            foot: loadout.foot.clone(),
        }
    }
}

pub struct FigureModelCache<Skel = anim::character::CharacterSkeleton>
where
    Skel: Skeleton,
{
    models: HashMap<FigureKey, (([Model<FigurePipeline>; 3], Skel::Attr), u64)>,
    manifest_indicator: ReloadIndicator,
}

impl<Skel: Skeleton> FigureModelCache<Skel> {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            manifest_indicator: ReloadIndicator::new(),
        }
    }

    fn bone_meshes(
        body: Body,
        loadout: Option<&Loadout>,
        character_state: Option<&CharacterState>,
        camera_mode: CameraMode,
        manifest_indicator: &mut ReloadIndicator,
        generate_mesh: fn(&Segment, Vec3<f32>) -> Mesh<FigurePipeline>,
    ) -> [Option<Mesh<FigurePipeline>>; 16] {
        match body {
            Body::Humanoid(body) => {
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

                // TODO: This is bad code, maybe this method should return Option<_>
                let default_loadout = Loadout::default();
                let loadout = loadout.unwrap_or(&default_loadout);

                [
                    match camera_mode {
                        CameraMode::ThirdPerson => Some(humanoid_head_spec.mesh_head(
                            body.race,
                            body.body_type,
                            body.hair_color,
                            body.hair_style,
                            body.beard,
                            body.eye_color,
                            body.skin,
                            body.eyebrows,
                            body.accessory,
                            generate_mesh,
                        )),
                        CameraMode::FirstPerson => None,
                    },
                    match camera_mode {
                        CameraMode::ThirdPerson => Some(humanoid_armor_chest_spec.mesh_chest(
                            &body,
                            loadout,
                            generate_mesh,
                        )),
                        CameraMode::FirstPerson => None,
                    },
                    match camera_mode {
                        CameraMode::ThirdPerson => {
                            Some(humanoid_armor_belt_spec.mesh_belt(&body, loadout, generate_mesh))
                        },
                        CameraMode::FirstPerson => None,
                    },
                    match camera_mode {
                        CameraMode::ThirdPerson => {
                            Some(humanoid_armor_back_spec.mesh_back(&body, loadout, generate_mesh))
                        },
                        CameraMode::FirstPerson => None,
                    },
                    match camera_mode {
                        CameraMode::ThirdPerson => Some(humanoid_armor_pants_spec.mesh_pants(
                            &body,
                            loadout,
                            generate_mesh,
                        )),
                        CameraMode::FirstPerson => None,
                    },
                    if camera_mode == CameraMode::FirstPerson
                        && character_state.map(|cs| cs.is_dodge()).unwrap_or_default()
                    {
                        None
                    } else {
                        Some(humanoid_armor_hand_spec.mesh_left_hand(&body, loadout, generate_mesh))
                    },
                    if character_state.map(|cs| cs.is_dodge()).unwrap_or_default() {
                        None
                    } else {
                        Some(humanoid_armor_hand_spec.mesh_right_hand(
                            &body,
                            loadout,
                            generate_mesh,
                        ))
                    },
                    match camera_mode {
                        CameraMode::ThirdPerson => Some(humanoid_armor_foot_spec.mesh_left_foot(
                            &body,
                            loadout,
                            generate_mesh,
                        )),
                        CameraMode::FirstPerson => None,
                    },
                    match camera_mode {
                        CameraMode::ThirdPerson => Some(humanoid_armor_foot_spec.mesh_right_foot(
                            &body,
                            loadout,
                            generate_mesh,
                        )),
                        CameraMode::FirstPerson => None,
                    },
                    match camera_mode {
                        CameraMode::ThirdPerson => {
                            Some(humanoid_armor_shoulder_spec.mesh_left_shoulder(
                                &body,
                                loadout,
                                generate_mesh,
                            ))
                        },
                        CameraMode::FirstPerson => None,
                    },
                    match camera_mode {
                        CameraMode::ThirdPerson => {
                            Some(humanoid_armor_shoulder_spec.mesh_right_shoulder(
                                &body,
                                loadout,
                                generate_mesh,
                            ))
                        },
                        CameraMode::FirstPerson => None,
                    },
                    Some(mesh_glider(generate_mesh)),
                    if camera_mode != CameraMode::FirstPerson
                        || character_state
                            .map(|cs| cs.is_attack() || cs.is_block() || cs.is_wield())
                            .unwrap_or_default()
                    {
                        Some(humanoid_main_weapon_spec.mesh_main_weapon(
                            loadout.active_item.as_ref().map(|i| &i.item.kind),
                            generate_mesh,
                        ))
                    } else {
                        None
                    },
                    None,
                    Some(humanoid_armor_lantern_spec.mesh_lantern(&body, loadout, generate_mesh)),
                    None,
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
                        generate_mesh,
                    )),
                    Some(quadruped_small_central_spec.mesh_chest(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_small_lateral_spec.mesh_foot_lf(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_small_lateral_spec.mesh_foot_rf(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_small_lateral_spec.mesh_foot_lb(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_small_lateral_spec.mesh_foot_rb(
                        body.species,
                        body.body_type,
                        generate_mesh,
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
                        generate_mesh,
                    )),
                    Some(quadruped_medium_central_spec.mesh_head_lower(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_medium_central_spec.mesh_jaw(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_medium_central_spec.mesh_tail(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_medium_central_spec.mesh_torso_f(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_medium_central_spec.mesh_torso_b(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_medium_central_spec.mesh_ears(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_foot_lf(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_foot_rf(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_foot_lb(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(quadruped_medium_lateral_spec.mesh_foot_rb(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    None,
                    None,
                    None,
                    None,
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
                        generate_mesh,
                    )),
                    Some(bird_medium_center_spec.mesh_torso(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(bird_medium_center_spec.mesh_tail(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(bird_medium_lateral_spec.mesh_wing_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(bird_medium_lateral_spec.mesh_wing_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(bird_medium_lateral_spec.mesh_foot_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(bird_medium_lateral_spec.mesh_foot_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
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
                Some(mesh_fish_medium_head(body.head, generate_mesh)),
                Some(mesh_fish_medium_torso(body.torso, generate_mesh)),
                Some(mesh_fish_medium_rear(body.rear, generate_mesh)),
                Some(mesh_fish_medium_tail(body.tail, generate_mesh)),
                Some(mesh_fish_medium_fin_l(body.fin_l, generate_mesh)),
                Some(mesh_fish_medium_fin_r(body.fin_r, generate_mesh)),
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
                Some(mesh_dragon_head(body.head, generate_mesh)),
                Some(mesh_dragon_chest_front(body.chest_front, generate_mesh)),
                Some(mesh_dragon_chest_rear(body.chest_rear, generate_mesh)),
                Some(mesh_dragon_tail_front(body.tail_front, generate_mesh)),
                Some(mesh_dragon_tail_rear(body.tail_rear, generate_mesh)),
                Some(mesh_dragon_wing_in_l(body.wing_in_l, generate_mesh)),
                Some(mesh_dragon_wing_in_r(body.wing_in_r, generate_mesh)),
                Some(mesh_dragon_wing_out_l(body.wing_out_l, generate_mesh)),
                Some(mesh_dragon_wing_out_r(body.wing_out_r, generate_mesh)),
                Some(mesh_dragon_foot_fl(body.foot_fl, generate_mesh)),
                Some(mesh_dragon_foot_fr(body.foot_fr, generate_mesh)),
                Some(mesh_dragon_foot_bl(body.foot_bl, generate_mesh)),
                Some(mesh_dragon_foot_br(body.foot_br, generate_mesh)),
                None,
                None,
                None,
            ],
            Body::BirdSmall(body) => [
                Some(mesh_bird_small_head(body.head, generate_mesh)),
                Some(mesh_bird_small_torso(body.torso, generate_mesh)),
                Some(mesh_bird_small_wing_l(body.wing_l, generate_mesh)),
                Some(mesh_bird_small_wing_r(body.wing_r, generate_mesh)),
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
                Some(mesh_fish_small_torso(body.torso, generate_mesh)),
                Some(mesh_fish_small_tail(body.tail, generate_mesh)),
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
                        generate_mesh,
                    )),
                    Some(biped_large_center_spec.mesh_torso_upper(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(biped_large_center_spec.mesh_torso_lower(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(biped_large_lateral_spec.mesh_shoulder_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(biped_large_lateral_spec.mesh_shoulder_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(biped_large_lateral_spec.mesh_hand_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(biped_large_lateral_spec.mesh_hand_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(biped_large_lateral_spec.mesh_leg_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(biped_large_lateral_spec.mesh_leg_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(biped_large_lateral_spec.mesh_foot_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(biped_large_lateral_spec.mesh_foot_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    None,
                    None,
                    None,
                    None,
                    None,
                ]
            },
            Body::Golem(body) => {
                let golem_center_spec = GolemCenterSpec::load_watched(manifest_indicator);
                let golem_lateral_spec = GolemLateralSpec::load_watched(manifest_indicator);

                [
                    Some(golem_center_spec.mesh_head(body.species, body.body_type, generate_mesh)),
                    Some(golem_center_spec.mesh_torso_upper(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(golem_lateral_spec.mesh_shoulder_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(golem_lateral_spec.mesh_shoulder_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(golem_lateral_spec.mesh_hand_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(golem_lateral_spec.mesh_hand_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(golem_lateral_spec.mesh_leg_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(golem_lateral_spec.mesh_leg_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(golem_lateral_spec.mesh_foot_l(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(golem_lateral_spec.mesh_foot_r(
                        body.species,
                        body.body_type,
                        generate_mesh,
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
                        generate_mesh,
                    )),
                    Some(critter_center_spec.mesh_chest(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(critter_center_spec.mesh_feet_f(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(critter_center_spec.mesh_feet_b(
                        body.species,
                        body.body_type,
                        generate_mesh,
                    )),
                    Some(critter_center_spec.mesh_tail(
                        body.species,
                        body.body_type,
                        generate_mesh,
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
        body: Body,
        loadout: Option<&Loadout>,
        tick: u64,
        camera_mode: CameraMode,
        character_state: Option<&CharacterState>,
    ) -> &([Model<FigurePipeline>; 3], Skel::Attr)
    where
        for<'a> &'a common::comp::Body: std::convert::TryInto<Skel::Attr>,
        Skel::Attr: Default,
    {
        let key = if let Some(loadout) = loadout {
            FigureKey::Complex(
                body,
                camera_mode,
                CharacterCacheKey::from(character_state, loadout),
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
                        let skeleton_attr = (&body)
                            .try_into()
                            .ok()
                            .unwrap_or_else(<Skel::Attr as Default>::default);

                        let manifest_indicator = &mut self.manifest_indicator;
                        let mut make_model = |generate_mesh| {
                            let mut mesh = Mesh::new();
                            Self::bone_meshes(
                                body,
                                loadout,
                                character_state,
                                camera_mode,
                                manifest_indicator,
                                generate_mesh,
                            )
                            .iter()
                            .enumerate()
                            .filter_map(|(i, bm)| bm.as_ref().map(|bm| (i, bm)))
                            .for_each(|(i, bone_mesh)| {
                                mesh.push_mesh_map(bone_mesh, |vert| vert.with_bone_idx(i as u8))
                            });
                            renderer.create_model(&mesh).unwrap()
                        };

                        fn generate_mesh(
                            segment: &Segment,
                            offset: Vec3<f32>,
                        ) -> Mesh<FigurePipeline> {
                            Meshable::<FigurePipeline, FigurePipeline>::generate_mesh(
                                segment,
                                (offset, Vec3::one()),
                            )
                            .0
                        }

                        fn generate_mesh_lod_mid(
                            segment: &Segment,
                            offset: Vec3<f32>,
                        ) -> Mesh<FigurePipeline> {
                            let lod_scale = Vec3::broadcast(0.6);
                            Meshable::<FigurePipeline, FigurePipeline>::generate_mesh(
                                &segment.scaled_by(lod_scale),
                                (offset * lod_scale, Vec3::one() / lod_scale),
                            )
                            .0
                        }

                        fn generate_mesh_lod_low(
                            segment: &Segment,
                            offset: Vec3<f32>,
                        ) -> Mesh<FigurePipeline> {
                            let lod_scale = Vec3::broadcast(0.3);
                            Meshable::<FigurePipeline, FigurePipeline>::generate_mesh(
                                &segment.scaled_by(lod_scale),
                                (offset * lod_scale, Vec3::one() / lod_scale),
                            )
                            .0
                        }

                        (
                            [
                                make_model(generate_mesh),
                                make_model(generate_mesh_lod_mid),
                                make_model(generate_mesh_lod_low),
                            ],
                            skeleton_attr,
                        )
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
