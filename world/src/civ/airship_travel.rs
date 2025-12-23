use crate::{
    Index,
    sim::WorldSim,
    site::{self, Site, plot::PlotKindMeta},
    util::{DHashMap, DHashSet, seed_expan},
};
use common::{
    store::{Id, Store},
    terrain::{MapSizeLg, TERRAIN_CHUNK_BLOCKS_LG},
    util::Dir,
};
use delaunator::{Point, Triangulation, triangulate};
use itertools::Itertools;
use rand::{SeedableRng, prelude::*};
use rand_chacha::ChaChaRng;
use tracing::error;
use vek::*;

#[cfg(feature = "airship_maps")]
use crate::civ::airship_route_map::*;

#[cfg(debug_assertions)]
macro_rules! debug_airships {
    ($level:expr, $($arg:tt)*) => {
        match $level {
            0 => tracing::info!($($arg)*),
            1 => tracing::warn!($($arg)*),
            2 => tracing::error!($($arg)*),
            3 => tracing::debug!($($arg)*),
            4 => tracing::trace!($($arg)*),
            _ => tracing::trace!($($arg)*),
        }
    }
}

#[cfg(not(debug_assertions))]
macro_rules! debug_airships {
    ($($arg:tt)*) => {};
}

/// A docking position (id, position). The docking position id is
/// an index of all docking positions in the world.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AirshipDockingPosition(pub u32, pub Vec3<f32>);

/// The AirshipDock Sites are always oriented along a cardinal direction.
/// The docking platforms are likewise on the sides of the dock perpendicular
/// to a cardinal axis.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash)]
pub enum AirshipDockPlatform {
    #[default]
    NorthPlatform,
    EastPlatform,
    SouthPlatform,
    WestPlatform,
}

/// An airship can dock with its port or starboard side facing the dock.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum AirshipDockingSide {
    #[default]
    Port,
    Starboard,
}

impl AirshipDockingSide {
    const EAST_REF_VEC: Vec2<f32> = Vec2 { x: 1.0, y: 0.0 };
    const NORTH_REF_VEC: Vec2<f32> = Vec2 { x: 0.0, y: 1.0 };
    const SOUTH_REF_VEC: Vec2<f32> = Vec2 { x: 0.0, y: -1.0 };
    const WEST_REF_VEC: Vec2<f32> = Vec2 { x: -1.0, y: 0.0 };

    /// When docking, the side to use depends on the angle the airship is
    /// approaching the dock from, and the platform of the airship dock that
    /// the airship is docking at.
    /// For example, when docking at the North Platform:
    ///
    /// | From the          |  Docking Side |
    /// |:----------------- |:--------------|
    /// | West              |  Starboard    |
    /// | Northwest         |  Starboard    |
    /// | North             |  Starboard    |
    /// | Northeast         |  Port         |
    /// | East              |  Port         |
    /// | Southeast         |  Port         |
    /// | South             |  Port         |
    /// | Southwest         |  Starboard    |
    pub fn from_dir_to_platform(dir: &Vec2<f32>, platform: &AirshipDockPlatform) -> Self {
        // get the reference vector and precompute whether to flip the angle based on
        // the dir input.
        let (ref_vec, negate_angle) = match platform {
            AirshipDockPlatform::NorthPlatform => (&AirshipDockingSide::NORTH_REF_VEC, dir.x < 0.0),
            AirshipDockPlatform::EastPlatform => (&AirshipDockingSide::EAST_REF_VEC, dir.y > 0.0),
            AirshipDockPlatform::SouthPlatform => (&AirshipDockingSide::SOUTH_REF_VEC, dir.x > 0.0),
            AirshipDockPlatform::WestPlatform => (&AirshipDockingSide::WEST_REF_VEC, dir.y < 0.0),
        };
        let mut angle = dir.angle_between(*ref_vec).to_degrees();
        if negate_angle {
            angle = -angle;
        }
        match angle as i32 {
            -360..=0 => AirshipDockingSide::Port,
            _ => AirshipDockingSide::Starboard,
        }
    }
}

/// Information needed for an airship to travel to and dock at an AirshipDock
/// plot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirshipDockingApproach {
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
    /// The midpoint of the cruise phase of flight.
    pub midpoint: Vec2<f32>,
    /// The end point of the cruise phase of flight.
    pub approach_transition_pos: Vec2<f32>,
    /// There are ramps on both the port and starboard sides of the airship.
    /// This gives the side that the airship will dock on.
    pub side: AirshipDockingSide,
    /// The site where the airship will be docked at the end of the
    /// approach.
    pub site_id: Id<Site>,
}

/// The docking postions at an AirshipDock plot.
#[derive(Clone, Debug)]
pub struct AirshipDockPositions {
    /// The center of the AirshipDock plot. From the world generation code.
    pub center: Vec2<f32>,
    /// The docking positions for the airship, derived from the
    /// positions calculated in the world generation code.
    pub docking_positions: Vec<AirshipDockingPosition>,
    /// The id of the Site where the AirshipDock is located.
    pub site_id: Id<site::Site>,
}

/// The flight phases of an airship.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[repr(usize)]
pub enum AirshipFlightPhase {
    DepartureCruise,
    ApproachCruise,
    Transition,
    Descent,
    #[default]
    Docked,
    Ascent,
}

impl std::fmt::Display for AirshipFlightPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AirshipFlightPhase::DepartureCruise => write!(f, "DepartureCruise"),
            AirshipFlightPhase::ApproachCruise => write!(f, "ApproachCruise"),
            AirshipFlightPhase::Transition => write!(f, "Transition"),
            AirshipFlightPhase::Descent => write!(f, "Descent"),
            AirshipFlightPhase::Docked => write!(f, "Docked"),
            AirshipFlightPhase::Ascent => write!(f, "Ascent"),
        }
    }
}

/// One flight phase of an airship route leg.
/// The position is the destination (or only) location for the segment.
#[derive(Clone, Default, Debug)]
pub struct RouteLegSegment {
    /// Flight phase describes what the airship is doing
    /// and is used in the NPC logic to define how the airship moves to the
    /// position.
    pub flight_phase: AirshipFlightPhase,
    /// The starting (or only) position for the segment.
    pub from_world_pos: Vec2<f32>,
    /// The destination (or only) position for the segment.
    pub to_world_pos: Vec2<f32>,
    /// The distance covered in world blocks.
    pub distance: f32,
    /// How long it's supposed to take to cover the distance in seconds.
    pub duration: f32,
    /// The time at which the airship is supposed to arrive at the destination,
    /// or the end of the docking phase. This is the cumulative time for the
    /// entire route including all previous leg segments.
    pub route_time: f64,
}

/// One leg of an airship route.
/// The leg starts when the airship leaves the docking area at the end of the
/// ascent phase and ends at the end of the ascent phase at the docking
/// destination. Leg segments are:
/// - Departure Cruise (DC) from the end of the previous ascent to the leg
///   midpoint.
/// - Approach Cruise (AC) from the leg midpoint to the transition start.
/// - Transition (T) from the transition pos to above the dock.
/// - Descent (D) to the docking position.
/// - Parked/Docked (P) at the dock.
/// - Ascent (A) back to cruising height above the dock.
#[derive(Clone, Default, Debug)]
pub struct AirshipRouteLeg {
    /// The index of the destination in Airships::docking_positions.
    pub dest_index: usize,
    /// The assigned docking platform at the destination dock for this leg.
    pub platform: AirshipDockPlatform,
    /// The leg segments.
    pub segments: [RouteLegSegment; 6],
}

