use crate::{
    ai::{Action, NpcCtx, State, finish, just, now, predicate::timeout},
    data::npc::SimulationMode,
};
use common::{
    comp::{
        self, Content,
        agent::{BrakingMode, FlightMode},
        compass::Direction,
    },
    rtsim::NpcId,
    store::Id,
    util::Dir,
};
use rand::prelude::*;
use std::{cmp::Ordering, time::Duration};
use tracing::debug;
use vek::*;
use world::{
    civ::airship_travel::Airships,
    site::Site,
    util::{CARDINALS, DHashMap},
};

/// Airships can slow down or hold position to avoid collisions with other
/// airships.
#[derive(Debug, Copy, Clone, Default)]
pub enum AirshipAvoidanceMode {
    #[default]
    None,
    Hold(Vec3<f32>, Dir),
    SlowDown,
}

// Context data for the airship route.
// This is the context data for the pilot_airship action.
#[derive(Debug, Default, Clone)]
pub struct AirshipRouteContext {
    // The current approach index, 0 1 or none
    pub current_approach_index: Option<usize>,
    // The names of the route's sites
    // pub site_names: [String; 2],
    pub site_ids: Option<[Id<Site>; 2]>,
}

#[derive(Debug, PartialEq)]
enum DistanceTrend {
    ApproachingDock,
    DepartingDock,
    Docked,
}

#[derive(Debug, PartialEq)]
enum DistanceZone {
    InsideReference,
    InsideMyDistance,
    OutsideMyDistance,
}

/// Tracks airship distance trend from a fixed position. Used for airship
/// traffic control.
#[derive(Debug, Default, Clone)]
struct ZoneDistanceTracker {
    fixed_pos: Vec2<f32>,
    stable_tolerance: f32,
    ref_dist: Option<f32>,
    prev_dist: Option<f32>,
    avg_dist: Option<f32>,
}

impl ZoneDistanceTracker {
    fn update(
        &mut self,
        mypos: Vec2<f32>,
        otherpos: Vec2<f32>,
    ) -> (Option<DistanceTrend>, Option<DistanceZone>) {
        const SMOOTHING_FACTOR: f32 = 0.3; // Damp out some fluctuations so that we know if the trend is stable.
        // There is no delta time here because the measurements are taken at larger time
        // intervals, on the order of seconds.
        let current_dist = otherpos.distance_squared(self.fixed_pos);
        let trend = if let Some(prev_dist) = self.prev_dist {
            let my_dist = mypos.distance_squared(self.fixed_pos);
            let avg_dist = self
                .avg_dist
                .map(|prev_avg_dist| Lerp::lerp(prev_avg_dist, current_dist, SMOOTHING_FACTOR))
                .unwrap_or(current_dist);
            self.avg_dist = Some(avg_dist);
            let zone = if current_dist < my_dist {
                if let Some(ref_dist) = self.ref_dist {
                    if current_dist < ref_dist {
                        DistanceZone::InsideReference
                    } else {
                        DistanceZone::InsideMyDistance
                    }
                } else {
                    DistanceZone::InsideMyDistance
                }
            } else {
                DistanceZone::OutsideMyDistance
            };
            if avg_dist.abs() < self.stable_tolerance {
                (Some(DistanceTrend::Docked), Some(zone))
            } else if current_dist < prev_dist {
                (Some(DistanceTrend::ApproachingDock), Some(zone))
            } else {
                (Some(DistanceTrend::DepartingDock), Some(zone))
            }
        } else {
            self.avg_dist = Some(current_dist);
            (None, None)
        };
        self.prev_dist = Some(current_dist);
        trend
    }
}

// This is the context data for the fly_airship action.
#[derive(Debug, Clone)]
struct FlyAirshipContext {
    // For determining the velocity and direction of this and other airships on the route.
    trackers: DHashMap<NpcId, ZoneDistanceTracker>,
    // The interval for updating the airship tracker.
    timer: Duration,
    // A timer for emitting pilot messages while holding position.
    hold_timer: Duration,
    // Whether the initial hold message has been sent to the client.
    hold_announced: bool,
    // The original speed factor passed to the fly_airship action.
    speed_factor: f32,
    // The current avoidance mode for the airship.
    avoid_mode: AirshipAvoidanceMode,
}

impl FlyAirshipContext {
    fn new(timer: Duration, speed_factor: f32) -> Self {
        Self {
            trackers: DHashMap::default(),
            timer,
            hold_timer: Duration::from_secs(0),
            hold_announced: false,
            speed_factor,
            avoid_mode: AirshipAvoidanceMode::None,
        }
    }
}

/// The flight phases of an airship.
/// When the airship is loaded from RTSim data there is no "current approach
/// index", so the 'Reset' phases are used to get the airship back on the route.
/// The non-'Reset' phases are used for the normal flight loop.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
enum AirshipFlightPhase {
    Ascent,
    Cruise,
    ApproachFinal,
    Transition,
    // Docking,
    #[default]
    Docked,
    ResetAscend,
    ResetResume,
    ResetInitial,
    ResetFinal,
}

