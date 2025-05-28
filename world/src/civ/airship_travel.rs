use crate::{
    Index,
    civ::airship_route_map::*,
    sim::WorldSim,
    site::{self, Site, plot::PlotKindMeta},
    util::{DHashMap, DHashSet, seed_expan},
};
use common::{
    store::{Id, Store},
    terrain::{CoordinateConversions, TERRAIN_CHUNK_BLOCKS_LG},
    util::Dir,
};
use delaunator::{Point, Triangulation, triangulate};
use itertools::Itertools;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::{fs::OpenOptions, io::Write};
use tracing::{error, warn};
use vek::*;

const AIRSHIP_TRAVEL_DEBUG: bool = false;

macro_rules! debug_airships {
    ($($arg:tt)*) => {
        if AIRSHIP_TRAVEL_DEBUG {
            println!($($arg)*);
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
    /// site\[0\] is the home site, site\[1\] is the away site.
    pub sites: [Id<site::Site>; 2],
    /// approaches\[0\] is flying from the home site to the away site.
    /// approaches\[1\] is flying from the away site to the home site.
    pub approaches: [AirshipDockingApproach; 2],
    /// The distance between the two sites.
    pub distance: u32,
}

impl AirshipRoute {
    fn new(
        site1: Id<site::Site>,
        site: Id<site::Site>,
        approaches: [AirshipDockingApproach; 2],
        distance: u32,
    ) -> Self {
        Self {
            sites: [site1, site],
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockNode {
    pub node_id: usize,
    pub on_hull: bool,
    //pub connected: DHashSet<usize>,
    pub connected: Vec<usize>,
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
    fn all_airshipdock_positions(sites: &Store<Site>) -> Vec<AirshipDockPositions> {
        let mut dock_pos_id = 0;
        sites
            .iter()
            .flat_map(|(site_id, site)| {
                site.plots().flat_map(move |plot| {
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
    pub fn generate_airship_routes(&mut self, world_sim: &mut WorldSim, index: &Index) {
        let all_docking_positions = Airships::all_airshipdock_positions(&index.sites);
        // Create a map of all possible dock to dock connections.
        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(index.seed));
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
                for (site1, site) in final_best_trial.0.iter() {
                    let dock1_route = routes.get(&DockConnectionHashKey(*site1, *site)).unwrap();
                    let dock2_route = routes.get(&DockConnectionHashKey(*site, *site1)).unwrap();
                    let con1 = dock_connections
                        .iter_mut()
                        .find(|con| con.dock.site_id == *site1)
                        .unwrap();
                    if con1.available_connections > 0 {
                        con1.add_connection(dock1_route);
                    }
                    let con2 = dock_connections
                        .iter_mut()
                        .find(|con| con.dock.site_id == *site)
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
        if AIRSHIP_TRAVEL_DEBUG {
            save_airship_routes_map(self, index, world_sim);
        }
    }

    pub fn generate_airship_routes2(&mut self, world_sim: &mut WorldSim, index: &Index) {
        let all_dock_positions = Airships::all_airshipdock_positions(&index.sites);
        let all_dock_points = all_dock_positions
            .iter()
            .map(|dock| Point {
                x: dock.center.x as f64,
                y: dock.center.y as f64,
            })
            .collect::<Vec<_>>();
        debug_airships!("all_dock_points: {:?}", all_dock_points);

        let triangulation = triangulate(&all_dock_points);
        save_airship_routes_triangulation(&triangulation, &all_dock_points, index, world_sim);

        // Docking positions are specified in world coordinates, not chunks.
        // Limit the max route leg length to 1000 chunks no matter the world size.
        let world_chunks = world_sim.map_size_lg().chunks();
        let blocks_per_chunk = 1 << TERRAIN_CHUNK_BLOCKS_LG;
        let world_blocks = world_chunks.map(|u| u as f32) * blocks_per_chunk as f32;
        let max_route_leg_length = 1000.0 * world_blocks.x;
        println!(
            "blocks_per_chunk: {}, world size in blocks: {:?}, max_route_leg_length: {}",
            blocks_per_chunk, world_blocks, max_route_leg_length
        );

        // eulerized_route_segments is fairly expensive as the number of docking sites
        // grows. Limit the number of iterations according to world size.
        // pow2     world size   iterations
        // 10       1024         50
        // 11       2048         22
        // 12       4096         10
        // 13       8192          2
        // Doing a least squares fit on the iterations gives the formula:
        // 3742931.0 * e.powf(-1.113823 * pow2)
        // 3742931.0 * 2.71828f32.powf(-1.113823 * pow2)

        let pow2 = world_sim.map_size_lg().vec().x;
        let max_iterations = (3742931.0 * std::f32::consts::E.powf(-1.113823 * pow2 as f32))
            .clamp(1.0, 100.0)
            .round() as usize;

        if let Some((best_segments, _, max_seg_len, min_spread, iteration)) = triangulation
            .eulerized_route_segments(
                &all_dock_points,
                max_iterations,
                max_route_leg_length as f64,
                index.seed,
            )
        {
            if cfg!(debug_assertions) {
                println!("Max segment length: {}", max_seg_len);
                println!("Min spread: {}", min_spread);
                println!("Iteration: {}", iteration);
                println!("Segments count:");
                best_segments.iter().enumerate().for_each(|segment| {
                    println!("  {} : {}", segment.0, segment.1.len());
                });
            }
            save_airship_route_segments(&best_segments, &all_dock_points, index, world_sim);
        } else {
            println!("Error - cannot eulerize the dock points.");
        }
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
        // debug_airships!(
        //     "airship_approaches_for_route - dock1_pos:{:?}, dock2_pos:{:?}, \
        //      dock1_to_dock2_angle:{}, dock2_to_dock1_angle:{}",
        //     docking_pos1,
        //     docking_pos2,
        //     dock1_to_dock2_angle,
        //     dock2_to_dock1_angle
        // );

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
    pub fn angle_between_vectors_ccw(v1: Vec2<f32>, v2: Vec2<f32>) -> f32 {
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
    pub fn angle_between_vectors_cw(v1: Vec2<f32>, v2: Vec2<f32>) -> f32 {
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

#[derive(Debug, Clone)]
enum EdgeRemovalStackNodeType {
    Find,
    Check,
    Undo,
}

const DEBUG_AIRSHIP_EULERIZATION: bool = false;

macro_rules! debug_airship_eulerization {
    ($($arg:tt)*) => {
        if DEBUG_AIRSHIP_EULERIZATION {
            println!($($arg)*);
        }
    }
}

type DockNodeGraph = DHashMap<usize, DockNode>;

trait TriangulationExt {
    fn count_edges_per_node(&self) -> DHashMap<usize, usize>;
    fn all_edges(&self) -> DHashSet<(usize, usize)>;
    fn connected_nodes(&self, node: usize) -> Vec<usize>;
    fn is_hull_node(&self, index: usize) -> bool;
    fn is_hull_edge(&self, edge: (usize, usize)) -> bool;
    fn node_connections(&self) -> DockNodeGraph;
    fn eulerized_route_segments(
        &self,
        all_dock_points: &[Point],
        iterations: usize,
        max_route_leg_length: f64,
        seed: u32,
    ) -> Option<(Vec<Vec<usize>>, Vec<usize>, usize, f32, usize)>;
}

fn first_odd_node(
    search_order: &[usize],
    start: usize,
    nodes: &DockNodeGraph,
) -> Option<(usize, usize)> {
    search_order
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, &node_index)| {
            if let Some(dock_node) = nodes.get(&node_index) {
                if dock_node.connected.len() % 2 == 1 {
                    Some((index, node_index))
                } else {
                    None
                }
            } else {
                None
            }
        })
}

/// Removes an edge between two nodes in the tesselation graph.
fn remove_edge(edge: (usize, usize), nodes: &mut DockNodeGraph) {
    if let Some(dock_node) = nodes.get_mut(&edge.0) {
        // Remove the edge from node_id1 to node_id2.
        // The edge may be present more than once, just remove one instance.
        if let Some(index) = dock_node
            .connected
            .iter()
            .position(|&node_id| node_id == edge.1)
        {
            dock_node.connected.remove(index);
        }
    }
    if let Some(dock_node) = nodes.get_mut(&edge.1) {
        // Remove the edge from node_id2 to node_id1.
        // The edge may be present more than once, just remove one instance.
        if let Some(index) = dock_node
            .connected
            .iter()
            .position(|&node_id| node_id == edge.0)
        {
            dock_node.connected.remove(index);
        }
    }
}

/// Adds an edge between two nodes in the tesselation graph.
fn add_edge(edge: (usize, usize), nodes: &mut DockNodeGraph) {
    if let Some(dock_node) = nodes.get_mut(&edge.0) {
        dock_node.connected.push(edge.1);
    }
    if let Some(dock_node) = nodes.get_mut(&edge.1) {
        dock_node.connected.push(edge.0);
    }
}

// Gives a score for line segments lengths as compared to the hull diameter.
// At 0 length, the score is 0.0.
// At 0.7 of the hull diameter, the score is 0.9.
// At 1.0 of the hull diameter, the score is 1.0.
fn hull_ratio_distance_score(rt_len: f64, hull_diameter: f64) -> f64 {
    if hull_diameter == 0.0 {
        return 0.0;
    }
    let ratio = rt_len / hull_diameter;
    // ratio    score
    // 0.0   0.0
    // 0.7   0.9
    // 1.0   1.0
    1.0 - (-3.2894 * ratio).exp()
}

impl TriangulationExt for Triangulation {
    fn count_edges_per_node(&self) -> DHashMap<usize, usize> {
        let mut edge_count_map = DHashMap::default();

        for &node in &self.triangles {
            *edge_count_map.entry(node).or_insert(0) += 1;
        }

        edge_count_map
    }

    fn connected_nodes(&self, node: usize) -> Vec<usize> {
        self.triangles
            .chunks(3)
            .filter(|&t| t.contains(&node))
            .map(|t| {
                if t[0] == node {
                    t[1]
                } else if t[1] == node {
                    t[2]
                } else {
                    t[0]
                }
            })
            .collect::<Vec<_>>()
    }

    fn all_edges(&self) -> DHashSet<(usize, usize)> {
        let mut edges = DHashSet::default();
        for t in self.triangles.chunks(3) {
            let a = t[0];
            let b = t[1];
            let c = t[2];
            // The edges hashset must have edges specified in increasing order to avoid
            // duplicates.
            edges.insert(if a < b { (a, b) } else { (b, a) });
            edges.insert(if b < c { (b, c) } else { (c, b) });
            edges.insert(if a < c { (a, c) } else { (c, a) });
        }
        edges
    }

    fn node_connections(&self) -> DockNodeGraph {
        let mut connections = DHashMap::default();

        // struct DockNode {
        //     pub node_index: usize,
        //     pub on_hull: bool,
        //     pub connected: DHashSet<usize>,
        // }
        self.triangles.chunks(3).for_each(|t| {
            for &node in t {
                let dock_node = connections.entry(node).or_insert_with(|| DockNode {
                    node_id: node,
                    on_hull: self.is_hull_node(node),
                    //connected: DHashSet::default(),
                    connected: Vec::default(),
                });
                for &connected_node in t {
                    if connected_node != node && !dock_node.connected.contains(&connected_node) {
                        dock_node.connected.push(connected_node);
                    }
                }
            }
        });
        for (_, dock_node) in connections.iter_mut() {
            dock_node.connected = dock_node.connected.to_vec();
        }
        connections
    }

    fn is_hull_node(&self, index: usize) -> bool { self.hull.contains(&index) }

    fn is_hull_edge(&self, edge: (usize, usize)) -> bool {
        // the hull is a vector of indices that reference points on the convex hull of
        // the triangulation, counter-clockwise. The hull is a closed loop, so
        // the last point is connected to the first point. The given edge from
        // edge.0 to edge.1 is on the hull if edge.0 is in the hull and edge.1 is either
        // the next point in the hull or the previous point in the hull.
        // pub hull: Vec<usize>,

        let hull_len = self.hull.len();
        if let Some(hull_index) = self.hull.iter().position(|&i| i == edge.0) {
            let next_hull_index = (hull_index + 1) % hull_len;
            if self.hull[next_hull_index] == edge.1 {
                return true;
            }
            let prev_hull_index = if hull_index == 0 {
                hull_len - 1
            } else {
                hull_index - 1
            };
            self.hull[prev_hull_index] == edge.1
        } else {
            false
        }
    }

    fn eulerized_route_segments(
        &self,
        all_dock_points: &[Point],
        iterations: usize,
        max_route_leg_length: f64,
        seed: u32,
    ) -> Option<(Vec<Vec<usize>>, Vec<usize>, usize, f32, usize)> {
        // let node_connections = self.node_connections();
        let mut edges_to_remove = DHashSet::default();

        // There can be at most four incoming and four outgoing edges per node because
        // there are only four docking positions per docking site and for deconfliction
        // purposes, no two "routes" can use the same docking position. This means that
        // the maximum number of edges per node is 8. Remove the shortest edges from
        // nodes with more than 8 edges.

        // The tessellation algorithm produces convex hull, and there can be edges
        // connecting outside nodes where the distance between the points is a
        // significant fraction of the hull diameter. We want to keep airship
        // route legs as short as possible, while not removing interior edges
        // that may already be fairly long due to the configuration of the
        // docking sites relative to the entire map. For the standard world map,
        // with 2^10 chunks (1024x1024), the hull diameter is about 1000 chunks.
        // Experimentally, the standard world map can have interior edges that are
        // around 800 chunks long. A world map with 2^12 chunks (4096x4096) can
        // have hull edges that are around 2000 chunks long, but interior edges
        // still have a max of around 800 chunks. For the larger world maps,
        // removing edges that are longer than 1000 chunks is a good heuristic.

        // First, use these heuristics to remove excess edges from the node graph.
        // 1. remove edges that are longer than 1000 blocks.
        // 2. remove the shortest edges from nodes with more than 8 edges

        let max_distance_squared = max_route_leg_length.powi(2);

        let all_edges = self.all_edges();
        for edge in all_edges.iter() {
            let pt1 = &all_dock_points[edge.0];
            let pt2 = &all_dock_points[edge.1];
            let v1 = Vec2 { x: pt1.x, y: pt1.y };
            let v2 = Vec2 { x: pt2.x, y: pt2.y };
            // Remove the edge if the distance between the points is greater than
            // max_leg_length
            if v1.distance_squared(v2) > max_distance_squared {
                edges_to_remove.insert(*edge);
            }
        }

        #[cfg(debug_assertions)]
        let long_edges = edges_to_remove.len();
        debug_airship_eulerization!(
            "Found {} long edges to remove out of {} total edges",
            edges_to_remove.len(),
            all_edges.len()
        );

        let node_connections = self.node_connections();
        node_connections.iter().for_each(|(&node_id, node)| {
            if node.connected.len() > 8 {
                let excess_edges_count = node.connected.len() - 8;
                // Find the shortest edge and remove it
                let mut connected_node_info = node
                    .connected
                    .iter()
                    .map(|&connected_node_id| {
                        let pt1 = &all_dock_points[node_id];
                        let pt2 = &all_dock_points[connected_node_id];
                        let v1 = Vec2 { x: pt1.x, y: pt1.y };
                        let v2 = Vec2 { x: pt2.x, y: pt2.y };
                        (connected_node_id, v1.distance_squared(v2) as i64)
                    })
                    .collect::<Vec<_>>();
                connected_node_info.sort_by(|a, b| a.1.cmp(&b.1));
                let mut excess_edges_remaining = excess_edges_count;
                let mut remove_index = 0;
                while excess_edges_remaining > 0 && remove_index < connected_node_info.len() {
                    let (connected_node_id, _) = connected_node_info[remove_index];
                    let edge = if node_id < connected_node_id {
                        (node_id, connected_node_id)
                    } else {
                        (connected_node_id, node_id)
                    };
                    if !edges_to_remove.contains(&edge) {
                        edges_to_remove.insert(edge);
                        excess_edges_remaining -= 1;
                    }
                    remove_index += 1;
                }
            }
        });

        let mut mutable_node_connections = node_connections.clone();

        if DEBUG_AIRSHIP_EULERIZATION {
            debug_airship_eulerization!(
                "Removing {} long edges and {} excess edges for a total of {} removed edges out \
                 of a total of {} edges",
                long_edges,
                edges_to_remove.len() - long_edges,
                edges_to_remove.len(),
                all_edges.len(),
            );
        }

        for edge in edges_to_remove {
            remove_edge(edge, &mut mutable_node_connections);
        }

        if DEBUG_AIRSHIP_EULERIZATION {
            // count the number of nodes with an odd connected count
            let odd_connected_count0 = mutable_node_connections
                .iter()
                .filter(|(_, node)| node.connected.len() % 2 == 1)
                .count();
            let total_connections1 = mutable_node_connections
                .iter()
                .map(|(_, node)| node.connected.len())
                .sum::<usize>();
            debug_airship_eulerization!(
                "After Removing, odd connected count: {} in {} nodes, total connections: {}",
                odd_connected_count0,
                mutable_node_connections.len(),
                total_connections1
            );
        }

        // Now eurlerize the node graph by adding edges to connect nodes with an odd
        // number of connections. Eurlerization means that every node will have
        // an even number of degrees (edges), which is a requirement for
        // creating a Eulerian Circuit.

        // Get the keys (node ids, which is the same as the node's index in the
        // all_dock_points vector) of nodes with an odd number of edges.
        let mut odd_keys: Vec<usize> = mutable_node_connections
            .iter()
            .filter(|(_, node)| node.connected.len() % 2 == 1)
            .map(|(node_id, _)| *node_id)
            .collect();

        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));

        // There will always be an even number of odd nodes in a connected graph (one
        // where all nodes are reachable from any other node). The goal is to
        // pair the odd nodes, adding edges between each pair such that the
        // added edges are as short as possible. After adding edges, the graph
        // will have an even number of edges for each node.

        // The starting node index for finding pairs is arbitrary, and starting from
        // different nodes will yield different Eulerian circuits.

        // Do a number of iterations and find the best results. The criteria is
        // 1. The number of route groups (the outer Vec in best_route_segments) This
        //    will be a maximum of 4 because there are at most 4 docking positions per
        //    docking site. More is better.
        // 2. The 'spread' of the lengths of the inner Vecs in best_route_segments. The
        //    calculated spread is the standard deviation of the lengths. Smaller is
        //    better (more uniform lengths of the route groups.)
        let mut best_circuit = Vec::new();
        let mut best_route_segments = Vec::new();
        let mut best_max_seg_len = 0;
        let mut best_min_spread = f32::MAX;
        let mut best_iteration = 0;

        for i in 0..iterations {
            // Deterministically randomize the node order to search for the best route
            // segments.
            let mut eulerized_node_connections = mutable_node_connections.clone();

            let mut odd_connected_count = odd_keys.len();
            assert!(
                odd_connected_count % 2 == 0,
                "Odd connected count should be even, got {}",
                odd_connected_count
            );
            assert!(
                odd_keys.len()
                    == eulerized_node_connections
                        .iter()
                        .filter(|(_, node)| node.connected.len() % 2 == 1)
                        .count()
            );

            // It's possible that the graphs starts with no odd nodes after removing edges
            // above.
            if odd_connected_count > 0 {
                odd_keys.shuffle(&mut rng);

                // The edges to be added. An edge is a tuple of two node ids/indices.
                let mut edges_to_add = DHashSet::default();
                // Each odd node will be paired with only one other odd node.
                // Keep track of which nodes have been paired already.
                let mut paired_odd_nodes = DHashSet::default();

                for node_key in odd_keys.iter() {
                    // Skip nodes that are already paired.
                    if paired_odd_nodes.contains(node_key) {
                        continue;
                    }
                    if let Some(node) = mutable_node_connections.get(node_key) {
                        // find the closest node other than nodes that are already connected to
                        // this one.
                        let mut closest_node_id = None;
                        let mut closest_distance = f64::MAX;
                        for candidate_key in odd_keys.iter() {
                            // Skip nodes that are already paired.
                            if paired_odd_nodes.contains(candidate_key) {
                                continue;
                            }
                            if let Some(candidate_node) =
                                mutable_node_connections.get(candidate_key)
                            {
                                // Skip the node itself and nodes that are already connected to this
                                // node.
                                if *candidate_key != *node_key
                                    && !node.connected.contains(candidate_key)
                                    && !candidate_node.connected.contains(node_key)
                                {
                                    // make sure the edge is specified in increasing node index
                                    // order to
                                    // avoid duplicates.
                                    let edge_to_add = if *node_key < *candidate_key {
                                        (*node_key, *candidate_key)
                                    } else {
                                        (*candidate_key, *node_key)
                                    };
                                    // Skip the edge if it is already in the edges_to_add set.
                                    if !edges_to_add.contains(&edge_to_add) {
                                        let pt1 = &all_dock_points[*node_key];
                                        let pt2 = &all_dock_points[*candidate_key];
                                        let v1 = Vec2 { x: pt1.x, y: pt1.y };
                                        let v2 = Vec2 { x: pt2.x, y: pt2.y };
                                        let distance = v1.distance_squared(v2);
                                        if distance < closest_distance {
                                            closest_distance = distance;
                                            closest_node_id = Some(*candidate_key);
                                        }
                                    }
                                }
                            }
                        }
                        // It's possible that the only odd nodes remaining are already connected to
                        // this node, but we still need to pair them. In
                        // this case, the connections become bidirectional,
                        // but that's okay for Eulerization and airships will still only follow each
                        // other in one direction.
                        if closest_node_id.is_none() {
                            // If no suitable node was found, repeat the search but allow
                            // connecting to nodes that are already connected to this one.
                            for candidate_key in odd_keys.iter() {
                                // Skip nodes that are already paired.
                                if paired_odd_nodes.contains(candidate_key) {
                                    continue;
                                }
                                // Skip the node itself
                                if *candidate_key != *node_key {
                                    // make sure the edge is specified in increasing node index
                                    // order to
                                    // avoid duplicates.
                                    let edge_to_add = if *node_key < *candidate_key {
                                        (*node_key, *candidate_key)
                                    } else {
                                        (*candidate_key, *node_key)
                                    };
                                    // Skip the edge if it is already in the edges_to_add set.
                                    if !edges_to_add.contains(&edge_to_add) {
                                        let pt1 = &all_dock_points[*node_key];
                                        let pt2 = &all_dock_points[*candidate_key];
                                        let v1 = Vec2 { x: pt1.x, y: pt1.y };
                                        let v2 = Vec2 { x: pt2.x, y: pt2.y };
                                        let distance = v1.distance_squared(v2);
                                        if distance < closest_distance {
                                            closest_distance = distance;
                                            closest_node_id = Some(*candidate_key);
                                        }
                                    }
                                }
                            }
                        }
                        // If a closest node was found that is not already paired, add the edge.
                        // Note that this should not fail since we are guaranteed to have
                        // an even number of odd nodes.
                        if let Some(close_node_id) = closest_node_id {
                            // add the edge between node_id and closest_node_id
                            let edge_to_add = if *node_key < close_node_id {
                                (*node_key, close_node_id)
                            } else {
                                (close_node_id, *node_key)
                            };
                            edges_to_add.insert(edge_to_add);
                            paired_odd_nodes.insert(*node_key);
                            paired_odd_nodes.insert(close_node_id);
                        } else {
                            error!("Cannot pair all odd nodes, this should not happen.");
                        }
                    }
                    if edges_to_add.len() == odd_connected_count / 2 {
                        // If we have paired all odd nodes, break out of the loop.
                        // The break is necessary because the outer loop iterates over
                        // all odd keys but we only need make 1/2 that many pairs of nodes.
                        break;
                    }
                }
                for edge in edges_to_add {
                    add_edge(edge, &mut eulerized_node_connections);
                }
                // count the number of nodes with an odd connected count
                odd_connected_count = eulerized_node_connections
                    .iter()
                    .filter(|(_, node)| node.connected.len() % 2 == 1)
                    .count();

                if DEBUG_AIRSHIP_EULERIZATION {
                    let total_connections = eulerized_node_connections
                        .iter()
                        .map(|(_, node)| node.connected.len())
                        .sum::<usize>();
                    debug_airship_eulerization!(
                        "Outer Iteration: {}, After Adding, odd connected count: {} in {} nodes, \
                         total connections: {}",
                        i,
                        odd_connected_count,
                        eulerized_node_connections.len(),
                        total_connections
                    );
                }
            }

            // If all nodes have an even number of edges, proceed with finding the best
            // Eulerian circuit for the given node configuration.
            if odd_connected_count == 0 {
                // Find the best Eulerian circuit for the current node connections
                if let Some((route_segments, circuit, max_seg_len, min_spread, _)) =
                    find_best_eulerian_circuit(&eulerized_node_connections)
                {
                    if DEBUG_AIRSHIP_EULERIZATION {
                        debug_airship_eulerization!("Outer Iteration: {}", i);
                        debug_airship_eulerization!("Max segment length: {}", max_seg_len);
                        debug_airship_eulerization!("Min spread: {}", min_spread);
                        debug_airship_eulerization!("Segments count:");
                        route_segments.iter().enumerate().for_each(|segment| {
                            debug_airship_eulerization!("  {} : {}", segment.0, segment.1.len());
                        });
                        // println!("Best segments: {:?}", route_segments);
                        // println!("Circuit: {:?}", circuit);
                    }
                    // A Eulerian circuit was found, apply the goal criteria to find the best
                    // circuit.
                    if max_seg_len > best_max_seg_len
                        || (max_seg_len == best_max_seg_len && min_spread < best_min_spread)
                    {
                        best_circuit = circuit;
                        best_route_segments = route_segments;
                        best_max_seg_len = max_seg_len;
                        best_min_spread = min_spread;
                        best_iteration = i;
                    }
                }
            } else {
                debug_airship_eulerization!(
                    "Error, this should not happen: iteration {}, odd connected count: {} of {} \
                     nodes, total connections: {}, SKIPPING iteration",
                    i,
                    odd_connected_count,
                    eulerized_node_connections.len(),
                    eulerized_node_connections
                        .iter()
                        .map(|(_, node)| node.connected.len())
                        .sum::<usize>()
                );
                error!(
                    "Eulerian circuit not found on iteration {}. Odd connected count is not zero, \
                     this should not happen",
                    i
                );
            }
        }
        if DEBUG_AIRSHIP_EULERIZATION {
            debug_airship_eulerization!("Max segment length: {}", best_max_seg_len);
            debug_airship_eulerization!("Min spread: {}", best_min_spread);
            debug_airship_eulerization!("Iteration: {}", best_iteration);
            debug_airship_eulerization!("Segments count:");
            best_route_segments.iter().enumerate().for_each(|segment| {
                debug_airship_eulerization!("  {} : {}", segment.0, segment.1.len());
            });
            // println!("Best segments: {:?}", best_route_segments);
            // println!("Circuit: {:?}", best_circuit);
        }

        if best_route_segments.is_empty() {
            return None;
        }
        Some((
            best_route_segments,
            best_circuit,
            best_max_seg_len,
            best_min_spread,
            best_iteration,
        ))
    }
}

fn find_best_eulerian_circuit(
    graph: &DockNodeGraph,
) -> Option<(Vec<Vec<usize>>, Vec<usize>, usize, f32, usize)> {
    let mut best_circuit = Vec::new();
    let mut best_route_segments = Vec::new();
    let mut best_max_seg_len = 0;
    let mut best_min_spread = f32::MAX;
    let mut best_iteration = 0;

    let graph_keys = graph.keys().copied().collect::<Vec<_>>();

    for (i, &start_vertex) in graph_keys.iter().enumerate() {
        let mut graph = graph.clone();
        let mut circuit = Vec::new();
        let mut stack = Vec::new();
        let mut circuit_nodes = DHashSet::default();

        let mut current_vertex = start_vertex;

        while !stack.is_empty() || !graph[&current_vertex].connected.is_empty() {
            if graph[&current_vertex].connected.is_empty() {
                circuit.push(current_vertex);
                circuit_nodes.insert(current_vertex);
                current_vertex = stack.pop()?;
            } else {
                stack.push(current_vertex);
                if let Some(&next_vertex) = graph
                    .get(&current_vertex)?
                    .connected
                    .iter()
                    .find(|&vertex| !circuit_nodes.contains(vertex))
                    .or(graph.get(&current_vertex)?.connected.first())
                {
                    remove_edge((current_vertex, next_vertex), &mut graph);
                    current_vertex = next_vertex;
                } else {
                    return None;
                }
            }
        }
        circuit.push(current_vertex);
        circuit.reverse();

        if let Some((route_segments, max_seg_len, min_spread)) =
            best_eulerian_circuit_segments(&graph, &circuit)
        {
            if max_seg_len > best_max_seg_len
                || (max_seg_len == best_max_seg_len && min_spread < best_min_spread)
            {
                best_circuit = circuit.clone();
                best_route_segments = route_segments;
                best_max_seg_len = max_seg_len;
                best_min_spread = min_spread;
                best_iteration = i;
            }
        }
    }
    if best_route_segments.is_empty() {
        return None;
    }
    Some((
        best_route_segments,
        best_circuit,
        best_max_seg_len,
        best_min_spread,
        best_iteration,
    ))
}

/// Get the optimal grouping of Eulerian Circuit nodes and edges such that a
/// maximum number of sub-circuits are created, and the length of each
/// sub-circuit is as similar as possible.
///
/// The Airship dock nodes are connected in a Eulerian Circuit, where each edge
/// of the tessellation is traversed exactly once. The circuit is a closed loop,
/// so the first and last node are the same. The single circuit can be broken
/// into multiple segments, each being also a closed loop. This is desirable for
/// airship routes, to limit the number of airships associated with each "route"
/// where a route is a closed circuit of docking sites. Since each edge is flown
/// in only one direction, the maximum number of possible closed loop segments
/// is equal to the maximum number of edges connected to any node, divided by 2.
fn best_eulerian_circuit_segments(
    graph: &DockNodeGraph,
    circuit: &[usize],
) -> Option<(Vec<Vec<usize>>, usize, f32)> {
    // get the node_connections keys, which are node ids.
    // Sort the nodes (node ids) by the number of connections to other nodes.
    let sorted_node_ids: Vec<usize> = graph
        .keys()
        .copied()
        .sorted_by_key(|&node_id| graph[&node_id].connected.len())
        .rev()
        .collect();

    let mut max_segments_count = 0;
    let mut min_segments_len_spread = f32::MAX;
    let mut best_segments = Vec::new();

    // For each node_id in the sorted node ids,
    // break the circuit into circular segments that start and end with that
    // node_id. The best set of segments is the one with the most segments and
    // where the length of the segments differ the least.
    sorted_node_ids.iter().for_each(|&node_id| {
        let mut segments = Vec::new();
        let mut current_segment = Vec::new();
        let circuit_len = circuit.len();
        let mut starting_index = usize::MAX;
        let mut end_index = usize::MAX;
        let mut prev_value = usize::MAX;

        for (index, &value) in circuit.iter().cycle().enumerate() {
            // println!("Index: {}, Value: {}", index, value);
            if value == node_id {
                if starting_index == usize::MAX {
                    starting_index = index;
                    if starting_index > 0 {
                        end_index = index + circuit_len - 1;
                    } else {
                        end_index = index + circuit_len - 2;
                    }
                }
                if !current_segment.is_empty() {
                    current_segment.push(value);
                    segments.push(current_segment);
                    current_segment = Vec::new();
                }
            }
            if starting_index < usize::MAX {
                if value != prev_value {
                    current_segment.push(value);
                }
                prev_value = value;
            }

            // Stop cycling once we've looped back to the value before the starting index
            if index == end_index {
                break;
            }
        }

        // Add the last segment
        if !current_segment.is_empty() {
            current_segment.push(node_id);
            segments.push(current_segment);
        }

        let avg_segment_length = segments.iter().map(|segment| segment.len()).sum::<usize>() as f32
            / segments.len() as f32;

        // We want similar segment lengths, so calculate the spread as the
        // standard deviation of the segment lengths.
        let seg_lengths_spread = segments
            .iter()
            .map(|segment| (segment.len() as f32 - avg_segment_length).powi(2))
            .sum::<f32>()
            .sqrt()
            / segments.len() as f32;

        // First take the longest segment count, then if the segment count is the same
        // as the longest so far, take the one with the least length spread.
        if segments.len() > max_segments_count {
            max_segments_count = segments.len();
            min_segments_len_spread = seg_lengths_spread;
            best_segments = segments;
        } else if segments.len() == max_segments_count
            && seg_lengths_spread < min_segments_len_spread
        {
            min_segments_len_spread = seg_lengths_spread;
            best_segments = segments;
        }
    });
    if best_segments.is_empty() {
        return None;
    }
    Some((best_segments, max_segments_count, min_segments_len_spread))
}

#[cfg(test)]
mod tests {
    use super::{
        AirshipDockingSide, Airships, DockNode, TriangulationExt,
        approx::assert_relative_eq, find_best_eulerian_circuit, remove_edge,
    };
    use delaunator::{Point, triangulate};
    use vek::{Quaternion, Vec2, Vec3};

    use crate::{
        util::{DHashMap, DHashSet},
    };

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
    fn best_eulerian_circuit_test() {
        let node_connections: DHashMap<usize, DockNode> = DHashMap::from_iter([
            (0, DockNode {
                node_id: 0,
                on_hull: false,
                connected: Vec::from_iter([23, 29, 26, 14, 19, 4]),
            }),
            (28, DockNode {
                node_id: 28,
                on_hull: false,
                connected: Vec::from_iter([23, 15, 25, 20, 21, 22]),
            }),
            (25, DockNode {
                node_id: 25,
                on_hull: false,
                connected: Vec::from_iter([23, 11, 28, 21]),
            }),
            (22, DockNode {
                node_id: 22,
                on_hull: false,
                connected: Vec::from_iter([23, 28, 27, 9, 3, 15]),
            }),
            (19, DockNode {
                node_id: 19,
                on_hull: false,
                connected: Vec::from_iter([0, 6, 29, 18, 2, 4]),
            }),
            (16, DockNode {
                node_id: 16,
                on_hull: false,
                connected: Vec::from_iter([10, 12, 20, 21]),
            }),
            (13, DockNode {
                node_id: 13,
                on_hull: true,
                connected: Vec::from_iter([7, 26, 9, 27, 3, 18]),
            }),
            (10, DockNode {
                node_id: 10,
                on_hull: false,
                connected: Vec::from_iter([24, 29, 11, 2, 16, 21]),
            }),
            (7, DockNode {
                node_id: 7,
                on_hull: true,
                connected: Vec::from_iter([26, 1, 13, 11]),
            }),
            (4, DockNode {
                node_id: 4,
                on_hull: false,
                connected: Vec::from_iter([0, 6, 14, 19]),
            }),
            (1, DockNode {
                node_id: 1,
                on_hull: true,
                connected: Vec::from_iter([7, 26, 8, 17]),
            }),
            (29, DockNode {
                node_id: 29,
                on_hull: false,
                connected: Vec::from_iter([0, 10, 24, 23, 19, 2]),
            }),
            (26, DockNode {
                node_id: 26,
                on_hull: false,
                connected: Vec::from_iter([0, 23, 14, 1, 27, 5, 7, 13]),
            }),
            (23, DockNode {
                node_id: 23,
                on_hull: false,
                connected: Vec::from_iter([0, 29, 25, 22, 28, 24, 11, 26]),
            }),
            (20, DockNode {
                node_id: 20,
                on_hull: true,
                connected: Vec::from_iter([18, 28, 12, 15, 16, 21]),
            }),
            (17, DockNode {
                node_id: 17,
                on_hull: false,
                connected: Vec::from_iter([5, 6, 8, 1]),
            }),
            (14, DockNode {
                node_id: 14,
                on_hull: false,
                connected: Vec::from_iter([0, 5, 26, 4]),
            }),
            (11, DockNode {
                node_id: 11,
                on_hull: false,
                connected: Vec::from_iter([10, 24, 23, 25, 21, 7]),
            }),
            (8, DockNode {
                node_id: 8,
                on_hull: true,
                connected: Vec::from_iter([18, 6, 1, 17]),
            }),
            (5, DockNode {
                node_id: 5,
                on_hull: false,
                connected: Vec::from_iter([6, 26, 14, 17]),
            }),
            (2, DockNode {
                node_id: 2,
                on_hull: false,
                connected: Vec::from_iter([10, 29, 12, 19]),
            }),
            (27, DockNode {
                node_id: 27,
                on_hull: false,
                connected: Vec::from_iter([26, 9, 13, 22]),
            }),
            (24, DockNode {
                node_id: 24,
                on_hull: false,
                connected: Vec::from_iter([10, 29, 11, 23]),
            }),
            (21, DockNode {
                node_id: 21,
                on_hull: false,
                connected: Vec::from_iter([10, 11, 25, 28, 20, 16]),
            }),
            (18, DockNode {
                node_id: 18,
                on_hull: true,
                connected: Vec::from_iter([6, 12, 8, 19, 20, 13]),
            }),
            (15, DockNode {
                node_id: 15,
                on_hull: true,
                connected: Vec::from_iter([28, 20, 3, 22]),
            }),
            (12, DockNode {
                node_id: 12,
                on_hull: false,
                connected: Vec::from_iter([18, 2, 16, 20]),
            }),
            (9, DockNode {
                node_id: 9,
                on_hull: false,
                connected: Vec::from_iter([13, 27, 3, 22]),
            }),
            (6, DockNode {
                node_id: 6,
                on_hull: false,
                connected: Vec::from_iter([4, 8, 5, 18, 19, 17]),
            }),
            (3, DockNode {
                node_id: 3,
                on_hull: true,
                connected: Vec::from_iter([13, 9, 15, 22]),
            }),
        ]);

        let (best_segments, circuit, max_seg_len, min_spread, iteration) =
            find_best_eulerian_circuit(&node_connections)
                .expect("a circuit should have been found");
        // if cfg!(debug_assertions) {
        //     println!("Max segment length: {}", max_seg_len);
        //     println!("Min spread: {}", min_spread);
        //     println!("Iteration: {}", iteration);
        //     println!("Best segments: {:?}", best_segments);
        //     println!("Circuit: {:?}", circuit);
        // }
        assert_eq!(max_seg_len, 4);
        assert_relative_eq!(min_spread, 1.0606601, epsilon = 0.0000001);
        assert_eq!(iteration, 6);
        let expected_segments = vec![
            vec![26, 0, 23, 29, 0, 14, 5, 6, 4, 0, 19, 6, 8, 18, 6, 17, 5, 26],
            vec![
                26, 23, 25, 11, 10, 24, 29, 10, 2, 29, 19, 18, 12, 2, 19, 4, 14, 26,
            ],
            vec![
                26, 1, 8, 17, 1, 7, 11, 24, 23, 22, 28, 23, 11, 21, 10, 16, 12, 20, 18, 13, 26,
            ],
            vec![
                26, 27, 9, 13, 27, 22, 9, 3, 15, 28, 25, 21, 28, 20, 16, 21, 20, 15, 22, 3, 13, 7,
                26,
            ],
        ];
        assert_eq!(best_segments, expected_segments);
        let expected_circuit = vec![
            13, 7, 26, 0, 23, 29, 0, 14, 5, 6, 4, 0, 19, 6, 8, 18, 6, 17, 5, 26, 23, 25, 11, 10,
            24, 29, 10, 2, 29, 19, 18, 12, 2, 19, 4, 14, 26, 1, 8, 17, 1, 7, 11, 24, 23, 22, 28,
            23, 11, 21, 10, 16, 12, 20, 18, 13, 26, 27, 9, 13, 27, 22, 9, 3, 15, 28, 25, 21, 28,
            20, 16, 21, 20, 15, 22, 3, 13,
        ];
        assert_eq!(circuit, expected_circuit);
    }

    fn large_map_docking_locations() -> Vec<Vec2<f32>> {
        [
            [384, 113],
            [713, 67],
            [1351, 17],
            [3146, 64],
            [720, 248],
            [775, 204],
            [829, 166],
            [1391, 161],
            [1812, 156],
            [3022, 204],
            [3094, 193],
            [781, 529],
            [860, 289],
            [889, 371],
            [892, 488],
            [975, 408],
            [1039, 509],
            [1050, 449],
            [1167, 379],
            [1359, 457],
            [1425, 382],
            [1468, 424],
            [1493, 363],
            [1752, 322],
            [1814, 452],
            [2139, 469],
            [2179, 343],
            [2283, 333],
            [2428, 299],
            [2499, 504],
            [2567, 498],
            [3110, 363],
            [3126, 503],
            [3248, 330],
            [3343, 491],
            [96, 837],
            [98, 752],
            [149, 884],
            [258, 679],
            [349, 873],
            [350, 676],
            [431, 983],
            [541, 842],
            [686, 640],
            [923, 728],
            [941, 537],
            [951, 654],
            [991, 575],
            [999, 955],
            [1164, 767],
            [1238, 669],
            [1250, 923],
            [1266, 808],
            [1343, 878],
            [1535, 711],
            [1633, 773],
            [1684, 705],
            [1690, 833],
            [1694, 982],
            [1742, 774],
            [1781, 821],
            [1833, 558],
            [1854, 623],
            [2169, 815],
            [2189, 966],
            [2232, 691],
            [2243, 778],
            [2266, 934],
            [2354, 742],
            [2423, 753],
            [2423, 999],
            [2438, 637],
            [2491, 758],
            [2497, 636],
            [2507, 855],
            [3066, 909],
            [3088, 568],
            [3124, 687],
            [3198, 681],
            [3241, 901],
            [3260, 603],
            [3276, 704],
            [3314, 652],
            [3329, 744],
            [3374, 888],
            [3513, 999],
            [3609, 708],
            [3864, 934],
            [3959, 933],
            [3959, 1000],
            [167, 1135],
            [229, 1072],
            [333, 1198],
            [349, 1481],
            [399, 1165],
            [473, 1350],
            [510, 1032],
            [523, 1481],
            [535, 1294],
            [552, 1080],
            [587, 1388],
            [789, 1103],
            [816, 1284],
            [886, 1183],
            [905, 1338],
            [1022, 1158],
            [1161, 1359],
            [1187, 1457],
            [1197, 1289],
            [1231, 1067],
            [1311, 1352],
            [1331, 1076],
            [1340, 1504],
            [1367, 1415],
            [1414, 1384],
            [1424, 1091],
            [1447, 1018],
            [1642, 1383],
            [1733, 1237],
            [1740, 1066],
            [1751, 1128],
            [1797, 1171],
            [1802, 1060],
            [1960, 1495],
            [1977, 1081],
            [2305, 1064],
            [2372, 1117],
            [2411, 1480],
            [2688, 1320],
            [2745, 1359],
            [2819, 1162],
            [2860, 1268],
            [2868, 1088],
            [2934, 1481],
            [2991, 1388],
            [3078, 1447],
            [3166, 1267],
            [3222, 1374],
            [3234, 1234],
            [3244, 1057],
            [3256, 1437],
            [3302, 1274],
            [3354, 1165],
            [3389, 1340],
            [3416, 1406],
            [3451, 1122],
            [3594, 1205],
            [3681, 1435],
            [3838, 1265],
            [3892, 1181],
            [3911, 1243],
            [200, 1663],
            [328, 1843],
            [363, 1630],
            [445, 1640],
            [505, 1756],
            [537, 1594],
            [560, 1779],
            [654, 1594],
            [713, 1559],
            [769, 1912],
            [970, 1782],
            [988, 1705],
            [1361, 1595],
            [1370, 1949],
            [1480, 1695],
            [1695, 1531],
            [1881, 1703],
            [2315, 1979],
            [2411, 1536],
            [2508, 1990],
            [2679, 1737],
            [2731, 1704],
            [2734, 1956],
            [2739, 1606],
            [2770, 1781],
            [2778, 1879],
            [2781, 1664],
            [2841, 1716],
            [2858, 1647],
            [2858, 1826],
            [2898, 1715],
            [2935, 1554],
            [3051, 1837],
            [3060, 1965],
            [3185, 1918],
            [3251, 1869],
            [3442, 1856],
            [3447, 1543],
            [3534, 1951],
            [3590, 1878],
            [3611, 1960],
            [3635, 1584],
            [3649, 1781],
            [3656, 1850],
            [3668, 1912],
            [3750, 1906],
            [3762, 1826],
            [3831, 1971],
            [3841, 1876],
            [3888, 1806],
            [3960, 1818],
            [177, 2260],
            [239, 2026],
            [358, 2364],
            [471, 2327],
            [528, 2100],
            [536, 2198],
            [588, 2244],
            [648, 2180],
            [665, 2038],
            [693, 2366],
            [852, 2410],
            [898, 2293],
            [969, 2205],
            [1095, 2322],
            [1198, 2217],
            [1267, 2284],
            [1278, 2220],
            [1339, 2114],
            [1419, 2203],
            [1470, 2049],
            [1487, 2108],
            [1959, 2257],
            [2087, 2061],
            [2226, 2048],
            [2231, 2319],
            [2385, 2251],
            [2417, 2039],
            [2598, 2035],
            [2686, 2071],
            [2715, 2204],
            [2778, 2188],
            [2900, 2128],
            [2910, 2007],
            [2988, 2087],
            [3002, 2435],
            [3082, 2433],
            [3115, 2006],
            [3167, 2143],
            [3170, 2361],
            [3360, 2433],
            [3472, 2370],
            [3514, 2022],
            [3599, 2045],
            [3662, 2365],
            [3676, 2172],
            [3838, 2208],
            [3921, 2060],
            [87, 2628],
            [239, 2604],
            [270, 2668],
            [327, 2726],
            [371, 2781],
            [419, 2583],
            [546, 2574],
            [620, 2776],
            [979, 2850],
            [1052, 2762],
            [1095, 2825],
            [1486, 2601],
            [1587, 2701],
            [1620, 2599],
            [1633, 2492],
            [1948, 2809],
            [2156, 2852],
            [2464, 2605],
            [2544, 2777],
            [2645, 2605],
            [2743, 2466],
            [2836, 2785],
            [2981, 2635],
            [3029, 2699],
            [3162, 2733],
            [3389, 2769],
            [3484, 2776],
            [3561, 2795],
            [3631, 2549],
            [3669, 2474],
            [3732, 2625],
            [33, 3129],
            [97, 3152],
            [191, 3289],
            [449, 2938],
            [450, 3000],
            [590, 3142],
            [654, 3065],
            [744, 3093],
            [870, 3042],
            [875, 2904],
            [921, 3103],
            [1018, 3034],
            [1040, 3135],
            [1079, 3238],
            [1122, 3316],
            [1136, 2996],
            [1237, 3366],
            [1294, 3127],
            [1360, 3297],
            [1366, 3043],
            [1368, 2985],
            [1381, 3128],
            [1464, 3089],
            [1514, 2965],
            [1529, 3046],
            [1901, 3052],
            [1954, 3272],
            [2117, 3121],
            [2182, 3381],
            [2225, 3212],
            [2241, 3142],
            [2250, 2949],
            [2340, 3333],
            [2395, 3195],
            [2496, 3383],
            [2521, 3162],
            [2604, 2959],
            [2635, 3287],
            [2644, 3021],
            [2657, 3140],
            [2716, 3367],
            [2726, 3184],
            [2734, 3264],
            [2799, 3300],
            [2866, 3361],
            [2907, 2893],
            [2938, 3362],
            [3058, 2982],
            [3187, 3076],
            [3357, 3200],
            [3467, 3300],
            [3511, 3359],
            [3522, 3105],
            [3538, 2997],
            [3791, 3348],
            [3866, 3261],
            [3947, 3223],
            [33, 3807],
            [109, 3828],
            [390, 3472],
            [468, 3510],
            [534, 3508],
            [563, 3659],
            [665, 3830],
            [668, 3732],
            [742, 3770],
            [896, 3818],
            [934, 3475],
            [1255, 3871],
            [1309, 3477],
            [1318, 3812],
            [1425, 3417],
            [1443, 3950],
            [1479, 3638],
            [1492, 3546],
            [1498, 3940],
            [1533, 3593],
            [1584, 3448],
            [1605, 3691],
            [1632, 3831],
            [1798, 3826],
            [1992, 3612],
            [2101, 3713],
            [2157, 3496],
            [2204, 3796],
            [2314, 3835],
            [2350, 3650],
            [2446, 3697],
            [2474, 3624],
            [2516, 3528],
            [2607, 3551],
            [2644, 3929],
            [2714, 3603],
            [2760, 3707],
            [2797, 3658],
            [2940, 3520],
            [2955, 3687],
            [2971, 3446],
            [3081, 3427],
            [3082, 3828],
            [3124, 3475],
            [3149, 3624],
            [3174, 3539],
            [3341, 3897],
            [3371, 3841],
            [3663, 3786],
            [3740, 3468],
            [3783, 3575],
            [3886, 3584],
            [3948, 3547],
        ]
        .iter()
        .map(|&[x, y]| Vec2::new(x as f32, y as f32))
        .collect()
    }

    fn large_map_docking_points() -> Vec<Point> {
        large_map_docking_locations()
            .iter()
            .map(|&loc| Point {
                x: loc.x as f64,
                y: loc.y as f64,
            })
            .collect()
    }

    #[test]
    fn large_map_graph_remove_edges_compare_test() {
        let all_dock_points1 = large_map_docking_points();
        let triangulation1 = triangulate(&all_dock_points1);
        let node_connections1 = triangulation1.node_connections();

        let all_dock_points2 = large_map_docking_points();
        let triangulation2 = triangulate(&all_dock_points2);
        let node_connections2 = triangulation2.node_connections();

        assert_eq!(
            all_dock_points1, all_dock_points2,
            "Dock points should be the same."
        );
        assert_eq!(
            node_connections1, node_connections2,
            "Node connections should be equal before removing edges."
        );

        let max_distance_squared = 1000.0f64.powi(2);

        let mut edges_to_remove1 = Vec::new();
        node_connections1.iter().for_each(|(node_id, node)| {
            for &connected_node_id in &node.connected {
                let pt1 = &all_dock_points1[*node_id];
                let pt2 = &all_dock_points1[connected_node_id];
                let v1 = Vec2 { x: pt1.x, y: pt1.y };
                let v2 = Vec2 { x: pt2.x, y: pt2.y };
                // Remove the edge if the distance is greater than 1000.0
                if v1.distance_squared(v2) > max_distance_squared {
                    edges_to_remove1.push((*node_id, connected_node_id));
                }
            }
        });
        let edges_to_remove1b = edges_to_remove1.clone();

        let mut edges_to_remove1c = DHashSet::default();
        edges_to_remove1b
            .iter()
            .for_each(|&(node_id, connected_node_id)| {
                edges_to_remove1c.insert(if node_id < connected_node_id {
                    (node_id, connected_node_id)
                } else {
                    (connected_node_id, node_id)
                });
            });

        let mut edges_to_remove2 = DHashSet::default();
        let all_edges2 = triangulation2.all_edges();
        for edge in all_edges2.iter() {
            let pt1 = &all_dock_points2[edge.0];
            let pt2 = &all_dock_points2[edge.1];
            let v1 = Vec2 { x: pt1.x, y: pt1.y };
            let v2 = Vec2 { x: pt2.x, y: pt2.y };
            // Remove the edge if the distance between the points is greater than
            // max_leg_length
            if v1.distance_squared(v2) > max_distance_squared {
                edges_to_remove2.insert(edge.clone());
            }
        }

        assert_eq!(
            edges_to_remove1c, edges_to_remove2,
            "Edges to remove should be the same in hashset form."
        );

        let mut mutable_node_connections1 = node_connections1.clone();
        for (node_id, connected_node_id) in edges_to_remove1 {
            remove_edge((node_id, connected_node_id), &mut mutable_node_connections1);
        }

        let mut mutable_node_connections1b = node_connections1.clone();
        for edge in edges_to_remove1c {
            remove_edge(edge, &mut mutable_node_connections1b);
        }

        assert_eq!(
            mutable_node_connections1, mutable_node_connections1b,
            "Node connections1 should be the same for either Vec or HashSet remove edges."
        );

        let mut mutable_node_connections2 = node_connections2.clone();
        for edge in edges_to_remove2 {
            remove_edge(edge, &mut mutable_node_connections2);
        }

        assert_eq!(
            mutable_node_connections1, mutable_node_connections2,
            "Node connections should be equal after removing edges."
        );

        assert_eq!(
            mutable_node_connections1, mutable_node_connections2,
            "Node connections should be equal after removing edges."
        );
    }

}
