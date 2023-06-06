use common::{
    comp::{Body, Mass, Ori, Pos, Scale, Vel},
    link::Is,
    resources::DeltaTime,
    tether::Follower,
    uid::IdMaps,
    util::Dir,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, Read, ReadStorage, WriteStorage};
use vek::*;

/// This system is responsible for controlling mounts
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, IdMaps>,
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Is<Follower>>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Mass>,
    );

    const NAME: &'static str = "tether";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            id_maps,
            entities,
            dt,
            is_followers,
            positions,
            mut velocities,
            mut orientations,
            bodies,
            scales,
            masses,
        ): Self::SystemData,
    ) {
        for (follower, is_follower, follower_body, follower_scale) in
            (&entities, &is_followers, bodies.maybe(), scales.maybe()).join()
        {
            let Some(leader) = id_maps.uid_entity(is_follower.leader) else { continue };

            let (Some(leader_pos), Some(follower_pos)) = (
                positions.get(leader).copied(),
                positions.get(follower).copied(),
            ) else { continue };

            let (Some(leader_mass), Some(follower_mass)) = (
                masses.get(leader).copied(),
                masses.get(follower).copied(),
            ) else { continue };

            if velocities.contains(follower) && velocities.contains(leader) {
                let attach_offset = orientations
                    .get(leader)
                    .map(|ori| {
                        ori.to_quat()
                            * bodies
                                .get(leader)
                                .map(|b| {
                                    b.tether_offset_leader()
                                        * scales.get(leader).copied().unwrap_or(Scale(1.0)).0
                                })
                                .unwrap_or_default()
                    })
                    .unwrap_or_default();
                let attach_pos = leader_pos.0 + attach_offset;

                let tether_offset = orientations
                    .get(follower)
                    .map(|ori| {
                        ori.to_quat()
                            * follower_body
                                .map(|b| {
                                    b.tether_offset_follower()
                                        * follower_scale.copied().unwrap_or(Scale(1.0)).0
                                })
                                .unwrap_or_default()
                    })
                    .unwrap_or_default();
                let tether_pos = follower_pos.0 + tether_offset;
                let pull_factor =
                    (attach_pos.distance(tether_pos) - is_follower.tether_length).max(0.0);
                let strength = pull_factor * 50000.0;
                let pull_dir = (leader_pos.0 - follower_pos.0)
                    .try_normalized()
                    .unwrap_or(Vec3::unit_y());
                let impulse = pull_dir * strength * dt.0;

                // Can't fail
                velocities.get_mut(follower).unwrap().0 += impulse / follower_mass.0;
                velocities.get_mut(leader).unwrap().0 -= impulse / leader_mass.0;

                if let Some(follower_ori) = orientations.get_mut(follower) {
                    let turn_strength = pull_factor
                        * (tether_offset.magnitude() * tether_pos.distance(attach_pos)
                            - tether_offset.dot(attach_pos - tether_pos).abs())
                        // TODO: proper moment of inertia
                        * 500.0
                        / follower_mass.0;
                    // TODO: Should consider the offset
                    let target_ori = follower_ori.yawed_towards(Dir::new(pull_dir));
                    *follower_ori = follower_ori.slerped_towards(target_ori, turn_strength * dt.0);
                }

                if let Some(leader_ori) = orientations.get_mut(leader) {
                    let turn_strength = pull_factor
                        * (attach_offset.magnitude() * tether_pos.distance(attach_pos)
                            - attach_offset.dot(tether_pos - attach_pos).abs())
                        // TODO: proper moment of inertia
                        * 500.0
                        / leader_mass.0;
                    // TODO: Should consider the offset
                    let target_ori = leader_ori.yawed_towards(Dir::new(pull_dir));
                    *leader_ori = leader_ori.slerped_towards(target_ori, turn_strength * dt.0);
                }
            }
        }
    }
}
