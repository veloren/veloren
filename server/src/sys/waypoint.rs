use super::SysTimer;
use crate::client::Client;
use common::{
    comp::{Player, Pos, Waypoint, WaypointArea},
    msg::{Notification, ServerMsg},
};
use specs::{Entities, Join, ReadStorage, System, Write, WriteStorage};

const NOTIFY_DISTANCE: f32 = 10.0;

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
        WriteStorage<'a, Client>,
        Write<'a, SysTimer<Self>>,
    );

    fn run(
        &mut self,
        (entities, positions, players, waypoint_areas, mut waypoints, mut clients, mut timer): Self::SystemData,
    ) {
        timer.start();

        for (entity, player_pos, _, client) in
            (&entities, &positions, &players, &mut clients).join()
        {
            for (waypoint_pos, waypoint_area) in (&positions, &waypoint_areas).join() {
                if player_pos.0.distance_squared(waypoint_pos.0) < waypoint_area.radius().powi(2) {
                    if let Some(wp) = waypoints.get(entity) {
                        if player_pos.0.distance_squared(wp.get_pos()) > NOTIFY_DISTANCE.powi(2) {
                            client
                                .postbox
                                .send_message(ServerMsg::Notification(Notification::WaypointSaved));
                        }
                    }
                    let _ = waypoints.insert(entity, Waypoint::new(player_pos.0));
                }
            }
        }

        timer.end();
    }
}
