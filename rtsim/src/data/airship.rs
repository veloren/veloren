use common::{
    rtsim::{NpcId, SiteId},
    util::Dir,
};
use std::cmp::Ordering;
use vek::*;
use world::{civ::airship_travel::Airships, index::Index, util::DHashMap};

/// Data for airship operations. This is part of RTSimData and is NOT persisted.
pub type AirshipRouteIdLegacy = u32;
#[derive(Clone, Default)]
pub struct AirshipSimLegacy {
    /// The pilot route assignments. The key is the pilot NpcId, the value is
    /// the route id.
    pub assigned_routes: DHashMap<NpcId, AirshipRouteIdLegacy>,
    /// The pilots assigned to a route. The value is a list of pilot NpcIds. The
    /// key is the route id.
    pub route_pilots: DHashMap<AirshipRouteIdLegacy, Vec<NpcId>>,
}

/// Data for airship operations. This is part of RTSimData and is NOT persisted.
#[derive(Clone, Default, Debug)]
pub struct AirshipSim {
    /// The pilot route assignments. The key is the pilot NpcId, the value is
    /// a tuple. The first element is the index for the outer Airships::routes
    /// Vec (the route loop index), and the second element is the index for the
    /// pilot's initial route leg in the inner Airships::routes Vec.
    pub assigned_routes: DHashMap<NpcId, (usize, usize)>,
    /// Map of the pilot ahead of a pilot.
    /// Every pilot has another pilot ahead of it on the route loop.
    /// The value is the pilot ahead on the route loop.
    pub next_pilots: DHashMap<NpcId, Option<NpcId>>,
}

const AIRSHIP_DEBUG: bool = true;

macro_rules! debug_airships {
    ($($arg:tt)*) => {
        if AIRSHIP_DEBUG {
            tracing::debug!($($arg)*);
        }
    }
}

impl AirshipSimLegacy {
    /// Connect the airshp captain NpcId to an airship route using the given
    /// docking position. This establishes the airship captain's route.
    /// Returns the position where the airship should be placed.
    pub fn register_airship_captain(
        &mut self,
        docking_pos: Vec3<f32>,
        captain_id: NpcId,
        airship_id: NpcId,
        index: &Index,
        airships: &Airships,
    ) -> Option<Vec3<f32>> {
        // Find the route where where either approach.dock_pos is equal (very close to)
        // the given docking_pos.
        if let Some((route_id, approach_index)) =
            airships.airship_route_for_docking_pos_legacy(docking_pos)
            && let Some(route) = airships.legacy_routes.get(&route_id)
        {
            let site0_name = index.sites.get(route.sites[0]).name().to_string();
            let site1_name = index.sites.get(route.sites[1]).name().to_string();
            debug_airships!(
                "Registering airship {:?}/{:?} for docking position {:?}, site0:{}, site1:{}",
                airship_id,
                captain_id,
                docking_pos,
                site0_name,
                site1_name
            );

            self.assigned_routes.insert(captain_id, route_id);
            let approach = &route.approaches[approach_index];
            let airship_wpos = approach
                .approach_final_pos
                .with_z(approach.airship_pos.z + approach.height);
            debug_airships!(
                "Airship {:?}/{:?}, approach index:{}, initial wpos:{:?}",
                airship_id,
                captain_id,
                approach_index,
                airship_wpos
            );
            Some(airship_wpos)
        } else {
            // It should be impossible to get here if
            // should_spawn_airship_at_docking_position_legacy is working correctly.
            tracing::warn!(
                "Failed to register airship captain {:?} for docking position {:?}",
                captain_id,
                docking_pos
            );
            None
        }
    }

    /// Collects the captain NPC ids for the airships flying each route (out and
    /// back).
    pub fn configure_route_pilots(&mut self, airships: &Airships) {
        // for each route, get the docking position id, then for for each assigned
        // route, get the pilot id and the routes the pilot is assigned to, then
        // if the pilot is assigned to the route, add the pilot to the route's
        // pilots
        self.route_pilots
            .extend(airships.legacy_routes.iter().map(|(route_id, _)| {
                (
                    *route_id,
                    self.assigned_routes
                        .iter()
                        .filter(|(_, assigned_route_id)| **assigned_route_id == *route_id)
                        .map(|(pilot_id, _)| *pilot_id)
                        .collect(),
                )
            }));
        debug_airships!("Route pilots: {:?}", self.route_pilots);
    }
}

