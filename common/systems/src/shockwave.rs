use common::{
    combat::{self, AttackOptions, AttackerInfo, TargetInfo},
    comp::{
        agent::{Sound, SoundKind},
        shockwave::ShockwaveDodgeable,
        Alignment, Body, Buffs, CharacterState, Combo, Energy, Group, Health, Inventory, Ori,
        PhysicsState, Player, Pos, Scale, Shockwave, ShockwaveHitEntities, Stats,
    },
    event::{
        BuffEvent, ComboChangeEvent, DeleteEvent, EmitExt, EnergyChangeEvent,
        EntityAttackedHookEvent, EventBus, HealthChangeEvent, KnockbackEvent, MineBlockEvent,
        ParryHookEvent, PoiseChangeEvent, SoundEvent,
    },
    event_emitters,
    outcome::Outcome,
    resources::{DeltaTime, Time},
    uid::{IdMaps, Uid},
    util::Dir,
    GroupTarget,
};
use common_ecs::{Job, Origin, Phase, System};
use rand::Rng;
use specs::{shred, Entities, Join, LendJoin, Read, ReadStorage, SystemData, WriteStorage};
use vek::*;

event_emitters! {
    struct Events[Emitters] {
        health_change: HealthChangeEvent,
        energy_change: EnergyChangeEvent,
        poise_change: PoiseChangeEvent,
        sound: SoundEvent,
        mine_block: MineBlockEvent,
        parry_hook: ParryHookEvent,
        kockback: KnockbackEvent,
        entity_attack_hoow: EntityAttackedHookEvent,
        combo_change: ComboChangeEvent,
        buff: BuffEvent,
        delete: DeleteEvent,
    }
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    events: Events<'a>,
    time: Read<'a, Time>,
    players: ReadStorage<'a, Player>,
    dt: Read<'a, DeltaTime>,
    id_maps: Read<'a, IdMaps>,
    uids: ReadStorage<'a, Uid>,
    positions: ReadStorage<'a, Pos>,
    orientations: ReadStorage<'a, Ori>,
    alignments: ReadStorage<'a, Alignment>,
    scales: ReadStorage<'a, Scale>,
    bodies: ReadStorage<'a, Body>,
    healths: ReadStorage<'a, Health>,
    inventories: ReadStorage<'a, Inventory>,
    groups: ReadStorage<'a, Group>,
    physics_states: ReadStorage<'a, PhysicsState>,
    energies: ReadStorage<'a, Energy>,
    stats: ReadStorage<'a, Stats>,
    combos: ReadStorage<'a, Combo>,
    character_states: ReadStorage<'a, CharacterState>,
    buffs: ReadStorage<'a, Buffs>,
}

