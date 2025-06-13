use crate::data::{Npc, Npcs};
use common::{
    rtsim::{NpcId, SiteId},
    util::Dir,
};
use std::cmp::Ordering;
use vek::*;
use world::{
    World,
    civ::airship_travel::{
        Airships,
        AirshipSpawningLocation
    },
    index::Index, util::DHashMap
};

/// Data for airship operations. This is part of RTSimData and is NOT persisted.
#[derive(Clone, Default, Debug)]
pub struct AirshipSim {
    /// The pilot route assignments. The key is the pilot NpcId, the value is
    /// a tuple. The first element is the index for the outer Airships::routes
    /// Vec (the route loop index), and the second element is the index for the
    /// pilot's initial route leg in the inner Airships::routes Vec.
    pub assigned_routes: DHashMap<NpcId, (usize, usize)>,
    
    // /// Map of the pilot ahead of a pilot.
    // /// Every pilot has another pilot ahead of it on the route loop.
    // /// The value is the pilot ahead on the route loop.
    // pub next_pilots: DHashMap<NpcId, Option<NpcId>>,
    
    /// The pilots assigned to a route in the order they fly the route.
    pub route_pilots: DHashMap<usize, Vec<NpcId>>,
}

const AIRSHIP_DEBUG: bool = true;

macro_rules! debug_airships {
    ($($arg:tt)*) => {
        if AIRSHIP_DEBUG {
            tracing::debug!($($arg)*);
        }
    }
}

impl AirshipSim {

