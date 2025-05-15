use crate::{
    civ::airship_route_map::*,
    Index,
    sim::WorldSim,
    site::{self, Site, plot::PlotKindMeta},
    util::{DHashMap, DHashSet, seed_expan},
};
use common::{
    store::{Id, Store},
    terrain::CoordinateConversions,
    util::Dir,
};
use delaunator::{Point, Triangulation, triangulate};
use itertools::Itertools;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::{fs::OpenOptions, io::Write};
use tracing::warn;
use vek::*;

const AIRSHIP_TRAVEL_DEBUG: bool = true;

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
    pub connected: DHashSet<usize>,
    //pub connected: Vec<usize>,
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

        let node_connections = triangulation.node_connections();

        let hull_dia_approx = approximate_hull_diameter(&all_dock_points);
        debug_airships!("Tessellation hull diameter approx: {}", hull_dia_approx);

        // node_connections.keys() are the node indices (ids).
        // Since they are in a hashmap, they are not sorted.
        // Below, we sort them to randomize the start point for the
        // modification of the triangulation to remove odd numbers of edges.
        let mut search_order = node_connections.keys().copied().collect::<Vec<_>>();

        let mut max_score = 0.0;
        let mut max_connections: Vec<(usize, usize)> = Vec::default();
        let mut max_final_indeces = vec![];

        for _ in 0..100 {
            search_order.shuffle(&mut rand::thread_rng());

            let mut node_conn_working_copy = node_connections.clone();
            let (high_score, best_connections, best_final_indeces) = triangulation
                .optimized_tessellation_edges(
                    &all_dock_points,
                    &mut node_conn_working_copy,
                    hull_dia_approx,
                    &search_order,
                );

            if high_score > max_score {
                max_score = high_score;
                max_connections = best_connections;
                max_final_indeces = best_final_indeces;
            }
        }
        debug_airships!("Max score: {}", max_score);
        debug_airships!("Max connections: {:?}", max_connections);
        debug_airships!("Max final indeces: {:?}", max_final_indeces);

        let mut node_connections_optimized = node_connections.clone();
        for (from_node_id, to_node_id) in max_connections {
            remove_edge(from_node_id, to_node_id, &mut node_connections_optimized);
        }
        max_final_indeces.chunks(2).for_each(|chunk| {
            if let [from_node_id, to_node_id] = chunk {
                add_edge(*from_node_id, *to_node_id, &mut node_connections_optimized);
            }
        });

        #[cfg(debug_assertions)]
        {
            let odd_nodes = node_connections_optimized
                .iter()
                .filter(|(_, dock_node)| dock_node.connected.len() % 2 == 1)
                .map(|(node_id, _)| *node_id)
                .collect::<Vec<_>>();
            assert!(odd_nodes.len() == 0);
            debug_airships!("node_connections_optimized: {:?}", node_connections_optimized);
        }



        save_airship_routes_triangulation(&triangulation, &all_dock_points, index, world_sim);
        save_airship_routes_optimized_tesselation(
            &triangulation,
            &all_dock_points,
            &node_connections_optimized,
            index,
            world_sim,
        );

        if let Some(circuit) = find_eulerian_circuit(
            &node_connections_optimized) 
            && let Some(best_route_segments) = best_eulerian_circuit_segments(
                &node_connections_optimized,
                &circuit,
            )
        {
            debug_airships!("Best route segments: {:?}", best_route_segments);
            save_airship_route_segments(
                &best_route_segments,
                &all_dock_points,
                index,
                world_sim,
            );
        }

        // let edge_counts_unsorted = result.count_edges_per_node();
        // let mut edge_counts_sorted: Vec<_> =
        // edge_counts_unsorted.iter().collect(); edge_counts_sorted.
        // sort_by(|a, b| a.1.cmp(&b.1));

        // #[cfg(debug_assertions)]
        // {
        //     debug_airships!("all_dock_points: {:?}", all_dock_points);
        //     debug_airships!("triangulation {:?}", result);

        //     debug_airships!("triangle chunks (len {})",
        // result.triangles.len());     for chunk in
        // result.triangles.chunks(3) {         if let [a, b, c] = chunk
        // {             debug_airships!("{}, {}, {}", a, b, c);
        //         }
        //     }

        //     debug_airships!("hull (len {}) {:?}", result.hull.len(),
        // result.hull);

        //     debug_airships!("halfedges (len {}):", result.halfedges.len());
        //     for i in 0..result.halfedges.len() {
        //         if result.halfedges[i] == delaunator::EMPTY {
        //             let vertex = result.triangles[i];
        //             debug_airships!(
        //                 "{}, {}, {}, 4294967295, 0, 0",
        //                 vertex,
        //                 all_dock_points[vertex].x,
        //                 all_dock_points[vertex].y
        //             );
        //         } else {
        //             let vertex1 = result.triangles[i];
        //             let vertex2 = result.triangles[result.halfedges[i]];
        //             debug_airships!(
        //                 "{}, {}, {}, {}, {}, {}",
        //                 vertex1,
        //                 all_dock_points[vertex1].x,
        //                 all_dock_points[vertex1].y,
        //                 vertex2,
        //                 all_dock_points[vertex2].x,
        //                 all_dock_points[vertex2].y
        //             );
        //         }
        //     }

        //     debug_airships!("Sorted by number of edges: {:?}",
        // edge_counts_sorted);

        //     for (node_index, count) in &edge_counts_sorted {
        //         debug_airships!("Node index {}: {} edges", node_index,
        // count);         let connected =
        // result.connected_nodes(**node_index);         for
        // connected_node in connected {             if
        // result.is_hull_edge((**node_index, connected_node)) {
        //                 debug_airships!(
        //                     "  {} on hull, {} edges",
        //                     connected_node,
        //                     edge_counts_unsorted[&connected_node]
        //                 );
        //             } else {
        //                 debug_airships!(
        //                     "  {}, {} edges",
        //                     connected_node,
        //                     edge_counts_unsorted[&connected_node]
        //                 );
        //             }
        //         }
        //     }

        //     let mut node_connections = result.node_connections();
        //     debug_airships!("Node connections: {:?}", node_connections);
        //     for (_, dock_node) in &node_connections {
        //         debug_airships!(
        //             "Node index {}: {} edges, on hull: {}",
        //             dock_node.node_id,
        //             dock_node.connected.len(),
        //             dock_node.on_hull
        //         );
        //         for connected_node in &dock_node.connected {
        //             debug_airships!(
        //                 "  {}, {} edges",
        //                 connected_node,
        //                 edge_counts_unsorted[connected_node]
        //             );
        //         }
        //     }
        // }
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

