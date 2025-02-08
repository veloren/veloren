use crate::{
    ai::{Action, NpcCtx, State, finish, just, now, predicate::timeout},
    data::npc::SimulationMode,
};
use common::{
    comp::{
        self, Content,
        agent::{PidGain, PidMode},
        compass::Direction,
    },
    resources::Time,
    rtsim::NpcId,
    util::Dir,
};
use rand::prelude::*;
use std::{cmp::Ordering, time::Duration};
use tracing::debug;
use vek::*;
use world::{
    civ::airship_travel::Airships,
    util::{CARDINALS, DHashMap},
};
const AIRSHIP_AI_DEBUG: bool = false;

macro_rules! debug_airship_ai {
    ($($arg:tt)*) => {
        if AIRSHIP_AI_DEBUG {
            debug!($($arg)*);
        }
    }
}

/// Airships can slow down or hold position to avoid collisions with other
/// airships.
#[derive(Debug, Copy, Clone, Default)]
pub enum AirshipAvoidanceMode {
    #[default]
    None,
    Hold(f32),
    SlowDown,
}

// Context data for the airship route.
// This is the context data for the pilot_airship action.
#[derive(Debug, Clone)]
pub struct AirshipRouteContext {
    // The current approach index, 0 1 or none
    pub current_approach_index: Option<usize>,
    // The names of the route's sites
    pub site_names: [String; 2],
}

impl Default for AirshipRouteContext {
    fn default() -> Self {
        Self {
            current_approach_index: None,
            site_names: ["".to_string(), "".to_string()],
        }
    }
}

/// Data for tracking NPC movement (velocity and direction).
/// Used for airship collision avoidance.
/// Npc movement direction is not accurate in RTSim or ECS.
/// In fact, it appears to be not calculated at all after the initial spawn.
/// This tracker is a workaround to get the direction of the NPC movement.
/// It calculates the average velocity and average direction of the NPC movement
/// over time. To work well, the update rate should be at least once per second.
#[derive(Debug, Clone)]
struct MovementTracker {
    prev_pos: Option<Vec2<f32>>,
    prev_time: Option<Time>,
    velocities: Vec<Vec2<f32>>,
    max_velocities: usize,
}

impl MovementTracker {
    fn new(max_velocities: usize) -> Self {
        Self {
            max_velocities,
            prev_pos: None,
            prev_time: None,
            velocities: Vec::new(),
        }
    }

    fn update(&mut self, pos: Vec2<f32>, time: Time) -> Option<(Vec2<f32>, Vec2<f32>)> {
        if let Some(last_pos) = self.prev_pos
            && let Some(last_time) = self.prev_time
        {
            self.prev_pos = Some(pos);
            self.prev_time = Some(time);
            // Only use the last max_velocities velocities
            if self.velocities.len() >= self.max_velocities {
                self.velocities.remove(0);
            }
            // Not sure how time can stand still, but just in case...
            if time.0 - last_time.0 > 0.0 {
                self.velocities
                    .push((pos - last_pos) / (time.0 - last_time.0) as f32);
            }
            // It's best to have at least 3 data points to get a good average.
            if self.velocities.len() > 2 {
                let avg_vel = self.velocities.iter().fold(Vec2::zero(), |acc, v| acc + *v)
                    / self.velocities.len() as f32;
                let avg_dir = avg_vel.try_normalized().unwrap_or_else(Vec2::zero);
                Some((avg_vel, avg_dir))
            } else {
                None
            }
        } else {
            self.prev_pos = Some(pos);
            self.prev_time = Some(time);
            None
        }
    }
}

