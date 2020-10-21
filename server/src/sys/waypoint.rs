use super::SysTimer;
use crate::streams::{GeneralStream, GetStream};
use common::{
    comp::{Player, Pos, Waypoint, WaypointArea},
    msg::{Notification, ServerGeneral},
    span,
    state::Time,
};
use specs::{Entities, Join, Read, ReadStorage, System, Write, WriteStorage};

/// Cooldown time (in seconds) for "Waypoint Saved" notifications
const NOTIFY_TIME: f64 = 10.0;

/// This system updates player waypoints
/// TODO: Make this faster by only considering local waypoints
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, WaypointArea>,
        WriteStorage<'a, Waypoint>,
        WriteStorage<'a, GeneralStream>,
        Read<'a, Time>,
        Write<'a, SysTimer<Self>>,
    );

    fn run(
        &mut self,
        (
            entities,
            positions,
            players,
            waypoint_areas,
            mut waypoints,
            mut general_streams,
            time,
            mut timer,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "waypoint::Sys::run");
        timer.start();

        for (entity, player_pos, _, general_stream) in
            (&entities, &positions, &players, &mut general_streams).join()
        {
            for (waypoint_pos, waypoint_area) in (&positions, &waypoint_areas).join() {
                if player_pos.0.distance_squared(waypoint_pos.0) < waypoint_area.radius().powi(2) {
                    if let Ok(wp_old) = waypoints.insert(entity, Waypoint::new(player_pos.0, *time))
                    {
                        if wp_old.map_or(true, |w| w.elapsed(*time) > NOTIFY_TIME) {
                            general_stream.send_fallible(ServerGeneral::Notification(
                                Notification::WaypointSaved,
                            ));
                        }
                    }
                }
            }
        }

        timer.end();
    }
}
