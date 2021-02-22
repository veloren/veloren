use common::{
    combat::AttackerInfo,
    comp::{Body, CharacterState, Energy, Group, Health, Inventory, Melee, Ori, Pos, Scale},
    event::{EventBus, ServerEvent},
    metrics::SysMetrics,
    span,
    uid::Uid,
    util::Dir,
    GroupTarget,
};
use specs::{
    shred::ResourceId, Entities, Join, Read, ReadExpect, ReadStorage, System, SystemData, World,
    WriteStorage,
};
use vek::*;

#[derive(SystemData)]
pub struct ImmutableData<'a> {
    entities: Entities<'a>,
    uids: ReadStorage<'a, Uid>,
    positions: ReadStorage<'a, Pos>,
    orientations: ReadStorage<'a, Ori>,
    scales: ReadStorage<'a, Scale>,
    bodies: ReadStorage<'a, Body>,
    healths: ReadStorage<'a, Health>,
    energies: ReadStorage<'a, Energy>,
    inventories: ReadStorage<'a, Inventory>,
    groups: ReadStorage<'a, Group>,
    char_states: ReadStorage<'a, CharacterState>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    metrics: ReadExpect<'a, SysMetrics>,
}

/// This system is responsible for handling accepted inputs like moving or
/// attacking
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (ImmutableData<'a>, WriteStorage<'a, Melee>);

    fn run(&mut self, (immutable_data, mut melee_attacks): Self::SystemData) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "melee::Sys::run");
        let mut server_emitter = immutable_data.server_bus.emitter();
        // Attacks
        for (attacker, uid, pos, ori, melee_attack, body) in (
            &immutable_data.entities,
            &immutable_data.uids,
            &immutable_data.positions,
            &immutable_data.orientations,
            &mut melee_attacks,
            &immutable_data.bodies,
        )
            .join()
        {
            if melee_attack.applied {
                continue;
            }
            melee_attack.applied = true;

            // Go through all other entities
            for (target, pos_b, health_b, body_b) in (
                &immutable_data.entities,
                &immutable_data.positions,
                &immutable_data.healths,
                &immutable_data.bodies,
            )
                .join()
            {
                let look_dir = *ori.look_dir();

                // 2D versions
                let pos2 = Vec2::from(pos.0);
                let pos_b2 = Vec2::<f32>::from(pos_b.0);
                let ori2 = Vec2::from(look_dir);

                // Scales
                let scale = immutable_data.scales.get(attacker).map_or(1.0, |s| s.0);
                let scale_b = immutable_data.scales.get(target).map_or(1.0, |s| s.0);
                let rad = body.radius() * scale;
                let rad_b = body_b.radius() * scale_b;

                // Check if entity is dodging
                let is_dodge = immutable_data
                    .char_states
                    .get(target)
                    .map_or(false, |c_s| c_s.is_melee_dodge());

                // Check if it is a hit
                if attacker != target
                    && !health_b.is_dead
                    // Spherical wedge shaped attack field
                    && pos.0.distance_squared(pos_b.0) < (rad + rad_b + scale * melee_attack.range).powi(2)
                    && ori2.angle_between(pos_b2 - pos2) < melee_attack.max_angle + (rad_b / pos2.distance(pos_b2)).atan()
                {
                    // See if entities are in the same group
                    let same_group = immutable_data
                        .groups
                        .get(attacker)
                        .map(|group_a| Some(group_a) == immutable_data.groups.get(target))
                        .unwrap_or(false);

                    let target_group = if same_group {
                        GroupTarget::InGroup
                    } else {
                        GroupTarget::OutOfGroup
                    };

                    let dir = Dir::new((pos_b.0 - pos.0).try_normalized().unwrap_or(look_dir));

                    let attacker_info = Some(AttackerInfo {
                        entity: attacker,
                        uid: *uid,
                        energy: immutable_data.energies.get(attacker),
                    });

                    melee_attack.attack.apply_attack(
                        target_group,
                        attacker_info,
                        target,
                        immutable_data.inventories.get(target),
                        dir,
                        is_dodge,
                        1.0,
                        |e| server_emitter.emit(e),
                    );

                    melee_attack.hit_count += 1;
                }
            }
        }
        immutable_data.metrics.melee_ns.store(
            start_time.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
