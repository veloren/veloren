use crate::data::Npcs;
use common::rtsim::NpcId;
use std::cmp::Ordering;
#[cfg(debug_assertions)] use tracing::debug;
use vek::*;
use world::{
    World,
    civ::airship_travel::{AirshipSpawningLocation, Airships},
    util::DHashMap,
};

/// Data for airship operations. This is part of RTSimData and is NOT persisted.
#[derive(Clone, Default, Debug)]
pub struct AirshipSim {
    /// The pilot route assignments. The key is the pilot NpcId, the value is
    /// a tuple. The first element is the index for the outer Airships::routes
    /// Vec (the route loop index), and the second element is the index for the
    /// pilot's initial route leg in the inner Airships::routes Vec.
    pub assigned_routes: DHashMap<NpcId, (usize, usize)>,

    /// The pilots assigned to a route in the order they fly the route.
    pub route_pilots: DHashMap<usize, Vec<NpcId>>,
}

#[cfg(debug_assertions)]
macro_rules! debug_airships {
    ($($arg:tt)*) => {
        debug!($($arg)*);
    }
}

#[cfg(not(debug_assertions))]
macro_rules! debug_airships {
    ($($arg:tt)*) => {};
}

impl AirshipSim {
    /// Called from world generation code to set the route and initial leg indexes for an
    /// airship captain NPC. World generation is dynamic and can change across runs, and
    /// existing captain (and ship) NPCs may change the assigned route and leg, and new
    /// NPCs may be added to the world. This is the function that connects the saved
    /// RTSim data to the world generation data.
    pub fn register_airship_captain(
        &mut self,
        location: &AirshipSpawningLocation,
        captain_id: NpcId,
        airship_id: NpcId,
        world: &World,
        npcs: &mut Npcs,
    ) {
        self.assigned_routes
            .insert(captain_id, (location.route_index, location.leg_index));

        assert!(
            location.dir.is_normalized(),
            "Airship direction {:?} is not normalized",
            location.dir
        );
        let airship_wpos3d = location.pos.with_z(
            world
                .sim()
                .get_alt_approx(location.pos.map(|e| e as i32))
                .unwrap_or(0.0)
                + location.height,
        );

        let airship_mount_offset = if let Some(airship) = npcs.get_mut(airship_id) {
            airship.wpos = airship_wpos3d;
            airship.dir = location.dir;
            airship.body.mount_offset()
        } else {
            tracing::warn!(
                "Failed to find airship {:?} for captain {:?}",
                airship_id,
                captain_id,
            );
            Vec3::new(0.0, 0.0, 0.0)
        };
        if let Some(captain) = npcs.get_mut(captain_id) {
            let captain_pos = airship_wpos3d
                + Vec3::new(
                    location.dir.x * airship_mount_offset.x,
                    location.dir.y * airship_mount_offset.y,
                    airship_mount_offset.z,
                );
            captain.wpos = captain_pos;
            captain.dir = location.dir;
        }

        debug_airships!(
            "Registering airship {:?}/{:?} for spawning location {:?}",
            airship_id,
            captain_id,
            location,
        );
    }

    /// Called from world generation code after all airship captains have been registered.
    /// This function generates the route_pilots hash map which provides a list of pilots
    /// assigned to each route index, in the order they will fly the route. This provides
    /// for determining the "next pilot" on a route, which is used for deconfliction and
    /// "traffic control" of airships.
    pub fn configure_route_pilots(&mut self, airships: &Airships, npcs: &Npcs) {
        debug_airships!("Airship Assigned Routes: {:?}", self.assigned_routes);
        // for each route index and leg index, find all pilots that are assigned to
        // the route index and leg index. Sort them by their distance to the starting
        // position of the leg. Repeat for all legs of the route, then add the resulting
        // list to the route_pilots hash map.

        for route_index in 0..airships.route_count() {
            let mut pilots_on_route = Vec::new();
            for leg_index in 0..airships.docking_site_count_for_route(route_index) {
                // Find all pilots that are spawned on the same route_index and leg_index.
                let mut pilots_on_leg: Vec<_> = self
                    .assigned_routes
                    .iter()
                    .filter(|(_, (rti, li))| *rti == route_index && *li == leg_index)
                    .map(|(pilot_id, _)| *pilot_id)
                    .collect();

                if !pilots_on_leg.is_empty() {
                    // Sort pilots by their distance to the starting position of the leg.
                    let start_pos = airships.route_leg_departure_location(route_index, leg_index);
                    pilots_on_leg.sort_by(|&pilot1, &pilot2| {
                        let pilot1_pos = npcs.get(pilot1).map_or(start_pos, |npc| npc.wpos.xy());
                        let pilot2_pos = npcs.get(pilot2).map_or(start_pos, |npc| npc.wpos.xy());
                        start_pos
                            .distance_squared(pilot1_pos)
                            .partial_cmp(&start_pos.distance_squared(pilot2_pos))
                            .unwrap_or(Ordering::Equal)
                    });
                    pilots_on_route.extend(pilots_on_leg);
                }
            }
            if !pilots_on_route.is_empty() {
                debug_airships!("Route {} pilots: {:?}", route_index, pilots_on_route);
                self.route_pilots.insert(route_index, pilots_on_route);
            }
        }
    }

    /// Given a route index and pilot id, find the next pilot on the route (the one
    /// that is ahead of the given pilot).
    pub fn next_pilot(&self, route_index: usize, pilot_id: NpcId) -> Option<NpcId> {
        if let Some(pilots) = self.route_pilots.get(&route_index) {
            if pilots.len() < 2 {
                // If there is only one pilot on the route, return the pilot itself.
                tracing::warn!(
                    "Route {} has only one pilot, 'next_pilot' doesn't make sense.",
                    route_index,
                );
                return None;
            }
            if let Some(pilot_index) = pilots.iter().position(|&p_id| p_id == pilot_id) {
                if pilot_index == pilots.len() - 1 {
                    // If the pilot is the last one in the list, return the first one.
                    return Some(pilots[0]);
                } else {
                    // Otherwise, return the next pilot in the list.
                    return Some(pilots[pilot_index + 1]);
                }
            }
        }
        tracing::warn!(
            "Failed to find next pilot for route index {} and pilot id {:?}",
            route_index,
            pilot_id
        );
        None
    }
}