#[derive(Debug, Clone)]
enum EdgeRemovalStackNodeType {
    Find,
    Check,
    Undo,
}

const DEBUG_AIRSHIP_TESSELATION_OPTIMIZATION: bool = false;

macro_rules! debug_airship_tess_opt {
    ($($arg:tt)*) => {
        if DEBUG_AIRSHIP_TESSELATION_OPTIMIZATION {
            println!($($arg)*);
        }
    }
}

type DockNodeGraph = DHashMap<usize, DockNode>;

trait TriangulationExt {
    fn count_edges_per_node(&self) -> DHashMap<usize, usize>;
    fn connected_nodes(&self, node: usize) -> Vec<usize>;
    fn is_hull_node(&self, index: usize) -> bool;
    fn is_hull_edge(&self, edge: (usize, usize)) -> bool;
    fn node_connections(&self) -> DockNodeGraph;
    fn optimized_tessellation_edges(
        &self,
        all_dock_points: &Vec<Point>,
        node_connections: &mut DockNodeGraph,
        hull_dia_approx: f64,
        search_order: &Vec<usize>,
    ) -> (f32, Vec<(usize, usize)>, Vec<usize>);
}

fn first_odd_node(
    search_order: &Vec<usize>,
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
fn remove_edge(node_id1: usize, node_id2: usize, nodes: &mut DockNodeGraph) {
    if let Some(dock_node) = nodes.get_mut(&node_id1) {
        dock_node.connected.remove(&node_id2);
        // dock_node.connected.retain(|&x| x != node_id2);
    }
    if let Some(dock_node) = nodes.get_mut(&node_id2) {
        dock_node.connected.remove(&node_id1);
        // dock_node.connected.retain(|&x| x != node_id1);
    }
}

/// Adds an edge between two nodes in the tesselation graph.
fn add_edge(node_id1: usize, node_id2: usize, nodes: &mut DockNodeGraph) {
    if let Some(dock_node) = nodes.get_mut(&node_id1) {
        dock_node.connected.insert(node_id2);
        // dock_node.connected.push(node_id2);
    }
    if let Some(dock_node) = nodes.get_mut(&node_id2) {
        dock_node.connected.insert(node_id1);
        // dock_node.connected.push(node_id1);
    }
}

fn approximate_hull_diameter(points: &Vec<Point>) -> f64 {
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for point in points {
        if point.x < min_x {
            min_x = point.x;
        }
        if point.x > max_x {
            max_x = point.x;
        }
        if point.y < min_y {
            min_y = point.y;
        }
        if point.y > max_y {
            max_y = point.y;
        }
    }

    let hull_width = max_x - min_x;
    let hull_height = max_y - min_y;
    let hull_dia1 = (hull_width.powi(2) + hull_height.powi(2)).sqrt();
    let hull_dia2 = (hull_width + hull_height) / 2.0;
    let hull_dia3 = (hull_dia1 + hull_dia2) / 2.0;

    hull_dia3
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

// When there are four odd nodes, two lines are formed. This function gives the
// score based on the angle between the two lines.
// The formula was approximated using a score of 1.0 at 90 degrees, 0.9 at 30
// degrees, and 0 at 0 degrees.
fn four_point_angle_score(angle_in_degrees: f64) -> f64 {
    1.0 - (-0.07675 * angle_in_degrees).exp()
}

/// Calculates a score for the tessellation based on the number of left-over odd
/// numbers of edges and the length of the edge or edges that must be added to
/// make the tessellation have an even number of edges. A low score is assigned
/// if there are more than 4 odd edges, essentially eliminating any
/// tessellations that have a lot of disconnected edges. A tessellation with no
/// left-over edges is perfect and gets a score of 1.0. For 2 or 4
/// left-over edges, there will be one or two lines that are needed to connect
/// the odd nodes. One goal of the airship evolutions RFC is to have most route
/// legs be of medium length (hence the tessellation), with some longer legs
/// that act as longer range routes. Therefore, the length score is based on the
/// ratio of the line length to the tessellation hull diameter, with a score of
/// 1.0 at 1.0 of the hull diameter, and a score of 0.9 at 0.7 of the hull
/// diameter, and a score of 0.0 at 0.0 of the hull diameter.
fn dock_nodes_score(
    nodes: &DockNodeGraph,
    points: &Vec<Point>,
    hull_diameter: f64,
) -> (f32, Vec<usize>) {
    if hull_diameter < 1.0 {
        return (0.0, vec![]);
    }
    /*
       No odd edges = perfect score.
       Two or Four odd edges = 90% score.
       Six or Eight odd edges = 50% score.
       More than eight odd edges = 0% score.

       For two or four odd edges, connect the odd nodes and check the distance
       between the nodes as compared to the hull diameter. Reduce the score for
       distances less than the hull diameter. For four odd nodes, calculate the
       combination of nodes that produces two lines that are as perpendicular as
       possible, and then reduce the score for line lengths that are less than
       the hull diameter.
    */
    let mut best_odd_indeces = vec![];
    let odd_nodes = nodes
        .iter()
        .filter(|(_, dock_node)| dock_node.connected.len() % 2 == 1)
        .map(|(node_id, _)| *node_id)
        .collect::<Vec<_>>();
    assert!(odd_nodes.len() % 2 == 0);

    if odd_nodes.len() == 2 {
        let point1 = &points[odd_nodes[0]];
        let point2 = &points[odd_nodes[1]];
        let line_len = ((point1.x - point2.x).powi(2) + (point1.y - point2.y).powi(2)).sqrt();
        debug_airship_tess_opt!(
            "dock_nodes_score: 2 odd nodes, line_len: {}, hull_percentage: {}, score: {}",
            line_len,
            line_len / hull_diameter,
            hull_ratio_distance_score(line_len, hull_diameter) * 0.9
        );
        (
            (hull_ratio_distance_score(line_len, hull_diameter) * 0.9) as f32,
            odd_nodes,
        )
    } else if odd_nodes.len() == 4 {
        // First connect the odd nodes to produce orhogonal lines as much as possible.
        // Map the odd node points to Vec2 points.
        let odd_nodes_points = odd_nodes
            .iter()
            .map(|&node_id| &points[node_id])
            .map(|point| Vec2::new(point.x as f32, point.y as f32))
            .collect::<Vec<_>>();
        assert!(odd_nodes_points.len() == 4);
        debug_airship_tess_opt!(
            "dock_nodes_score: 4 odd nodes, odd_nodes_points: {:?}",
            odd_nodes_points
        );

        // There are three pairs of points.
        // 0, 1, 2, 3
        // 0, 2, 1, 3
        // 0, 3, 1, 2

        // The reverse of the pairs are the same because
        // only the orientation between the lines matters.
        // I.e., 0, 1, 3, 2 OR 1, 0, 2, 3 etc. are the same except if the answer is
        // greater than 90 degrees, then subtract it from 180 degrees.
        // For example in the give test case
        // 0, 1, 2, 3 : 29.675674
        // 1, 0, 2, 3 : 150.32431
        // 0, 1, 3, 2 : 150.32431
        // 1, 0, 3, 2 : 29.675674
        // and 180 - 150.32431 = 29.675674
        // So for the first pair of points, the lines are about 30 degrees apart.

        let indices_sets = [[0, 1, 2, 3], [0, 2, 1, 3], [0, 3, 1, 2]];
        // value	score
        // 0		0
        // 30		0.9
        // 90		1.0

        let mut best_score = 0.0f32;
        for (index, indices) in indices_sets.iter().enumerate() {
            let a = odd_nodes_points[indices[0]];
            let b = odd_nodes_points[indices[1]];
            let c = odd_nodes_points[indices[2]];
            let d = odd_nodes_points[indices[3]];

            let mut angle = (a - b).angle_between(c - d) * (180.0 / std::f32::consts::PI);
            if angle > 90.0 {
                // subtract from 180 degrees
                angle = 180.0 - angle;
            }
            let angle_score = four_point_angle_score(angle as f64) as f32;

            let dist1 = a.distance(b);
            let dist2 = c.distance(d);
            let total_dist = dist1 + dist2;
            let dist_score =
                hull_ratio_distance_score(total_dist as f64, 2.0 * hull_diameter) as f32;

            let score = angle_score * dist_score;
            if score > best_score {
                best_score = score;
                best_odd_indeces = vec![
                    odd_nodes[indices[0]],
                    odd_nodes[indices[1]],
                    odd_nodes[indices[2]],
                    odd_nodes[indices[3]],
                ];
            }

            debug_airship_tess_opt!(
                "Index: {}, Indices {:?}, ∂: {}, ∂score: {}, len1: {}, len2: {}, total: {}, \
                 percentage: {} dist score: {}, score: {}",
                index,
                indices,
                angle,
                angle_score,
                dist1,
                dist2,
                dist1 + dist2,
                total_dist / (2.0 * hull_diameter) as f32,
                dist_score,
                score
            );
        }
        (best_score, best_odd_indeces)
    } else if odd_nodes.len() == 6 || odd_nodes.len() == 8 {
        debug_airship_tess_opt!("dock_nodes_score: {} odd nodes", odd_nodes.len());
        (0.25, best_odd_indeces)
    } else {
        debug_airship_tess_opt!("dock_nodes_score: more than 8 odd nodes, score is 0.0");
        (0.0, best_odd_indeces)
    }
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
                    connected: DHashSet::default(),
                    // connected: Vec::default(),
                });
                for &connected_node in t {
                    if connected_node != node {
                        dock_node.connected.insert(connected_node);
                    }
                }
            }
        });
        for (_, dock_node) in connections.iter_mut() {
            dock_node.connected = dock_node.connected.iter().copied().collect();
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

    /// A triangulation tessellation produces nodes that can have an odd number
    /// of edges. The algorithm that computes routes through the
    /// tessellation requires that all nodes have an even number of edges.
    /// This function will remove edges from the tessellation by finding odd
    /// nodes and pairing them with other connected odd nodes until all odd
    /// nodes have been paired or there are no more connected odd nodes. If
    /// there are still odd nodes, the algorithm will add edges to the odd
    /// nodes until all odd nodes have an even number of edges.
    ///
    /// Note: another way to solve this problem would be to simply add edges as
    /// required to the tessellation. This would be a more efficient
    /// solution, but it would not adhere to the requirement that all edges
    /// are as short as possible (which is one goal of the
    /// Airship evolutions RFC).
    ///
    /// This process of removing edges is sensitive to the starting node and the
    /// order of node traversal. To achieve the best results, this function
    /// is usually called multiple times with different starting nodes and
    /// orderings. The best result is the one with the highest score.
    fn optimized_tessellation_edges(
        &self,
        all_dock_points: &Vec<Point>,
        node_connections: &mut DockNodeGraph,
        hull_dia_approx: f64,
        search_order: &Vec<usize>,
    ) -> (f32, Vec<(usize, usize)>, Vec<usize>) {
        /*
           This is a recursive algorithm. The Rust best practice is to use a data stack
           rather than a recursive function. The stack will be used to store
           the current state of the algorithm, the next node actions, and undo
           actions to reverse the downstream modifications to the tesselation.

           Need a stack with three types of nodes
           1. Find - find the next available node with an odd number of edges.
           2. Check - modify the tesselation and add the other connections to check
           3. Undo - reverse the modifications to the tesselation.

           Need a connection vec to record the connections in work

           Need a best score vec to record the connections with the high score so far

           let find_index = 0

           While pop
               node type matches
                   Find
                       If Find Odd Node (find_index)
                           push all Check nodes with odd connections,
                               if the from and to nodes are not both on the hull,
                               in reverse order
                           If no Check nodes were added (no connections for the from-node)
                               push Find node to stack with index = find_index + 1
                           End
                       Else
                           Compute Score
                           If higher score
                               Replace best_connections with current_connections
                           End
                       End
                   Check
                       Modify from & to DockNodes
                       push from-to onto current_connections
                       push Undo node to stack to undo the from-to modifications
                       push Find node to stack with index = from index + 1
                   Undo
                       Undo modifications for from & to DockNodes
                       Remove last from current_connections
        */

        // The first value in mod_stack will be a Find, Check, or Undo enum value.
        // The second value in mod_stack will be the index in the search order.
        // The third value in mod_stack will be the node id for the from-node or zero
        // for Find stack nodes. The fourth value in mod_stack will be the node
        // id for the to-node or zero for Find stack nodes.
        let mut mod_stack: Vec<(EdgeRemovalStackNodeType, usize, usize, usize)> = Vec::default();
        let mut curr_connections: Vec<(usize, usize)> = Vec::default();
        let mut best_connections: Vec<(usize, usize)> = Vec::default();

        let mut high_score = 0.0f32;
        let mut best_final_indeces = vec![];

        mod_stack.push((EdgeRemovalStackNodeType::Find, 0, 0, 0));
        debug_airship_tess_opt!("Mod stack: {:?}", mod_stack);

        while let Some((mod_type, index, node_id1, node_id2)) = mod_stack.pop() {
            debug_airship_tess_opt!(
                "Mod type: {:?}, index: {}, node_id1: {}, node_id2: {}",
                mod_type,
                index,
                node_id1,
                node_id2
            );

            match mod_type {
                EdgeRemovalStackNodeType::Find => {
                    // Find
                    if let Some((index, node_id)) =
                        first_odd_node(&search_order, index, &node_connections)
                    {
                        debug_airship_tess_opt!(
                            "Odd node found: {}, search_order index {}",
                            node_id,
                            index
                        );
                        if let Some(node) = node_connections.get(&node_id) {
                            let mut added_checks = false;
                            // push Check nodes with (odd) connections in reverse order
                            node.connected.iter().for_each(|con_node_id| {
                                if let Some(con_node) = node_connections.get(con_node_id) {
                                    if con_node.connected.len() % 2 == 1
                                        && !(node.on_hull && con_node.on_hull)
                                    {
                                        mod_stack.push((
                                            EdgeRemovalStackNodeType::Check,
                                            index,
                                            node_id,
                                            *con_node_id,
                                        ));
                                        added_checks = true;
                                        debug_airship_tess_opt!("Mod stack: {:?}", mod_stack);
                                    }
                                }
                            });
                            if !added_checks {
                                debug_airship_tess_opt!(
                                    "No odd connections found for odd node {}",
                                    node_id
                                );
                                mod_stack.push((EdgeRemovalStackNodeType::Find, index + 1, 0, 0));
                                debug_airship_tess_opt!("Mod stack: {:?}", mod_stack);
                            }
                        }
                    } else {
                        // Compute Score
                        // If higher score
                        //     Replace best_connections with current_connections
                        // End
                        let (score, best_odd_indeces) =
                            dock_nodes_score(&node_connections, &all_dock_points, hull_dia_approx);
                        debug_airship_tess_opt!(
                            "No more odd nodes from index {}, Score: {}",
                            index,
                            score
                        );
                        if score > high_score {
                            high_score = score;
                            best_connections = curr_connections.clone();
                            best_final_indeces = best_odd_indeces.clone();
                            debug_airship_tess_opt!("New high score: {}", score);
                            debug_airship_tess_opt!("Best connections: {:?}", best_connections);
                            debug_airship_tess_opt!("Best final indeces: {:?}", best_final_indeces);
                        }
                    }
                },
                EdgeRemovalStackNodeType::Check => {
                    // Check
                    // If from and to are not on hull
                    //  Modify from & to DockNodes
                    //  push from-to onto current_connections
                    //  push Undo node to stack to undo the from-to modifications
                    //  push Find node to stack with index = from index + 1
                    // End
                    if let Some(node) = node_connections.get(&node_id1)
                        && let Some(con_node) = node_connections.get(&node_id2)
                    {
                        // Make sure that hull edges have been eliminated already.
                        assert!(
                            !(node.on_hull && con_node.on_hull),
                            "Hull edges should have been eliminated already"
                        );
                        // The edge to be removed is not on the hull, remove the edge.
                        debug_airship_tess_opt!("Removing edge {} -> {}", node_id1, node_id2);
                        remove_edge(node_id1, node_id2, node_connections);
                        curr_connections.push((node_id1, node_id2));
                        debug_airship_tess_opt!("Current connections: {:?}", curr_connections);
                        mod_stack.push((EdgeRemovalStackNodeType::Undo, index, node_id1, node_id2));
                        mod_stack.push((EdgeRemovalStackNodeType::Find, index + 1, 0, 0));
                        debug_airship_tess_opt!("Mod stack: {:?}", mod_stack);
                    }
                },
                EdgeRemovalStackNodeType::Undo => {
                    // Undo
                    // Undo modifications for from & to DockNodes
                    // Remove last from current_connections
                    debug_airship_tess_opt!(
                        "Undoing edge removal, adding edge {} -> {}",
                        node_id1,
                        node_id2
                    );
                    add_edge(node_id1, node_id2, node_connections);
                    curr_connections.pop();
                },
            }
        }
        (high_score, best_connections, best_final_indeces)
    }
}

