use common::{
    rtsim::{NpcId, SiteId},
    util::Dir,
};
use std::cmp::Ordering;
use vek::*;
use world::{civ::airship_travel::Airships, index::Index, util::DHashMap};

/// Data for airship operations. This is part of RTSimData and is NOT persisted.
pub type AirshipRouteId = u32;
#[derive(Clone, Default)]
pub struct AirshipSim {
    /// The pilot route assignments. The key is the pilot NpcId, the value is
    /// the route id.
    pub assigned_routes: DHashMap<NpcId, AirshipRouteId>,
    /// The pilots assigned to a route. The value is a list of pilot NpcIds. The
    /// key is the route id.
    pub route_pilots: DHashMap<AirshipRouteId, Vec<NpcId>>,
}

const AIRSHIP_DEBUG: bool = false;

macro_rules! debug_airships {
    ($($arg:tt)*) => {
        if AIRSHIP_DEBUG {
            tracing::debug!($($arg)*);
        }
    }
}

impl AirshipSim {
    /// Connect the airshp captain NpcId to an airship route using the given
    /// docking position. This establishes the airship captain's route.
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
            airships.airship_route_for_docking_pos(docking_pos)
            && let Some(route) = airships.routes.get(&route_id)
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
                .with_z(approach.airship_pos.z + approach.cruise_hat);
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
            // should_spawn_airship_at_docking_position is working correctly.
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
        airships.routes.iter().for_each(|(route_id, _)| {
            let mut route_pilots = Vec::<NpcId>::new();
            self.assigned_routes
                .iter()
                .for_each(|(pilot_id, assigned_route_id)| {
                    if *assigned_route_id == *route_id {
                        route_pilots.push(*pilot_id);
                    }
                });
            self.route_pilots.insert(*route_id, route_pilots);
        });
        debug_airships!("Route pilots: {:?}", self.route_pilots);
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

impl AirshipSpawningLocation {
    pub fn new(
        pos: Vec3<f32>,
        dir: Dir,
        center: Vec2<i32>,
        docking_pos: Vec3<i32>,
        site_id: SiteId,
        site_name: String,
    ) -> Self {
        Self {
            pos,
            dir,
            center,
            docking_pos,
            site_id,
            site_name,
        }
    }
}

impl PartialEq for AirshipSpawningLocation {
    fn eq(&self, other: &Self) -> bool {
        self.center == other.center
            && self.docking_pos == other.docking_pos
            && self.site_id == other.site_id
            && self.site_name == other.site_name
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