impl AirshipSim {
    pub fn register_airship_captain(
        &mut self,
        docking_pos: Vec3<f32>,
        captain_id: NpcId,
        airship_id: NpcId,
        airships: &Airships,
    ) -> Option<Vec3<f32>> {
        if let Some((dock_index, platform)) =
            airships.dock_index_and_platform_for_docking_pos(&docking_pos.map(|f| f as i32))
            && let Some((route_index, leg_index)) =
                airships
                    .routes
                    .iter()
                    .enumerate()
                    .find_map(|(routei, route)| {
                        route.iter().enumerate().find_map(|(legi, leg)| {
                            if leg.dest_index == dock_index && leg.platform == platform {
                                Some((routei, legi))
                            } else {
                                None
                            }
                        })
                    })
        {
            self.assigned_routes
                .insert(captain_id, (route_index, leg_index));
            let route_leg = &airships.routes[route_index][leg_index];
            let previous_route_leg = if leg_index == 0 {
                &airships.routes[route_index][airships.routes[route_index].len() - 1]
            } else {
                &airships.routes[route_index][leg_index - 1]
            };
            if let Some(from_pos) = airships.airship_docks.get(previous_route_leg.dest_index) {
                let transition_pos = airships.approach_transition_point(
                    route_leg.dest_index,
                    route_index,
                    route_leg.platform,
                    from_pos.center + (docking_pos.xy() - from_pos.center).normalized() * 1000.0,
                );
                debug_airships!(
                    "Registering airship {:?}/{:?} for docking position {:?}, route_index:{}, \
                     leg_index:{}, route_leg:{:?}, previous_route_leg:{:?}, transition_pos:{:?}",
                    airship_id,
                    captain_id,
                    docking_pos,
                    route_index,
                    leg_index,
                    route_leg,
                    previous_route_leg,
                    transition_pos
                );
                return transition_pos;
            } else {
                // It should be impossible to get here if
                // should_spawn_airship_at_docking_position is working correctly.
                tracing::warn!(
                    "Failed to register airship captain {:?} for docking position {:?}, invalid \
                     previous route leg",
                    captain_id,
                    docking_pos
                );
                None
            }
        } else {
            // It should be impossible to get here if
            // should_spawn_airship_at_docking_position is working correctly.
            tracing::warn!(
                "Failed to register airship captain {:?} for docking position {:?}, invalid \
                 docking position",
                captain_id,
                docking_pos
            );
            None
        }
    }

    pub fn configure_route_pilots(&mut self, airships: &Airships) {
        debug_airships!("Airship Assigned Routes: {:?}", self.assigned_routes);
        // for each pilot in assigned_routes, get the assigned route index and leg index.
        // Increment the leg index and then find the pilot in assigned_routes that has that leg index.
        // Add the pilot pair to next_pilots.
        for (pilot_id, (route_index, leg_index)) in &self.assigned_routes {
            // Get the next leg index with wrapping.
            let next_leg_index = (leg_index + 1) % airships.routes[*route_index].len();
            // Find the pilot with the next leg index.
            if let Some(next_pilot_id) = self
                .assigned_routes
                .iter()
                .find_map(|(id, (routei, legi))| {
                    if *routei == *route_index && *legi == next_leg_index {
                        Some(*id)
                    } else {
                        None
                    }
                })
            {
                self.next_pilots.insert(*pilot_id, Some(next_pilot_id));
            } else {
                // This should not happen.
                tracing::error!("Failed to find next pilot with route index {} and leg index {}",
                    route_index, next_leg_index);
                self.next_pilots.insert(*pilot_id, None);
            }
        }
        debug_airships!("Next pilots: {:?}", self.next_pilots);
    }
}

#[derive(Debug)]
pub struct AirshipSpawningLocation {
    pub pos: Vec3<f32>,
    pub dir: Dir,
    pub center: Vec2<i32>,
    pub docking_pos: Vec3<i32>,
    pub site_id: SiteId,
    pub site_name: String,
}

impl PartialEq for AirshipSpawningLocation {
    fn eq(&self, other: &Self) -> bool {
        self.center == other.center
            && self.docking_pos == other.docking_pos
            && self.site_id == other.site_id
    }
}

impl Eq for AirshipSpawningLocation {}

impl Ord for AirshipSpawningLocation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.site_id
            .cmp(&other.site_id)
            .then_with(|| self.docking_pos.cmp(&other.docking_pos))
    }
}

