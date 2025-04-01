use crate::{
    sim::WorldSim,
    site::{self, Site},
    site2::plot::PlotKindMeta,
    util::{DHashMap, DHashSet, seed_expan},
};
use common::{
    store::{Id, Store},
    terrain::CoordinateConversions,
    util::Dir,
};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::{fs::OpenOptions, io::Write};
use tracing::{debug, warn};
use vek::*;

const AIRSHIP_TRAVEL_DEBUG: bool = false;

macro_rules! debug_airships {
    ($($arg:tt)*) => {
        if AIRSHIP_TRAVEL_DEBUG {
            debug!($($arg)*);
        }
    }
}

/// A docking position (id, position). The docking position id is
/// an index of all docking positions in the world.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AirshipDockingPosition(pub u32, pub Vec3<f32>);

/// An airship can dock with its port or starboard side facing the dock.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum AirshipDockingSide {
    #[default]
    Port,
    Starboard,
}

/// An approach for the airship provides the data needed to fly to a docking
/// position and stop at the docking position. The approach provides a target
/// 'final' postion that is offset from the docking postion such that
/// when the airship flys from the final position to the docking position, the
/// airship will be naturally aligned with the direction of the docking
/// position, and only very small orientation adjustments will be needed
/// before docking. The approach final position is selected to minimize the
/// change of direction when flying from the takeoff location to the target
/// docking position.
#[derive(Clone, Debug, PartialEq)]
pub struct AirshipDockingApproach {
    pub dock_pos: AirshipDockingPosition,
    /// The position of the airship when docked.
    /// This is different from dock_pos because the airship is offset to align
    /// the ramp with the dock.
    pub airship_pos: Vec3<f32>,
    /// The direction the airship is facing when docked.
    pub airship_direction: Dir,
    /// Then center of the AirshipDock Plot.
    pub dock_center: Vec2<f32>,
    /// The height above terrain the airship cruises at.
    pub height: f32,
    /// A 3D position that is offset from the direct line between the dock sites
    /// to allow for a transition to the direction the airship will be
    /// facing when it is docked.
    pub approach_initial_pos: Vec2<f32>,
    /// Intermediate position from the initial position to smooth out the
    /// directional changes.
    pub approach_final_pos: Vec2<f32>,
    /// There are ramps on both the port and starboard sides of the airship.
    /// This gives the side that the airship will dock on.
    pub side: AirshipDockingSide,
    /// The site name where the airship will be docked at the end of the
    /// approach.
    pub site_id: Id<Site>,
}

/// A route that an airship flies round-trip between two sites.
#[derive(Clone, Debug)]
pub struct AirshipRoute {
    /// site[0] is the home site, site[1] is the away site.
    pub sites: [Id<site::Site>; 2],
    /// approaches[0] is flying from the home site to the away site.
    /// approaches[1] is flying from the away site to the home site.
    pub approaches: [AirshipDockingApproach; 2],
    /// The distance between the two sites.
    pub distance: u32,
}

impl AirshipRoute {
    fn new(
        site1: Id<site::Site>,
        site2: Id<site::Site>,
        approaches: [AirshipDockingApproach; 2],
        distance: u32,
    ) -> Self {
        Self {
            sites: [site1, site2],
            approaches,
            distance,
        }
    }
}

/// Airship routes are identified by a unique serial number starting from zero.
type AirshipRouteId = u32;

/// Data for airship operations. This is generated world data.
#[derive(Clone, Default)]
pub struct Airships {
    /// The airship routes between sites.
    pub routes: DHashMap<AirshipRouteId, AirshipRoute>,
}

// Internal data structures

/// The docking postions at an AirshipDock plot.
/// The center is the center of the plot. The docking_positions
/// are the positions where the airship can dock.
#[derive(Clone, Debug)]
struct AirshipDockPositions {
    pub center: Vec2<f32>,
    pub docking_positions: Vec<AirshipDockingPosition>,
    pub site_id: Id<site::Site>,
}

impl AirshipDockPositions {
    fn from_plot_meta(
        first_id: u32,
        center: Vec2<i32>,
        docking_positions: &[Vec3<i32>],
        site_id: Id<site::Site>,
    ) -> Self {
        let mut dock_pos_id = first_id;
        Self {
            center: center.map(|i| i as f32),
            docking_positions: docking_positions
                .iter()
                .map(|pos: &Vec3<i32>| {
                    let docking_position =
                        AirshipDockingPosition(dock_pos_id, pos.map(|i| i as f32));
                    dock_pos_id += 1;
                    docking_position
                })
                .collect(),
            site_id,
        }
    }
}

/// Used while generating the airship routes to connect the airship docks.
/// Encapsulates the connection between two airship docks, including the angle
/// and distance.
#[derive(Clone, Debug)]
struct AirRouteConnection<'a> {
    pub dock1: &'a AirshipDockPositions,
    pub dock2: &'a AirshipDockPositions,
    pub angle: f32,    // angle from dock1 to dock2, from dock2 the angle is -angle
    pub distance: i64, // distance squared between dock1 and dock2
}

impl<'a> AirRouteConnection<'a> {
    fn new(dock1: &'a AirshipDockPositions, dock2: &'a AirshipDockPositions) -> Self {
        let angle = Airships::angle_between_vectors_ccw(
            Airships::ROUTES_NORTH,
            dock2.center - dock1.center,
        );
        let distance = dock1.center.distance_squared(dock2.center) as i64;
        Self {
            dock1,
            dock2,
            angle,
            distance,
        }
    }
}

/// Dock connnections are a hash map (DHashMap) of DockConnectionHashKey to
/// AirRouteConnection. The hash map is used internally during the generation of
/// the airship routes.
#[derive(Eq, PartialEq, Hash, Debug)]
struct DockConnectionHashKey(Id<site::Site>, Id<site::Site>);

