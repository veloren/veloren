use common::{
    combat::RiderEffects,
    comp::{
        Body, Buff, BuffCategory, BuffChange, Buffs, CharacterActivity, Collider, ControlAction,
        Controller, InputKind, Mass, Ori, PhysicsState, Pos, Scale, Stats, Vel, buff::DestInfo,
    },
    event::{BuffEvent, EmitExt},
    event_emitters,
    link::Is,
    mounting::{Mount, Rider, VolumeRider},
    resources::Time,
    terrain::TerrainGrid,
    uid::IdMaps,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, WriteStorage};
use vek::*;

event_emitters! {
    struct Events[EventEmitters] {
        buff: BuffEvent,
    }
}

/// This system is responsible for controlling mounts
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, IdMaps>,
        Read<'a, Time>,
        ReadExpect<'a, TerrainGrid>,
        Events<'a>,
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
        ReadStorage<'a, Buffs>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Mass>,
        ReadStorage<'a, RiderEffects>,
    );

    const NAME: &'static str = "mount";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            id_maps,
            time,
            terrain,
            events,
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
            buffs,
            stats,
            masses,
            rider_effects,
        ): Self::SystemData,
    ) {
        let mut emitters = events.get_emitters();
        // For each mount...
        for (entity, is_mount, body, rider_effects) in
            (&entities, &is_mounts, bodies.maybe(), rider_effects.maybe()).join()
        {
            let Some(rider_entity) = id_maps.uid_entity(is_mount.rider) else {
                continue;
            };

            // Rider effects from mount.
            if let Some(rider_effects) = rider_effects
                && let Some(target_buffs) = buffs.get(rider_entity)
            {
                for effect in rider_effects.0.iter() {
                    let emit_buff = !target_buffs.buffs.iter().any(|(_, buff)| {
                        buff.cat_ids.iter()
                            .any(|cat_id| matches!(cat_id, BuffCategory::FromLink(link) if link.is_link(is_mount.get_link())))
                            && buff.kind == effect.kind && buff.data.strength >= effect.data.strength
                    });

                    if emit_buff {
                        let dest_info = DestInfo {
                            stats: stats.get(rider_entity),
                            mass: masses.get(rider_entity),
                        };
                        let mut cat_ids = effect.cat_ids.clone();
                        cat_ids.push(BuffCategory::FromLink(
                            is_mount.get_link().downgrade().into_dyn(),
                        ));

                        emitters.emit(BuffEvent {
                            entity: rider_entity,
                            buff_change: BuffChange::Add(Buff::new(
                                effect.kind,
                                effect.data,
                                cat_ids,
                                common::comp::BuffSource::Character { by: is_mount.mount },
                                *time,
                                dest_info,
                                masses.get(entity),
                            )),
                        });
                    }
                }
            }
            // ...find the rider...
            let Some(inputs_and_actions) = controllers.get_mut(rider_entity).map(|c| {
                // Only take inputs and actions from the rider if the mount is not
                // intelligent (TODO: expand the definition of 'intelligent').
                if body.is_some_and(|b| !b.has_free_will()) {
                    let actions = c
                        .actions
                        .extract_if(.., |action| match action {
                            ControlAction::StartInput { input: i, .. }
                            | ControlAction::CancelInput { input: i } => {
                                matches!(i, InputKind::Jump | InputKind::Fly | InputKind::Roll)
                            },
                            _ => false,
                        })
                        .collect();
                    Some((c.inputs.clone(), actions))
                } else {
                    None
                }
            }) else {
                continue;
            };

            // ...apply the mount's position/ori/velocity to the rider...
            let pos = positions.get(entity).copied();
            let ori = orientations.get(entity).copied();
            let vel = velocities.get(entity).copied();
            if let (Some(pos), Some(ori), Some(vel)) = (pos, ori, vel) {
                let mounter_body = bodies.get(rider_entity);
                let mounting_offset = body.map_or(Vec3::unit_z(), Body::mount_offset)
                    * scales.get(entity).map_or(1.0, |s| s.0)
                    + mounter_body.map_or(Vec3::zero(), Body::rider_offset)
                        * scales.get(rider_entity).map_or(1.0, |s| s.0);
                let _ =
                    positions.insert(rider_entity, Pos(pos.0 + ori.to_quat() * mounting_offset));
                let _ = orientations.insert(rider_entity, ori);
                let _ = velocities.insert(rider_entity, vel);
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
        for (physics_state, _) in (
            &mut physics_states,
            is_riders.mask() | is_volume_riders.mask(),
        )
            .join()
        {
            *physics_state = PhysicsState::default();
        }

        // For each volume rider.
        for (entity, is_volume_rider) in (&entities, &is_volume_riders).join() {
            if let Some((mat, _)) = is_volume_rider.pos.get_mount_mat(
                &terrain,
                &id_maps,
                |e| positions.get(e).copied().zip(orientations.get(e).copied()),
                &colliders,
            ) {
                if let Some(pos) = positions.get_mut(entity) {
                    pos.0 = mat.mul_point(Vec3::zero());
                }
                if let Some(ori) = orientations.get_mut(entity) {
                    *ori = Ori::from_unnormalized_vec(mat.mul_direction(Vec3::unit_y()))
                        .unwrap_or_default();
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

            // Check if the volume has buffs if they do apply them to the rider via a
            // BuffEvent

            // TODO: This is code copy of the mounting effects. We can probably consolidate
            // at some point.
            if let Some(target_buffs) = buffs.get(entity)
                && let Some(block_buffs) = is_volume_rider.block.mount_buffs()
            {
                for effect in block_buffs.iter() {
                    let emit_buff = !target_buffs.buffs.iter().any(|(_, buff)| {
                        buff.cat_ids.iter()
                            .any(|cat_id| matches!(cat_id, BuffCategory::FromLink(link) if link.is_link(is_volume_rider.get_link())))
                            && buff.kind == effect.kind && buff.data.strength >= effect.data.strength
                    });

                    if emit_buff {
                        let dest_info = DestInfo {
                            stats: stats.get(entity),
                            mass: masses.get(entity),
                        };
                        let mut cat_ids = effect.cat_ids.clone();
                        cat_ids.push(BuffCategory::FromLink(
                            is_volume_rider.get_link().downgrade().into_dyn(),
                        ));

                        emitters.emit(BuffEvent {
                            entity,
                            buff_change: BuffChange::Add(Buff::new(
                                effect.kind,
                                effect.data,
                                cat_ids,
                                common::comp::BuffSource::Block,
                                *time,
                                dest_info,
                                masses.get(entity),
                            )),
                        });
                    }
                }
            }

            let inputs = controllers.get_mut(entity).map(|c| {
                let actions: Vec<_> = c
                    .actions
                    .extract_if(.., |action| match action {
                        ControlAction::StartInput { input: i, .. }
                        | ControlAction::CancelInput { input: i } => {
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