/// This system is responsible for handling accepted inputs like moving or
/// attacking
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, Shockwave>,
        WriteStorage<'a, ShockwaveHitEntities>,
        Read<'a, EventBus<Outcome>>,
    );

    const NAME: &'static str = "shockwave";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (read_data, mut shockwaves, mut shockwave_hit_lists, outcomes): Self::SystemData,
    ) {
        let mut emitters = read_data.events.get_emitters();
        let mut outcomes_emitter = outcomes.emitter();
        let mut rng = rand::thread_rng();

        let time = read_data.time.0;
        let dt = read_data.dt.0;

        // Shockwaves
        for (entity, pos, ori, shockwave, shockwave_hit_list) in (
            &read_data.entities,
            &read_data.positions,
            &read_data.orientations,
            &shockwaves,
            &mut shockwave_hit_lists,
        )
            .join()
        {
            let creation_time = match shockwave.creation {
                Some(time) => time,
                // Skip newly created shockwaves
                None => continue,
            };

            let end_time = creation_time + shockwave.duration.as_secs_f64();

            let shockwave_owner = shockwave
                .owner
                .and_then(|uid| read_data.id_maps.uid_entity(uid));

            if rng.gen_bool(0.05) {
                emitters.emit(SoundEvent {
                    sound: Sound::new(SoundKind::Shockwave, pos.0, 40.0, time),
                });
            }

            // If shockwave is out of time emit destroy event but still continue since it
            // may have traveled and produced effects a bit before reaching it's end point
            if time > end_time {
                emitters.emit(DeleteEvent(entity));
                continue;
            }

            // Determine area that was covered by the shockwave in the last tick
            let time_since_creation = (time - creation_time) as f32;
            let frame_start_dist = (shockwave.speed * (time_since_creation - dt)).max(0.0);
            let frame_end_dist = (shockwave.speed * time_since_creation).max(frame_start_dist);
            let pos2 = Vec2::from(pos.0);
            let look_dir = ori.look_dir();

            // From one frame to the next a shockwave travels over a strip of an arc
            // This is used for collision detection
            let arc_strip = ArcStrip {
                origin: pos2,
                // TODO: make sure this is not Vec2::new(0.0, 0.0)
                dir: look_dir.xy(),
                angle: shockwave.angle,
                start: frame_start_dist,
                end: frame_end_dist,
            };

            // Group to ignore collisions with
            // Might make this more nuanced if shockwaves are used for non damage effects
            let group = shockwave_owner.and_then(|e| read_data.groups.get(e));

            // Go through all other effectable entities
            for (target, uid_b, pos_b, health_b, body_b, physics_state_b) in (
                &read_data.entities,
                &read_data.uids,
                &read_data.positions,
                &read_data.healths,
                &read_data.bodies,
                &read_data.physics_states,
            )
                .join()
            {
                // Check to see if entity has already been hit
                if shockwave_hit_list
                    .hit_entities
                    .iter()
                    .any(|&uid| uid == *uid_b)
                {
                    continue;
                }

                // 2D versions
                let pos_b2 = pos_b.0.xy();

                // Scales
                let scale_b = read_data.scales.get(target).map_or(1.0, |s| s.0);
                // TODO: use Capsule Prism instead of Cylinder
                let rad_b = body_b.max_radius() * scale_b;

                // Angle checks
                let pos_b_ground = Vec3::new(pos_b.0.x, pos_b.0.y, pos.0.z);
                let max_angle = shockwave.vertical_angle.to_radians();

                // See if entities are in the same group
                let same_group = group
                    .map(|group_a| Some(group_a) == read_data.groups.get(target))
                    .unwrap_or(Some(*uid_b) == shockwave.owner);

                let target_group = if same_group {
                    GroupTarget::InGroup
                } else {
                    GroupTarget::OutOfGroup
                };

                // Check if it is a hit
                let hit = entity != target
                    && !health_b.is_dead
                    && (pos_b.0 - pos.0).magnitude() < frame_end_dist + rad_b
                    // Collision shapes
                    && {
                        // TODO: write code to collide rect with the arc strip so that we can do
                        // more complete collision detection for rapidly moving entities
                        arc_strip.collides_with_circle(Disk::new(pos_b2, rad_b))
                    }
                    && (pos_b_ground - pos.0).angle_between(pos_b.0 - pos.0) < max_angle
                    && match shockwave.dodgeable {
                        ShockwaveDodgeable::Roll | ShockwaveDodgeable::No => true,
                        ShockwaveDodgeable::Jump => physics_state_b.on_ground.is_some()
                    };

                if hit {
                    let dir = Dir::from_unnormalized(pos_b.0 - pos.0).unwrap_or(look_dir);

                    let attacker_info =
                        shockwave_owner
                            .zip(shockwave.owner)
                            .map(|(entity, uid)| AttackerInfo {
                                entity,
                                uid,
                                group: read_data.groups.get(entity),
                                energy: read_data.energies.get(entity),
                                combo: read_data.combos.get(entity),
                                inventory: read_data.inventories.get(entity),
                                stats: read_data.stats.get(entity),
                            });

                    let target_info = TargetInfo {
                        entity: target,
                        uid: *uid_b,
                        inventory: read_data.inventories.get(target),
                        stats: read_data.stats.get(target),
                        health: read_data.healths.get(target),
                        pos: pos_b.0,
                        ori: read_data.orientations.get(target),
                        char_state: read_data.character_states.get(target),
                        energy: read_data.energies.get(target),
                        buffs: read_data.buffs.get(target),
                    };

                    let target_dodging = read_data
                        .character_states
                        .get(target)
                        .and_then(|cs| cs.attack_immunities())
                        .map_or(false, |i| match shockwave.dodgeable {
                            ShockwaveDodgeable::Roll => i.air_shockwaves,
                            ShockwaveDodgeable::Jump => i.ground_shockwaves,
                            ShockwaveDodgeable::No => false,
                        });
                    // PvP check
                    let may_harm = combat::may_harm(
                        &read_data.alignments,
                        &read_data.players,
                        &read_data.id_maps,
                        shockwave_owner,
                        target,
                    );
                    // Shockwaves aren't precise, and thus cannot be a precise strike
                    let precision_mult = None;
                    let attack_options = AttackOptions {
                        target_dodging,
                        may_harm,
                        target_group,
                        precision_mult,
                    };

                    shockwave.properties.attack.apply_attack(
                        attacker_info,
                        &target_info,
                        dir,
                        attack_options,
                        1.0,
                        shockwave.dodgeable.to_attack_source(),
                        *read_data.time,
                        &mut emitters,
                        |o| outcomes_emitter.emit(o),
                        &mut rng,
                        0,
                    );

                    shockwave_hit_list.hit_entities.push(*uid_b);
                }
            }
        }

        // Set start time on new shockwaves
        // This change doesn't need to be recorded as it is not sent to the client
        shockwaves.set_event_emission(false);
        (&mut shockwaves).lend_join().for_each(|mut shockwave| {
            if shockwave.creation.is_none() {
                shockwave.creation = Some(time);
            }
        });
        shockwaves.set_event_emission(true);
    }
}

