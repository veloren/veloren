use common::{
    comp::{Body, Collider, InputKind, Mass, Ori, Pos, Scale, Vel},
    link::Is,
    resources::DeltaTime,
    tether::{Follower, Leader},
    uid::UidAllocator,
    util::Dir,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, Read, ReadExpect, ReadStorage, WriteStorage,
};
use tracing::error;
use vek::*;

/// This system is responsible for controlling mounts
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, UidAllocator>,
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Is<Leader>>,
        ReadStorage<'a, Is<Follower>>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Collider>,
        ReadStorage<'a, Mass>,
    );

    const NAME: &'static str = "tether";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            uid_allocator,
            entities,
            dt,
            is_leaders,
            is_followers,
            mut positions,
            mut velocities,
            mut orientations,
            bodies,
            scales,
            colliders,
            masses,
        ): Self::SystemData,
    ) {
        for (follower, is_follower, follower_body) in
            (&entities, &is_followers, bodies.maybe()).join()
        {
            let Some(leader) = uid_allocator
                .retrieve_entity_internal(is_follower.leader.id())
            else { continue };

            let (Some(leader_pos), Some(follower_pos)) = (
                positions.get(leader).copied(),
                positions.get(follower).copied(),
            ) else { continue };

            let (Some(leader_mass), Some(follower_mass)) = (
                masses.get(leader).copied(),
                masses.get(follower).copied(),
            ) else { continue };

            if velocities.contains(follower) && velocities.contains(leader) {
                let tether_offset = orientations
                    .get(follower)
                    .map(|ori| {
                        ori.to_quat() * follower_body.map(|b| b.tether_offset()).unwrap_or_default()
                    })
                    .unwrap_or_default();
                let tether_pos = follower_pos.0 + tether_offset;
                let pull_factor = (leader_pos.0.distance(tether_pos) - is_follower.tether_length)
                    .clamp(0.0, 1.0)
                    .powf(2.0);
                let strength = pull_factor * 50000.0;
                let pull_dir = (leader_pos.0 - follower_pos.0)
                    .try_normalized()
                    .unwrap_or(Vec3::unit_y());
                let impulse = pull_dir * strength * dt.0;

                // Can't fail
                velocities.get_mut(follower).unwrap().0 += impulse / follower_mass.0;
                velocities.get_mut(leader).unwrap().0 -= impulse / leader_mass.0;

                if let Some(follower_ori) = orientations.get_mut(follower) {
                    let turn_strength = pull_factor.min(0.2)
                        // * (tether_offset.magnitude() - tether_offset.dot(pull_dir).abs())
                        * 50.0;
                    let target_ori = follower_ori.yawed_towards(Dir::new(pull_dir));
                    *follower_ori = follower_ori.slerped_towards(target_ori, turn_strength * dt.0);
                }
            }
        }
    }
}