/// An airship route is a series of legs that form a continuous loop.
/// Each leg goes from one airship docking site to another.
#[derive(Debug, Clone)]
pub struct AirshipRoute {
    pub legs: Vec<AirshipRouteLeg>,
    pub total_time: f64,
    pub airship_time_spacing: f64,
    pub cruising_height: f32,
    pub spawning_locations: Vec<AirshipSpawningLocation>,
}

/// Data for airship operations. This is generated world data.
#[derive(Clone, Default)]
pub struct Airships {
    /// The docking positions for all airship docks in the world.
    pub airship_docks: Vec<AirshipDockPositions>,
    /// The routes flown by the collective airships in the world.
    pub routes: Vec<AirshipRoute>,
    /// The speed of simulated airships (and the nominal speed of loaded
    /// airships) in world blocks per second.
    pub nominal_speed: f32,
}

/// Information needed for placing an airship in the world when the world is
/// generated (each time the server starts).
#[derive(Debug, Clone)]
pub struct AirshipSpawningLocation {
    /// The flight phase the airship is in when spawned.
    pub flight_phase: AirshipFlightPhase,
    /// The 2D position of the airship when spawned.
    pub pos: Vec2<f32>,
    /// The direction the airship is facing when spawned.
    pub dir: Vec2<f32>,
    /// For cruise and transition phases, the height above terrain.
    /// For descent, docked, and ascent phases, the actual z position.
    pub height: f32,
    /// The index of the route the airship is flying.
    pub route_index: usize,
    /// The index of the leg in the route the airship is flying.
    pub leg_index: usize,
    /// The effective route time when the airship is spawned.
    /// This will be less than the route time for the end
    /// of the phase the airship is in.
    pub spawn_route_time: f64,
}

// Internal data structures

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

    /// Get the docking position that matches the given platform.
    fn docking_position(&self, platform: AirshipDockPlatform) -> Vec3<f32> {
        self.docking_positions
            .iter()
            .find_map(|&docking_position| {
                // The docking position is the one that matches the platform.
                // The platform is determined by the direction of the docking position
                // relative to the center of the dock.
                let docking_position_platform =
                    AirshipDockPlatform::from_dir(docking_position.1.xy() - self.center);
                if docking_position_platform == platform {
                    Some(docking_position.1)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                // If no docking position is found, return the dock center.
                self.center.with_z(1000.0)
            })
    }
}

// These are used in AirshipDockPlatform::choices_from_dir

static SWEN_PLATFORMS: [AirshipDockPlatform; 4] = [
    AirshipDockPlatform::SouthPlatform,
    AirshipDockPlatform::WestPlatform,
    AirshipDockPlatform::EastPlatform,
    AirshipDockPlatform::NorthPlatform,
];

static SEWN_PLATFORMS: [AirshipDockPlatform; 4] = [
    AirshipDockPlatform::SouthPlatform,
    AirshipDockPlatform::EastPlatform,
    AirshipDockPlatform::WestPlatform,
    AirshipDockPlatform::NorthPlatform,
];

static WNSE_PLATFORMS: [AirshipDockPlatform; 4] = [
    AirshipDockPlatform::WestPlatform,
    AirshipDockPlatform::NorthPlatform,
    AirshipDockPlatform::SouthPlatform,
    AirshipDockPlatform::EastPlatform,
];

static WSNE_PLATFORMS: [AirshipDockPlatform; 4] = [
    AirshipDockPlatform::WestPlatform,
    AirshipDockPlatform::SouthPlatform,
    AirshipDockPlatform::NorthPlatform,
    AirshipDockPlatform::EastPlatform,
];

static NEWS_PLATFORMS: [AirshipDockPlatform; 4] = [
    AirshipDockPlatform::NorthPlatform,
    AirshipDockPlatform::EastPlatform,
    AirshipDockPlatform::WestPlatform,
    AirshipDockPlatform::SouthPlatform,
];

static NWES_PLATFORMS: [AirshipDockPlatform; 4] = [
    AirshipDockPlatform::NorthPlatform,
    AirshipDockPlatform::WestPlatform,
    AirshipDockPlatform::EastPlatform,
    AirshipDockPlatform::SouthPlatform,
];

static ESNW_PLATFORMS: [AirshipDockPlatform; 4] = [
    AirshipDockPlatform::EastPlatform,
    AirshipDockPlatform::SouthPlatform,
    AirshipDockPlatform::NorthPlatform,
    AirshipDockPlatform::WestPlatform,
];

static ENSW_PLATFORMS: [AirshipDockPlatform; 4] = [
    AirshipDockPlatform::EastPlatform,
    AirshipDockPlatform::NorthPlatform,
    AirshipDockPlatform::SouthPlatform,
    AirshipDockPlatform::WestPlatform,
];

/// The docking platforms used on each leg of the airship route segments is
/// determined when the routes are generated. Route segments are continuous
/// loops that are deconflicted by using only one docking platform for any given
/// leg of a route segment. Since there are four docking platforms per airship
/// dock, there are at most four route segments passing through a given airship
/// dock. The docking platforms are also optimized so that on the incoming leg
/// of a route segment, the airship uses the docking platform that is closest to
/// the arrival direction (if possible), while still using only one docking
/// platform per route segment leg.
impl AirshipDockPlatform {
    /// Get the preferred docking platform based on the direction vector.
    pub fn from_dir(dir: Vec2<f32>) -> Self {
        if let Some(dir) = dir.try_normalized() {
            let mut angle = dir.angle_between(Vec2::unit_y()).to_degrees();
            if dir.x < 0.0 {
                angle = -angle;
            }
            match angle as i32 {
                -360..=-135 => AirshipDockPlatform::SouthPlatform,
                -134..=-45 => AirshipDockPlatform::WestPlatform,
                -44..=45 => AirshipDockPlatform::NorthPlatform,
                46..=135 => AirshipDockPlatform::EastPlatform,
                136..=360 => AirshipDockPlatform::SouthPlatform,
                _ => AirshipDockPlatform::NorthPlatform, // should never happen
            }
        } else {
            AirshipDockPlatform::NorthPlatform // default value, should never happen
        }
    }