impl PartialOrd for AirshipSpawningLocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
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

    fn airship_sim_from_test_data() -> AirshipSim {

        let mut dummy_npcs: SlotMap<NpcId, u32> = SlotMap::with_key();

        let assigned_routes = DHashMap::from_iter([
            (dummy_npcs.insert(1536), (2, 20)),
            (dummy_npcs.insert(1664), (3, 15)),
            (dummy_npcs.insert(1530), (0, 11)),
            (dummy_npcs.insert(1658), (1, 8)),
            (dummy_npcs.insert(1652), (1, 14)),
            (dummy_npcs.insert(1646), (2, 6)),
            (dummy_npcs.insert(1640), (2, 7)),
            (dummy_npcs.insert(1634), (0, 9)),
            (dummy_npcs.insert(1628), (2, 8)),
            (dummy_npcs.insert(1622), (3, 13)),
            (dummy_npcs.insert(1616), (0, 2)),
            (dummy_npcs.insert(1610), (2, 5)),
            (dummy_npcs.insert(1604), (0, 14)),
            (dummy_npcs.insert(1598), (1, 1)),
            (dummy_npcs.insert(1592), (2, 1)),
            (dummy_npcs.insert(1586), (0, 3)),
            (dummy_npcs.insert(1580), (0, 12)),
            (dummy_npcs.insert(1574), (2, 10)),
            (dummy_npcs.insert(1568), (2, 0)),
            (dummy_npcs.insert(1562), (2, 11)),
            (dummy_npcs.insert(1556), (2, 2)),
            (dummy_npcs.insert(1550), (0, 7)),
            (dummy_npcs.insert(1544), (3, 16)),
            (dummy_npcs.insert(1672), (2, 13)),
            (dummy_npcs.insert(1538), (1, 17)),
            (dummy_npcs.insert(1666), (3, 2)),
            (dummy_npcs.insert(1532), (3, 0)),
            (dummy_npcs.insert(1660), (2, 15)),
            (dummy_npcs.insert(1526), (0, 8)),
            (dummy_npcs.insert(1654), (2, 3)),
            (dummy_npcs.insert(1648), (1, 9)),
            (dummy_npcs.insert(1642), (1, 0)),
            (dummy_npcs.insert(1636), (0, 13)),
            (dummy_npcs.insert(1630), (2, 9)),
            (dummy_npcs.insert(1624), (2, 18)),
            (dummy_npcs.insert(1618), (1, 2)),
            (dummy_npcs.insert(1612), (1, 4)),
            (dummy_npcs.insert(1606), (1, 3)),
            (dummy_npcs.insert(1600), (3, 5)),
            (dummy_npcs.insert(1594), (2, 4)),
            (dummy_npcs.insert(1588), (1, 10)),
            (dummy_npcs.insert(1582), (3, 6)),
            (dummy_npcs.insert(1576), (3, 7)),
            (dummy_npcs.insert(1570), (3, 1)),
            (dummy_npcs.insert(1564), (1, 13)),
            (dummy_npcs.insert(1558), (1, 6)),
            (dummy_npcs.insert(1552), (2, 14)),
            (dummy_npcs.insert(1546), (0, 16)),
            (dummy_npcs.insert(1674), (3, 4)),
            (dummy_npcs.insert(1540), (3, 18)),
            (dummy_npcs.insert(1668), (2, 19)),
            (dummy_npcs.insert(1534), (3, 3)),
            (dummy_npcs.insert(1662), (0, 10)),
            (dummy_npcs.insert(1528), (0, 5)),
            (dummy_npcs.insert(1656), (3, 11)),
            (dummy_npcs.insert(1650), (3, 8)),
            (dummy_npcs.insert(1644), (0, 1)),
            (dummy_npcs.insert(1638), (3, 10)),
            (dummy_npcs.insert(1632), (3, 9)),
            (dummy_npcs.insert(1626), (0, 15)),
            (dummy_npcs.insert(1620), (1, 5)),
            (dummy_npcs.insert(1614), (1, 15)),
            (dummy_npcs.insert(1608), (1, 7)),
            (dummy_npcs.insert(1602), (2, 16)),
            (dummy_npcs.insert(1596), (1, 16)),
            (dummy_npcs.insert(1590), (0, 6)),
            (dummy_npcs.insert(1584), (0, 0)),
            (dummy_npcs.insert(1578), (2, 12)),
            (dummy_npcs.insert(1572), (1, 11)),
            (dummy_npcs.insert(1566), (3, 12)),
            (dummy_npcs.insert(1560), (0, 4)),
            (dummy_npcs.insert(1554), (2, 17)),
            (dummy_npcs.insert(1548), (3, 17)),
            (dummy_npcs.insert(1676), (3, 14)),
            (dummy_npcs.insert(1542), (0, 17)),
            (dummy_npcs.insert(1670), (1, 12))
        ]);

        AirshipSim {
            assigned_routes,
            next_pilots: DHashMap::default(),
        }
    }


    #[test]
    fn test_configure_route_pilots() {
        let mut airship_sim = airship_sim_from_test_data();
        let airships = airships_from_test_data();
        airship_sim.configure_route_pilots(&airships);
        airship_sim.next_pilots.iter().for_each(|(pilot_id, opt_next_pilot_id)|
            if let Some((route1, leg1)) = airship_sim.assigned_routes.get(pilot_id) 
                && let Some(next_pilot_id) = opt_next_pilot_id
                && let Some((route2, leg2)) = airship_sim.assigned_routes.get(next_pilot_id)
            {
                assert!(*route1 == *route2, "Pilot {:?} and next pilot {:?} are not on the same route", pilot_id, next_pilot_id);
                assert!(*leg2 == (*leg1 + 1) % airships.routes[*route1].len(),
                    "Pilot {:?} and next pilot {:?} are not on consecutive legs of route {}",
                    pilot_id, next_pilot_id, *route1);
            }
        );
    }
}