    pub fn register_airship_captain(
        &mut self,
        location: &AirshipSpawningLocation,
        captain_id: NpcId,
        airship_id: NpcId,
        world: &World,
        npcs: &mut Npcs,
    ) {
        // if let Some(leg_index) = world.civs().airships.route_leg_index_for_spawning_location(location)
        // {
        self.assigned_routes
            .insert(captain_id, (location.route_index, location.leg_index));
        
        assert!(location.dir.is_normalized(), "Airship direction {:?} is not normalized", location.dir);
        let airship_wpos3d = location.pos.with_z(
            world.sim().get_alt_approx(location.pos.map(|e| e as i32))
            .unwrap_or(0.0) + location.height);

        let airship_mount_offset =
        if let Some(airship) = npcs.get_mut(airship_id) {
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
        // } else {
        //     // It should be impossible to get here because the spawning location comes from airships.
        //     tracing::warn!(
        //         "Failed to register airship captain {:?} for spawning location {:?}",
        //         captain_id,
        //         location,
        //     );
        // }
    }

    pub fn configure_route_pilots(&mut self, airships: &Airships, npcs: &Npcs) {
        debug_airships!("Airship Assigned Routes: {:?}", self.assigned_routes);
        // for each route index and leg index, find all pilots that are assigned to
        // the route index and leg index. Sort them by their distance to the starting
        // position of the leg. Repeat for all legs of the route, then add the resulting list
        // to the route_pilots hash map.

        /*
            For route_index in 0..airships.route_count() {
                let route_pilots = Vec::new();
                for leg_index in 0..airships.docking_site_count_for_route(route_index) {
                    // Find all pilots that are spawned on the same route_index and leg_index.
                    let pilots_on_leg = self.assigned_routes.iter()
                        .filter(|(_, (rti, li))| *rti == route_index && *li == leg_index)
                        .map(|(pilot_id, _)| *pilot_id)
                        .collect::<Vec<_>>();
                    if !pilots_on_leg.is_empty() {
                        let p1 = starting position of the leg
                        sort pilots_on_leg by the distance from p1
                        add pilots_on_leg to route_pilots
                    }
                }
                route_pilots is now a list of the pilots on the route in the order they fly the route.
                self.route_pilots.insert(route_index, route_pilots);
            }
        */

        for route_index in 0..airships.route_count() {
            let mut pilots_on_route = Vec::new();
            for leg_index in 0..airships.docking_site_count_for_route(route_index) {
                // Find all pilots that are spawned on the same route_index and leg_index.
                let mut pilots_on_leg: Vec<_> = self.assigned_routes
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
                        start_pos.distance_squared(pilot1_pos)
                            .partial_cmp(&start_pos.distance_squared(pilot2_pos))
                            .unwrap_or(Ordering::Equal)
                    });
                    pilots_on_route.extend(pilots_on_leg);
                }
            }
            if !pilots_on_route.is_empty() {
                debug_airships!(
                    "Route {} pilots: {:?}",
                    route_index,
                    pilots_on_route
                );
                self.route_pilots.insert(route_index, pilots_on_route);
            }
        }
        // for (pilot_id, (route_index, leg_index)) in &self.assigned_routes {
        //     // Find all pilots that have the same leg_index (more than one pilot can have a spawn location
        //     // on the same leg).
        //     // Get the next leg index with wrapping.
        //     let next_leg_index = (leg_index + 1) % airships.routes[*route_index].len();
        //     // Find the pilot with the next leg index.
        //     if let Some(next_pilot_id) = self
        //         .assigned_routes
        //         .iter()
        //         .find_map(|(id, (routei, legi))| {
        //             if *routei == *route_index && *legi == next_leg_index {
        //                 Some(*id)
        //             } else {
        //                 None
        //             }
        //         })
        //     {
        //         self.next_pilots.insert(*pilot_id, Some(next_pilot_id));
        //     } else {
        //         // This should not happen.
        //         tracing::error!("Failed to find next pilot with route index {} and leg index {}",
        //             route_index, next_leg_index);
        //         self.next_pilots.insert(*pilot_id, None);
        //     }
        // }
        // debug_airships!("Next pilots: {:?}", self.next_pilots);
    }

    pub fn next_pilot(
        &self,
        route_index: usize,
        pilot_id: NpcId,
    ) -> Option<NpcId> {
        if let Some(pilots) =  self.route_pilots.get(&route_index) {
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

#[cfg(test)]
mod tests {
    use super::{
        AirshipSim,
    };
    use common::rtsim::NpcId;
    use slotmap::SlotMap;
    use world::{
        civ::airship_travel::{
            airships_from_test_data,
        },
        util::DHashMap,
    };

    // fn airship_sim_from_test_data() -> AirshipSim {

    //     let mut dummy_npcs: SlotMap<NpcId, u32> = SlotMap::with_key();

    //     let assigned_routes = DHashMap::from_iter([
    //         (dummy_npcs.insert(1536), (2, 20)),
    //         (dummy_npcs.insert(1664), (3, 15)),
    //         (dummy_npcs.insert(1530), (0, 11)),
    //         (dummy_npcs.insert(1658), (1, 8)),
    //         (dummy_npcs.insert(1652), (1, 14)),
    //         (dummy_npcs.insert(1646), (2, 6)),
    //         (dummy_npcs.insert(1640), (2, 7)),
    //         (dummy_npcs.insert(1634), (0, 9)),
    //         (dummy_npcs.insert(1628), (2, 8)),
    //         (dummy_npcs.insert(1622), (3, 13)),
    //         (dummy_npcs.insert(1616), (0, 2)),
    //         (dummy_npcs.insert(1610), (2, 5)),
    //         (dummy_npcs.insert(1604), (0, 14)),
    //         (dummy_npcs.insert(1598), (1, 1)),
    //         (dummy_npcs.insert(1592), (2, 1)),
    //         (dummy_npcs.insert(1586), (0, 3)),
    //         (dummy_npcs.insert(1580), (0, 12)),
    //         (dummy_npcs.insert(1574), (2, 10)),
    //         (dummy_npcs.insert(1568), (2, 0)),
    //         (dummy_npcs.insert(1562), (2, 11)),
    //         (dummy_npcs.insert(1556), (2, 2)),
    //         (dummy_npcs.insert(1550), (0, 7)),
    //         (dummy_npcs.insert(1544), (3, 16)),
    //         (dummy_npcs.insert(1672), (2, 13)),
    //         (dummy_npcs.insert(1538), (1, 17)),
    //         (dummy_npcs.insert(1666), (3, 2)),
    //         (dummy_npcs.insert(1532), (3, 0)),
    //         (dummy_npcs.insert(1660), (2, 15)),
    //         (dummy_npcs.insert(1526), (0, 8)),
    //         (dummy_npcs.insert(1654), (2, 3)),
    //         (dummy_npcs.insert(1648), (1, 9)),
    //         (dummy_npcs.insert(1642), (1, 0)),
    //         (dummy_npcs.insert(1636), (0, 13)),
    //         (dummy_npcs.insert(1630), (2, 9)),
    //         (dummy_npcs.insert(1624), (2, 18)),
    //         (dummy_npcs.insert(1618), (1, 2)),
    //         (dummy_npcs.insert(1612), (1, 4)),
    //         (dummy_npcs.insert(1606), (1, 3)),
    //         (dummy_npcs.insert(1600), (3, 5)),
    //         (dummy_npcs.insert(1594), (2, 4)),
    //         (dummy_npcs.insert(1588), (1, 10)),
    //         (dummy_npcs.insert(1582), (3, 6)),
    //         (dummy_npcs.insert(1576), (3, 7)),
    //         (dummy_npcs.insert(1570), (3, 1)),
    //         (dummy_npcs.insert(1564), (1, 13)),
    //         (dummy_npcs.insert(1558), (1, 6)),
    //         (dummy_npcs.insert(1552), (2, 14)),
    //         (dummy_npcs.insert(1546), (0, 16)),
    //         (dummy_npcs.insert(1674), (3, 4)),
    //         (dummy_npcs.insert(1540), (3, 18)),
    //         (dummy_npcs.insert(1668), (2, 19)),
    //         (dummy_npcs.insert(1534), (3, 3)),
    //         (dummy_npcs.insert(1662), (0, 10)),
    //         (dummy_npcs.insert(1528), (0, 5)),
    //         (dummy_npcs.insert(1656), (3, 11)),
    //         (dummy_npcs.insert(1650), (3, 8)),
    //         (dummy_npcs.insert(1644), (0, 1)),
    //         (dummy_npcs.insert(1638), (3, 10)),
    //         (dummy_npcs.insert(1632), (3, 9)),
    //         (dummy_npcs.insert(1626), (0, 15)),
    //         (dummy_npcs.insert(1620), (1, 5)),
    //         (dummy_npcs.insert(1614), (1, 15)),
    //         (dummy_npcs.insert(1608), (1, 7)),
    //         (dummy_npcs.insert(1602), (2, 16)),
    //         (dummy_npcs.insert(1596), (1, 16)),
    //         (dummy_npcs.insert(1590), (0, 6)),
    //         (dummy_npcs.insert(1584), (0, 0)),
    //         (dummy_npcs.insert(1578), (2, 12)),
    //         (dummy_npcs.insert(1572), (1, 11)),
    //         (dummy_npcs.insert(1566), (3, 12)),
    //         (dummy_npcs.insert(1560), (0, 4)),
    //         (dummy_npcs.insert(1554), (2, 17)),
    //         (dummy_npcs.insert(1548), (3, 17)),
    //         (dummy_npcs.insert(1676), (3, 14)),
    //         (dummy_npcs.insert(1542), (0, 17)),
    //         (dummy_npcs.insert(1670), (1, 12))
    //     ]);

        // AirshipSim {
        //     assigned_routes,
        //     next_pilots: DHashMap::default(),
        // }
    // }


    // #[test]
    // fn test_configure_route_pilots() {
    //     let mut airship_sim = airship_sim_from_test_data();
    //     let airships = airships_from_test_data();
    //     airship_sim.configure_route_pilots(&airships);
    //     airship_sim.next_pilots.iter().for_each(|(pilot_id, opt_next_pilot_id)|
    //         if let Some((route1, leg1)) = airship_sim.assigned_routes.get(pilot_id) 
    //             && let Some(next_pilot_id) = opt_next_pilot_id
    //             && let Some((route2, leg2)) = airship_sim.assigned_routes.get(next_pilot_id)
    //         {
    //             assert!(*route1 == *route2, "Pilot {:?} and next pilot {:?} are not on the same route", pilot_id, next_pilot_id);
    //             assert!(*leg2 == (*leg1 + 1) % airships.routes[*route1].len(),
    //                 "Pilot {:?} and next pilot {:?} are not on consecutive legs of route {}",
    //                 pilot_id, next_pilot_id, *route1);
    //         }
    //     );
    // }
}