fn find_eulerian_circuit(graph: &DockNodeGraph) -> Option<Vec<usize>> {
    let mut graph = graph.clone();
    let mut circuit = Vec::new();
    let mut stack = Vec::new();

    let mut current_vertex = *graph.keys().next()?;

    while !stack.is_empty() || !graph[&current_vertex].connected.is_empty() {
        if graph[&current_vertex].connected.is_empty() {
            circuit.push(current_vertex);
            current_vertex = stack.pop().unwrap();
        } else {
            stack.push(current_vertex);
            if let Some(&next_vertex) =
                graph.get(&current_vertex).unwrap().connected.iter().next()
            {
                graph
                    .get_mut(&current_vertex)
                    .unwrap()
                    .connected
                    .remove(&next_vertex);
                graph
                    .get_mut(&next_vertex)
                    .unwrap()
                    .connected
                    .remove(&current_vertex);
                current_vertex = next_vertex;
            } else {
                return None;
            }
        }
    }

    circuit.push(current_vertex);
    circuit.reverse();
    Some(circuit)
}

/// Get the optimal grouping of Eulerian Circuit nodes and edges such that a maximum number of sub-circuits
/// are created, and the length of each sub-circuit is as similar as possible.
/// 
/// The Airship dock nodes are connected in a Eulerian Circuit, where each edge of the tessellation
/// is traversed exactly once. The circuit is a closed loop, so the first and last node are the same.
/// The single circuit can be broken into multiple segments, each being also a closed loop. This is
/// desirable for airship routes, to limit the number of airships associated with each "route" where a route
/// is a closed circuit of docking sites. Since each edge is flown in only one direction, the maximum number
/// of possible closed loop segments is equal to the maximum number of edges connected to any node, divided by 2.
fn best_eulerian_circuit_segments(graph: &DockNodeGraph, circuit: &Vec<usize>) -> Option<Vec<Vec<usize>>> {
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
    // break the circuit into circular segments that start and end with that node_id.
    // The best set of segments is the one with the most segments and where the
    // length of the segments differ the least.
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
        
        let avg_segment_length = segments.iter()
            .map(|segment| segment.len())
            .sum::<usize>() as f32 / segments.len() as f32;

        // We want similar segment lengths, so calculate the spread as the
        // standard deviation of the segment lengths.
        let seg_lengths_spread = segments.iter()
        .map(|segment| (segment.len() as f32 - avg_segment_length).powi(2))
        .sum::<f32>()
        .sqrt() / segments.len() as f32;
        
        // First take the longest segment count, then if the segment count is the same
        // as the longest so far, take the one with the least length spread.
        if segments.len() > max_segments_count {
            max_segments_count = segments.len();
            min_segments_len_spread = seg_lengths_spread;
            best_segments = segments;
        } else if segments.len() == max_segments_count && seg_lengths_spread < min_segments_len_spread {
            min_segments_len_spread = seg_lengths_spread;
            best_segments = segments;
        }
    });
    if best_segments.is_empty() {
        return None;
    }
    Some(best_segments)
}