/// Represents potential connections between two airship docks. Used during the
/// generation of the airship routes.
#[derive(Clone, Debug)]
struct DockConnection<'a> {
    pub dock: &'a AirshipDockPositions,
    pub available_connections: usize,
    pub connections: Vec<&'a AirRouteConnection<'a>>,
}

impl<'a> DockConnection<'a> {
    fn new(dock: &'a AirshipDockPositions) -> Self {
        Self {
            dock,
            available_connections: dock.docking_positions.len(),
            connections: Vec::new(),
        }
    }

    fn add_connection(&mut self, connection: &'a AirRouteConnection<'a>) {
        self.connections.push(connection);
        self.available_connections -= 1;
    }
}

impl Airships {
    /// The Z offset between the docking alignment point and the AirshipDock
    /// plot docking position.
    const AIRSHIP_TO_DOCK_Z_OFFSET: f32 = -3.0;
    // the generated docking positions in world gen are a little low
    const DEFAULT_DOCK_DURATION: f32 = 90.0;
    /// The vector from the dock alignment point when the airship is docked on
    /// the port side.
    const DOCK_ALIGN_POS_PORT: Vec2<f32> =
        Vec2::new(Airships::DOCK_ALIGN_X, -Airships::DOCK_ALIGN_Y);
    /// The vector from the dock alignment point on the airship when the airship
    /// is docked on the starboard side.
    const DOCK_ALIGN_POS_STARBOARD: Vec2<f32> =
        Vec2::new(-Airships::DOCK_ALIGN_X, -Airships::DOCK_ALIGN_Y);
    /// The absolute offset from the airship's position to the docking alignment
    /// point on the X axis. The airship is assumed to be facing positive Y.
    const DOCK_ALIGN_X: f32 = 18.0;
    /// The offset from the airship's position to the docking alignment point on
    /// the Y axis. The airship is assumed to be facing positive Y.
    /// This is positive if the docking alignment point is in front of the
    /// airship's center position.
    const DOCK_ALIGN_Y: f32 = 1.0;
    const ROUTES_NORTH: Vec2<f32> = Vec2::new(0.0, 15000.0);
    const STD_CRUISE_HEIGHT: f32 = 400.0;
    const TAKEOFF_ASCENT_ALT: f32 = 150.0;

    #[inline(always)]
    pub fn docking_duration() -> f32 { Airships::DEFAULT_DOCK_DURATION }

    #[inline(always)]
    pub fn takeoff_ascent_height() -> f32 { Airships::TAKEOFF_ASCENT_ALT }

    /// Get all the airship docking positions from the world sites.
    fn all_airshipdock_positions(sites: &mut Store<Site>) -> Vec<AirshipDockPositions> {
        let mut dock_pos_id = 0;
        sites
            .iter()
            .flat_map(|(site_id, site)| site.site2().map(|site2| (site_id, site2)))
            .flat_map(|(site_id, site2)| {
                site2.plots().flat_map(move |plot| {
                    if let Some(PlotKindMeta::AirshipDock {
                        center,
                        docking_positions,
                        ..
                    }) = plot.kind().meta()
                    {
                        Some((center, docking_positions, site_id))
                    } else {
                        None
                    }
                })
            })
            .map(|(center, docking_positions, site_id)| {
                let positions = AirshipDockPositions::from_plot_meta(
                    dock_pos_id,
                    center,
                    docking_positions,
                    site_id,
                );

                dock_pos_id += positions.docking_positions.len() as u32;
                positions
            })
            .collect::<Vec<_>>()
    }