#[derive(Clone, Copy)]
struct ArcStrip {
    origin: Vec2<f32>,
    /// Normalizable direction
    dir: Vec2<f32>,
    /// Angle in degrees
    angle: f32,
    /// Start radius
    start: f32,
    /// End radius
    end: f32,
}

impl ArcStrip {
    fn collides_with_circle(self, d: Disk<f32, f32>) -> bool {
        // Quit if aabb's don't collide
        if (self.origin.x - d.center.x).abs() > self.end + d.radius
            || (self.origin.y - d.center.y).abs() > self.end + d.radius
        {
            return false;
        }

        let dist = self.origin.distance(d.center);
        let half_angle = self.angle.to_radians() / 2.0;

        if dist > self.end + d.radius || dist + d.radius < self.start {
            // Completely inside or outside full ring
            return false;
        }

        let inside_edge = Disk::new(self.origin, self.start);
        let outside_edge = Disk::new(self.origin, self.end);
        let inner_corner_in_circle = || {
            let midpoint = self.dir.normalized() * self.start;
            d.contains_point(midpoint.rotated_z(half_angle) + self.origin)
                || d.contains_point(midpoint.rotated_z(-half_angle) + self.origin)
        };
        let arc_segment_in_circle = || {
            let midpoint = self.dir.normalized();
            let segment_in_circle = |angle| {
                let dir = midpoint.rotated_z(angle);
                let side = LineSegment2 {
                    start: dir * self.start + self.origin,
                    end: dir * self.end + self.origin,
                };
                d.contains_point(side.projected_point(d.center))
            };
            segment_in_circle(half_angle) || segment_in_circle(-half_angle)
        };

        if dist > self.end {
            // Circle center is outside ring
            // Check intersection with line segments
            arc_segment_in_circle() || {
                // Check angle of intersection points on outside edge of ring
                let (p1, p2) = intersection_points(outside_edge, d, dist);
                self.dir.angle_between(p1 - self.origin) < half_angle
                    || self.dir.angle_between(p2 - self.origin) < half_angle
            }
        } else if dist < self.start {
            // Circle center is inside ring
            // Check angle of intersection points on inside edge of ring
            // Check if circle contains one of the inner points of the arc
            inner_corner_in_circle()
                || (
                    // Check that the circles aren't identical
                    inside_edge != d && {
                        let (p1, p2) = intersection_points(inside_edge, d, dist);
                        self.dir.angle_between(p1 - self.origin) < half_angle
                            || self.dir.angle_between(p2 - self.origin) < half_angle
                    }
                )
        } else if d.radius > dist {
            // Circle center inside ring
            // but center of ring is inside the circle so we can't calculate the angle
            inner_corner_in_circle()
        } else {
            // Circle center inside ring
            // Calculate extra angle to account for circle radius
            let extra_angle = (d.radius / dist).asin();
            self.dir.angle_between(d.center - self.origin) < half_angle + extra_angle
        }
    }
}

// Assumes an intersection is occuring at 2 points
// Uses precalculated distance
// https://www.xarg.org/2016/07/calculate-the-intersection-points-of-two-circles/
fn intersection_points(
    disk1: Disk<f32, f32>,
    disk2: Disk<f32, f32>,
    dist: f32,
) -> (Vec2<f32>, Vec2<f32>) {
    let e = (disk2.center - disk1.center) / dist;

    let x = (disk1.radius.powi(2) - disk2.radius.powi(2) + dist.powi(2)) / (2.0 * dist);
    let y = (disk1.radius.powi(2) - x.powi(2)).sqrt();

    let pxe = disk1.center + x * e;
    let eyx = e.yx();

    let p1 = pxe + Vec2::new(-y, y) * eyx;
    let p2 = pxe + Vec2::new(y, -y) * eyx;

    (p1, p2)
}