/// Called from pilot_airship to move the airship along phases of the route
/// and for initial routing after server startup. The bulk of this action
/// is collision-avoidance monitoring. The frequency of the collision-avoidance
/// tests is controlled by the radar_interval parameter.
/// The collision-avoidance logic has the ability to change the airship's speed
/// and to hold position.
///
/// # Avoidance Logic
/// All airships on the same route follow what is essentially a race track.
/// All collision issues are caused by airships catching up to each other on the
/// route. To avoid collisions, the postion and movement of other airships on
/// the route is monitored at a interval of between 1 and 5 seconds. If another
/// airship is moving towards the same docking position, it may be ahead or
/// behind the monitoring airship. If the other airship is ahead, and the
/// monitoring airship is approaching the docking position, the monitoring
/// airship will slow down or stop to avoid a potential conflict.
///
/// # Parameters
///
/// - `route_context`: The AirshipRouteContext owned by the pilot_airship
///   action.
/// - `phase`: One of the phases of the airship flight loop or reset stages.
/// - `wpos`: The fly-to target position for the airship.
/// - `goal_dist`: The distance to the target position at which this action
///   (this phase) will stop.
/// - `initial_speed_factor`: The initial speed factor for the airship. Can be
///   modified by collision-avoidance.
/// - `height_offset`: The height offset for the airship (height above terrain).
///   This is only used if with_terrain_following is true.
/// - `with_terrain_following`: Whether to follow the terrain. If true, the
///   airship will fly at a height above the terrain. If false, the airship
///   flies directly to the target position.
/// - `direction_override`: An optional direction override for the airship. If
///   Some, the airship will be oriented (pointed) in this direction.
/// - `flight_mode`: Influences the positioning of the airship. When approaching
///   or at the target position, the airship either slows (brakes) and holds or
///   flys through, expecting to continue on the next flight phase.
/// - `with_collision_avoidance`: Whether to perform collision avoidance. It's
///   not needed for docking or initial ascent because other airships must give
///   way to this airship.
/// - `radar_interval`: The interval at which to check on the positions and
///   movements of other airships on the same route.
///
/// # Returns
///
/// An Action
fn fly_airship<S: State>(
    route_context: &AirshipRouteContext,
    phase: AirshipFlightPhase,
    wpos: Vec3<f32>,
    goal_dist: f32,
    initial_speed_factor: f32,
    height_offset: f32,
    with_terrain_following: bool,
    direction_override: Option<Dir>,
    flight_mode: FlightMode,
    with_collision_avoidance: bool,
    radar_interval: Duration,
) -> impl Action<S> {
    let current_approach_index = route_context.current_approach_index.unwrap_or(0);
    just(
        move |ctx, airship_context: &mut Option<FlyAirshipContext>| {
            // init context
            let airship_context = airship_context.get_or_insert_with(|| {
                FlyAirshipContext::new(radar_interval, initial_speed_factor)
            });

            let remaining = airship_context
                .timer
                .checked_sub(Duration::from_secs_f32(ctx.dt));
            if remaining.is_none() {
                airship_context.timer = radar_interval;
                // The collision avoidance checks are not done every tick (no dt required).
                if with_collision_avoidance {
                    if let Some(route_id) = ctx
                        .state
                        .data()
                        .airship_sim
                        .assigned_routes
                        .get(&ctx.npc_id)
                        && let Some(route) = ctx.world.civs().airships.routes.get(route_id)
                        && let Some(approach) = route.approaches.get(current_approach_index)
                        && let Some(pilots) =
                            ctx.state.data().airship_sim.route_pilots.get(route_id)
                    {
                        let mypos = ctx.npc.wpos;
                        // The intermediate reference distance is either the approach initial point
                        // (when cruising) or final point (when on
                        // approach).
                        let tracker_ref_dist = match phase {
                            AirshipFlightPhase::Cruise => Some(
                                approach
                                    .approach_initial_pos
                                    .distance_squared(approach.airship_pos.xy()),
                            ),
                            AirshipFlightPhase::ApproachFinal => Some(
                                approach
                                    .approach_final_pos
                                    .distance_squared(approach.airship_pos.xy()),
                            ),
                            _ => None,
                        };
                        // Collection the avoidance modes for other airships on the route.
                        let avoidance: Vec<AirshipAvoidanceMode> = pilots
                            .iter()
                            .filter(|pilot_id| **pilot_id != ctx.npc_id)
                            .filter_map(|pilot_id| {
                                ctx.state.data().npcs.get(*pilot_id).and_then(|pilot| {
                                    let pilot_wpos = pilot.wpos;
                                    let other_tracker = airship_context
                                        .trackers
                                        .entry(*pilot_id)
                                        .or_insert_with(|| ZoneDistanceTracker {
                                            fixed_pos: approach.airship_pos.xy(),
                                            stable_tolerance: 20.0,
                                            ref_dist: tracker_ref_dist,
                                            ..Default::default()
                                        });
                                    // the tracker determines if the other airship is moving towards
                                    // or away from my docking position
                                    // and if towards, whether it is inside of my position (ahead of
                                    // me) and if ahead, whether it is
                                    // inside the reference distance. If ahead of me but no inside
                                    // the reference distance, I should slow down.
                                    // If ahead of me and inside the reference distance, I should
                                    // slow down or hold depending on which
                                    // phase I'm in.
                                    let (trend, zone) =
                                        other_tracker.update(mypos.xy(), pilot_wpos.xy());
                                    match trend {
                                        Some(DistanceTrend::ApproachingDock) => {
                                            // other airship is moving towards the (my) docking
                                            // position
                                            match zone {
                                                Some(DistanceZone::InsideMyDistance) => {
                                                    // other airship is ahead, on the same route,
                                                    // but outside
                                                    // the reference distance (either the approach
                                                    // initial point or final point)
                                                    match phase {
                                                        // If I'm currently cruising, slow down if
                                                        // the other airship is
                                                        // within 2000 blocks
                                                        AirshipFlightPhase::Cruise => {
                                                            let dist2 = mypos
                                                                .xy()
                                                                .distance_squared(pilot_wpos.xy());
                                                            if dist2 < 2000.0f32.powi(2) {
                                                                Some(AirshipAvoidanceMode::SlowDown)
                                                            } else {
                                                                None
                                                            }
                                                        },
                                                        // If I'm currently on approach, stop and
                                                        // hold
                                                        AirshipFlightPhase::ApproachFinal => {
                                                            Some(AirshipAvoidanceMode::Hold(
                                                                mypos,
                                                                Dir::from_unnormalized(
                                                                    (approach.approach_final_pos
                                                                        - mypos.xy())
                                                                    .with_z(0.0),
                                                                )
                                                                .unwrap_or_default(),
                                                            ))
                                                        },
                                                        // If I'm currently transitioning to above
                                                        // the dock, hold position
                                                        AirshipFlightPhase::Transition => {
                                                            Some(AirshipAvoidanceMode::Hold(
                                                                mypos,
                                                                approach.airship_direction,
                                                            ))
                                                        },
                                                        _ => None,
                                                    }
                                                },
                                                Some(DistanceZone::InsideReference) => {
                                                    // other airship is ahead, on the same route,
                                                    // and inside
                                                    // the reference distance (either the approach
                                                    // initial point or final point)
                                                    match phase {
                                                        // If I'm currently on approach, slow down,
                                                        // to eventually
                                                        // hold near the dock.
                                                        AirshipFlightPhase::ApproachFinal => {
                                                            Some(AirshipAvoidanceMode::SlowDown)
                                                        },
                                                        // else I'm cruising, the other airship is
                                                        // well ahead, and
                                                        // I might eventually have to hold nearer
                                                        // the dock.
                                                        // There is no reference distance if on
                                                        // final.
                                                        _ => None,
                                                    }
                                                },
                                                // else other airship is behind me, ignore
                                                _ => None,
                                            }
                                        },
                                        // other airship is at the dock (or desending to the dock)
                                        Some(DistanceTrend::Docked) => {
                                            // other airship is ahead, on the same route, near the
                                            // dock.
                                            match phase {
                                                // If I'm currently on approach, slow down, to
                                                // eventually probably hold near the dock.
                                                AirshipFlightPhase::ApproachFinal => {
                                                    Some(AirshipAvoidanceMode::SlowDown)
                                                },
                                                // If I'm currently transitioning to above the dock,
                                                // hold position
                                                AirshipFlightPhase::Transition => {
                                                    Some(AirshipAvoidanceMode::Hold(
                                                        mypos,
                                                        approach.airship_direction,
                                                    ))
                                                },
                                                // otherwise continue until some other condition is
                                                // met
                                                _ => None,
                                            }
                                        },
                                        // else other airship is moving away from my dock or there
                                        // isn't enough data to determine the trend.
                                        // Do nothing.
                                        _ => None,
                                    }
                                })
                            })
                            .collect();

                        if let Some((hold_pos, hold_dir)) = avoidance.iter().find_map(|mode| {
                            if let AirshipAvoidanceMode::Hold(hold_pos, hold_dir) = mode {
                                Some((hold_pos, hold_dir))
                            } else {
                                None
                            }
                        }) {
                            if !matches!(airship_context.avoid_mode, AirshipAvoidanceMode::Hold(..))
                            {
                                airship_context.avoid_mode =
                                    AirshipAvoidanceMode::Hold(*hold_pos, *hold_dir);
                                // airship_context.hold_wpos = mypos + (other_pos -
                                // mypos.xy()).normalized() * 25.0;
                                airship_context.hold_timer =
                                    Duration::from_secs_f32(ctx.rng.gen_range(4.0..7.0));
                                airship_context.hold_announced = false;
                            }
                        } else if avoidance
                            .iter()
                            .any(|mode| matches!(mode, AirshipAvoidanceMode::SlowDown))
                        {
                            airship_context.avoid_mode = AirshipAvoidanceMode::SlowDown;
                            airship_context.speed_factor = initial_speed_factor * 0.5;
                        } else {
                            airship_context.avoid_mode = AirshipAvoidanceMode::None;
                            airship_context.speed_factor = initial_speed_factor;
                        }
                    }
                } else {
                    airship_context.avoid_mode = AirshipAvoidanceMode::None;
                    airship_context.speed_factor = initial_speed_factor;
                }
            } else {
                airship_context.timer = remaining.unwrap();
            }

            if let AirshipAvoidanceMode::Hold(hold_pos, hold_dir) = airship_context.avoid_mode {
                let hold_remaining = airship_context
                    .hold_timer
                    .checked_sub(Duration::from_secs_f32(ctx.dt));
                if hold_remaining.is_none() {
                    if !airship_context.hold_announced {
                        airship_context.hold_announced = true;
                        ctx.controller
                            .say(None, Content::localized("npc-speech-pilot-announce_hold"));
                    } else {
                        ctx.controller
                            .say(None, Content::localized("npc-speech-pilot-continue_hold"));
                    }
                    airship_context.hold_timer =
                        Duration::from_secs_f32(ctx.rng.gen_range(10.0..20.0));
                } else {
                    airship_context.hold_timer = hold_remaining.unwrap();
                }
                // Hold position (same idea as holding station at the dock except allow
                // oscillations)
                let hold_pos = if matches!(ctx.npc.mode, SimulationMode::Simulated) {
                    hold_pos
                } else {
                    // Airship is loaded, add some randomness to the hold position
                    // so that the airship doesn't look like it's stuck in one place.
                    // This also keeps the propellers spinning slowly and somewhat randomly.
                    hold_pos
                        + (Vec3::new(0.7, 0.8, 0.9).map(|e| (e * ctx.time.0).sin())
                            * Vec3::new(5.0, 5.0, 10.0))
                        .map(|e| e as f32)
                };

                // Holding position
                ctx.controller.do_goto_with_height_and_dir(
                    hold_pos,
                    0.15,
                    None,
                    Some(hold_dir),
                    FlightMode::Braking(BrakingMode::Normal),
                );
            } else {
                // use terrain height offset if specified
                let height_offset_opt = if with_terrain_following {
                    Some(height_offset)
                } else {
                    None
                };
                // Move the airship
                ctx.controller.do_goto_with_height_and_dir(
                    wpos,
                    airship_context.speed_factor,
                    height_offset_opt,
                    direction_override,
                    flight_mode,
                );
            }
        },
    )
    .repeat()
    .boxed()
    .with_state(None)
    .stop_if(move |ctx: &mut NpcCtx| {
        if flight_mode == FlightMode::FlyThrough {
            // we only care about the xy distance (just get close to the target position)
            ctx.npc.wpos.xy().distance_squared(wpos.xy()) < goal_dist.powi(2)
        } else {
            // Braking mode means the PID controller will be controlling all three axes
            ctx.npc.wpos.distance_squared(wpos) < goal_dist.powi(2)
        }
    })
    .debug(move || {
        format!(
            "fly airship, phase:{:?}, tgt pos:({}, {}, {}), goal dist:{}, speed:{}, height:{}, \
             terrain following:{}, FlightMode:{:?}, collision avoidance:{}, radar interval:{}",
            phase,
            wpos.x,
            wpos.y,
            wpos.z,
            goal_dist,
            initial_speed_factor,
            height_offset,
            with_terrain_following,
            flight_mode,
            with_collision_avoidance,
            radar_interval.as_secs_f32()
        )
    })
    .map(|_, _| ())
}