    /// Generate the network of airship routes between all the sites with
    /// airship docks. This is called only from the world generation code.
    ///
    /// After world sites are generated, the airship operations center creates a
    /// network of airship routes between all the sites containing an
    /// airship dock plot, and there are airships placed at each docking
    /// position that will be used for an airship route. Each airship travels
    /// between two sites. This is the airship's route (out and back).  When
    /// an airship is created, the ops center internally assigns the airship
    /// a route based on the airship's home docking position and the airship
    /// routing network. Since a route is between two sites, and therefore
    /// between two docking positions, there are two airships flying in opposite
    /// directions.
    ///
    /// Todo: On longer routes, it should be possible to determine the flight
    /// time and add airships to the route to maintain a schedule. The
    /// airships would be spawned midair so that they don't appear our of
    /// nowhere near the ground.
    ///
    /// Airships are assigned a flying height based on the direction of
    /// travel to deconflict as much as possible.
    pub fn generate_airship_routes(
        &mut self,
        sites: &mut Store<Site>,
        world_sim: &mut WorldSim,
        seed: u32,
    ) {
        let all_docking_positions = Airships::all_airshipdock_positions(sites);
        // Create a map of all possible dock to dock connections.
        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));
        let mut routes = DHashMap::<DockConnectionHashKey, AirRouteConnection>::default();
        all_docking_positions.iter().for_each(|from_dock| {
            all_docking_positions
                .iter()
                .filter(|to_dock| to_dock.site_id != from_dock.site_id)
                .for_each(|to_dock| {
                    routes.insert(
                        DockConnectionHashKey(from_dock.site_id, to_dock.site_id),
                        AirRouteConnection::new(from_dock, to_dock),
                    );
                });
        });

        // Do four rounds of connections.
        // In each round, attempt to connect each dock to another dock that has at least
        // one connection remaining. Assign scores to each candidate route based
        // on:
        // 1. How close the candidate route angle is to the optimal angle. The optimal
        //    angle is calculated as follows:
        //    - 0 existing connections - any angle, all angles are equally good.
        //    - 1 existing connection - the angle for the vector opposite to the vector
        //      for the existing connection.
        //    - 2 existing connections - calculate the triangle formed by the first two
        //      route endpoints and the dock center. Find the centroid of the triangle
        //      and calculate the angle from the dock center to the centroid. The
        //      optimal angle is the angle opposite to the vector from the dock center
        //      to the centroid.
        //    - 3 existing connections - calculate the triangle formed by the first
        //      three connection endpoints. Find the centroid of the triangle and
        //      calculate the angle from the dock center to the centroid. The optimal
        //      angle is the angle opposite to the vector from the dock center to the
        //      centroid.
        // 2. The distance from the dock center to the connection endpoint. Generally,
        //    the further the better, but the score function should be logarithmic to
        //    not favor so much amongst the longer distances.

        let mut dock_connections = all_docking_positions
            .iter()
            .map(DockConnection::new)
            .collect::<Vec<_>>();

        // The simple angle score is how close a2 is to the opposite of a1. E.g. if a1
        // is 0.0, the the best score is when a2 is PI.
        let angle_score_fn = |a1: f32, a2: f32| {
            let optimal_angle = (a1 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
            let angle_diff = (optimal_angle - a2)
                .abs()
                .min(std::f32::consts::TAU - (optimal_angle - a2).abs());
            1.0 - (angle_diff / std::f32::consts::PI)
        };

        // The centroid angle score function calculates the angle from the dock center
        // to the given 'centroid' vector, then applies the angle score function
        // with a1 being the centroid angle and a2 being the route angle.
        let centroid_angle_score_fn =
            |centroid: Vec2<f32>, dock_center: Vec2<f32>, rt: &AirRouteConnection| {
                let centroid_dir = centroid - dock_center;
                if centroid_dir.is_approx_zero() {
                    return 0.0;
                }
                let centroid_angle =
                    Airships::angle_between_vectors_ccw(Airships::ROUTES_NORTH, centroid_dir);
                angle_score_fn(centroid_angle, rt.angle)
            };

        // The distance score function is logarithmic and favors long distances (but not
        // by much). The lower cutoff is so that docks within 5000 blocks of
        // each other are not connected unless there are no other options. The final
        // division and subtraction are used to scale the score so that middle
        // distances have a score of around 1.0.
        let distance_score_fn = |distance: i64| {
            // Note the distance argument is the square of the actual distance.
            if distance > 25000000 {
                (((distance - 24999000) / 1000) as f32).ln() / 8.0 - 0.5
            } else {
                0.0
            }
        };

        // overall score function
        let score_fn = |con: &DockConnection, rt: &AirRouteConnection| {
            let mut angle_score = match con.connections.len() {
                // Anything goes
                0 => 1.0,
                // Opposite angle
                1 => angle_score_fn(con.connections[0].angle, rt.angle),
                // Centroid angle of triangle formed by the first two connections and the dock
                // center
                2 => {
                    let centroid = (con.connections[0].dock2.center
                        + con.connections[1].dock2.center
                        + con.dock.center)
                        / 3.0;
                    centroid_angle_score_fn(centroid, con.dock.center, rt)
                },
                // Centroid angle of triangle formed by the first three connections
                3 => {
                    let centroid = (con.connections[0].dock2.center
                        + con.connections[1].dock2.center
                        + con.connections[2].dock2.center)
                        / 3.0;
                    centroid_angle_score_fn(centroid, con.dock.center, rt)
                },
                _ => 0.0,
            };
            let distance_score = distance_score_fn(rt.distance);
            // The 5.0 multiplier was established by trial and error. Without the
            // multiplier, the routes tend to have a long distance bias. Giving
            // the angle score more weight helps to balance the resulting route
            // network.
            angle_score *= 5.0;
            (angle_score, distance_score, angle_score + distance_score)
        };

        for _ in 0..4 {
            let mut best_trial: Option<(Vec<(Id<site::Site>, Id<site::Site>)>, f32)> = None;
            // 100 loops to shuffle the dock connections and try different combinations is
            // enough. Using 1000 loops doesn't improve the results.
            for _ in 0..100 {
                dock_connections.shuffle(&mut rng);
                let candidates = dock_connections
                    .iter()
                    .filter(|con| con.available_connections > 0)
                    .collect::<Vec<_>>();
                let mut trial = Vec::new();
                let mut trial_score = 0f32;
                for chunk in candidates.chunks(2) {
                    if let [con1, con2] = chunk {
                        let dock1_id = con1.dock.site_id;
                        let dock2_id = con2.dock.site_id;
                        let dock1_route = routes
                            .get(&DockConnectionHashKey(dock1_id, dock2_id))
                            .unwrap();
                        let dock2_route = routes
                            .get(&DockConnectionHashKey(dock2_id, dock1_id))
                            .unwrap();
                        let score1 = score_fn(con1, dock1_route);
                        let score2 = score_fn(con2, dock2_route);
                        trial_score += score1.2 + score2.2;
                        trial.push((dock1_id, dock2_id));
                    }
                }
                if let Some(current_best_trial) = best_trial.as_mut() {
                    if trial_score > current_best_trial.1 {
                        *current_best_trial = (trial, trial_score);
                    }
                } else {
                    best_trial = Some((trial, trial_score));
                }
            }
            if let Some(ref final_best_trial) = best_trial {
                for (site1, site2) in final_best_trial.0.iter() {
                    let dock1_route = routes.get(&DockConnectionHashKey(*site1, *site2)).unwrap();
                    let dock2_route = routes.get(&DockConnectionHashKey(*site2, *site1)).unwrap();
                    let con1 = dock_connections
                        .iter_mut()
                        .find(|con| con.dock.site_id == *site1)
                        .unwrap();
                    if con1.available_connections > 0 {
                        con1.add_connection(dock1_route);
                    }
                    let con2 = dock_connections
                        .iter_mut()
                        .find(|con| con.dock.site_id == *site2)
                        .unwrap();
                    if con2.available_connections > 0 {
                        con2.add_connection(dock2_route);
                    }
                }
            }
        }

        // The dock connections are now set.
        // At this point, we now have a network of airship routes between all the sites
        // with airship docks, and we have a list of docking positions for each
        // site. As airships are generated, they can be assigned a route based
        // on their home docking position and the airship routing network.
        // The number of airships per dock is determined by the number of connections at
        // the dock. The docking positions used at the dock can be random. Each
        // airship will have a route assigned that it will fly, out and back,
        // round trip between two sites. This needs to remain constant so that
        // travelers can know where the airship is going. The routes can be generated
        // before the airships, and when an airship is generated, the appropriate route
        // can be found by finding the docking position id for the docking position with
        // the wpos closest to the airship position. When an airship is is loaded from
        // saved RTSim data, the assigned routes will already be available. The airship
        // routes will be persisted in the rtsim data.

        let mut routes_added = DHashSet::<DockConnectionHashKey>::default();
        // keep track of the docking positions that have been used on either end of the
        // route.
        let mut used_docking_positions = DHashSet::<u32>::default();

        let mut random_dock_pos_fn =
            |dock: &AirshipDockPositions, used_positions: &DHashSet<u32>| {
                let mut dock_pos_index = rng.gen_range(0..dock.docking_positions.len());
                let begin = dock_pos_index;
                while used_positions.contains(&dock.docking_positions[dock_pos_index].0) {
                    dock_pos_index = (dock_pos_index + 1) % dock.docking_positions.len();
                    if dock_pos_index == begin {
                        return None;
                    }
                }
                Some(dock_pos_index)
            };

        let mut airship_route_id: u32 = 0;
        dock_connections.iter().for_each(|con| {
            con.connections.iter().for_each(|rt| {
                if !routes_added
                    .contains(&DockConnectionHashKey(rt.dock1.site_id, rt.dock2.site_id))
                {
                    if let Some(from_dock_pos_index) =
                        random_dock_pos_fn(rt.dock1, &used_docking_positions)
                    {
                        if let Some(to_dock_pos_index) =
                            random_dock_pos_fn(rt.dock2, &used_docking_positions)
                        {
                            let from_dock_pos_id =
                                rt.dock1.docking_positions[from_dock_pos_index].0;
                            let to_dock_pos_id = rt.dock2.docking_positions[to_dock_pos_index].0;
                            let approaches = Airships::airship_approaches_for_route(
                                world_sim,
                                rt,
                                from_dock_pos_id,
                                to_dock_pos_id,
                            );
                            let distance = rt.dock1.docking_positions[from_dock_pos_index]
                                .1
                                .xy()
                                .distance(rt.dock2.docking_positions[to_dock_pos_index].1.xy())
                                as u32;

                            self.routes.insert(
                                airship_route_id,
                                AirshipRoute::new(
                                    rt.dock1.site_id,
                                    rt.dock2.site_id,
                                    approaches,
                                    distance,
                                ),
                            );
                            airship_route_id += 1;

                            used_docking_positions.insert(from_dock_pos_id);
                            used_docking_positions.insert(to_dock_pos_id);
                            routes_added
                                .insert(DockConnectionHashKey(rt.dock1.site_id, rt.dock2.site_id));
                            routes_added
                                .insert(DockConnectionHashKey(rt.dock2.site_id, rt.dock1.site_id));
                        }
                    }
                }
            });
        });
    }

    /// Given a docking position, find the airship route and approach index
    /// where the approach endpoint is closest to the docking position.
    /// Return the route id (u32) and the approach index (0 or 1).
    pub fn airship_route_for_docking_pos(
        &self,
        docking_pos: Vec3<f32>,
    ) -> Option<(AirshipRouteId, usize)> {
        // Find the route where where either approach.dock_pos is equal (very close to)
        // the given docking_pos.
        if let Some((route_id, min_index, _)) = self
            .routes
            .iter()
            .flat_map(|(rt_id, rt)| {
                rt.approaches
                    .iter()
                    .enumerate()
                    .map(move |(index, approach)| {
                        let distance =
                            approach.dock_pos.1.xy().distance_squared(docking_pos.xy()) as i64;
                        (rt_id, index, distance)
                    })
            })
            .min_by_key(|(_, _, distance)| *distance)
        {
            Some((*route_id, min_index))
        } else {
            // It should be impossible to get here if
            // should_spawn_airship_at_docking_position is working correctly.
            warn!(
                "No airship route has a docking postion near {:?}",
                docking_pos
            );
            None
        }
    }

    /// Given a airship dock docking position, determine if an airship should be
    /// spawned at the docking position. Some airship docks will not have
    /// the docking positions completely filled because some docks are not
    /// connected to the maximum number of sites. E.g., if there are an odd
    /// number of sites with airship docks. Another reason is the way the
    /// routes are generated.
    pub fn should_spawn_airship_at_docking_position(
        &self,
        docking_pos: &Vec3<i32>,
        site_name: &str,
    ) -> bool {
        let use_docking_pos = self.routes.iter().any(|(_, rt)| {
            rt.approaches.iter().any(|approach| {
                approach
                    .dock_pos
                    .1
                    .xy()
                    .distance_squared(docking_pos.map(|i| i as f32).xy())
                    < 10.0
            })
        });
        if !use_docking_pos {
            debug_airships!(
                "Skipping docking position {:?} for site {}",
                docking_pos,
                site_name
            );
        }
        use_docking_pos
    }

    /// Get the position and direction for the airship to dock at the given
    /// docking position. If use_starboard_boarding is None, the side for
    /// boarding is randomly chosen. The center of the airship position with
    /// respect to the docking position is an asymmetrical offset depending on
    /// which side of the airship will be used for boarding and where the
    /// captain is located on the airship. The returned position is the
    /// position where the captain will be when the airship is docked
    /// (because the captain NPC is the one that is positioned in the agent
    /// or rtsim code).
    pub fn airship_vec_for_docking_pos(
        docking_pos: Vec3<f32>,
        airship_dock_center: Vec2<f32>,
        docking_side: Option<AirshipDockingSide>,
    ) -> (Vec3<f32>, Dir) {
        // choose a random side for docking if not specified
        let dock_side = docking_side.unwrap_or_else(|| {
            if thread_rng().gen::<bool>() {
                AirshipDockingSide::Starboard
            } else {
                AirshipDockingSide::Port
            }
        });
        // Get the vector from the dock alignment position on the airship to the
        // captain's position and the rotation angle for the ship to dock on the
        // specified side. The dock alignment position is where the airship
        // should touch or come closest to the dock. The side_rotation is the
        // angle the ship needs to rotate from to be perpendicular to the vector
        // from the dock center to the docking position. For example, if the docking
        // position is directly north (0 degrees, or aligned with the unit_y
        // vector), the ship needs to rotate 90 degrees CCW to dock on the port
        // side or 270 degrees CCW to dock on the starboard side.
        let (dock_align_to_captain, side_rotation) = if dock_side == AirshipDockingSide::Starboard {
            (
                Airships::DOCK_ALIGN_POS_STARBOARD,
                3.0 * std::f32::consts::FRAC_PI_2,
            )
        } else {
            (Airships::DOCK_ALIGN_POS_PORT, std::f32::consts::FRAC_PI_2)
        };
        // get the vector from the dock center to the docking platform point where the
        // airship should touch or come closest to.
        let dock_pos_offset = (docking_pos - airship_dock_center).xy();
        // The airship direction when docked is the dock_pos_offset rotated by the
        // side_rotation angle.
        let airship_dir =
            Dir::from_unnormalized(dock_pos_offset.rotated_z(side_rotation).with_z(0.0))
                .unwrap_or_default();
        // The dock_align_to_captain vector is rotated by the angle between unit_y and
        // the airship direction.
        let ship_dock_rotation =
            Airships::angle_between_vectors_ccw(Vec2::unit_y(), airship_dir.vec().xy());
        let captain_offset = dock_align_to_captain
            .rotated_z(ship_dock_rotation)
            .with_z(Airships::AIRSHIP_TO_DOCK_Z_OFFSET);

        // To get the location of the pilot when the ship is docked, add the
        // captain_offset to the docking position.
        (docking_pos + captain_offset, airship_dir)
    }

    // Get the docking approach for the given docking position.
    fn docking_approach_for(
        depart_center: Vec2<f32>,
        dest_center: Vec2<f32>,
        docking_pos: &AirshipDockingPosition,
        depart_to_dest_angle: f32,
        map_center: Vec2<f32>,
        max_dims: Vec2<f32>,
        site_id: Id<Site>,
    ) -> AirshipDockingApproach {
        let (airship_pos, airship_direction) = Airships::airship_vec_for_docking_pos(
            docking_pos.1,
            dest_center,
            Some(AirshipDockingSide::Starboard),
        );
        // calculate port final point. It is a 500 block extension from the docking
        // position in the direction of the docking direction.
        let port_final_pos = docking_pos.1.xy() + airship_direction.to_vec().xy() * 500.0;
        let starboard_final_pos = docking_pos.1.xy() - airship_direction.to_vec().xy() * 500.0;
        // calculate the turn angle required to align with the port. The port final
        // point is the origin. One vector is the reverse of the vector from the
        // port final point to the departure center. The other vector is from
        // the port final point to the docking position.
        let port_final_angle =
            (airship_pos.xy() - port_final_pos).angle_between(-(depart_center - port_final_pos));
        // The starboard angle is calculated similarly.
        let starboard_final_angle = (airship_pos.xy() - starboard_final_pos)
            .angle_between(-(depart_center - starboard_final_pos));

        // If the angles are approximately equal, it means the departure position and
        // the docking position are on the same line (angle near zero) or are
        // perpendicular to each other (angle near 90). If perpendicular, pick
        // the side where the final approach point is furthest from the edge of the map.
        // If on the same line, pick the side where the final approach point is closest
        // to the departure position.
        let side = if (port_final_angle - starboard_final_angle).abs() < 0.1 {
            // equal angles
            if port_final_angle < std::f32::consts::FRAC_PI_4 {
                // same line
                if port_final_pos.distance_squared(depart_center)
                    < starboard_final_pos.distance_squared(depart_center)
                {
                    // dock on port side
                    AirshipDockingSide::Port
                } else {
                    // dock on starboard side
                    AirshipDockingSide::Starboard
                }
            } else {
                // perpendicular
                // Use the final point closest to the center of the map.
                if port_final_pos.distance_squared(map_center)
                    < starboard_final_pos.distance_squared(map_center)
                {
                    // dock on port side
                    AirshipDockingSide::Port
                } else {
                    // dock on starboard side
                    AirshipDockingSide::Starboard
                }
            }
        } else {
            // pick the side with the least turn angle.
            if port_final_angle < starboard_final_angle {
                // port side
                AirshipDockingSide::Port
            } else {
                // starboard side
                AirshipDockingSide::Starboard
            }
        };

        let height = if depart_to_dest_angle < std::f32::consts::PI {
            Airships::STD_CRUISE_HEIGHT
        } else {
            Airships::STD_CRUISE_HEIGHT + 100.0
        };

        let check_pos_fn = |pos: Vec2<f32>, what: &str| {
            if pos.x < 0.0 || pos.y < 0.0 || pos.x > max_dims.x || pos.y > max_dims.y {
                warn!("{} pos out of bounds: {:?}", what, pos);
            }
        };

        let initial_pos_fn = |final_pos: Vec2<f32>| {
            // Get the angle between a line (1) connecting the final_pos and the
            // depart_center and line (2) from the final_pos to the docking
            // position. divide the angle in half then rotate line 1 CCW by that
            // angle + 270 degrees. The initial approach point is on this
            // rotated line 1, 500 blocks from the final_pos.
            let line1 = (depart_center - final_pos).normalized();
            let angle = line1.angle_between((airship_pos.xy() - final_pos).normalized());
            let initial_pos_line = line1.rotated_z(angle / 2.0 + 3.0 * std::f32::consts::FRAC_PI_2);
            let initial_pos = final_pos + initial_pos_line * 500.0;
            check_pos_fn(final_pos, "final_pos");
            check_pos_fn(initial_pos, "initial_pos");
            initial_pos
        };

        if side == AirshipDockingSide::Starboard {
            AirshipDockingApproach {
                dock_pos: *docking_pos,
                airship_pos,
                airship_direction,
                dock_center: dest_center,
                height,
                approach_initial_pos: initial_pos_fn(starboard_final_pos),
                approach_final_pos: starboard_final_pos,
                side,
                site_id,
            }
        } else {
            // recalculate the actual airship position and direction for the port side.
            let (airship_pos, airship_direction) = Airships::airship_vec_for_docking_pos(
                docking_pos.1,
                dest_center,
                Some(AirshipDockingSide::Port),
            );
            AirshipDockingApproach {
                dock_pos: *docking_pos,
                airship_pos,
                airship_direction,
                dock_center: dest_center,
                height,
                approach_initial_pos: initial_pos_fn(port_final_pos),
                approach_final_pos: port_final_pos,
                side,
                site_id,
            }
        }
    }

    /// Builds approaches for the given route connection.
    /// Each docking position has two possible approaches, based on the
    /// port and starboard sides of the airship. The approaches are aligned
    /// with the docking position direction, which is always perpendicular
    /// to the vector from the airship dock plot center to the docking position.
    /// The airship can pivot around the z axis, but it does so slowly. To
    /// ensure that the airship is oriented in the correct direction for
    /// landing, and to make it more realistic, the airship approaches
    /// the docking position pre-aligned with the landing direction. The
    /// approach consists of two positions, the initial point where the
    /// airship will turn toward the final point, at the final point it will
    /// turn toward the docking position and will be aligned with the docking
    /// direction.
    fn airship_approaches_for_route(
        world_sim: &mut WorldSim,
        route: &AirRouteConnection,
        dock1_position_id: u32,
        dock2_position_id: u32,
    ) -> [AirshipDockingApproach; 2] {
        /*  o Pick the docking side with the least rotation angle from the departure position.
              If the angles are approximately equal, it means the departure position and
              the docking position are on the same line (angle near zero) or are perpendicular to
              each other (angle near 90). If perpendicular, pick the side where the final approach
              point is furthest from the edge of the map. If on the same line, pick the side where
              the final approach point is closest to the departure position.
            o The cruising height above terrain is based on the angle between North and the
              line between the docking positions.
        */

        let map_size_chunks = world_sim.get_size().map(|u| u as i32);
        let max_dims = map_size_chunks.cpos_to_wpos().map(|u| u as f32);
        let map_center = Vec2::new(max_dims.x / 2.0, max_dims.y / 2.0);

        let dock1_positions = &route.dock1;
        let dock2_positions = &route.dock2;
        let dock1_center = dock1_positions.center;
        let dock2_center = dock2_positions.center;
        let docking_pos1 = dock1_positions
            .docking_positions
            .iter()
            .find(|dp| dp.0 == dock1_position_id)
            .unwrap();
        let docking_pos2 = dock2_positions
            .docking_positions
            .iter()
            .find(|dp| dp.0 == dock2_position_id)
            .unwrap();
        let dock1_to_dock2_angle = Airships::angle_between_vectors_ccw(
            Airships::ROUTES_NORTH,
            docking_pos2.1.xy() - docking_pos1.1.xy(),
        );
        let dock2_to_dock1_angle = std::f32::consts::TAU - dock1_to_dock2_angle;
        debug_airships!(
            "airship_approaches_for_route - dock1_pos:{:?}, dock2_pos:{:?}, \
             dock1_to_dock2_angle:{}, dock2_to_dock1_angle:{}",
            docking_pos1,
            docking_pos2,
            dock1_to_dock2_angle,
            dock2_to_dock1_angle
        );

        [
            Airships::docking_approach_for(
                dock1_center,
                dock2_center,
                docking_pos2,
                dock1_to_dock2_angle,
                map_center,
                max_dims,
                dock2_positions.site_id,
            ),
            Airships::docking_approach_for(
                dock2_center,
                dock1_center,
                docking_pos1,
                dock2_to_dock1_angle,
                map_center,
                max_dims,
                dock1_positions.site_id,
            ),
        ]
    }

    /// Returns the angle from vec v1 to vec v2 in the CCW direction.
    fn angle_between_vectors_ccw(v1: Vec2<f32>, v2: Vec2<f32>) -> f32 {
        let dot_product = v1.dot(v2);
        let det = v1.x * v2.y - v1.y * v2.x; // determinant
        let angle = det.atan2(dot_product); // atan2(det, dot_product) gives the CCW angle
        if angle < 0.0 {
            angle + std::f32::consts::TAU
        } else {
            angle
        }
    }

    /// Returns the angle from vec v1 to vec v2 in the CW direction.
    fn angle_between_vectors_cw(v1: Vec2<f32>, v2: Vec2<f32>) -> f32 {
        let ccw_angle = Airships::angle_between_vectors_ccw(v1, v2);
        std::f32::consts::TAU - ccw_angle
    }
}