#[cfg(test)]
mod tests {
    use super::{
        AirshipDockingSide, Airships, DockNode, TriangulationExt, approx::assert_relative_eq,
        approximate_hull_diameter, four_point_angle_score, hull_ratio_distance_score,
        find_eulerian_circuit, best_eulerian_circuit_segments
    };
    use delaunator::{Point, Triangulation, triangulate};
    use itertools::Itertools;
    use rand::prelude::*;
    use vek::{Quaternion, Vec2, Vec3};

    use crate::{
        Index, all,
        civ::airship_route_map::*,
        sim::WorldSim,
        site::{self, Site, plot::PlotKindMeta},
        util::{DHashMap, DHashSet, seed_expan},
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

    fn approximate_hull_diameter_exact(
        nodes: &DHashMap<usize, DockNode>,
        points: &Vec<Point>,
    ) -> f64 {
        let mut max_distance = 0.0;
        for (node_id1, dock_node1) in nodes.iter() {
            for (node_id2, dock_node2) in nodes.iter() {
                if node_id1 != node_id2 && dock_node1.on_hull && dock_node2.on_hull {
                    let point1 = &points[*node_id1];
                    let point2 = &points[*node_id2];
                    let distance = (point1.x - point2.x).powi(2) + (point1.y - point2.y).powi(2);
                    let distance = distance.sqrt();
                    if distance < 0.0 {
                        panic!("Negative distance");
                    }
                    if distance > max_distance {
                        max_distance = distance;
                    }
                }
            }
        }
        max_distance
    }

    #[test]
    fn triangulation_hull_ratio_test() {
        let all_dock_points: Vec<Point> = [
            [31791, 24953],
            [6733, 26407],
            [4759, 31201],
            [5405, 6841],
            [959, 16927],
            [19085, 16359],
            [18766, 10742],
            [17307, 29523],
            [3004, 14014],
            [16633, 1021],
            [20626, 6630],
            [20185, 31775],
            [16639, 15775],
            [2764, 11666],
            [4788, 11350],
            [29767, 24404],
            [20907, 29217],
            [22692, 15360],
            [19558, 2056],
            [13343, 2217],
            [24279, 13001],
            [16879, 8183],
            [12342, 9998],
            [18943, 24631],
            [27777, 2311],
            [24860, 16049],
            [12267, 24077],
            [6123, 2813],
        ]
        .iter()
        .map(|&[x, y]| Point {
            x: x as f64,
            y: y as f64,
        })
        .collect();
        let triangulation = triangulate(&all_dock_points);
        let node_connections = triangulation.node_connections();
        let hull_dia_bounds = approximate_hull_diameter_exact(&node_connections, &all_dock_points);
        let hull_dia_approx = approximate_hull_diameter(&all_dock_points);
        assert_relative_eq!(hull_dia_bounds, hull_dia_approx, max_relative = 300.0);
        // println!("Hull diameter exact: {}, hull diameter approx: {}",
        // hull_dia_bounds, hull_dia_approx);
    }

    // $$$ Delete this test when done
    #[test]
    fn distance_score_test() {
        let hull_diameter = 100.0;
        let distances = [
            0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0,
        ];
        for distance in distances.iter() {
            println!(
                "Distance: {}, score: {}",
                distance,
                hull_ratio_distance_score(*distance, hull_diameter) * 0.9
            );
        }
    }

    // $$$ Delete this test when done
    #[test]
    fn angles_and_distance_test() {
        let angles = [
            0.0, 5.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0,
        ];
        for angle in angles.iter() {
            println!(
                "Angle: {}, score: {}",
                angle,
                four_point_angle_score(*angle)
            );
        }
    }

    #[test]
    fn triangulation_node_connections_test() {
        let all_dock_points: Vec<Point> = [
            [31791, 24953],
            [6733, 26407],
            [4759, 31201],
            [5405, 6841],
            [959, 16927],
            [19085, 16359],
            [18766, 10742],
            [17307, 29523],
            [3004, 14014],
            [16633, 1021],
            [20626, 6630],
            [20185, 31775],
            [16639, 15775],
            [2764, 11666],
            [4788, 11350],
            [29767, 24404],
            [20907, 29217],
            [22692, 15360],
            [19558, 2056],
            [13343, 2217],
            [24279, 13001],
            [16879, 8183],
            [12342, 9998],
            [18943, 24631],
            [27777, 2311],
            [24860, 16049],
            [12267, 24077],
            [6123, 2813],
        ]
        .iter()
        .map(|&[x, y]| Point {
            x: x as f64,
            y: y as f64,
        })
        .collect();
        let triangulation = triangulate(&all_dock_points);

        let node_connections = triangulation.node_connections();
        println!("Node count: {}, odd con: ", node_connections.len());

        // for node_id in 0..node_connections.len() {
        //     if let Some(node) = node_connections.get(&node_id) {
        //         println!("Node id {}: {} edges, on hull: {}", node.node_id,
        // node.connected.len(), node.on_hull);         for connected_node in
        // &node.connected {             println!("  {}, {} edges",
        // connected_node, node_connections[connected_node].connected.len());
        //         }
        //     } else {
        //         panic!("Node {} not found", node_id);
        //     }
        // }

        let hull_dia_approx = approximate_hull_diameter(&all_dock_points);
        println!("Hull diameter approx: {}", hull_dia_approx);

        // node_connections.keys() are the node indices (ids).
        // Since they are in a hashmap, they are not sorted.
        let mut search_order = node_connections.keys().copied().collect::<Vec<_>>();
        // search_order.sort();

        let mut loop_count = 0;
        let mut target_score_loop_iteration = 0;
        let mut max_score = 0.0;
        let mut max_connections: Vec<(usize, usize)> = Vec::default();
        let mut max_final_indeces = vec![];

        loop {
            search_order.shuffle(&mut rand::thread_rng());

            let mut node_conn_working_copy = node_connections.clone();
            let (high_score, best_connections, best_final_indeces) = triangulation
                .optimized_tessellation_edges(
                    &all_dock_points,
                    &mut node_conn_working_copy,
                    hull_dia_approx,
                    &search_order,
                );

            // println!("High score: {}, Best connections: {:?}, Best final indeces: {:?}",
            // high_score, best_connections, best_final_indeces);
            if high_score > max_score {
                max_score = high_score;
                max_connections = best_connections;
                max_final_indeces = best_final_indeces;
                println!("New max score: {}", max_score);
            }
            loop_count += 1;
            if target_score_loop_iteration == 0 && high_score > 0.8365520 {
                println!(
                    "Found target score 0.8365523 after {} iterations",
                    loop_count
                );
                target_score_loop_iteration = loop_count;
            }
            if loop_count > 100 {
                println!("Loop count exceeded 100, breaking");
                break;
            }
        }
        println!(
            "Max score: {}, target score loop count: {}",
            max_score, target_score_loop_iteration
        );
        println!("Max connections: {:?}", max_connections);
        println!("Max final indeces: {:?}", max_final_indeces);
    }

    #[test]
    fn eulerian_circuit_test() {
        let node_connections: DHashMap<usize, DockNode> = DHashMap::from_iter([
            (0, DockNode {
                node_id: 0,
                on_hull: false,
                connected: DHashSet::from_iter([23, 29, 26, 14, 19, 4]),
            }),
            (28, DockNode {
                node_id: 28,
                on_hull: false,
                connected: DHashSet::from_iter([23, 15, 25, 20, 21, 22]),
            }),
            (25, DockNode {
                node_id: 25,
                on_hull: false,
                connected: DHashSet::from_iter([23, 11, 28, 21]),
            }),
            (22, DockNode {
                node_id: 22,
                on_hull: false,
                connected: DHashSet::from_iter([23, 28, 27, 9, 3, 15]),
            }),
            (19, DockNode {
                node_id: 19,
                on_hull: false,
                connected: DHashSet::from_iter([0, 6, 29, 18, 2, 4]),
            }),
            (16, DockNode {
                node_id: 16,
                on_hull: false,
                connected: DHashSet::from_iter([10, 12, 20, 21]),
            }),
            (13, DockNode {
                node_id: 13,
                on_hull: true,
                connected: DHashSet::from_iter([7, 26, 9, 27, 3, 18]),
            }),
            (10, DockNode {
                node_id: 10,
                on_hull: false,
                connected: DHashSet::from_iter([24, 29, 11, 2, 16, 21]),
            }),
            (7, DockNode {
                node_id: 7,
                on_hull: true,
                connected: DHashSet::from_iter([26, 1, 13, 11]),
            }),
            (4, DockNode {
                node_id: 4,
                on_hull: false,
                connected: DHashSet::from_iter([0, 6, 14, 19]),
            }),
            (1, DockNode {
                node_id: 1,
                on_hull: true,
                connected: DHashSet::from_iter([7, 26, 8, 17]),
            }),
            (29, DockNode {
                node_id: 29,
                on_hull: false,
                connected: DHashSet::from_iter([0, 10, 24, 23, 19, 2]),
            }),
            (26, DockNode {
                node_id: 26,
                on_hull: false,
                connected: DHashSet::from_iter([0, 23, 14, 1, 27, 5, 7, 13]),
            }),
            (23, DockNode {
                node_id: 23,
                on_hull: false,
                connected: DHashSet::from_iter([0, 29, 25, 22, 28, 24, 11, 26]),
            }),
            (20, DockNode {
                node_id: 20,
                on_hull: true,
                connected: DHashSet::from_iter([18, 28, 12, 15, 16, 21]),
            }),
            (17, DockNode {
                node_id: 17,
                on_hull: false,
                connected: DHashSet::from_iter([5, 6, 8, 1]),
            }),
            (14, DockNode {
                node_id: 14,
                on_hull: false,
                connected: DHashSet::from_iter([0, 5, 26, 4]),
            }),
            (11, DockNode {
                node_id: 11,
                on_hull: false,
                connected: DHashSet::from_iter([10, 24, 23, 25, 21, 7]),
            }),
            (8, DockNode {
                node_id: 8,
                on_hull: true,
                connected: DHashSet::from_iter([18, 6, 1, 17]),
            }),
            (5, DockNode {
                node_id: 5,
                on_hull: false,
                connected: DHashSet::from_iter([6, 26, 14, 17]),
            }),
            (2, DockNode {
                node_id: 2,
                on_hull: false,
                connected: DHashSet::from_iter([10, 29, 12, 19]),
            }),
            (27, DockNode {
                node_id: 27,
                on_hull: false,
                connected: DHashSet::from_iter([26, 9, 13, 22]),
            }),
            (24, DockNode {
                node_id: 24,
                on_hull: false,
                connected: DHashSet::from_iter([10, 29, 11, 23]),
            }),
            (21, DockNode {
                node_id: 21,
                on_hull: false,
                connected: DHashSet::from_iter([10, 11, 25, 28, 20, 16]),
            }),
            (18, DockNode {
                node_id: 18,
                on_hull: true,
                connected: DHashSet::from_iter([6, 12, 8, 19, 20, 13]),
            }),
            (15, DockNode {
                node_id: 15,
                on_hull: true,
                connected: DHashSet::from_iter([28, 20, 3, 22]),
            }),
            (12, DockNode {
                node_id: 12,
                on_hull: false,
                connected: DHashSet::from_iter([18, 2, 16, 20]),
            }),
            (9, DockNode {
                node_id: 9,
                on_hull: false,
                connected: DHashSet::from_iter([13, 27, 3, 22]),
            }),
            (6, DockNode {
                node_id: 6,
                on_hull: false,
                connected: DHashSet::from_iter([4, 8, 5, 18, 19, 17]),
            }),
            (3, DockNode {
                node_id: 3,
                on_hull: true,
                connected: DHashSet::from_iter([13, 9, 15, 22]),
            }),
        ]);
        if cfg!(debug_assertions) {
            println!("Node connections: {:?}", node_connections);
        }

        let all_dock_points: Vec<Vec2<f32>> = [
            [20687, 16148],
            [28958, 28286],
            [23493, 7853],
            [2896, 31605],
            [25291, 17355],
            [24565, 26093],
            [28670, 19708],
            [18119, 31519],
            [32012, 25350],
            [3974, 29160],
            [20837, 6546],
            [17480, 7396],
            [28063, 9733],
            [6810, 31960],
            [23526, 21192],
            [1239, 1089],
            [25816, 5518],
            [29657, 26071],
            [30661, 13099],
            [27523, 14145],
            [28255, 3729],
            [18914, 5284],
            [2345, 20351],
            [10481, 14845],
            [17950, 9572],
            [16656, 5288],
            [20282, 27928],
            [4008, 23006],
            [6021, 2687],
            [19126, 11214],
        ]
        .iter()
        .map(|&[x, y]| Vec2::new(x as f32, y as f32))
        .collect();

        if let Some(circuit) = find_eulerian_circuit(&node_connections) {

            if cfg!(debug_assertions) {
                println!("Eulerian circuit: {:?}", circuit);
            }

            // Print the circuit with distances
            if cfg!(debug_assertions) {
                let mut prev_node_id: Option<usize> = None;
                circuit.iter().for_each(|&to_node_id| {
                    if let Some(from_node_id) = prev_node_id {
                        if let Some(from_pt) = all_dock_points.get(from_node_id)
                            && let Some(to_pt) = all_dock_points.get(to_node_id)
                        {
                            println!(
                                "From node {} {:?}, To node {} {:?}, Distance: {}",
                                from_node_id,
                                from_pt,
                                to_node_id,
                                to_pt,
                                from_pt.distance(*to_pt)
                            );
                        } else {
                            println!("Node id {} not found", from_node_id);
                        }
                    }
                    prev_node_id = Some(to_node_id);
                });
            }
            // get the node_connections keys, which are node ids.
            // Sort the nodes (node ids) by the number of connections to other nodes.
            let sorted_node_ids: Vec<usize> = node_connections
                .keys()
                .copied()
                .sorted_by_key(|&node_id| node_connections[&node_id].connected.len())
                .rev()
                .collect();
            if cfg!(debug_assertions) {
                // debug print the sorted node ids
                sorted_node_ids.iter().for_each(|&node_id| {
                    if let Some(node) = node_connections.get(&node_id) {
                        println!(
                            "Node id {}: {} edges",
                            node.node_id,
                            node.connected.len(),
                        );
                    }
                });
            }

            let mut max_segments_count = 0;
            let mut min_segments_len_spread = f32::MAX;
            let mut best_segments = Vec::new();

            // For each node_id in the sorted node ids,
            // break the circuit into circular segments that start and end with that node_id.
            // The best set of segments is the one with the most segments and where the
            // length of the segments differ the least.
            sorted_node_ids.iter().for_each(|&node_id| {
                if cfg!(debug_assertions) {
                    println!("Segments starting with node id {}:", node_id);
                }

                let mut segments = Vec::new();
                let mut current_segment = Vec::new();
                let circuit_len = circuit.len();
                let mut starting_index = usize::MAX;
                let mut end_index = usize::MAX;
                let mut prev_value = usize::MAX;
                
                for (index, &value) in circuit.iter().cycle().enumerate() {
                    if value == node_id {
                        if starting_index == usize::MAX {
                            starting_index = index;
                            if starting_index > 0 {
                                end_index = index + circuit_len - 1;
                            } else {
                                end_index = index + circuit_len - 2;
                            }
                            if cfg!(debug_assertions) {
                                println!("starting_index: {}, circuit_len: {}, end_index: {}", starting_index, circuit_len, end_index);
                            }
                        }
                        if !current_segment.is_empty() {
                            current_segment.push(value);
                            if cfg!(debug_assertions) {
                                println!("Pushing segment: {:?}", current_segment);
                            }
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
                        if cfg!(debug_assertions) {
                            println!("Breaking out of cycle at index {}", index);
                        }
                        break;
                    }
                }
                
                // Add the last segment
                if !current_segment.is_empty() {
                    current_segment.push(node_id);
                    if cfg!(debug_assertions) {
                        println!("Pushing segment: {:?}", current_segment);
                    }
                    segments.push(current_segment);
                }
                
                if cfg!(debug_assertions) {
                    println!("Segments: {:?}", segments);
                }

                let avg_segment_length = segments.iter()
                    .map(|segment| segment.len())
                    .sum::<usize>() as f32 / segments.len() as f32;

                // We want similar segment lengths, so calculate the spread as the
                // standard deviation of the segment lengths.
                let seg_lengths_spread = segments.iter()
                .map(|segment| (segment.len() as f32 - avg_segment_length).powi(2))
                .sum::<f32>()
                .sqrt() / segments.len() as f32;
                
                if cfg!(debug_assertions) {
                    println!("avg_segment_length: {}, seg_lengths_spread: {}", avg_segment_length, seg_lengths_spread);
                }
                // First take the longest segment count, then if the segment count is the same
                // as the longest so far, take the one with the least length spread.
                if segments.len() > max_segments_count {
                    max_segments_count = segments.len();
                    min_segments_len_spread = seg_lengths_spread;
                    best_segments = segments;
                } else if segments.len() == max_segments_count && seg_lengths_spread < min_segments_len_spread {
                    min_segments_len_spread = seg_lengths_spread;
                    best_segments = segments;
                }
            });
            if cfg!(debug_assertions) {
                println!("max_segments_count: {}, min_segments_len_spread: {}", max_segments_count, min_segments_len_spread);
                println!("Best segments: {:?}", best_segments);
            }

            if let Some(best_segments2) = best_eulerian_circuit_segments(
                &node_connections,
                &circuit,
            ) {
                if cfg!(debug_assertions) {
                    println!("Best segments2: {:?}", best_segments2);
                }
                assert_eq!(best_segments.len(), best_segments2.len());
                assert_eq!(best_segments2.len(), 4);
                assert_eq!(best_segments[0].len(), best_segments2[0].len());
                assert_eq!(best_segments[1].len(), best_segments2[1].len());
                assert_eq!(best_segments[2].len(), best_segments2[2].len());
                assert_eq!(best_segments[3].len(), best_segments2[3].len());
                assert_eq!(best_segments[3].len(), best_segments2[3].len());
                assert_eq!(best_segments[0], best_segments2[0]);
                assert_eq!(best_segments[1], best_segments2[1]);
                assert_eq!(best_segments[2], best_segments2[2]);
                assert_eq!(best_segments[3], best_segments2[3]);
            } else {
                panic!("No best segments2 found");
            }

        } else {
            panic!("No Eulerian circuit found");
        }
    }

}