    /// Get the platform choices in order of preference based on the direction
    /// vector. The first choice is always the most direct plaform given the
    /// approach direction. Then, the next two choices are the platforms for
    /// the cardinal directions on each side of the approach direction, and
    /// the last choice is the platform on the opposite side of the dock.
    /// The return value is one of the ABCD_PLATFORMS arrays defined above.
    pub fn choices_from_dir(dir: Vec2<f32>) -> &'static [AirshipDockPlatform] {
        if let Some(dir) = dir.try_normalized() {
            let mut angle = dir.angle_between(Vec2::unit_y()).to_degrees();
            if dir.x < 0.0 {
                angle = -angle;
            }
            // This code works similar to the Direction enum in the common crate.
            // Angle between produces the smallest angle between two vectors,
            // so when dir.x is negative, we force the angle to be negative.
            // 0 or 360 is North. It is assumed that the angle ranges from -360 to 360
            // degrees even though angles less than -180 or greater than 180
            // should never be seen.
            match angle as i32 {
                -360..=-135 => {
                    // primary is SouthPlatform
                    // As a fallback (for when the south platform is already claimed),
                    // if the direction is more towards the west, use the west platform,
                    // and if the direction is more towards the east, use the east platform.
                    // The north platform is always the last resort. All fallback blocks
                    // below work similarly.
                    if angle as i32 > -180 {
                        &SWEN_PLATFORMS
                    } else {
                        &SEWN_PLATFORMS
                    }
                },
                -134..=-45 => {
                    // primary is WestPlatform
                    if angle as i32 > -90 {
                        &WNSE_PLATFORMS
                    } else {
                        &WSNE_PLATFORMS
                    }
                },
                -44..=45 => {
                    // primary is NorthPlatform
                    if angle as i32 > 0 {
                        &NEWS_PLATFORMS
                    } else {
                        &NWES_PLATFORMS
                    }
                },
                46..=135 => {
                    // primary is EastPlatform
                    if angle as i32 > 90 {
                        &ESNW_PLATFORMS
                    } else {
                        &ENSW_PLATFORMS
                    }
                },
                136..=360 => {
                    // primary is SouthPlatform
                    if angle as i32 > 180 {
                        &SWEN_PLATFORMS
                    } else {
                        &SEWN_PLATFORMS
                    }
                },
                _ => &SEWN_PLATFORMS,
            }
        } else {
            &SEWN_PLATFORMS
        }
    }

    /// Get the direction vector that the airship would be facing when docked.
    fn airship_dir_for_side(&self, side: AirshipDockingSide) -> Dir {
        match self {
            AirshipDockPlatform::NorthPlatform => match side {
                AirshipDockingSide::Starboard => Dir::new(Vec2::unit_x().with_z(0.0)),
                AirshipDockingSide::Port => Dir::new(-Vec2::unit_x().with_z(0.0)),
            },
            AirshipDockPlatform::EastPlatform => match side {
                AirshipDockingSide::Starboard => Dir::new(-Vec2::unit_y().with_z(0.0)),
                AirshipDockingSide::Port => Dir::new(Vec2::unit_y().with_z(0.0)),
            },
            AirshipDockPlatform::SouthPlatform => match side {
                AirshipDockingSide::Starboard => Dir::new(-Vec2::unit_x().with_z(0.0)),
                AirshipDockingSide::Port => Dir::new(Vec2::unit_x().with_z(0.0)),
            },
            AirshipDockPlatform::WestPlatform => match side {
                AirshipDockingSide::Starboard => Dir::new(Vec2::unit_y().with_z(0.0)),
                AirshipDockingSide::Port => Dir::new(-Vec2::unit_y().with_z(0.0)),
            },
        }
    }
}

/// A node on the triangulation of the world docking sites, with
/// data on the nodes that are connected to it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockNode {
    /// An index into the array of all nodes in the graph.
    pub node_id: usize,
    /// True if the node is on the outer hull (convex hull) of the
    /// triangulation.
    pub on_hull: bool,
    /// The nodes that are connected to this node.
    pub connected: Vec<usize>,
}

impl Airships {
    /// The duration of the ascent flight phase.
    pub const AIRSHIP_ASCENT_DURATION: f64 = 30.0;
    /// The duration of the descent flight phase.
    pub const AIRSHIP_DESCENT_DURATION: f64 = 30.0;
    /// The duration of the docked phase.
    pub const AIRSHIP_DOCKING_DURATION: f64 = 60.0;
    /// The time spacing between airships on the same route.
    pub const AIRSHIP_TIME_SPACING: f64 = 240.0;
    /// The Z offset between the docking alignment point and the AirshipDock
    /// plot docking position.
    const AIRSHIP_TO_DOCK_Z_OFFSET: f32 = -3.0;
    /// The ratio of the speed used when transitioning from cruising to docking
    /// as a percentage of the nominal airship speed.
    pub const AIRSHIP_TRANSITION_SPEED_RATIO: f32 = 0.75;
    /// The cruising height varies by route index and there can be only four
    /// routes.
    pub const CRUISE_HEIGHTS: [f32; 4] = [400.0, 475.0, 550.0, 625.0];
    /// The distance from the docking position where the airship starts the
    /// transition flight phase.
    const DOCKING_TRANSITION_OFFSET: f32 = 175.0;
    /// The vector from the dock alignment point when the airship is docked on
    /// the port side.
    const DOCK_ALIGN_POS_PORT: Vec2<f32> =
        Vec2::new(Airships::DOCK_ALIGN_X, -Airships::DOCK_ALIGN_Y);
    /// The vector from the dock alignment point on the airship when the airship
    /// is docked on the starboard side.
    const DOCK_ALIGN_POS_STARBOARD: Vec2<f32> =
        Vec2::new(-Airships::DOCK_ALIGN_X, -Airships::DOCK_ALIGN_Y);
    // TODO: These alignment offsets are specific to the airship model. If new
    // models are added, a more generic way to determine the alignment offsets
    // should be used.
    /// The absolute offset from the airship's position to the docking alignment
    /// point on the X axis. The airship is assumed to be facing positive Y.
    const DOCK_ALIGN_X: f32 = 18.0;
    /// The offset from the airship's position to the docking alignment point on
    /// the Y axis. The airship is assumed to be facing positive Y.
    /// This is positive if the docking alignment point is in front of the
    /// airship's center position.
    const DOCK_ALIGN_Y: f32 = 1.0;
    /// The minimum distance from the route leg midpoint to the world
    /// boundaries.
    const ROUTE_LEG_MIDPOINT_MARGIN: f32 = 200.0;
    /// The angle for calculating the route leg midpoint.
    const ROUTE_LEG_MIDPOINT_OFFSET_RADIANS: f32 = 0.087266;

    // 5 degrees

    #[inline(always)]
    pub fn docking_duration() -> f32 { Airships::AIRSHIP_DOCKING_DURATION as f32 }

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

    /// Convienence function that returns the next route leg accounting for wrap
    /// around.
    pub fn increment_route_leg(&self, route_index: usize, leg_index: usize) -> usize {
        if route_index >= self.routes.len() {
            error!("Invalid route index: {}", route_index);
            return 0;
        }
        (leg_index + 1) % self.routes[route_index].legs.len()
    }

    /// Convienence function that returns the previous route leg accounting for
    /// wrap around.
    pub fn decrement_route_leg(&self, route_index: usize, leg_index: usize) -> usize {
        if route_index >= self.routes.len() {
            error!("Invalid route index: {}", route_index);
            return 0;
        }
        if leg_index > 0 {
            leg_index - 1
        } else {
            self.routes[route_index].legs.len() - 1
        }
    }

    /// Convienence function returning the number of routes.
    pub fn route_count(&self) -> usize { self.routes.len() }

    /// Safe function to get the number of AirshipDock sites on a route.
    pub fn docking_site_count_for_route(&self, route_index: usize) -> usize {
        if route_index >= self.routes.len() {
            error!("Invalid route index: {}", route_index);
            return 0;
        }
        self.routes[route_index].legs.len()
    }