// This is the context data for the fly_airship action.
#[derive(Debug, Clone)]
struct FlyAirshipContext {
    // For determining the velocity and direction of this and other airships on the route.
    trackers: DHashMap<NpcId, MovementTracker>,
    // The interval for updating the airship tracker.
    timer: Duration,
    // The orinal target position passed to the fly_airship action.
    target_wpos: Vec3<f32>,
    // The current position at which the airship is holding.
    hold_wpos: Vec3<f32>,
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
    fn new(timer: Duration, target_wpos: Vec3<f32>, speed_factor: f32) -> Self {
        Self {
            trackers: DHashMap::default(),
            timer,
            target_wpos,
            hold_wpos: target_wpos,
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
/// and height above terrain, and to hold position.
///
/// # Avoidance Logic
/// Conditions that would cause this pilot to hold
/// 1. Another airship is really close and heading same way
///    - within 500 blocks
///    - bearing is fwd (within 90 degrees)
///    - direction is within 30 degrees of my direction
/// 2. Another airship is really close and other near dock
///    - within 500 blocks
///    - other within 500 blocks of the docking point for my approach
///    - bearing is fwd (within 90 degrees)
///
/// Conditions that would cause me to slow down
/// 1. Another airship is close and heading same way
///    - within 2000 blocks
///    - bearing is within 30 degrees of my direction
///    - Dir is within 30 degrees of my direction
///
/// ```ignore
/// for each other pilot on self.route
///   get
///     - distance
///     - bearing
///     - z (alt)
///     - angle between my direction and their direction
///     - dock_dist (dist they are from my docking point)
///   if dist < 500 && bearing < 90 && (angle < 30 or (other_dock_dist < 500 && my_z > other_z))
///     if zdiff.abs < 50
///       if zdiff > 0
///           start AvoidMode::Hold(mypos + z(75))
///       else
///           start AvoidMode::Hold(mypos - z(75))
///       end
///     else
///       start AvoidMode::Hold(mypos)
///     end
///   else if dist < 2000 && bearing < 30 && angle < 30
///       start AvoidMode::SlowDown
///   end
/// end
///
/// If any pilot has AvoidMode::Hold
///   switch PID mode to Hold & maintain hold position
/// else if any pilot has AvoidMode::SlowDown
///   speed = 50% of normal for the current route phase
/// else
///   resume normal flight
/// end
/// ```
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
/// - `initial_height_offset`: The initial height offset for the airship. Can be
///   modified by collision-avoidance. This is only used if
///   with_terrain_following is true.
/// - `with_terrain_following`: Whether to follow the terrain. If true, the
///   airship will fly at a height above the terrain. If false, the airship
///   flies directly to the target position.
/// - `direction_override`: An optional direction override for the airship. If
///   Some, the airship will be oriented (pointed) in this direction.
/// - `pid_mode`: The PID mode for the airship. Either FixedDirection (all axes)
///   or Z axis only.
/// - `pid_gain`: The PID gain for the airship. When PID mode is FixedDirection,
///   the PID effect is modified by this gain.
/// - `with_collision_avoidance`: Whether to perform collision avoidance. It's
///   not needed for docking because other airships must give way to this
///   airship.
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
    initial_height_offset: f32,
    with_terrain_following: bool,
    direction_override: Option<Dir>,
    pid_mode: PidMode,
    pid_gain: PidGain,
    with_collision_avoidance: bool,
    radar_interval: Duration,
) -> impl Action<S> {
    let current_approach_index = route_context.current_approach_index.unwrap_or(0);
    just(
        move |ctx, airship_context: &mut Option<FlyAirshipContext>| {
            // init context
            let airship_context = airship_context.get_or_insert_with(|| {
                FlyAirshipContext::new(radar_interval, wpos, initial_speed_factor)
            });

            // The collision avoidance checks are not done every tick, and are variable
            // according to the flight phase.
            let remaining = airship_context
                .timer
                .checked_sub(Duration::from_secs_f32(ctx.dt));
            if remaining.is_none() {
                airship_context.timer = radar_interval;
                if with_collision_avoidance {
                    let mypos = ctx.npc.wpos;
                    let my_tracker = airship_context
                        .trackers
                        .entry(ctx.npc_id)
                        .or_insert_with(|| MovementTracker::new(5));
                    if let Some((_, my_avg_dir)) = my_tracker.update(mypos.xy(), ctx.time)
                        && !my_avg_dir.is_approx_zero()
                        && let Some(route_id) = ctx
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
                        let avoidance: Vec<AirshipAvoidanceMode> = pilots
                            .iter()
                            .filter(|pilot_id| **pilot_id != ctx.npc_id)
                            .filter_map(|pilot_id| {
                                ctx.state.data().npcs.get(*pilot_id).and_then(|pilot| {
                                    let pilot_wpos = pilot.wpos;

                                    let other_tracker = airship_context
                                        .trackers
                                        .entry(*pilot_id)
                                        .or_insert_with(|| MovementTracker::new(5));
                                    if let Some((_, other_avg_dir)) =
                                        other_tracker.update(pilot_wpos.xy(), ctx.time)
                                        && !other_avg_dir.is_approx_zero()
                                    {
                                        let dist2 =
                                            mypos.xy().distance_squared(pilot_wpos.xy()) as i32;
                                        let to_other = pilot_wpos.xy() - mypos.xy();
                                        let bearing =
                                            to_other.angle_between(my_avg_dir).to_degrees();
                                        let angle =
                                            my_avg_dir.angle_between(other_avg_dir).to_degrees();
                                        let other_dock_dist = approach
                                            .airship_pos
                                            .xy()
                                            .distance_squared(pilot_wpos.xy())
                                            as i32;

                                        let really_close = dist2 < 500i32.pow(2);
                                        let close = dist2 < 2000i32.pow(2);
                                        let other_near_dock = other_dock_dist < 500i32.pow(2);

                                        if (really_close && bearing < 90.0)
                                            && (angle < 30.0
                                                || (other_near_dock && mypos.z > pilot_wpos.z))
                                        {
                                            if (mypos.z - pilot_wpos.z).abs() < 50.0 {
                                                if mypos.z - pilot_wpos.z > 0.0 {
                                                    Some(AirshipAvoidanceMode::Hold(75.0))
                                                } else {
                                                    Some(AirshipAvoidanceMode::Hold(-75.0))
                                                }
                                            } else {
                                                Some(AirshipAvoidanceMode::Hold(0.0))
                                            }
                                        } else if close && bearing < 30.0 && angle < 30.0 {
                                            Some(AirshipAvoidanceMode::SlowDown)
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                            })
                            .collect();

                        if let Some(hold_z_adjust) = avoidance.iter().find_map(|mode| {
                            if let AirshipAvoidanceMode::Hold(z_adjust) = mode {
                                Some(z_adjust)
                            } else {
                                None
                            }
                        }) {
                            if !matches!(airship_context.avoid_mode, AirshipAvoidanceMode::Hold(_))
                            {
                                airship_context.avoid_mode =
                                    AirshipAvoidanceMode::Hold(*hold_z_adjust);
                                airship_context.hold_wpos = mypos;
                                airship_context.hold_wpos.z += *hold_z_adjust;
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
                }
            } else {
                airship_context.timer = remaining.unwrap();
            }

            if matches!(airship_context.avoid_mode, AirshipAvoidanceMode::Hold(_)) {
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
                // Hold position (same idea as holding station at the dock)
                let hold_pos = if matches!(ctx.npc.mode, SimulationMode::Simulated) {
                    airship_context.hold_wpos
                } else {
                    // Airship is loaded, add some randomness to the hold position
                    // so that the airship doesn't look like it's stuck in one place.
                    // This also keeps the propellers spinning slowly and somewhat randomly.
                    airship_context.hold_wpos
                        + [
                            ctx.rng.gen_range(-5.0..5.0),
                            ctx.rng.gen_range(-5.0..5.0),
                            ctx.rng.gen_range(-10.0..10.0),
                        ]
                };

                ctx.controller.do_goto_with_height_and_dir(
                    hold_pos,
                    0.15,
                    None,
                    Dir::from_unnormalized(ctx.npc.dir.with_z(0.0)),
                    PidMode::FixedDirection,
                    PidGain::Normal,
                );
            } else {
                // use terrain height offset if specified
                let height_offset_opt = if with_terrain_following {
                    Some(initial_height_offset)
                } else {
                    None
                };
                // Move the airship
                ctx.controller.do_goto_with_height_and_dir(
                    airship_context.target_wpos,
                    airship_context.speed_factor,
                    height_offset_opt,
                    direction_override,
                    pid_mode,
                    pid_gain,
                );
            }
        },
    )
    .repeat()
    .boxed()
    .with_state(None)
    .stop_if(move |ctx: &mut NpcCtx| {
        if pid_mode == PidMode::FixedDirection {
            // FixedDirection means the PID controller will be controlling all three axes
            ctx.npc.wpos.distance_squared(wpos) < goal_dist.powi(2)
        } else {
            // otherwise, we only care about the xy distance (like in goto_flying)
            ctx.npc.wpos.xy().distance_squared(wpos.xy()) < goal_dist.powi(2)
        }
    })
    .debug(move || {
        format!(
            "fly airship, phase:{:?}, tgt pos:({}, {}, {}), goal dist:{}, speed:{}, height:{}, \
             terrain following:{}, PID mode:{:?},PID gain:{:?}, collision avoidance:{}, radar \
             interval:{}",
            phase,
            wpos.x,
            wpos.y,
            wpos.z,
            goal_dist,
            initial_speed_factor,
            initial_height_offset,
            with_terrain_following,
            pid_mode,
            pid_gain,
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
fn resume_route(airships: &Airships, route_id: &u32, ctx: &mut NpcCtx<'_>) -> usize {
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

            route_context.site_names = [route.approaches[0].site_name.clone(), route.approaches[1].site_name.clone()];

            if let Some(current_approach_index) = route_context.current_approach_index {
                // when current_approach_index exists, it means we're repeating the flight loop
                // if approach index is 0, then the airship is fly from site 0 to site 1, and vice versa

                let ship_body = comp::Body::from(ship);
                let approach1 = route.approaches[current_approach_index].clone();
                let approach2 = route.approaches[(current_approach_index + 1) % 2].clone();

                // Regular Flight Loop
                // Fly 3D to Docking Transition Point, full PID control

                fly_airship(
                    route_context,
                    AirshipFlightPhase::Transition,
                    approach1.airship_pos + Vec3::unit_z() * (approach1.cruise_hat),
                    50.0,
                    0.2,
                    approach1.cruise_hat,
                    true,
                    Some(approach1.airship_direction),
                    PidMode::FixedDirection,
                    PidGain::Normal,
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
                                PidMode::FixedDirection,
                                PidGain::Normal,
                            );
                    })
                    .repeat()
                    .stop_if(timeout(9.0))) // timeout(9.0 + approach_pause as f64),
                .then(
                    // descend to 50 blocks above the dock
                    just(move |ctx, _| {
                        ctx.controller
                            .do_goto_with_height_and_dir(
                                approach1.airship_pos + Vec3::unit_z() * 50.0,
                                0.6, None,
                                Some(approach1.airship_direction),
                                PidMode::FixedDirection,
                                PidGain::Normal,
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
                                PidMode::FixedDirection,
                                PidGain::High,
                            );
                    })
                    .repeat()
                    .stop_if(timeout(6.0)))
                // Announce arrival
                .then(just(move |ctx, route_context:&mut AirshipRouteContext| {
                    debug_airship_ai!("{}, Docked, {}", format!("{:?}", ctx.npc_id), route_context.site_names[current_approach_index]);
                    ctx.controller
                        .say(None, Content::localized("npc-speech-pilot-landed"));
                }))

                // Docked - Wait at Dock (default time + extension time)
                .then(
                    just(move |ctx, _| {
                        ctx.controller
                        .do_goto_with_height_and_dir(
                            approach1.airship_pos + ship_body.mount_offset(),
                            0.4, None,
                            Some(approach1.airship_direction),
                            PidMode::FixedDirection,
                            PidGain::High,
                        );
                    })
                    .repeat()
                    .stop_if(timeout(ctx.rng.gen_range(10.0..20.0)))
                    // While waiting, every now and then announce where the airship is going next.
                    .then(
                        just(move |ctx, route_context:&mut AirshipRouteContext| {
                            let to_site_name = if route_context.current_approach_index == Some(0) {
                                &route_context.site_names[1] // the approach index hasn't been switched yet
                            } else {
                                &route_context.site_names[0]
                            };
                            ctx.controller.say(
                                None,
                                Content::localized_with_args("npc-speech-pilot-announce_next", [
                                (
                                    "dir",
                                    Direction::from_dir(approach2.approach_initial_pos - ctx.npc.wpos.xy()).localize_npc(),
                                ),
                                ("dst", Content::Plain(to_site_name.to_string())),
                                ]),
                            );
                        })
                    )
                    .repeat()
                    .stop_if(timeout(Airships::docking_duration()))
                ).then(
                    // rotate the approach to the next approach index. Note the approach2 is already known,
                    // this is just changing the approach index in the context data for the next loop.
                    just(move |ctx, route_context:&mut AirshipRouteContext| {
                        let next_approach_index = (route_context.current_approach_index.unwrap() + 1) % 2;
                        route_context.current_approach_index = Some(next_approach_index);

                        let from_site_name = if next_approach_index == 0 {
                            &route_context.site_names[1]
                        } else {
                            &route_context.site_names[0]
                        };
                        let to_site_name = if next_approach_index == 0 {
                            &route_context.site_names[0]
                        } else {
                            &route_context.site_names[1]
                        };
                        ctx.controller.say(
                            None,
                            Content::localized_with_args("npc-speech-pilot-takeoff", [
                                ("src", Content::Plain(from_site_name.to_string())),
                                ("dst", Content::Plain(to_site_name.to_string())),
                            ]),
                        );
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
                        PidMode::FixedDirection,
                        PidGain::Normal,
                        false,
                        Duration::from_secs_f32(120.0),
                    )
                ).then(
                    // Fly 2D to Destination Initial Point
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::Cruise,
                        approach_target_pos(ctx, approach2.approach_initial_pos, approach2.airship_pos.z + approach2.cruise_hat, approach2.cruise_hat),
                        250.0,
                        1.0,
                        approach2.cruise_hat,
                        true,
                        None,
                        PidMode::PureZ,
                        PidGain::Normal,
                        true,
                        Duration::from_secs_f32(1.0),
                    )
                ).then(
                    // Fly 3D to Destination Final Point, z PID control
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::ApproachFinal,
                        approach_target_pos(ctx, approach2.approach_final_pos, approach2.airship_pos.z + approach2.cruise_hat, approach2.cruise_hat),
                        250.0,
                        0.3,
                        approach2.cruise_hat,
                        true,
                        Some(approach2.airship_direction),
                        PidMode::PureZ,
                        PidGain::Normal,
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
                    approach_target_pos(ctx, ctx.npc.wpos.xy(), 0.0, approach.cruise_hat),
                    100.0,
                    0.08,
                    0.0,
                    false,
                    Some(Dir::from_unnormalized((resume_pos - ctx.npc.wpos.xy()).with_z(0.0)).unwrap_or_default()),
                    PidMode::FixedDirection,
                    PidGain::Normal,
                    false,
                    Duration::from_secs_f32(120.0),
                )
                .then(
                    // Fly to the resume position
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::ResetResume,
                        approach_target_pos(ctx, resume_pos, 0.0, approach.cruise_hat),
                        250.0,
                        1.0,
                        approach.cruise_hat,
                        true,
                        None,
                        PidMode::PureZ,
                        PidGain::Normal,
                        true,
                        Duration::from_secs_f32(1.0),
                    ))
                .then(
                    // Fly to the approach initial point
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::ResetInitial,
                        approach_target_pos(ctx, approach.approach_initial_pos, approach.airship_pos.z + approach.cruise_hat, approach.cruise_hat),
                        250.0,
                        0.4,
                        approach.cruise_hat,
                        true,
                        None,
                        PidMode::PureZ,
                        PidGain::Normal,
                        true,
                        Duration::from_secs_f32(1.0),
                    ))
                .then(
                    // fly to approach final point
                    fly_airship(
                        route_context,
                        AirshipFlightPhase::ResetFinal,
                        approach_target_pos(ctx, approach.approach_final_pos, approach.airship_pos.z + approach.cruise_hat, approach.cruise_hat),
                        250.0,
                        0.3,
                        approach.cruise_hat,
                        true,
                        Some(approach.airship_direction),
                        PidMode::PureZ,
                        PidGain::Normal,
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