/// Get the target position for airship movement given the target position, the
/// default height above terrain, and the height above terrain for the airship
/// route cruise phase. This samples terrain points aound the target pos to get
/// the maximum terrain altitude in a 200 block radius of the target position
/// (only checking 4 cardinal directions). and returns the input approach_pos
/// with z equal to the maximum terrain alt + height or the default_alt
/// whichever is greater.
fn approach_target_pos(
    ctx: &mut NpcCtx,
    approach_pos: Vec2<f32>,
    default_alt: f32,
    height: f32,
) -> Vec3<f32> {
    // sample 4 terrain altitudes around the final approach point and take the max.
    let max_alt = CARDINALS
        .iter()
        .map(|rpos| {
            ctx.world
                .sim()
                .get_surface_alt_approx(approach_pos.as_() + rpos * 200)
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
        .unwrap_or(default_alt);
    approach_pos.with_z(max_alt + height)
}

/// Calculates how to resume a route. Called when loading an airship from
/// saved rtSim data.
///
/// If the airship is within 700 blocks of a docking position, go to the
/// approach initial point for that docking position. If the airship is
/// outside of 700 blocks of either docking position, go to the nearest
/// approach initial point.
///
/// ### Returns
///
/// The index of the approach to resume on.
fn resume_route(airships: &Airships, route_id: &u32, ctx: &mut NpcCtx<'_, '_>) -> usize {
    if let Some(route) = airships.routes.get(route_id) {
        // (approach index, distance to docking position, distance to approach initial
        // pos)
        let resume_data: Vec<(usize, i32, i32)> = route
            .approaches
            .iter()
            .enumerate()
            .map(|(index, approach)| {
                (
                    index,
                    approach
                        .airship_pos
                        .xy()
                        .distance_squared(ctx.npc.wpos.xy()) as i32,
                    approach
                        .approach_initial_pos
                        .distance_squared(ctx.npc.wpos.xy()) as i32,
                )
            })
            .collect::<Vec<_>>();

        // The approach initial position is between 700 to 1000 blocks from the docking
        // position. If the distance from the docking position is less than
        // 100 blocks, the airship was docked or landing to dock.
        // else if the distance from the docking position is less than 700 blocks, it
        // was in the approach phase. else it was cruising or was just past
        // the initial approach point.

        // Consider three zones:
        // 1. Near Site 1 docking position
        // 2. Near Site 2 docking position
        // 3. Cruising in between the two sites.

        // Unfortunately, we can't use the npc.dir to determine where it was pointed.
        // The npc.dir is the direction the npc is facing, and for airship pilots, it
        // does not change. Apparently, the npc.dir is relative the ship body or
        // something.

        // If the airship is within 700 blocks of the approach docking position
        //      Go to approach initial point
        // Else if within 700 blocks of the opposite approach docking position
        //      Go to the opposite approach initial point
        // Else
        //      Go to the nearest approach initial point
        // End

        if let Some((index, _, _)) = resume_data
            .iter()
            .find(|(_, dock_dist2, _)| *dock_dist2 < 700i32.pow(2))
        {
            // Go to approach initial point at normal altitude + a random offset.
            *index
        // Else if the distance to either dock is greater than 700 blocks,
        // the airship was cruising between the two sites, so go
        // to the nearest approach initial point.
        } else if let Some((index, _, _)) = resume_data
            .iter()
            .min_by_key(|(_, _, initial_dist2)| *initial_dist2)
        {
            *index
        } else {
            // there must be no approached, unexpected.
            // pick a random approach.
            ctx.rng.gen_range(0..2)
        }
    } else {
        // no route, unexpected.
        // pick a random approach.
        ctx.rng.gen_range(0..2)
    }
}

/// The NPC is the airship captain. This action defines the flight loop for the
/// airship. The captain NPC is autonomous and will fly the airship along the
/// assigned route. The routes are established and assigned to the captain NPCs
/// when the world is generated.
pub fn pilot_airship<S: State>(ship: common::comp::ship::Body) -> impl Action<S> {
    /*
        Phases of the airship flight:
        1. Docked at a docking position. Hovering in place. Mode 3d with direction vector.
        2. Ascending staight up, to flight transition point. Mode 3d with direction vector.
        3. Cruising to destination initial point. Mode 2d with variable height offset (from terrain).
        4. Flying to final point. Mode 3d with direction vector.
        5. Flying to docking transition point. Mode 3d with direction vector.
        6. Descending and docking. Mode 3d with direction vector.

                                                                                +
                                                                                +...+
                                                                                +  <------------ Docking Position
                                                                                |
                                                            Aligned with        |
                                                            Docking Direction   |
                                                                                |
                                                                                |
                                                                                |
                                                                Turn Towards    | <----------- Final Point
                                                                Docking Pos    /-
        Docked Position (start)                                              /-
            |                                              Turn Towards    /--
            |                                              Final Point  /-
            -> -------------------Cruise------------------------------  <-------------------- Initial Point
            +
            +...+
            +

        Algorithm for piloting the airship:

        Phase  State    Parameters               Completion Conditions
        1      Docked   3D Position, Dir         Docking Timeout
        2      Ascent   3D Position, Dir         Altitude reached
        3      Cruise   2D Position, Height      2D Position reached
        4      Approach 3D Position, Dir         2D Position reached
        5      Final    3D Position, Dir         3D Position reached
    */

    now(move |ctx, route_context: &mut AirshipRouteContext| {

        /*
            Get the assigned route for the airship (the captain NpcId)
            and then figure out where in the route the airship is and what it's supposed to do next.
            The airship flight action loop can be entered at any point because the airship/captain
            position is saved in RTSim saved data but not what they were doing. When the server is
            restarted, they will be in the same position but not doing anything and this code needs
            to figure out what they should be doing.

            If there is a current approach

                Regular Flight Loop
                Assumes that the airship starts at the Approach Final Point
                This is the first de-confliction point for airships on the same route.
                When the airship gets to the Approach Final Point, if there are any other
                airships (pilots assigned to the same route) within some radius of the docking
                position, then the airship should wait at the Approach Final Point until the
                other airships are no longer within that radius. This is to prevent collisions.

                Repeat until no other route pilot is within some radius of the docking position
                    Wait
                End
                Fly 3D to Docking Transition Point
                Descend and Dock
                Wait at Dock (default time + extension time)
                Ascend to Cruise Alt
                Fly 2D to Destination Initial Point
                Fly 3D to Destination Final Point

            Else (no current approach in the context data)

                No current approach happens when first starting up, both from scratch with a new
                world and when loading rtSim data from persistent storage. The airship could be anywhere
                along a route.

                If the airship was cruising, resume the cruise flight phase.
                If the airship was docked or nearly docked, send it off towards the opposite site.
                Else the airship should be somewhere near the Approach Final Point. Fly 3D to the
                Approach Final Point and start the regular flight loop.
            End
            Repeat
         */

        // get the assigned route

        if let Some(route_id) = ctx.state.data().airship_sim.assigned_routes.get(&ctx.npc_id)
            && let Some(route) = ctx.world.civs().airships.routes.get(route_id) {

            route_context.site_ids = Some([route.approaches[0].site_id, route.approaches[1].site_id]);

            if let Some(current_approach_index) = route_context.current_approach_index {
                // when current_approach_index exists, it means we're repeating the flight loop
                // if approach index is 0, then the airship is fly from site 0 to site 1, and vice versa

                let ship_body = comp::Body::from(ship);
                let approach1 = route.approaches[current_approach_index].clone();
                let approach2 = route.approaches[(current_approach_index + 1) % 2].clone();

                // Regular Flight Loop
                // Fly 3D to Docking Transition Point, full PID control
                // 

                fly_airship(
                    route_context,
                    AirshipFlightPhase::Transition,
                    approach1.airship_pos + Vec3::unit_z() * (approach1.height),
                    50.0,
                    0.2,
                    approach1.height,
                    true,
                    Some(approach1.airship_direction),
                    FlightMode::Braking(BrakingMode::Normal),
                    true,
                    Duration::from_secs_f32(1.0),
                )

                // Descend and Dock
                //    Docking
                //      Stop in increments and settle at 150 blocks and then 50 blocks from the dock.
                //      This helps to ensure that the airship docks vertically and avoids collisions
                //      with other airships and the dock. The speed_factor is high to
                //      give a strong response to the PID controller for the first
                //      three docking phases. The speed_factor is reduced for the final docking phase
                //      to give the impression that the airship propellers are not rotating.
                .then(
                    // descend to 150 blocks above the dock
                    just(move |ctx, _| {
                        ctx.controller
                            .do_goto_with_height_and_dir(
                                approach1.airship_pos + Vec3::unit_z() * 150.0,
                                0.7, None,
                                Some(approach1.airship_direction),
                                FlightMode::Braking(BrakingMode::Normal),
                            );
                    })
                    .repeat()
                    .stop_if(timeout(9.0)))
                .then(
                    // descend to 50 blocks above the dock
                    just(move |ctx, _| {
                        ctx.controller
                            .do_goto_with_height_and_dir(
                                approach1.airship_pos + Vec3::unit_z() * 50.0,
                                0.6, None,
                                Some(approach1.airship_direction),
                                FlightMode::Braking(BrakingMode::Normal),
                            );
                    })
                    .repeat()
                    .stop_if(timeout(6.0)))
                .then(
                    // descend to docking position
                    just(move |ctx, _| {
                        ctx.controller
                            .do_goto_with_height_and_dir(
                                approach1.airship_pos + ship_body.mount_offset(),
                                0.5, None,
                                Some(approach1.airship_direction),
                                FlightMode::Braking(BrakingMode::Precise),
                            );
                    })
                    .repeat()
                    .stop_if(timeout(6.0)))
                // Announce arrival
                .then(just(move |ctx, route_context:&mut AirshipRouteContext| {
                    #[cfg(debug_assertions)]
                    {
                        if let Some(site_ids) = route_context.site_ids {
                            let docked_site_id = site_ids[route_context.current_approach_index.unwrap_or(0)];
                            let docked_site_name = ctx.index.sites.get(docked_site_id).name().to_string();
                            debug!("{}, Docked at {}", format!("{:?}", ctx.npc_id), docked_site_name);
                        }
                    }
                    ctx.controller
                        .say(None, Content::localized("npc-speech-pilot-landed"));
                }))

                // Docked - Wait at Dock
                .then(
                    just(move |ctx, _| {
                        ctx.controller
                        .do_goto_with_height_and_dir(
                            approach1.airship_pos + ship_body.mount_offset(),
                            0.4, None,
                            Some(approach1.airship_direction),
                            FlightMode::Braking(BrakingMode::Precise),
                        );
                    })
                    .repeat()
                    .stop_if(timeout(ctx.rng.gen_range(10.0..20.0)))
                    // While waiting, every now and then announce where the airship is going next.
                    .then(
                        just(move |ctx, route_context:&mut AirshipRouteContext| {
                            // get the name of the site the airship is going to next.
                            // The route_context.current_approach_index has not been switched yet,
                            // so the index is the opposite of the current approach index.
                            if let Some(site_ids) = route_context.site_ids {
                                let next_site_id = site_ids[(route_context.current_approach_index.unwrap_or(0) + 1) % 2];
                                let next_site_name = ctx.index.sites.get(next_site_id).name().to_string();
                                ctx.controller.say(
                                    None,
                                    Content::localized_with_args("npc-speech-pilot-announce_next", [
                                    (
                                        "dir",
                                        Direction::from_dir(approach2.approach_initial_pos - ctx.npc.wpos.xy()).localize_npc(),
                                    ),
                                    ("dst", Content::Plain(next_site_name.to_string())),
                                    ]),
                                );
                            }
                        })
                    )
                    .repeat()
                    .stop_if(timeout(Airships::docking_duration()))
                ).then(
                    // rotate the approach to the next approach index. Note the approach2 is already known,
                    // this is just changing the approach index in the context data for the next loop.
                    just(move |ctx, route_context:&mut AirshipRouteContext| {
                        let from_index = route_context.current_approach_index.unwrap_or(0);
                        let next_approach_index = (from_index + 1) % 2;
                        route_context.current_approach_index = Some(next_approach_index);
                        if let Some(site_ids) = route_context.site_ids {
                            ctx.controller.say(
                            None,
                                Content::localized_with_args("npc-speech-pilot-takeoff", [
                                    ("src", Content::Plain(ctx.index.sites.get(site_ids[from_index]).name().to_string())),
                                    ("dst", Content::Plain(ctx.index.sites.get(site_ids[next_approach_index]).name().to_string())),
                                ]),
                            );
                        }
                    })
                ).then(
                    // Ascend to Cruise Alt, full PID control
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::Ascent,
                        approach1.airship_pos + Vec3::unit_z() * Airships::takeoff_ascent_hat(),
                        50.0,
                        0.08,
                        0.0,
                        false,
                        Some(Dir::from_unnormalized((approach2.approach_initial_pos - ctx.npc.wpos.xy()).with_z(0.0)).unwrap_or_default()),
                        FlightMode::Braking(BrakingMode::Normal),
                        false,
                        Duration::from_secs_f32(120.0),
                    )
                ).then(
                    // Fly 2D to Destination Initial Point
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::Cruise,
                        approach_target_pos(ctx, approach2.approach_initial_pos, approach2.airship_pos.z + approach2.height, approach2.height),
                        250.0,
                        1.0,
                        approach2.height,
                        true,
                        None,
                        FlightMode::FlyThrough,
                        true,
                        Duration::from_secs_f32(3.0),
                    )
                ).then(
                    // Fly 3D to Destination Final Point, z PID control
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::ApproachFinal,
                        approach_target_pos(ctx, approach2.approach_final_pos, approach2.airship_pos.z + approach2.height, approach2.height),
                        250.0,
                        0.3,
                        approach2.height,
                        true,
                        Some(approach2.airship_direction),
                        FlightMode::FlyThrough,
                        true,
                        Duration::from_secs_f32(1.0),
                    )
                ).map(|_, _| ()).boxed()
            // there is no current approach, we must be just starting up. Find the nearest approach final point.
            } else if let Some(route_id) = ctx.state.data().airship_sim.assigned_routes.get(&ctx.npc_id)
                && let Some(route) = ctx.world.civs().airships.routes.get(route_id)
            {
                let resume_approach_index = resume_route(&ctx.world.civs().airships, route_id, ctx);

                // This is a bit tricky because another airship could have been within 700
                // blocks of a docking position (one arriving and one leaving), or more than
                // one airship could be between the initial points, so don't just
                // go directly to the final point.
                // Choose a random offset from the approach initial point and go there at a random altitude.
                // The airship deconfliction code in fly_airship will quickly sort out the situation.

                route_context.current_approach_index = Some(resume_approach_index);
                let approach= &route.approaches.get(resume_approach_index).unwrap();

                // get the direction from the approach final point to the approach initial point
                let mut approach_dir = approach.approach_initial_pos - approach.approach_final_pos;

                // rotate the direction from final pos to initial pos by +- 30 or 60 degrees,
                // and then extend that by a random factor of (1.15, 1.3 or 1.45) to get the resume position.
                let rotation = ctx.rng.gen_range(-2..3) as f32 * std::f32::consts::FRAC_PI_6;
                let mult_factor = 1.0 + ctx.rng.gen_range(1..4) as f32 * 0.15;
                approach_dir.rotate_z(rotation);
                let resume_pos = approach.approach_final_pos + approach_dir * mult_factor;

                fly_airship(
                    // Make sure the airship is at altitude
                    route_context,
                    AirshipFlightPhase::ResetAscend,
                    approach_target_pos(ctx, ctx.npc.wpos.xy(), 0.0, approach.height),
                    100.0,
                    0.08,
                    0.0,
                    false,
                    Some(Dir::from_unnormalized((resume_pos - ctx.npc.wpos.xy()).with_z(0.0)).unwrap_or_default()),
                    FlightMode::Braking(BrakingMode::Normal),
                    false,
                    Duration::from_secs_f32(120.0),
                )
                .then(
                    // Fly to the resume position
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::ResetResume,
                        approach_target_pos(ctx, resume_pos, 0.0, approach.height),
                        250.0,
                        1.0,
                        approach.height,
                        true,
                        None,
                        FlightMode::FlyThrough,
                        true,
                        Duration::from_secs_f32(1.0),
                    ))
                .then(
                    // Fly to the approach initial point
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::ResetInitial,
                        approach_target_pos(ctx, approach.approach_initial_pos, approach.airship_pos.z + approach.height, approach.height),
                        250.0,
                        0.4,
                        approach.height,
                        true,
                        None,
                        FlightMode::FlyThrough,
                        true,
                        Duration::from_secs_f32(1.0),
                    ))
                .then(
                    // fly to approach final point
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::ResetFinal,
                        approach_target_pos(ctx, approach.approach_final_pos, approach.airship_pos.z + approach.height, approach.height),
                        250.0,
                        0.3,
                        approach.height,
                        true,
                        Some(approach.airship_direction),
                        FlightMode::FlyThrough,
                        true,
                        Duration::from_secs_f32(1.0),
                    )).map(|_, _| ()).boxed()
            } else {
                // no resume mode, this is unexpected and never happens in testing, just finish so the compiler doesn't complain.
                finish().map(|_, _| ()).boxed()
            }
        } else {
            //  There are no routes assigned.
            //  This is unexpected and never happens in testing, just do nothing so the compiler doesn't complain.
            finish().map(|_, _| ()).boxed()
        }
    })
    .repeat()
    .with_state(AirshipRouteContext::default())
    .map(|_, _| ())
}