    /// Calculate the legs for each route.
    /// A docking platform is assigned for each leg of each route. Each route
    /// in the route_segments argument is a series (Vec) of docking node indices
    /// on the docking site graph. This function loops over the routes docking
    /// nodes and assigns a docking platform based on the approach direction
    /// to each dock node while making sure that no docking platform is used
    /// more than once (globally, over all routes). It then calculates the
    /// leg segments (flight phases) for each leg of each route. The output
    /// is a Vec of up to four routes, each of which is a closed loop of
    /// route legs.
    fn create_route_legs(
        &mut self,
        route_segments: &[Vec<usize>],
        dock_locations: &[Vec2<f32>],
        map_size_lg: &MapSizeLg,
    ) -> Vec<AirshipRoute> {
        let mut incoming_edges = DHashMap::default();
        for segment in route_segments.iter() {
            if segment.len() < 3 {
                continue;
            }
            let mut prev_node_id = segment[0];
            segment.iter().skip(1).for_each(|&node_id| {
                incoming_edges
                    .entry(node_id)
                    .or_insert_with(Vec::new)
                    .push(prev_node_id);
                prev_node_id = node_id;
            });
        }

        let mut leg_platforms = DHashMap::default();

        incoming_edges.iter().for_each(|(node_id, edges)| {
            let dock_location = dock_locations[*node_id];
            let mut used_platforms = DHashSet::default();
            for origin in edges {
                let origin_location = dock_locations[*origin];
                // Determine the platform to dock using the direction from the dock location
                // to the origin location
                let rev_approach_dir = origin_location - dock_location;
                let docking_platforms = AirshipDockPlatform::choices_from_dir(rev_approach_dir);
                let docking_platform = docking_platforms
                    .iter()
                    .find(|&platform| !used_platforms.contains(platform))
                    .copied()
                    .unwrap_or(AirshipDockPlatform::NorthPlatform);
                leg_platforms.insert((*origin, *node_id), docking_platform);
                used_platforms.insert(docking_platform);
            }
        });

        #[cfg(debug_assertions)]
        {
            debug_airships!(4, "Route segments: {:?}", route_segments);
            debug_airships!(4, "Leg platforms: {:?}", leg_platforms);
        }

        self.nominal_speed = common::comp::ship::Body::DefaultAirship.get_speed();
        debug_airships!(4, "Nominal speed {}", self.nominal_speed);

        // The incoming edges control the docking platforms used for each leg of the
        // route. The outgoing platform for leg i must match the incoming
        // platform for leg i-1. For the first leg, get the 'from' platform from
        // the last pair of nodes in the segment.
        let mut routes = Vec::new();
        route_segments
            .iter()
            .enumerate()
            .for_each(|(route_index, segment)| {
                assert!(
                    segment.len() > 2,
                    "Segments must have at least two nodes and they must wrap around."
                );
                let mut route_legs = Vec::new();
                let mut route_time = 0.0;
                let leg_start = &segment[segment.len() - 2..];
                for leg_index in 0..segment.len() - 1 {
                    let from_node = segment[leg_index];
                    let to_node = segment[leg_index + 1];
                    if leg_index == 0 {
                        assert!(
                            from_node == leg_start[1],
                            "The 'previous' leg's 'to' node must match the current leg's 'from' \
                             node."
                        );
                    }
                    let to_platform = leg_platforms.get(&(from_node, to_node)).copied().unwrap_or(
                        AirshipDockPlatform::from_dir(
                            dock_locations[from_node] - dock_locations[to_node],
                        ),
                    );
                    let dest_dock_positions = &self.airship_docks[to_node];
                    let from_dock_positions = &self.airship_docks[from_node];
                    let (midpoint, approach_transition_pos) = self.approach_waypoints(
                        &from_dock_positions.center,
                        dest_dock_positions,
                        to_platform,
                        map_size_lg,
                    );
                    let dest_dock_pos = dest_dock_positions.docking_position(to_platform).xy();

                    // depature cruise (DC) from the end of the previous ascent to the leg midpoint.
                    // The departure platform is not known here, so just use the dock center.
                    // distance is from the departure dock center to the midpoint.
                    // duration is distance / nominal speed
                    let dc_dist = from_dock_positions
                        .center
                        .as_::<f64>()
                        .distance(midpoint.as_());
                    let dc_dur = dc_dist / self.nominal_speed as f64;
                    let dc_completion_time = route_time + dc_dur;

                    // approach cruise (AC) from the leg midpoint to the transition start.
                    // distance is from the midpoint to approach_transition_pos.
                    // duration is distance / nominal speed
                    let ac_dist = midpoint
                        .as_::<f64>()
                        .distance(approach_transition_pos.as_());
                    let ac_dur = ac_dist / self.nominal_speed as f64;
                    let ac_completion_time = dc_completion_time + ac_dur;

                    // transition (T) from approach_transition_pos to above the dock.
                    // distance is from approach_transition_pos to the dock position.
                    // duration is distance / (nominal speed * AIRSHIP_TRANSITION_SPEED_RATIO)
                    let t_dist = approach_transition_pos
                        .as_::<f64>()
                        .distance(dest_dock_pos.as_());
                    let t_dur = t_dist
                        / (self.nominal_speed * Airships::AIRSHIP_TRANSITION_SPEED_RATIO) as f64;
                    let t_completion_time = ac_completion_time + t_dur;

                    // descent (D) to the docking position.
                    // distance is 0 (no x,y movement)
                    // duration is fixed at AIRSHIP_DESCENT_DURATION
                    let d_completion_time = t_completion_time + Airships::AIRSHIP_DESCENT_DURATION;

                    // parked/docked (P) at the dock.
                    // distance is 0
                    // duration is fixed at AIRSHIP_DOCKING_DURATION
                    let p_completion_time = d_completion_time + Airships::AIRSHIP_DOCKING_DURATION;

                    // ascent (A) back to cruising height above the dock.
                    // distance is 0
                    // duration is fixed at AIRSHIP_ASCENT_DURATION
                    let a_completion_time = p_completion_time + Airships::AIRSHIP_ASCENT_DURATION;

                    route_legs.push(AirshipRouteLeg {
                        dest_index: to_node,
                        platform: to_platform,
                        segments: [
                            RouteLegSegment {
                                flight_phase: AirshipFlightPhase::DepartureCruise,
                                from_world_pos: from_dock_positions.center,
                                to_world_pos: midpoint,
                                distance: from_dock_positions.center.distance(midpoint),
                                duration: dc_dur as f32,
                                route_time: dc_completion_time,
                            },
                            RouteLegSegment {
                                flight_phase: AirshipFlightPhase::ApproachCruise,
                                from_world_pos: midpoint,
                                to_world_pos: approach_transition_pos,
                                distance: midpoint.distance(approach_transition_pos),
                                duration: ac_dur as f32,
                                route_time: ac_completion_time,
                            },
                            RouteLegSegment {
                                flight_phase: AirshipFlightPhase::Transition,
                                from_world_pos: approach_transition_pos,
                                to_world_pos: dest_dock_pos,
                                distance: approach_transition_pos.distance(dest_dock_pos),
                                duration: t_dur as f32,
                                route_time: t_completion_time,
                            },
                            RouteLegSegment {
                                flight_phase: AirshipFlightPhase::Descent,
                                from_world_pos: dest_dock_pos,
                                to_world_pos: dest_dock_pos,
                                distance: 0.0,
                                duration: Airships::AIRSHIP_DESCENT_DURATION as f32,
                                route_time: d_completion_time,
                            },
                            RouteLegSegment {
                                flight_phase: AirshipFlightPhase::Docked,
                                from_world_pos: dest_dock_pos,
                                to_world_pos: dest_dock_pos,
                                distance: 0.0,
                                duration: Airships::AIRSHIP_DOCKING_DURATION as f32,
                                route_time: p_completion_time,
                            },
                            RouteLegSegment {
                                flight_phase: AirshipFlightPhase::Ascent,
                                from_world_pos: dest_dock_pos,
                                to_world_pos: dest_dock_pos,
                                distance: 0.0,
                                duration: Airships::AIRSHIP_ASCENT_DURATION as f32,
                                route_time: a_completion_time,
                            },
                        ],
                    });
                    route_time = a_completion_time;
                }
                let spawning_location_count =
                    (route_time / Airships::AIRSHIP_TIME_SPACING).floor() as usize;
                let route_sep = (route_time / spawning_location_count as f64).floor();
                debug_airships!(
                    4,
                    "Route {} total_time: {}, spawning_location_count: {}, route_sep: {}",
                    route_index,
                    route_time,
                    spawning_location_count,
                    route_sep
                );

                routes.push(AirshipRoute {
                    legs: route_legs,
                    total_time: route_time,
                    airship_time_spacing: route_sep,
                    cruising_height: Airships::CRUISE_HEIGHTS
                        [route_index % Airships::CRUISE_HEIGHTS.len()],
                    spawning_locations: Vec::new(),
                });
            });
        #[cfg(debug_assertions)]
        {
            routes.iter().enumerate().for_each(|(i, route)| {
                debug_airships!(4, "Route {} legs: {}", i, route.legs.len());
                route.legs.iter().enumerate().for_each(|(j, leg)| {
                    debug_airships!(4, "  Leg {}: dest_index: {}", j, leg.dest_index);
                    leg.segments.iter().enumerate().for_each(|(k, seg)| {
                        debug_airships!(
                            4,
                            "route {} leg {} segment {}: phase: {:?}, from: {}, to: {}, dist: {}, \
                             route_time: {}",
                            i,
                            j,
                            k,
                            seg.flight_phase,
                            seg.from_world_pos,
                            seg.to_world_pos,
                            seg.distance,
                            seg.route_time
                        );
                    });
                });
            });
        }
        routes
    }

