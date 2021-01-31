use common::{
    comp::{group, Body, CharacterState, Energy, Health, Inventory, MeleeAttack, Ori, Pos, Scale},
    event::{EventBus, LocalEvent, ServerEvent},
    metrics::SysMetrics,
    span,
    uid::Uid,
    util::Dir,
    GroupTarget,
};
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

/// This system is responsible for handling accepted inputs like moving or
/// attacking
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        ReadExpect<'a, SysMetrics>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Health>,
        ReadStorage<'a, Energy>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, group::Group>,
        WriteStorage<'a, MeleeAttack>,
        ReadStorage<'a, CharacterState>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_bus,
            local_bus,
            sys_metrics,
            uids,
            positions,
            orientations,
            scales,
            bodies,
            healths,
            energies,
            inventories,
            groups,
            mut attacking_storage,
            char_states,
        ): Self::SystemData,
    ) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "melee::Sys::run");
        let mut server_emitter = server_bus.emitter();
        let _local_emitter = local_bus.emitter();
        // Attacks
        for (entity, uid, pos, ori, scale_maybe, attack, body) in (
            &entities,
            &uids,
            &positions,
            &orientations,
            scales.maybe(),
            &mut attacking_storage,
            &bodies,
        )
            .join()
        {
            if attack.applied {
                continue;
            }
            attack.applied = true;

            // Go through all other entities
            for (
                b,
                pos_b,
                scale_b_maybe,
                health_b,
                body_b,
                char_state_b_maybe,
                inventory_b_maybe,
            ) in (
                &entities,
                &positions,
                scales.maybe(),
                &healths,
                &bodies,
                char_states.maybe(),
                inventories.maybe(),
            )
                .join()
            {
                // 2D versions
                let pos2 = Vec2::from(pos.0);
                let pos_b2 = Vec2::<f32>::from(pos_b.0);
                let ori2 = Vec2::from(*ori.0);

                // Scales
                let scale = scale_maybe.map_or(1.0, |s| s.0);
                let scale_b = scale_b_maybe.map_or(1.0, |s| s.0);
                let rad = body.radius() * scale;
                let rad_b = body_b.radius() * scale_b;

                // Check if entity is dodging
                let is_dodge = char_state_b_maybe.map_or(false, |c_s| c_s.is_melee_dodge());

                // Check if it is a hit
                if entity != b
                    && !health_b.is_dead
                    // Spherical wedge shaped attack field
                    && pos.0.distance_squared(pos_b.0) < (rad + rad_b + scale * attack.range).powi(2)
                    && ori2.angle_between(pos_b2 - pos2) < attack.max_angle + (rad_b / pos2.distance(pos_b2)).atan()
                {
                    // See if entities are in the same group
                    let same_group = groups
                        .get(entity)
                        .map(|group_a| Some(group_a) == groups.get(b))
                        .unwrap_or(false);

                    let target_group = if same_group {
                        GroupTarget::InGroup
                    } else {
                        GroupTarget::OutOfGroup
                    };

                    let dir = Dir::new((pos_b.0 - pos.0).try_normalized().unwrap_or(*ori.0));

                    let server_events = attack.attack.apply_attack(
                        target_group,
                        entity,
                        b,
                        inventory_b_maybe,
                        *uid,
                        energies.get(entity),
                        dir,
                        is_dodge,
                        1.0,
                    );

                    if !server_events.is_empty() {
                        attack.hit_count += 1;
                    }

                    for event in server_events {
                        server_emitter.emit(event);
                    }
                }
            }
        }
        sys_metrics.melee_ns.store(
            start_time.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
