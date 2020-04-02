use super::SysTimer;
use common::comp::{Player, Pos, Waypoint, WaypointArea};
use specs::{Entities, Join, ReadStorage, System, Write, WriteStorage};

/// This system updates player waypoints
/// TODO: Make this faster by only considering local waypoints
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, WaypointArea>,
        WriteStorage<'a, Waypoint>,
        Write<'a, SysTimer<Self>>,
    );

    fn run(
        &mut self,
        (entities, positions, players, waypoint_areas, mut waypoints, mut timer): Self::SystemData,
    ) {
        timer.start();

        for (entity, player_pos, _) in (&entities, &positions, &players).join() {
            for (waypoint_pos, waypoint_area) in (&positions, &waypoint_areas).join() {
                if player_pos.0.distance_squared(waypoint_pos.0) < waypoint_area.radius().powf(2.0)
                {
                    let _ = waypoints.insert(entity, Waypoint::new(player_pos.0));
                }
            }
        }

        timer.end();
    }
}