    pub fn calculate_spawning_locations(&mut self) {
        // Spawn airships according to route time so that they are spaced out evenly in
        // time.
        for (route_index, route) in self.routes.iter_mut().enumerate() {
            let spawn_time_limit = route.total_time - route.airship_time_spacing;
            let mut next_spawn_time = 0.0;
            let mut prev_seg_route_time = 0.0;
            let nominal_speed = common::comp::ship::Body::DefaultAirship.get_speed();
            let mut spawning_locations = Vec::new();
            // if route.legs.is_empty() || route.legs[0].segments.is_empty() {
            //     continue;
            // }
            //let mut prev_leg_segment = &route.legs[route.legs.len() -
            // 1].segments[route.legs[route.legs.len() - 1].segments.len() - 1];
            for (leg_index, leg) in route.legs.iter().enumerate() {
                let leg_count = route.legs.len();
                let from_route_leg = &route.legs[(leg_index + leg_count - 1) % leg_count];
                let dest_dock_positions = &self.airship_docks[leg.dest_index];
                let from_dock_positions = &self.airship_docks[from_route_leg.dest_index];

                for seg in leg.segments.iter() {
                    while next_spawn_time <= seg.route_time && next_spawn_time <= spawn_time_limit {
                        // spawn an airship on this leg segment at time next_spawn_time
                        // The spawning location depends on the flight phase.
                        // DepartureCruise:
                        //     dist = (next_spawn_time - prev_leg.segments[5].route_time) *
                        // nominal_speed     pos = (midpoint - previous leg
                        // docking position).normalized() * dist + previous leg docking position
                        // ApproachCruise:
                        //     dist = (next_spawn_time - leg.segments[0].route_time) * nominal_speed
                        //     pos = (transition_pos - midpoint).normalized() * dist + midpoint
                        // Transition:
                        //     dist = (next_spawn_time - leg.segments[1].route_time)
                        //     pos = (dock_pos - transition_pos).normalized() * dist +
                        // transition_pos Descent:
                        //     pos = dock_pos
                        // Docked:
                        //     pos = dock_pos
                        // Ascent:
                        //     pos = dock_pos
                        match seg.flight_phase {
                            AirshipFlightPhase::DepartureCruise => {
                                let dist =
                                    (next_spawn_time - prev_seg_route_time) as f32 * nominal_speed;
                                let dir = seg.to_world_pos - seg.from_world_pos;
                                let pos = seg.from_world_pos
                                    + dir.try_normalized().unwrap_or(Vec2::zero()) * dist;
                                spawning_locations.push(AirshipSpawningLocation {
                                    flight_phase: seg.flight_phase,
                                    pos,
                                    dir: dir.try_normalized().unwrap_or(Vec2::unit_y()),
                                    height: route.cruising_height,
                                    route_index,
                                    leg_index,
                                    spawn_route_time: next_spawn_time,
                                });
                                debug_airships!(
                                    4,
                                    "route {} leg {} DepartureCruise prev_seg_route_time: {}, \
                                     next_spawn_time: {}, seg.route_time: {}",
                                    route_index,
                                    leg_index,
                                    prev_seg_route_time,
                                    next_spawn_time,
                                    seg.route_time
                                );
                                next_spawn_time += route.airship_time_spacing;
                            },
                            AirshipFlightPhase::ApproachCruise => {
                                let dist =
                                    (next_spawn_time - prev_seg_route_time) as f32 * nominal_speed;
                                let dir = seg.to_world_pos - seg.from_world_pos;
                                let pos = seg.from_world_pos
                                    + dir.try_normalized().unwrap_or(Vec2::zero()) * dist;
                                spawning_locations.push(AirshipSpawningLocation {
                                    flight_phase: seg.flight_phase,
                                    pos,
                                    dir: dir.try_normalized().unwrap_or(Vec2::unit_y()),
                                    height: route.cruising_height,
                                    route_index,
                                    leg_index,
                                    spawn_route_time: next_spawn_time,
                                });
                                debug_airships!(
                                    4,
                                    "route {} leg {} ApproachCruise prev_seg_route_time: {}, \
                                     next_spawn_time: {}, seg.route_time: {}",
                                    route_index,
                                    leg_index,
                                    prev_seg_route_time,
                                    next_spawn_time,
                                    seg.route_time
                                );
                                next_spawn_time += route.airship_time_spacing;
                            },
                            AirshipFlightPhase::Transition => {
                                let dist = (next_spawn_time - prev_seg_route_time) as f32
                                    * nominal_speed
                                    * Airships::AIRSHIP_TRANSITION_SPEED_RATIO;
                                let dir = seg.to_world_pos - seg.from_world_pos;
                                let pos = seg.from_world_pos
                                    + dir.try_normalized().unwrap_or(Vec2::zero()) * dist;
                                spawning_locations.push(AirshipSpawningLocation {
                                    flight_phase: seg.flight_phase,
                                    pos,
                                    dir: dir.try_normalized().unwrap_or(Vec2::unit_y()),
                                    height: route.cruising_height,
                                    route_index,
                                    leg_index,
                                    spawn_route_time: next_spawn_time,
                                });
                                debug_airships!(
                                    4,
                                    "route {} leg {} Transition prev_seg_route_time: {}, \
                                     next_spawn_time: {}, seg.route_time: {}",
                                    route_index,
                                    leg_index,
                                    prev_seg_route_time,
                                    next_spawn_time,
                                    seg.route_time
                                );
                                next_spawn_time += route.airship_time_spacing;
                            },
                            AirshipFlightPhase::Descent => {
                                let (airship_pos, airship_direction) =
                                    Airships::docking_position_and_dir_for_route_and_leg(
                                        from_dock_positions,
                                        dest_dock_positions,
                                        leg.platform,
                                    );
                                let dt = next_spawn_time - prev_seg_route_time;
                                let dd = route.cruising_height - airship_pos.z;
                                let height = airship_pos.z
                                    + dd * (dt / Airships::AIRSHIP_DESCENT_DURATION) as f32;
                                let dir = airship_direction
                                    .vec()
                                    .xy()
                                    .try_normalized()
                                    .unwrap_or(Vec2::unit_y());
                                spawning_locations.push(AirshipSpawningLocation {
                                    flight_phase: seg.flight_phase,
                                    pos: seg.from_world_pos,
                                    dir,
                                    height,
                                    route_index,
                                    leg_index,
                                    spawn_route_time: next_spawn_time,
                                });
                                debug_airships!(
                                    4,
                                    "route {} leg {} Descent prev_seg_route_time: {}, \
                                     next_spawn_time: {}, seg.route_time: {}",
                                    route_index,
                                    leg_index,
                                    prev_seg_route_time,
                                    next_spawn_time,
                                    seg.route_time
                                );
                                next_spawn_time += route.airship_time_spacing;
                            },
                            AirshipFlightPhase::Docked => {
                                let (airship_pos, airship_direction) =
                                    Airships::docking_position_and_dir_for_route_and_leg(
                                        from_dock_positions,
                                        dest_dock_positions,
                                        leg.platform,
                                    );
                                let dir = airship_direction
                                    .vec()
                                    .xy()
                                    .try_normalized()
                                    .unwrap_or(Vec2::unit_y());
                                spawning_locations.push(AirshipSpawningLocation {
                                    flight_phase: seg.flight_phase,
                                    pos: seg.from_world_pos,
                                    dir,
                                    height: airship_pos.z,
                                    route_index,
                                    leg_index,
                                    spawn_route_time: next_spawn_time,
                                });
                                debug_airships!(
                                    4,
                                    "route {} leg {} Docked prev_seg_route_time: {}, \
                                     next_spawn_time: {}, seg.route_time: {}",
                                    route_index,
                                    leg_index,
                                    prev_seg_route_time,
                                    next_spawn_time,
                                    seg.route_time
                                );
                                next_spawn_time += route.airship_time_spacing;
                            },
                            AirshipFlightPhase::Ascent => {
                                let (airship_pos, airship_direction) =
                                    Airships::docking_position_and_dir_for_route_and_leg(
                                        from_dock_positions,
                                        dest_dock_positions,
                                        leg.platform,
                                    );
                                let dt = next_spawn_time - prev_seg_route_time;
                                let dd = route.cruising_height - airship_pos.z;
                                let height = airship_pos.z
                                    + dd * (dt / Airships::AIRSHIP_ASCENT_DURATION) as f32;
                                let dir = airship_direction
                                    .vec()
                                    .xy()
                                    .try_normalized()
                                    .unwrap_or(Vec2::unit_y());
                                spawning_locations.push(AirshipSpawningLocation {
                                    flight_phase: seg.flight_phase,
                                    pos: seg.from_world_pos,
                                    dir,
                                    height,
                                    route_index,
                                    leg_index,
                                    spawn_route_time: next_spawn_time,
                                });
                                debug_airships!(
                                    4,
                                    "route {} leg {} Ascent prev_seg_route_time: {}, \
                                     next_spawn_time: {}, seg.route_time: {}",
                                    route_index,
                                    leg_index,
                                    prev_seg_route_time,
                                    next_spawn_time,
                                    seg.route_time
                                );
                                next_spawn_time += route.airship_time_spacing;
                            },
                        }
                    }
                    prev_seg_route_time = seg.route_time;
                }
            }
            route.spawning_locations = spawning_locations;
        }
    }

