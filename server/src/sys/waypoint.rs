use crate::client::Client;
use common::{
    comp::{PhysicsState, Player, Pos, Vel, Waypoint, WaypointArea},
    resources::Time,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{Notification, ServerGeneral};
use specs::{Entities, Join, Read, ReadStorage, WriteStorage};

/// Cooldown time (in seconds) for "Waypoint Saved" notifications
const NOTIFY_TIME: f64 = 10.0;

/// This system updates player waypoints
/// TODO: Make this faster by only considering local waypoints
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, WaypointArea>,
        WriteStorage<'a, Waypoint>,
        ReadStorage<'a, Client>,
        Read<'a, Time>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Vel>,
    );

    const NAME: &'static str = "waypoint";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            positions,
            players,
            waypoint_areas,
            mut waypoints,
            clients,
            time,
            physics_states,
            velocities,
        ): Self::SystemData,
    ) {
        for (entity, player_pos, _, client, physics, velocity) in (
            &entities,
            &positions,
            &players,
            &clients,
            physics_states.maybe(),
            &velocities,
        )
            .join()
        {
            if physics.map_or(true, |ps| ps.on_ground.is_some()) && velocity.0.z >= 0.0 {
                for (waypoint_pos, waypoint_area) in (&positions, &waypoint_areas).join() {
                    if player_pos.0.distance_squared(waypoint_pos.0)
                        < waypoint_area.radius().powi(2)
                    {
                        if let Ok(wp_old) =
                            waypoints.insert(entity, Waypoint::new(player_pos.0, *time))
                        {
                            if wp_old.map_or(true, |w| w.elapsed(*time) > NOTIFY_TIME) {
                                client.send_fallible(ServerGeneral::Notification(
                                    Notification::WaypointSaved,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}