#[cfg(test)]
mod tests {
    use super::{DistanceTrend, DistanceZone, ZoneDistanceTracker};
    use vek::{Vec2, Vec3};

    #[test]
    fn transition_zone_other_approaching_test() {
        let dock_pos = Vec2::new(0.0, 0.0);
        let my_pos: Vec2<f32> = Vec2::new(0.0, -100.0);
        let mut other_pos = Vec2::new(0.0, -50.0);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ..Default::default()
        };
        for _ in 0..50 {
            other_pos.y += 1.0;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(!(matches!(trend, Some(DistanceTrend::DepartingDock))));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn transition_zone_other_docked_test() {
        let dock_pos = Vec3::new(1050.0, 8654.33, 874.2);
        let my_pos: Vec3<f32> = Vec3::new(1000.0, 8454.33, 574.2);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos.xy(),
            stable_tolerance: 20.0,
            ..Default::default()
        };
        let time_0 = 27334.98f64;
        for i in 0..100 {
            let other_pos = dock_pos
                + (Vec3::new(0.7, 0.8, 0.9).map(|e| e * (time_0 + i as f64 * 1.37).sin())
                    * Vec3::new(5.0, 5.0, 10.0))
                .map(|e| e as f32)
                .xy();
            let (trend, zone) = tracker.update(my_pos.xy(), other_pos.xy());
            assert!(matches!(trend, Some(DistanceTrend::Docked) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn transition_zone_other_departing_test() {
        let dock_pos = Vec2::new(0.0, 0.0);
        let my_pos: Vec2<f32> = Vec2::new(-100.0, -100.0);
        let mut other_pos = Vec2::new(0.0, -1.0);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ..Default::default()
        };
        for _ in 0..50 {
            other_pos.y -= 1.0;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(!(matches!(trend, Some(DistanceTrend::ApproachingDock))));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn approach_other_approaching_behind_test() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(9965.0, 4501.8);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_y = (my_pos.y - other_pos.y) / 51.0;
        for _ in 0..50 {
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::ApproachingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::OutsideMyDistance) | None));
        }
    }

    #[test]
    fn approach_other_approaching_in_zone2() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(9965.0, 5407.3);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_y = (afp_pos.y - other_pos.y) / 51.0;
        for _ in 0..50 {
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::ApproachingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn approach_other_departing_in_zone2() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(9965.0, 5518.3);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_y = (my_pos.y - other_pos.y) / 51.0;
        for _ in 0..50 {
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::DepartingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn approach_other_approaching_in_zone1() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(10655.0, 5518.7);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_x = (dock_pos.x - other_pos.x) / 50.0;
        let step_y = (dock_pos.y - other_pos.y) / 50.0;
        for _ in 0..50 {
            other_pos.x += step_x;
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::ApproachingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideReference) | None));
        }
    }

    #[test]
    fn approach_other_docked() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let time_0 = 354334.98f64;
        for i in 0..50 {
            let other_pos = dock_pos
                + (Vec3::new(0.7, 0.8, 0.9).map(|e| e * (time_0 + i as f64 * 1.37).sin())
                    * Vec3::new(5.0, 5.0, 10.0))
                .map(|e| e as f32)
                .xy();
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::Docked) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideReference) | None));
        }
    }

    #[test]
    fn approach_other_departing_in_zone1() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(10987.0, 5634.0);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_x = (afp_pos.x - dock_pos.x) / 51.0;
        let step_y = (afp_pos.y - dock_pos.y) / 51.0;
        for _ in 0..50 {
            other_pos.x += step_x;
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(!(matches!(trend, Some(DistanceTrend::ApproachingDock))));
            assert!(matches!(zone, Some(DistanceZone::InsideReference) | None));
        }
    }

    #[test]
    fn approach_other_departing_behind() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(9964.8, 5406.55);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_x = -11.37;
        let step_y = -23.87;
        for _ in 0..50 {
            other_pos.x += step_x;
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::DepartingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::OutsideMyDistance) | None));
        }
    }
}