    /// Generates the airship routes.
    pub fn generate_airship_routes_inner(
        &mut self,
        map_size_lg: &MapSizeLg,
        seed: u32,
        _index: Option<&Index>,
        _sampler: Option<&WorldSim>,
        _map_image_path: Option<&str>,
    ) {
        let all_dock_points = self
            .airship_docks
            .iter()
            .map(|dock| Point {
                x: dock.center.x as f64,
                y: dock.center.y as f64,
            })
            .collect::<Vec<_>>();
        debug_airships!(4, "all_dock_points: {:?}", all_dock_points);

        // Run the delaunay triangulation on the docking points.
        let triangulation = triangulate(&all_dock_points);

        #[cfg(feature = "airship_maps")]
        save_airship_routes_triangulation(
            &triangulation,
            &all_dock_points,
            map_size_lg,
            seed,
            _index,
            _sampler,
            _map_image_path,
        );

        // Docking positions are specified in world coordinates, not chunks.
        // Limit the max route leg length to 1000 chunks no matter the world size.
        let blocks_per_chunk = 1 << TERRAIN_CHUNK_BLOCKS_LG;
        let max_route_leg_length = 1000.0 * blocks_per_chunk as f32;

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

        let pow2 = map_size_lg.vec().x;
        let max_iterations = (3742931.0 * std::f32::consts::E.powf(-1.113823 * pow2 as f32))
            .clamp(1.0, 100.0)
            .round() as usize;

        if let Some((best_segments, _, _max_seg_len, _min_spread, _iteration)) = triangulation
            .eulerized_route_segments(
                &all_dock_points,
                max_iterations,
                max_route_leg_length as f64,
                seed,
            )
        {
            #[cfg(debug_assertions)]
            {
                debug_airships!(4, "Max segment length: {}", _max_seg_len);
                debug_airships!(4, "Min spread: {}", _min_spread);
                debug_airships!(4, "Iteration: {}", _iteration);
                debug_airships!(4, "Segments count:");
                let mut bidirectional_segments = Vec::new();
                best_segments.iter().enumerate().for_each(|segment| {
                    debug_airships!(4, "  {} : {}", segment.0, segment.1.len());
                    let seg_bidir = {
                        if segment.1.len() > 2 {
                            let slen = segment.1.len();
                            let mut bidir_found = false;
                            for index in 0..slen {
                                let back2 = segment.1[(index + slen - 2) % slen];
                                let curr = segment.1[index];
                                if curr == back2 {
                                    debug_airships!(
                                        4,
                                        "Segment {} bidir at index {}",
                                        segment.0,
                                        index
                                    );
                                    bidir_found = true;
                                }
                            }
                            bidir_found
                        } else {
                            false
                        }
                    };
                    bidirectional_segments.push(seg_bidir);
                });
                debug_airships!(4, "Best segments: {:?}", best_segments);
                debug_airships!(4, "Bi-dir: {:?}", bidirectional_segments);
                #[cfg(feature = "airship_maps")]
                if let Some(index) = _index
                    && let Some(world_sim) = _sampler
                    && let Err(e) = export_world_map(index, world_sim)
                {
                    eprintln!("Failed to export world map: {:?}", e);
                }
            }

            self.routes = self.create_route_legs(
                &best_segments,
                all_dock_points
                    .iter()
                    .map(|p| Vec2::new(p.x as f32, p.y as f32))
                    .collect::<Vec<_>>()
                    .as_slice(),
                map_size_lg,
            );

            // Calculate the spawning locations for airships on the routes.
            self.calculate_spawning_locations();

            #[cfg(debug_assertions)]
            {
                self.routes.iter().enumerate().for_each(|(i, route)| {
                    debug_airships!(4, "Route {} spawning locations", i);
                    route.spawning_locations.iter().for_each(|loc| {
                        debug_airships!(
                            4,
                            "{} {:02} {:7.1}, {:7.1}, {} {}",
                            loc.route_index,
                            loc.leg_index,
                            loc.pos.x,
                            loc.pos.y,
                            loc.flight_phase,
                            loc.height,
                        );
                    });
                });
            }

            #[cfg(feature = "airship_maps")]
            save_airship_route_segments(
                &self.routes,
                &all_dock_points,
                &self.airship_spawning_locations(),
                map_size_lg,
                seed,
                _index,
                _sampler,
                _map_image_path,
            );
        } else {
            eprintln!("Error - cannot eulerize the dock points.");
        }
    }

    pub fn generate_airship_routes(&mut self, world_sim: &mut WorldSim, index: &Index) {
        self.airship_docks = Airships::all_airshipdock_positions(&index.sites);

        self.generate_airship_routes_inner(
            &world_sim.map_size_lg(),
            index.seed,
            Some(index),
            Some(world_sim),
            None,
        );
    }

