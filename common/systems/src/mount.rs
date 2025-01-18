use common::{
    comp::{
        Body, CharacterActivity, Collider, ControlAction, Controller, InputKind, Ori, PhysicsState,
        Pos, Scale, Vel,
    },
    link::Is,
    mounting::{Mount, Rider, VolumeRider},
    terrain::TerrainGrid,
    uid::IdMaps,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, WriteStorage};
use tracing::error;
use vek::*;

/// This system is responsible for controlling mounts
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, IdMaps>,
        ReadExpect<'a, TerrainGrid>,
        Entities<'a>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, Is<Rider>>,
        ReadStorage<'a, Is<Mount>>,
        ReadStorage<'a, Is<VolumeRider>>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, CharacterActivity>,
        WriteStorage<'a, PhysicsState>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Collider>,
    );

    const NAME: &'static str = "mount";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            id_maps,
            terrain,
            entities,
            mut controllers,
            is_riders,
            is_mounts,
            is_volume_riders,
            mut positions,
            mut velocities,
            mut orientations,
            mut character_activities,
            mut physics_states,
            bodies,
            scales,
            colliders,
        ): Self::SystemData,
    ) {
        // For each mount...
        for (entity, is_mount, body) in (&entities, &is_mounts, bodies.maybe()).join() {
            // ...find the rider...
            let Some((inputs_and_actions, rider)) =
                id_maps.uid_entity(is_mount.rider).and_then(|rider| {
                    controllers.get_mut(rider).map(|c| {
                        (
                            // Only take inputs and actions from the rider if the mount is not
                            // intelligent (TODO: expand the definition of 'intelligent').
                            if !matches!(body, Some(Body::Humanoid(_))) {
                                let actions = c
                                    .actions
                                    .extract_if(|action| match action {
                                        ControlAction::StartInput { input: i, .. }
                                        | ControlAction::CancelInput(i) => matches!(
                                            i,
                                            InputKind::Jump | InputKind::Fly | InputKind::Roll
                                        ),
                                        _ => false,
                                    })
                                    .collect();
                                Some((c.inputs.clone(), actions))
                            } else {
                                None
                            },
                            rider,
                        )
                    })
                })
            else {
                continue;
            };

            // ...apply the mount's position/ori/velocity to the rider...
            let pos = positions.get(entity).copied();
            let ori = orientations.get(entity).copied();
            let vel = velocities.get(entity).copied();
            if let (Some(pos), Some(ori), Some(vel)) = (pos, ori, vel) {
                let mounter_body = bodies.get(rider);
                let mounting_offset = body.map_or(Vec3::unit_z(), Body::mount_offset)
                    * scales.get(entity).map_or(1.0, |s| s.0)
                    + mounter_body.map_or(Vec3::zero(), Body::rider_offset)
                        * scales.get(rider).map_or(1.0, |s| s.0);
                let _ = positions.insert(rider, Pos(pos.0 + ori.to_quat() * mounting_offset));
                let _ = orientations.insert(rider, ori);
                let _ = velocities.insert(rider, vel);
            }
            // ...and apply the rider's inputs to the mount's controller
            if let Some((inputs, actions)) = inputs_and_actions
                && let Some(controller) = controllers.get_mut(entity)
            {
                controller.inputs = inputs;
                controller.actions = actions;
            }
        }

        // Since physics state isn't updated while riding we set it to default.
        // TODO: Could this be done only once when the link is first created? Has to
        // happen on both server and client.
        for (physics_state, _) in (&mut physics_states, is_riders.mask()).join() {
            *physics_state = PhysicsState::default();
        }
        for (physics_state, _) in (&mut physics_states, is_volume_riders.mask()).join() {
            *physics_state = PhysicsState::default();
        }

        // For each volume rider.
        for (entity, is_volume_rider) in (&entities, &is_volume_riders).join() {
            if let Some((mut mat, volume_ori, _)) = is_volume_rider.pos.get_block_and_transform(
                &terrain,
                &id_maps,
                |e| positions.get(e).copied().zip(orientations.get(e).copied()),
                &colliders,
            ) {
                let Some((mount_offset, mount_dir)) = is_volume_rider.block.mount_offset() else {
                    error!("Mounted on unmountable block");
                    continue;
                };

                let mount_block_ori = if let Some(ori) = is_volume_rider.block.get_ori() {
                    mat *= Mat4::identity()
                        .translated_3d(mount_offset)
                        .rotated_z(std::f32::consts::PI * 0.25 * ori as f32)
                        .translated_3d(Vec3::new(0.5, 0.5, 0.0));
                    ori
                } else {
                    mat *= Mat4::identity().translated_3d(mount_offset + Vec3::new(0.5, 0.5, 0.0));
                    0
                };

                if let Some(pos) = positions.get_mut(entity) {
                    pos.0 = mat.mul_point(Vec3::zero());
                }
                if let Some(ori) = orientations.get_mut(entity) {
                    *ori = volume_ori.rotated(
                        Ori::from_unnormalized_vec(mount_dir)
                            .unwrap_or_default()
                            .to_quat()
                            .rotated_z(std::f32::consts::PI * 0.25 * mount_block_ori as f32),
                    );
                }
            }
            let v = match is_volume_rider.pos.kind {
                common::mounting::Volume::Terrain => Vec3::zero(),
                common::mounting::Volume::Entity(uid) => {
                    if let Some(v) = id_maps.uid_entity(uid).and_then(|e| velocities.get(e)) {
                        v.0
                    } else {
                        Vec3::zero()
                    }
                },
            };
            if let Some(vel) = velocities.get_mut(entity) {
                vel.0 = v;
            }

            let inputs = controllers.get_mut(entity).map(|c| {
                let actions: Vec<_> = c
                    .actions
                    .extract_if(|action| match action {
                        ControlAction::StartInput { input: i, .. }
                        | ControlAction::CancelInput(i) => {
                            matches!(i, InputKind::Jump | InputKind::Fly | InputKind::Roll)
                        },
                        _ => false,
                    })
                    .collect();
                let inputs = c.inputs.clone();

                (actions, inputs)
            });

            if is_volume_rider.block.is_controller() {
                if let Some((actions, inputs)) = inputs {
                    if let Some(mut character_activity) = character_activities
                        .get_mut(entity)
                        .filter(|c| c.steer_dir != inputs.move_dir.y)
                    {
                        character_activity.steer_dir = inputs.move_dir.y;
                    }
                    match is_volume_rider.pos.kind {
                        common::mounting::Volume::Entity(uid) => {
                            if let Some(controller) =
                                id_maps.uid_entity(uid).and_then(|e| controllers.get_mut(e))
                            {
                                controller.inputs = inputs;
                                controller.actions = actions;
                            }
                        },
                        common::mounting::Volume::Terrain => {},
                    }
                }
            }
        }
    }
}
