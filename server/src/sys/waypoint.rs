use common::comp::{Player, Pos, Waypoint, WaypointArea};
use specs::{Entities, Join, Read, ReadStorage, System, Write, WriteStorage};

/// This system will handle loading generated chunks and unloading uneeded chunks.
///     1. Inserts newly generated chunks into the TerrainGrid
///     2. Sends new chunks to neaby clients
///     3. Handles the chunk's supplement (e.g. npcs)
///     4. Removes chunks outside the range of players
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, WaypointArea>,
        WriteStorage<'a, Waypoint>,
    );

    fn run(
        &mut self,
        (entities, positions, players, waypoint_areas, mut waypoints): Self::SystemData,
    ) {
        for (entity, player_pos, _) in (&entities, &positions, &players).join() {
            for (waypoint_pos, waypoint_area) in (&positions, &waypoint_areas).join() {
                if player_pos.0.distance_squared(waypoint_pos.0) < waypoint_area.radius().powf(2.0)
                {
                    let _ = waypoints.insert(entity, Waypoint::new(player_pos.0));
                }
            }
        }
    }
}