    /// Compute the route midpoint and the transition point where the airship
    /// should stop the cruise flight phase and start the docking phase.
    /// ```text
    ///  F : From position
    ///  M : Midpoint
    ///  T : Transition point
    ///  D : Docking position
    ///  C : Center of the airship dock
    ///  X : Airship dock
    ///
    ///                            F
    ///                         •
    ///                      •
    ///                   M
    ///                  ∙
    ///                 ∙
    ///                T
    ///               ∙
    ///              ∙
    ///             D
    ///
    ///           XXXXX
    ///         XX     XX
    ///        X         X
    ///        X    C    X
    ///        X         X
    ///         XX     XX
    ///           XXXXX
    /// ```
    /// The midpoint is for route leg deconfliction and is where the airship
    /// makes a coarse correction to point at the destination. The
    /// transition point (T) between cruise flight and docking approach is
    /// on a line between the route leg midpoint (M) and the docking
    /// position (D), short of the docking position by
    /// Airships::DOCKING_TRANSITION_OFFSET blocks.
    ///
    /// # Arguments
    ///
    /// * `dock_index` - The airship dock index in airship_docks.
    /// * `route_index` - The index of the route (outer vector of
    ///   airships.routes). This is used to determine the cruise height.
    /// * `platform` - The platform on the airship dock where the airship is to
    ///   dock.
    /// * `from` - The position from which the airship is approaching the dock.
    /// # Returns
    /// The 2D position calculated with the Z coordinate set to the
    /// docking_position.z + cruise height.
    pub fn approach_waypoints(
        &self,
        from_dock_center: &Vec2<f32>,
        to_dock_positions: &AirshipDockPositions,
        platform: AirshipDockPlatform,
        map_size_lg: &MapSizeLg,
    ) -> (Vec2<f32>, Vec2<f32>) {
        // Get the route leg midpoint. This is the vector from the from_dock_position
        // to the to_dock_position rotated ROUTE_LEG_MID_POINT_OFFSET_RADIANS
        // at 1/2 the distance from the from_dock_position to the to_dock_position (so
        // not quite the exact midpoint but close enough).
        // Clamp midpoint so that it stays within the world bounds (with some margin).
        let blocks_per_chunk = 1 << TERRAIN_CHUNK_BLOCKS_LG;
        let world_blocks = map_size_lg.chunks().map(|u| u as f32) * blocks_per_chunk as f32;
        let midpoint = {
            // let from_pos = from_dock_positions.center;
            // let to_pos = dest_dock_positions.center;
            let dir = (to_dock_positions.center - from_dock_center).normalized();
            let mid_len = from_dock_center.distance(to_dock_positions.center) * 0.5;
            let mid_dir = dir.rotated_z(Airships::ROUTE_LEG_MIDPOINT_OFFSET_RADIANS);
            from_dock_center + mid_dir * mid_len
        }
        .clamped(
            Vec2::new(
                Airships::ROUTE_LEG_MIDPOINT_MARGIN,
                Airships::ROUTE_LEG_MIDPOINT_MARGIN,
            ),
            Vec2::new(
                world_blocks.x - Airships::ROUTE_LEG_MIDPOINT_MARGIN,
                world_blocks.y - Airships::ROUTE_LEG_MIDPOINT_MARGIN,
            ),
        );

        let transition_point = {
            // calculate the transition point looking from the destination position back to
            // the midpoint.
            let to_dir_rev = (midpoint - to_dock_positions.center).normalized();
            let docking_position = to_dock_positions.docking_position(platform);
            docking_position.xy() + to_dir_rev * Airships::DOCKING_TRANSITION_OFFSET
        };

        (midpoint, transition_point)
    }

    fn vec3_relative_eq(a: &vek::Vec3<f32>, b: &vek::Vec3<f32>, epsilon: f32) -> bool {
        (a.x - b.x).abs() < epsilon && (a.y - b.y).abs() < epsilon && (a.z - b.z).abs() < epsilon
    }

    pub fn docking_position_and_dir_for_route_and_leg(
        from_dock_positions: &AirshipDockPositions,
        to_dock_positions: &AirshipDockPositions,
        platform: AirshipDockPlatform,
    ) -> (Vec3<f32>, Dir) {
        let docking_side = AirshipDockingSide::from_dir_to_platform(
            &(to_dock_positions.center - from_dock_positions.center),
            &platform,
        );

        // get the airship position and direction when docked
        let (airship_pos, airship_direction) = Airships::airship_vec_for_docking_pos(
            to_dock_positions.docking_position(platform),
            to_dock_positions.center,
            Some(docking_side),
        );
        (airship_pos, airship_direction)
    }

    pub fn approach_for_route_and_leg(
        &self,
        route_index: usize,
        leg_index: usize,
        map_size_lg: &MapSizeLg,
    ) -> AirshipDockingApproach {
        // Get the docking positions for the route and leg.
        let to_route_leg = &self.routes[route_index].legs[leg_index];
        let leg_count = self.routes[route_index].legs.len();
        let from_route_leg =
            &self.routes[route_index].legs[(leg_index + leg_count - 1) % leg_count];
        let dest_dock_positions = &self.airship_docks[to_route_leg.dest_index];
        let from_dock_positions = &self.airship_docks[from_route_leg.dest_index];

        let docking_side = AirshipDockingSide::from_dir_to_platform(
            &(dest_dock_positions.center - from_dock_positions.center),
            &to_route_leg.platform,
        );

        // get the airship position and direction when docked
        let (airship_pos, airship_direction) = Airships::airship_vec_for_docking_pos(
            dest_dock_positions.docking_position(to_route_leg.platform),
            dest_dock_positions.center,
            Some(docking_side),
        );

        // get the leg midpoint and transition point
        let (midpoint, approach_transition_pos) = self.approach_waypoints(
            &from_dock_positions.center,
            dest_dock_positions,
            to_route_leg.platform,
            map_size_lg,
        );

        AirshipDockingApproach {
            airship_pos,
            airship_direction,
            dock_center: dest_dock_positions.center,
            height: Airships::CRUISE_HEIGHTS[route_index],
            midpoint,
            approach_transition_pos,
            side: docking_side,
            site_id: dest_dock_positions.site_id,
        }
    }

    pub fn airship_spawning_locations(&self) -> Vec<AirshipSpawningLocation> {
        // collect all spawning locations from all routes
        self.routes
            .iter()
            .flat_map(|route| route.spawning_locations.iter())
            .cloned()
            .collect()
    }