/// For debuging the airship routes. Writes the airship routes to a json file.
fn write_airship_routes_log(file_path: &str, jsonstr: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;
    file.write_all(jsonstr.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AirshipDockingSide, Airships, approx::assert_relative_eq};
    use vek::{Quaternion, Vec2, Vec3};

    #[test]
    fn basic_vec_test() {
        let vec1 = Vec3::new(0.0f32, 10.0, 0.0);
        let vec2 = Vec3::new(10.0, 0.0, 0.0);
        let a12 = vec2.angle_between(vec1);
        assert_relative_eq!(a12, std::f32::consts::FRAC_PI_2, epsilon = 0.00001);

        let rotc2 = Quaternion::rotation_z(a12);
        let vec3 = rotc2 * vec2;
        assert!(vec3 == vec1);
    }

    #[test]
    fn std_vec_angles_test() {
        let refvec = Vec2::new(0.0f32, 10.0);

        let vec1 = Vec2::new(0.0f32, 10.0);
        let vec2 = Vec2::new(10.0f32, 0.0);
        let vec3 = Vec2::new(0.0f32, -10.0);
        let vec4 = Vec2::new(-10.0f32, 0.0);

        let a1r = vec1.angle_between(refvec);
        assert!(a1r == 0.0f32);

        let a2r = vec2.angle_between(refvec);
        assert_relative_eq!(a2r, std::f32::consts::FRAC_PI_2, epsilon = 0.00001);

        let a3r: f32 = vec3.angle_between(refvec);
        assert_relative_eq!(a3r, std::f32::consts::PI, epsilon = 0.00001);

        let a4r = vec4.angle_between(refvec);
        assert_relative_eq!(a4r, std::f32::consts::FRAC_PI_2, epsilon = 0.00001);
    }

    #[test]
    fn vec_angles_test() {
        let refvec = Vec3::new(0.0f32, 10.0, 0.0);

        let vec1 = Vec3::new(0.0f32, 10.0, 0.0);
        let vec2 = Vec3::new(10.0f32, 0.0, 0.0);
        let vec3 = Vec3::new(0.0f32, -10.0, 0.0);
        let vec4 = Vec3::new(-10.0f32, 0.0, 0.0);

        let a1r = vec1.angle_between(refvec);
        let a1r3 = Airships::angle_between_vectors_ccw(vec1.xy(), refvec.xy());
        assert!(a1r == 0.0f32);
        assert!(a1r3 == 0.0f32);

        let a2r = vec2.angle_between(refvec);
        let a2r3 = Airships::angle_between_vectors_ccw(vec2.xy(), refvec.xy());
        assert_relative_eq!(a2r, std::f32::consts::FRAC_PI_2, epsilon = 0.00001);
        assert_relative_eq!(a2r3, std::f32::consts::FRAC_PI_2, epsilon = 0.00001);

        let a3r: f32 = vec3.angle_between(refvec);
        let a3r3 = Airships::angle_between_vectors_ccw(vec3.xy(), refvec.xy());
        assert_relative_eq!(a3r, std::f32::consts::PI, epsilon = 0.00001);
        assert_relative_eq!(a3r3, std::f32::consts::PI, epsilon = 0.00001);

        let a4r = vec4.angle_between(refvec);
        let a4r3 = Airships::angle_between_vectors_ccw(vec4.xy(), refvec.xy());
        assert_relative_eq!(a4r, std::f32::consts::FRAC_PI_2, epsilon = 0.00001);
        assert_relative_eq!(a4r3, std::f32::consts::FRAC_PI_2 * 3.0, epsilon = 0.00001);
    }

    #[test]
    fn airship_angles_test() {
        let refvec = Vec2::new(0.0f32, 37.0);
        let ovec = Vec2::new(-4.0f32, -14.0);
        let oveccw0 = Vec2::new(-4, -14);
        let oveccw90 = Vec2::new(-14, 4);
        let oveccw180 = Vec2::new(4, 14);
        let oveccw270 = Vec2::new(14, -4);
        let ovecccw0 = Vec2::new(-4, -14);
        let ovecccw90 = Vec2::new(14, -4);
        let ovecccw180 = Vec2::new(4, 14);
        let ovecccw270 = Vec2::new(-14, 4);

        let vec1 = Vec2::new(0.0f32, 37.0);
        let vec2 = Vec2::new(37.0f32, 0.0);
        let vec3 = Vec2::new(0.0f32, -37.0);
        let vec4 = Vec2::new(-37.0f32, 0.0);

        assert!(
            ovec.rotated_z(Airships::angle_between_vectors_cw(vec1, refvec))
                .map(|x| x.round() as i32)
                == oveccw0
        );
        assert!(
            ovec.rotated_z(Airships::angle_between_vectors_cw(vec2, refvec))
                .map(|x| x.round() as i32)
                == oveccw90
        );
        assert!(
            ovec.rotated_z(Airships::angle_between_vectors_cw(vec3, refvec))
                .map(|x| x.round() as i32)
                == oveccw180
        );
        assert!(
            ovec.rotated_z(Airships::angle_between_vectors_cw(vec4, refvec))
                .map(|x| x.round() as i32)
                == oveccw270
        );

        assert!(
            ovec.rotated_z(Airships::angle_between_vectors_ccw(vec1, refvec))
                .map(|x| x.round() as i32)
                == ovecccw0
        );
        assert!(
            ovec.rotated_z(Airships::angle_between_vectors_ccw(vec2, refvec))
                .map(|x| x.round() as i32)
                == ovecccw90
        );
        assert!(
            ovec.rotated_z(Airships::angle_between_vectors_ccw(vec3, refvec))
                .map(|x| x.round() as i32)
                == ovecccw180
        );
        assert!(
            ovec.rotated_z(Airships::angle_between_vectors_ccw(vec4, refvec))
                .map(|x| x.round() as i32)
                == ovecccw270
        );
    }

    #[test]
    fn airship_vec_test() {
        {
            let dock_pos = Vec3::new(10.0f32, 10.0, 0.0);
            let airship_dock_center = Vec2::new(0.0, 0.0);
            let mut left_tested = false;
            let mut right_tested = false;
            {
                for _ in 0..1000 {
                    let (airship_pos, airship_dir) =
                        Airships::airship_vec_for_docking_pos(dock_pos, airship_dock_center, None);
                    if airship_pos.x > 23.0 {
                        assert_relative_eq!(
                            airship_pos,
                            Vec3 {
                                x: 23.435028,
                                y: 22.020815,
                                z: -3.0
                            },
                            epsilon = 0.00001
                        );
                        assert_relative_eq!(
                            airship_dir.to_vec(),
                            Vec3 {
                                x: -0.70710677,
                                y: 0.70710677,
                                z: 0.0
                            },
                            epsilon = 0.00001
                        );
                        left_tested = true;
                    } else {
                        assert_relative_eq!(
                            airship_pos,
                            Vec3 {
                                x: 22.020815,
                                y: 23.435028,
                                z: -3.0
                            },
                            epsilon = 0.00001
                        );
                        assert_relative_eq!(
                            airship_dir.to_vec(),
                            Vec3 {
                                x: 0.70710677,
                                y: -0.70710677,
                                z: 0.0
                            },
                            epsilon = 0.00001
                        );
                        right_tested = true;
                    }
                    if left_tested && right_tested {
                        break;
                    }
                }
            }
            {
                let (airship_pos, airship_dir) = Airships::airship_vec_for_docking_pos(
                    dock_pos,
                    airship_dock_center,
                    Some(AirshipDockingSide::Port),
                );
                assert_relative_eq!(
                    airship_pos,
                    Vec3 {
                        x: 23.435028,
                        y: 22.020815,
                        z: -3.0
                    },
                    epsilon = 0.00001
                );
                assert_relative_eq!(
                    airship_dir.to_vec(),
                    Vec3 {
                        x: -0.70710677,
                        y: 0.70710677,
                        z: 0.0
                    },
                    epsilon = 0.00001
                );
            }
            {
                let (airship_pos, airship_dir) = Airships::airship_vec_for_docking_pos(
                    dock_pos,
                    airship_dock_center,
                    Some(AirshipDockingSide::Starboard),
                );
                assert_relative_eq!(
                    airship_pos,
                    Vec3 {
                        x: 22.020815,
                        y: 23.435028,
                        z: -3.0
                    },
                    epsilon = 0.00001
                );
                assert_relative_eq!(
                    airship_dir.to_vec(),
                    Vec3 {
                        x: 0.70710677,
                        y: -0.70710677,
                        z: 0.0
                    },
                    epsilon = 0.00001
                );
            }
        }
        {
            let dock_pos = Vec3::new(28874.0, 18561.0, 0.0);
            let airship_dock_center = Vec2::new(28911.0, 18561.0);
            {
                let (airship_pos, airship_dir) = Airships::airship_vec_for_docking_pos(
                    dock_pos,
                    airship_dock_center,
                    Some(AirshipDockingSide::Port),
                );
                assert_relative_eq!(
                    airship_pos,
                    Vec3 {
                        x: 28856.0,
                        y: 18562.0,
                        z: -3.0
                    },
                    epsilon = 0.00001
                );
                assert_relative_eq!(
                    airship_dir.to_vec(),
                    Vec3 {
                        x: 4.371139e-8,
                        y: -1.0,
                        z: 0.0
                    },
                    epsilon = 0.00001
                );
            }
            {
                let (airship_pos, airship_dir) = Airships::airship_vec_for_docking_pos(
                    dock_pos,
                    airship_dock_center,
                    Some(AirshipDockingSide::Starboard),
                );
                assert_relative_eq!(
                    airship_pos,
                    Vec3 {
                        x: 28856.0,
                        y: 18560.0,
                        z: -3.0
                    },
                    epsilon = 0.00001
                );
                assert_relative_eq!(
                    airship_dir.to_vec(),
                    Vec3 {
                        x: -1.1924881e-8,
                        y: 1.0,
                        z: 0.0
                    },
                    epsilon = 0.00001
                );
            }
        }
    }

    #[test]
    fn angle_score_test() {
        let rt_angles = [
            0.0,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            std::f32::consts::FRAC_PI_2 * 3.0,
        ];
        let con_angles = [
            0.0,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            std::f32::consts::FRAC_PI_2 * 3.0,
        ];
        let scores = [
            [0.0, 2.5, 5.0, 2.5],
            [2.5, 0.0, 2.5, 5.0],
            [5.0, 2.5, 0.0, 2.5],
            [2.5, 5.0, 2.5, 0.0],
        ];
        let score_fn2 = |a1: f32, a2: f32| {
            let optimal_angle = (a1 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
            let angle_diff = (optimal_angle - a2)
                .abs()
                .min(std::f32::consts::TAU - (optimal_angle - a2).abs());
            (1.0 - (angle_diff / std::f32::consts::PI)) * 5.0
        };
        let mut i = 0;
        let mut j = 0;
        rt_angles.iter().for_each(|rt_angle| {
            j = 0;
            con_angles.iter().for_each(|con_angle| {
                let score = score_fn2(*con_angle, *rt_angle);
                assert_relative_eq!(score, scores[i][j], epsilon = 0.00001);
                j += 1;
            });
            i += 1;
        });
    }

    #[test]
    fn distance_score_test() {
        let distances = [1, 1000, 5001, 6000, 15000, 30000, 48000];
        let scores = [
            0.0,
            0.0,
            -0.20026308,
            0.66321766,
            1.0257597,
            1.2102475,
            1.329906,
        ];
        let score_fn = |distance: i64| {
            if distance > 25000000 {
                (((distance - 24999000) / 1000) as f32).ln() / 8.0 - 0.5
            } else {
                0.0
            }
        };
        let mut i = 0;
        distances.iter().for_each(|distance| {
            let dist2 = *distance * *distance;
            let score = score_fn(dist2);
            assert_relative_eq!(score, scores[i], epsilon = 0.00001);
            i += 1;
        });
    }
}