    /// Get the position a route leg originates from.
    pub fn route_leg_departure_location(&self, route_index: usize, leg_index: usize) -> Vec2<f32> {
        if route_index >= self.routes.len() || leg_index >= self.routes[route_index].legs.len() {
            error!("Invalid index: rt {}, leg {}", route_index, leg_index);
            return Vec2::zero();
        }

        let prev_leg = if leg_index == 0 {
            &self.routes[route_index].legs[self.routes[route_index].legs.len() - 1]
        } else {
            &self.routes[route_index].legs[leg_index - 1]
        };

        self.airship_docks[prev_leg.dest_index]
            .docking_position(prev_leg.platform)
            .xy()
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
            if rand::rng().random::<bool>() {
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

fn time_is_in_cruise_phase(time: f32, cruise_segments: &[(f32, f32)]) -> bool {
    for seg in cruise_segments {
        if time >= seg.0 && time < seg.1 {
            return true;
        }
        if seg.1 > time {
            // segments are in order, so if this segment ends after the time,
            // no need to check further segments
            break;
        }
    }
    false
}

#[cfg(debug_assertions)]
macro_rules! debug_airship_eulerization {
    ($($arg:tt)*) => {
        tracing::trace!($($arg)*);
    };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_airship_eulerization {
    ($($arg:tt)*) => {};
}

/// A map of node index to DockNode, where DockNode contains a list of
/// nodes that the node is connected to.
type DockNodeGraph = DHashMap<usize, DockNode>;

/// Extension functions for Triangulation (from the triangulate crate).
trait TriangulationExt {
    fn all_edges(&self) -> DHashSet<(usize, usize)>;
    fn is_hull_node(&self, index: usize) -> bool;
    fn node_connections(&self) -> DockNodeGraph;
    fn eulerized_route_segments(
        &self,
        all_dock_points: &[Point],
        iterations: usize,
        max_route_leg_length: f64,
        seed: u32,
    ) -> Option<(Vec<Vec<usize>>, Vec<usize>, usize, f32, usize)>;
}

/// Find the first node in the graph where the DockNode has an odd number of
/// connections to other nodes.
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

/// Implementation of extension functions for the Triangulation struct.
impl TriangulationExt for Triangulation {
    /// Get the set of all edges in the triangulation.
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

    /// For all triangles in the tessellation, create a map of nodes to their
    /// connected nodes.
    fn node_connections(&self) -> DockNodeGraph {
        let mut connections = DHashMap::default();

        self.triangles.chunks(3).for_each(|t| {
            for &node in t {
                let dock_node = connections.entry(node).or_insert_with(|| DockNode {
                    node_id: node,
                    on_hull: self.is_hull_node(node),
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

    /// True if the node is on the outer hull of the triangulation.
    fn is_hull_node(&self, index: usize) -> bool { self.hull.contains(&index) }

    /// Calculates the best way to modify the triangulation so that
    /// all nodes have an even number of connections (all nodes have
    /// an even 'degree'). The steps are:
    ///
    /// 1. Remove very long edges (not important for eurelization, but this is a
    ///    goal of the airship routes design.
    /// 2. Remove the shortest edges from all nodes that have more than 8
    ///    connections to other nodes. This is because the airship docking sites
    ///    have at most 4 docking positions, and for deconfliction purposes, no
    ///    two "routes" can use the same docking position.
    /// 3. Add edges to the triangulation so that all nodes have an even number
    ///    of connections to other nodes. There are many combinations of added
    ///    edges that can make all nodes have an even number of connections. The
    ///    goal is to find a graph with the maximum number of 'routes'
    ///    (sub-graphs of connected nodes that form a closed loop), where the
    ///    routes are all the same length. Since this is a graph, the algorithm
    ///    is sensitive to the starting point. Several iterations are tried with
    ///    different starting points (node indices), and the best result is
    ///    returned.
    ///
    /// Returns a tuple with the following elements:
    ///  - best_route_segments (up to 4 routes, each route is a vector of node
    ///    indices)
    ///  - best_circuit (the full eulerian circuit)
    ///  - max_seg_len (the length of the longest route segment)
    ///  - min_spread (the standard deviation of the route segment lengths)
    ///  - best_iteration (for debugging, the iteration that produced the best
    ///    result)
    fn eulerized_route_segments(
        &self,
        all_dock_points: &[Point],
        iterations: usize,
        max_route_leg_length: f64,
        seed: u32,
    ) -> Option<(Vec<Vec<usize>>, Vec<usize>, usize, f32, usize)> {
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

        debug_airship_eulerization!(
            "Removing {} long edges and {} excess edges for a total of {} removed edges out of a \
             total of {} edges",
            long_edges,
            edges_to_remove.len() - long_edges,
            edges_to_remove.len(),
            all_edges.len(),
        );

        for edge in edges_to_remove {
            remove_edge(edge, &mut mutable_node_connections);
        }

        #[cfg(debug_assertions)]
        {
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
                odd_connected_count.is_multiple_of(2),
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

                #[cfg(debug_assertions)]
                {
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
                    #[cfg(debug_assertions)]
                    {
                        debug_airship_eulerization!("Outer Iteration: {}", i);
                        debug_airship_eulerization!("Max segment length: {}", max_seg_len);
                        debug_airship_eulerization!("Min spread: {}", min_spread);
                        debug_airship_eulerization!("Segments count:");
                        route_segments.iter().enumerate().for_each(|segment| {
                            debug_airship_eulerization!("  {} : {}", segment.0, segment.1.len());
                        });
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
        #[cfg(debug_assertions)]
        {
            debug_airship_eulerization!("Max segment length: {}", best_max_seg_len);
            debug_airship_eulerization!("Min spread: {}", best_min_spread);
            debug_airship_eulerization!("Iteration: {}", best_iteration);
            debug_airship_eulerization!("Segments count:");
            best_route_segments.iter().enumerate().for_each(|segment| {
                debug_airship_eulerization!("  {} : {}", segment.0, segment.1.len());
            });
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

/// Find the best Eulerian circuit for the given graph of dock nodes.
/// Try each node as the starting point for a circuit.
/// The best circuit is the one with the longest routes (sub-segments
/// of the circuit), and where the route lengths are equal as possible.
fn find_best_eulerian_circuit(
    graph: &DockNodeGraph,
) -> Option<(Vec<Vec<usize>>, Vec<usize>, usize, f32, usize)> {
    let mut best_circuit = Vec::new();
    let mut best_route_segments = Vec::new();
    let mut best_max_seg_len = 0;
    let mut best_min_spread = f32::MAX;
    let mut best_iteration = 0;

    let graph_keys = graph.keys().copied().collect::<Vec<_>>();

    // Repeat for each node as the starting point.
    for (i, &start_vertex) in graph_keys.iter().enumerate() {
        let mut graph = graph.clone();
        let mut circuit = Vec::new();
        let mut stack = Vec::new();
        let mut circuit_nodes = DHashSet::default();

        let mut current_vertex = start_vertex;

        // The algorithm for finding a Eulerian Circuit (Hierholzer's algorithm).
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
            && (max_seg_len > best_max_seg_len
                || (max_seg_len == best_max_seg_len && min_spread < best_min_spread))
        {
            best_circuit = circuit.clone();
            best_route_segments = route_segments;
            best_max_seg_len = max_seg_len;
            best_min_spread = min_spread;
            best_iteration = i;
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
    use super::{AirshipDockPlatform, AirshipDockingSide, Airships, approx::assert_relative_eq};
    use vek::{Vec2, Vec3};

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
    fn docking_side_for_platform_test() {
        // Approximately: 0, 22, 45, 67, 90, 112, 135, 157, 180, 202, 225, 247, 270,
        // 292, 315, 337 degrees
        let dirs = [
            Vec2::new(0.0, 100.0) - Vec2::zero(),
            Vec2::new(100.0, 100.0) - Vec2::zero(),
            Vec2::new(100.0, 0.0) - Vec2::zero(),
            Vec2::new(100.0, -100.0) - Vec2::zero(),
            Vec2::new(0.0, -100.0) - Vec2::zero(),
            Vec2::new(-100.0, -100.0) - Vec2::zero(),
            Vec2::new(-100.0, 0.0) - Vec2::zero(),
            Vec2::new(-100.0, 100.0) - Vec2::zero(),
        ];
        let expected = [
            AirshipDockingSide::Port,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Port,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Starboard,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Port,
            AirshipDockingSide::Starboard,
        ];
        for platform in [
            AirshipDockPlatform::NorthPlatform,
            AirshipDockPlatform::EastPlatform,
            AirshipDockPlatform::SouthPlatform,
            AirshipDockPlatform::WestPlatform,
        ]
        .iter()
        {
            for (i, dir) in dirs.iter().enumerate() {
                let side = AirshipDockingSide::from_dir_to_platform(dir, platform);
                assert_eq!(side, expected[*platform as usize * 8 + i]);
            }
        }
    }
}
