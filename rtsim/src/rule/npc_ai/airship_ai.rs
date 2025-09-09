#[cfg(feature = "airship_log")]
use crate::rule::npc_ai::airship_logger::airship_logger;
use crate::{
    ai::{Action, NpcCtx, State, finish, just, now, predicate::timeout},
    data::npc::SimulationMode,
};

use common::{
    comp::{
        Content,
        agent::{BrakingMode, FlightMode},
        compass::Direction,
    },
    consts::AIR_DENSITY,
    rtsim::NpcId,
    util::Dir,
};
use num_traits::cast::FromPrimitive;
use rand::prelude::*;
use std::{cmp::Ordering, collections::VecDeque, fmt, time::Duration};
use vek::*;
use world::{
    civ::airship_travel::{AirshipDockingApproach, Airships},
    util::CARDINALS,
};

#[cfg(debug_assertions)] use tracing::debug;

use tracing::warn;

const CLOSE_TO_DOCKING_SITE_DISTANCE_SQR: f32 = 225.0f32 * 225.0f32;
const VERY_CLOSE_AIRSHIP_DISTANCE_SQR: f32 = 400.0f32 * 400.0f32;
const CRUISE_CHECKPOINT_DISTANCE: f32 = 800.0;
const NEXT_PILOT_CRUISE_SPEED_TOLERANCE: f32 = 2.0;
const MOVING_AVERAGE_SCALE_FACTOR: f64 = 10000.0;
const NEXT_PILOT_MOVING_VELOCITY_AVERAGE_CAPACITY: usize = 5;
const NEXT_PILOT_MOVING_VELOCITY_AVERAGE_MIN_SIZE: usize = 3;
const NEXT_PILOT_MOVING_DIST_AVERAGE_CAPACITY: usize = 3;
const NEXT_PILOT_MOVING_DIST_AVERAGE_MIN_SIZE: usize = 2;
const NEXT_PILOT_MOVING_DIST_TRACKER_THRESHOLD_SQR: usize = 5 * 5;
const NEXT_PILOT_VELOCITY_RATIO_MIN: f32 = 1.05;
const NEXT_PILOT_SPACING_THRESHOLD_SQR: f32 = 0.6 * 0.6; // squared because distances are compared while squared;
const CLOSE_AIRSHIP_SPEED_FACTOR: f32 = 0.9;

/// Airships can slow down or hold position to avoid collisions with other
/// airships. Stuck mode means the airship was stuck in one position and
/// is not backing out and climbing to clear the obstacle.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
enum AirshipAvoidanceMode {
    #[default]
    None,
    Slow(f32),
    Hold(Vec3<f32>, Dir),
    Stuck(Vec3<f32>),
}

/// The context data for the pilot_airship action.
#[derive(Debug, Clone)]
struct AirshipRouteContext {
    /// The route index (index into the outer vec of airships.routes)
    route_index: usize,
    /// The expected airship speed when simulated.
    simulated_airship_speed: f32,
    /// The next route leg index.
    next_leg: usize,
    /// The NpcId of the captain ahead of this one.
    next_pilot_id: NpcId,
    /// The current approach.
    current_leg_approach: Option<AirshipDockingApproach>,
    /// The next approach.
    next_leg_approach: Option<AirshipDockingApproach>,
    /// A point short of the approach transition point where the frequency of
    /// avoidance checks increases.
    next_leg_cruise_checkpoint_pos: Vec3<f32>,
    /// Value used to convert a target velocity to a speed factor.
    speed_factor_conversion_factor: f32,

    // Avoidance Data
    /// For tracking the airship's position history to determine if the airship
    /// is stuck.
    my_stuck_tracker: Option<StuckAirshipTracker>,
    /// For tracking airship velocity towards the approach transition point.
    my_rate_tracker: Option<RateTracker>,
    /// For tracking the next pilot's velocity towards the approach transition
    /// point.
    next_pilot_rate_tracker_: Option<RateTracker>,
    /// Timer for checking the airship trackers.
    avoidance_timer: Duration,
    /// Timer used when holding, either on approach or at the dock.
    hold_timer: f32,
    /// Whether the initial hold message has been sent to the client.
    hold_announced: bool,
    /// The moving average of the next pilot's velocity during cruise phase.
    next_pilot_average_velocity: MovingAverage<
        i64,
        NEXT_PILOT_MOVING_VELOCITY_AVERAGE_CAPACITY,
        NEXT_PILOT_MOVING_VELOCITY_AVERAGE_MIN_SIZE,
    >,
    /// The moving average of the next pilot's distance from my current docking
    /// position target pos.
    next_pilot_dist_to_my_docking_pos_tracker:
        DistanceTrendTracker<NEXT_PILOT_MOVING_DIST_TRACKER_THRESHOLD_SQR>,
    /// The current avoidance mode for the airship.
    avoid_mode: AirshipAvoidanceMode,
    /// Whether the airship had to hold during the last flight.
    did_hold: bool,
    /// The extra docking time due to holding.
    extra_hold_dock_time: f32,
}

impl Default for AirshipRouteContext {
    fn default() -> Self {
        Self {
            route_index: usize::MAX,
            simulated_airship_speed: 0.0,
            next_leg: 0,
            next_pilot_id: NpcId::default(),
            current_leg_approach: None,
            next_leg_approach: None,
            next_leg_cruise_checkpoint_pos: Vec3::default(),
            speed_factor_conversion_factor: 0.0,
            my_stuck_tracker: None,
            my_rate_tracker: None,
            next_pilot_rate_tracker_: None,
            avoidance_timer: Duration::default(),
            hold_timer: 0.0,
            hold_announced: false,
            next_pilot_average_velocity: MovingAverage::default(),
            next_pilot_dist_to_my_docking_pos_tracker: DistanceTrendTracker::default(),
            avoid_mode: AirshipAvoidanceMode::default(),
            did_hold: false,
            extra_hold_dock_time: 0.0,
        }
    }
}

/// Tracks the airship position history.
/// Used for determining if an airship is stuck.
#[derive(Debug, Default, Clone)]
struct StuckAirshipTracker {
    /// The airship's position history. Used for determining if the airship is
    /// stuck in one place.
    pos_history: Vec<Vec3<f32>>,
    /// The route to follow for backing out of a stuck position.
    backout_route: Vec<Vec3<f32>>,
}

impl StuckAirshipTracker {
    /// The distance to back out from the stuck position.
    const BACKOUT_DIST: f32 = 100.0;
    /// The tolerance for determining if the airship has reached a backout
    /// position.
    const BACKOUT_TARGET_DIST: f32 = 50.0;
    /// The number of positions to track in the position history.
    const MAX_POS_HISTORY_SIZE: usize = 5;
    /// The height for testing if the airship is near the ground.
    const NEAR_GROUND_HEIGHT: f32 = 10.0;

    /// Add a new position to the position history, maintaining a fixed size.
    fn add_position(&mut self, new_pos: Vec3<f32>) {
        if self.pos_history.len() >= StuckAirshipTracker::MAX_POS_HISTORY_SIZE {
            self.pos_history.remove(0);
        }
        self.pos_history.push(new_pos);
    }

    /// Get the current backout position.
    /// If the backout route is not empty, return the first position in the
    /// route. As a side effect, if the airship position is within the
    /// target distance of the first backout position, remove the backout
    /// position. If there are no more backout postions, the position
    /// history is cleared (because it will be stale data), and return None.
    fn current_backout_pos(&mut self, ctx: &mut NpcCtx) -> Option<Vec3<f32>> {
        if !self.backout_route.is_empty()
            && let Some(pos) = self.backout_route.first().cloned()
        {
            if ctx.npc.wpos.distance_squared(pos) < StuckAirshipTracker::BACKOUT_TARGET_DIST.powi(2)
            {
                self.backout_route.remove(0);
            }
            Some(pos)
        } else {
            self.pos_history.clear();
            None
        }
    }

    /// Check if the airship is stuck in one place. This check is done only in
    /// cruise flight when the PID controller is affecting the Z axis
    /// movement only. When the airship gets stuck, it will stop moving. The
    /// only recourse is reverse direction, back up, and then ascend to
    /// hopefully fly over the top of the obstacle. This may be repeated if the
    /// airship gets stuck again. When the determination is made that the
    /// airship is stuck, two positions are generated for the backout
    /// procedure: the first is in the reverse of the direction the airship
    /// was recently moving, and the second is straight up from the first
    /// position. If the airship was near the ground when it got stuck, the
    /// initial backout is done while climbing slightly to avoid any other
    /// near-ground objects. If the airship was not near the ground, the
    /// initial backout position is at the same height as the current position,
    fn is_stuck(
        &mut self,
        ctx: &mut NpcCtx,
        current_pos: &Vec3<f32>,
        target_pos: &Vec2<f32>,
    ) -> bool {
        self.add_position(*current_pos);
        // The position history must be full to determine if the airship is stuck.
        if self.pos_history.len() == StuckAirshipTracker::MAX_POS_HISTORY_SIZE
            && self.backout_route.is_empty()
            && let Some(last_pos) = self.pos_history.last()
        {
            // If all the positions in the history are within 10 of the last position,
            if self
                .pos_history
                .iter()
                .all(|pos| pos.distance_squared(*last_pos) < 10.0)
            {
                // Airship is stuck on some obstacle.

                // The direction to backout is opposite to the direction from the airship
                // to where it was going before it got stuck.
                if let Some(backout_dir) = (ctx.npc.wpos.xy() - target_pos)
                    .with_z(0.0)
                    .try_normalized()
                {
                    let ground = ctx
                        .world
                        .sim()
                        .get_surface_alt_approx(last_pos.xy().map(|e| e as i32));
                    // The position to backout to is the current position + a distance in the
                    // backout direction.
                    let mut backout_pos =
                        ctx.npc.wpos + backout_dir * StuckAirshipTracker::BACKOUT_DIST;
                    // Add a z offset to the backout pos if the airship is near the ground.
                    if (ctx.npc.wpos.z - ground).abs() < StuckAirshipTracker::NEAR_GROUND_HEIGHT {
                        backout_pos.z += 50.0;
                    }
                    self.backout_route = vec![backout_pos, backout_pos + Vec3::unit_z() * 200.0];
                    // The airship is stuck.
                    #[cfg(debug_assertions)]
                    debug!(
                        "Airship {} Stuck! at {} {} {}, backout_dir:{:?}, backout_pos:{:?}",
                        format!("{:?}", ctx.npc_id),
                        ctx.npc.wpos.x,
                        ctx.npc.wpos.y,
                        ctx.npc.wpos.z,
                        backout_dir,
                        backout_pos
                    );
                }
            }
        }
        !self.backout_route.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
/// Tracks previous position and time to get a rough estimate of an NPC's
/// velocity.
struct RateTracker {
    /// The previous position of the NPC in the XY plane.
    last_pos: Option<Vec2<f32>>,
    /// The game time when the last position was recorded.
    last_time: f32,
}

impl RateTracker {
    /// Calculates the velocity estimate, updates the previous values and
    /// returns the velocity.
    fn update(&mut self, current_pos: Vec2<f32>, time: f32) -> f32 {
        let rate = if let Some(last_pos) = self.last_pos {
            let dt = time - self.last_time;
            if dt > 0.0 {
                current_pos.distance(last_pos) / dt // blocks per second
            } else {
                0.0
            }
        } else {
            0.0
        };
        self.last_pos = Some(current_pos);
        self.last_time = time;
        rate
    }
}

#[derive(Debug, PartialEq)]
/// The trend of the distance changes. The distance measured could be
/// between a fixed position and an NPC or between two NPCs.
enum DistanceTrend {
    /// The distance is decreasing.
    Towards,
    /// The distance is increasing.
    Away,
    /// The distance is not changing significantly.
    Neutral,
}

/// Tracks airship distance trend from a fixed position. Used for airship
/// traffic control. The parameter C is the threshold for determining if the
/// trend is stable (i.e., not increasing or decreasing too much).
#[derive(Debug, Default, Clone)]
struct DistanceTrendTracker<const C: usize> {
    /// The fixed position to track the distance from.
    fixed_pos: Vec2<f32>,
    /// A moving average of the distance change rate.
    avg_rate: MovingAverage<
        f64,
        NEXT_PILOT_MOVING_DIST_AVERAGE_CAPACITY,
        NEXT_PILOT_MOVING_DIST_AVERAGE_MIN_SIZE,
    >,
    /// The most recent (previous) distance measurement.
    prev_dist: Option<f32>,
    /// The time when the previous distance was measured.
    prev_time: f64,
}

impl<const C: usize> DistanceTrendTracker<C> {
    /// Updates the distance trend based on the current position and time versus
    /// the previous position and time.
    fn update(&mut self, pos: Vec2<f32>, time: f64) -> Option<DistanceTrend> {
        let current_dist = pos.distance(self.fixed_pos);
        if let Some(prev) = self.prev_dist {
            // rate is blocks per second
            // Greater than 0 means the distance is increasing (going away from the target
            // pos). Near zero means the airship is stationary or moving
            // perpendicular to the target pos.
            if time - self.prev_time > f64::EPSILON {
                let rate = (current_dist - prev) as f64 / (time - self.prev_time);
                self.prev_dist = Some(current_dist);
                self.prev_time = time;

                self.avg_rate.add(rate);
                if let Some(avg) = self.avg_rate.average() {
                    if avg > C as f64 {
                        Some(DistanceTrend::Away)
                    } else if avg < -(C as f64) {
                        Some(DistanceTrend::Towards)
                    } else {
                        Some(DistanceTrend::Neutral)
                    }
                } else {
                    None
                }
            } else {
                // If not enough time has passed, keep the older previous values.
                None
            }
        } else {
            self.prev_dist = Some(current_dist);
            self.prev_time = time;
            None
        }
    }

    fn reset(&mut self, pos: Vec2<f32>) {
        self.fixed_pos = pos;
        self.avg_rate.reset();
        self.prev_dist = None;
    }
}

/// A moving average of at least N values and at most S values.
#[derive(Clone, Debug)]
struct MovingAverage<T, const S: usize, const N: usize>
where
    T: Default
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::Sub<Output = T>
        + std::ops::Div<Output = T>
        + Copy,
{
    values: VecDeque<T>,
    sum: T,
}

impl<T, const S: usize, const N: usize> MovingAverage<T, S, N>
where
    T: Default
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::Sub<Output = T>
        + std::ops::Div<Output = T>
        + Copy,
{
    /// Add a value to the average. Maintains the sum without needing to iterate
    /// the values.
    fn add(&mut self, value: T) {
        if self.values.len() == S
            && let Some(old_value) = self.values.pop_front()
        {
            self.sum = self.sum - old_value;
        }
        self.values.push_back(value);
        self.sum += value;
    }

    /// Returns the current average, if enough values have been added.
    fn average(&self) -> Option<T> {
        if self.values.len() < N {
            None
        } else {
            Some(self.sum / T::from_u32(self.values.len() as u32).unwrap())
        }
    }

    fn reset(&mut self) {
        self.values.clear();
        self.sum = T::from_u32(0).unwrap();
    }
}

impl<T, const S: usize, const N: usize> Default for MovingAverage<T, S, N>
where
    T: Default
        + FromPrimitive
        + std::ops::AddAssign
        + std::ops::Sub<Output = T>
        + std::ops::Div<Output = T>
        + Copy,
{
    fn default() -> Self {
        Self {
            values: VecDeque::with_capacity(S),
            sum: T::from_u32(0).unwrap(),
        }
    }
}

/// The flight phases of an airship.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum AirshipFlightPhase {
    Ascent,
    DepartureCruise,
    ApproachCruise,
    Transition,
    #[default]
    Docked,
}

impl fmt::Display for AirshipFlightPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AirshipFlightPhase::Ascent => write!(f, "Ascent"),
            AirshipFlightPhase::DepartureCruise => write!(f, "DepartureCruise"),
            AirshipFlightPhase::ApproachCruise => write!(f, "ApproachCruise"),
            AirshipFlightPhase::Transition => write!(f, "Transition"),
            AirshipFlightPhase::Docked => write!(f, "Docked"),
        }
    }
}

/// Wrapper for the fly_airship action, so the route context fields can be
/// reset.
fn fly_airship(
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
) -> impl Action<AirshipRouteContext> {
    now(move |_, airship_context: &mut AirshipRouteContext| {
        airship_context.avoidance_timer = radar_interval;
        airship_context.avoid_mode = AirshipAvoidanceMode::None;
        if matches!(phase, AirshipFlightPhase::DepartureCruise) {
            // Reset the stuck tracker.
            airship_context.my_stuck_tracker = Some(StuckAirshipTracker::default());
        }
        fly_airship_inner(
            phase,
            wpos,
            goal_dist,
            initial_speed_factor,
            height_offset,
            with_terrain_following,
            direction_override,
            flight_mode,
            with_collision_avoidance,
            radar_interval,
        )
    })
}

/// My airship should hold position if the next pilot is moving towards my
/// docking target and is close to my docking target and my pilot is close to
/// the next pilot.
fn should_hold(
    my_pilot_distance_to_next_pilot: f32,
    next_pilot_dist_to_docking_target: f32,
    next_pilot_to_docking_target_trend: &DistanceTrend,
) -> bool {
    *next_pilot_to_docking_target_trend == DistanceTrend::Towards
        && next_pilot_dist_to_docking_target < CLOSE_TO_DOCKING_SITE_DISTANCE_SQR
        && my_pilot_distance_to_next_pilot < VERY_CLOSE_AIRSHIP_DISTANCE_SQR
}

/// My pilot should slow down if the pilot ahead is moving towards my target
/// docking position and the ratio of next pilot velocity over my velocity is
/// less than a threshold (i.e. the next pilot is moving slower than my pilot),
/// and the distance between the next pilot and my pilot is less than some
/// fraction of the standard airship spacing, and the distance between my pilot
/// and my docking target position is greater than the distance between the next
/// pilot and my docking target position (i.e. the next pilot is inside my
/// radius from my target docking position).
fn should_slow_down(
    my_pilot_pos: &Vec2<f32>,
    my_pilot_velocity: f32,
    my_pilot_dist_to_docking_target: f32,
    next_pilot_pos: &Vec2<f32>,
    next_pilot_velocity: f32,
    next_pilot_dist_to_docking_target: f32,
    next_pilot_to_docking_target_trend: &DistanceTrend,
) -> bool {
    *next_pilot_to_docking_target_trend == DistanceTrend::Towards
        && next_pilot_velocity / my_pilot_velocity < NEXT_PILOT_VELOCITY_RATIO_MIN
        && my_pilot_pos.distance_squared(*next_pilot_pos)
            < Airships::AIRSHIP_SPACING.powi(2) * NEXT_PILOT_SPACING_THRESHOLD_SQR
        && my_pilot_dist_to_docking_target > next_pilot_dist_to_docking_target
}

/// The normal controller movement action of the airship. Called from
/// fly_airship_inner() for cases that do not mean the airship is avoiding the
/// airship ahead of it on the route.
fn fly_inner_default_goto(
    ctx: &mut NpcCtx,
    wpos: Vec3<f32>,
    speed_factor: f32,
    height_offset: f32,
    with_terrain_following: bool,
    direction_override: Option<Dir>,
    flight_mode: FlightMode,
) {
    let height_offset_opt = if with_terrain_following {
        Some(height_offset)
    } else {
        None
    };
    ctx.controller.do_goto_with_height_and_dir(
        wpos,
        speed_factor,
        height_offset_opt,
        direction_override,
        flight_mode,
    );
}

/// The action that moves the airship.
fn fly_airship_inner(
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
) -> impl Action<AirshipRouteContext> {
    just(
        move |ctx, airship_context: &mut AirshipRouteContext| {

            // It's safe to unwrap here, this function is not called if either airship_context.current_leg_approach
            // or airship_context.next_leg_approach are None. The pos_tracker_target_loc is for determining the reverse
            // direction for 'unsticking' the airship if it gets stuck in one place; In DepartureCruise phase, it is the
            // next_leg_cruise_checkpoint_pos and in ApproachCruise phase, it is the transition pos of the current destination.
            let (current_approach, pos_tracker_target_loc) =
                match phase {
                    AirshipFlightPhase::DepartureCruise => (&airship_context.next_leg_approach.unwrap(), airship_context.next_leg_cruise_checkpoint_pos.xy()),
                    _ => (&airship_context.current_leg_approach.unwrap(), airship_context.current_leg_approach.unwrap().approach_transition_pos.xy()),
                };

            // Decrement the avoidance timer. (the timer is always counting down)..
            // The collision avoidance checks are not done every tick, only
            // every 1-5 seconds depending on flight phase.
            let remaining = airship_context
                .avoidance_timer
                .checked_sub(Duration::from_secs_f32(ctx.dt));
            // If it's time for avoidance checks
            if remaining.is_none() {
                // reset the timer
                airship_context.avoidance_timer = radar_interval;

                #[cfg(feature = "airship_log")]
                // log my position
                log_airship_position(ctx, &phase);

                // If actually doing avoidance checks..
                if with_collision_avoidance
                    && matches!(phase, AirshipFlightPhase::DepartureCruise | AirshipFlightPhase::ApproachCruise)
                {
                    // This runs every time the avoidance timer counts down (1-5 seconds)

                    // get my velocity (my_rate_tracker)
                    // get my distance to my docking target position
                    // get the next pilot's position
                    // update next_pilot_trend (movement relative to my docking target position)
                    // update_next_pilot_average_velocity
                    // get next pilot's distance to my docking target position

                    let mypos = ctx.npc.wpos;
                    let my_velocity = if let Some(my_rate_tracker) = &mut airship_context.my_rate_tracker {
                        my_rate_tracker.update(mypos.xy(), ctx.time.0 as f32)
                    } else {
                        0.0
                    };
                    let my_distance_to_docking_target = mypos.xy().distance_squared(current_approach.airship_pos.xy());

                    let (next_pilot_wpos, next_pilot_dist_trend) = if let Some(next_pilot_rate_tracker) = &mut airship_context.next_pilot_rate_tracker_
                        && let Some(next_pilot) = ctx.state.data().npcs.get(airship_context.next_pilot_id)
                    {
                        let next_rate = next_pilot_rate_tracker.update(next_pilot.wpos.xy(), ctx.time.0 as f32);

                        // Estimate the distance trend of the next pilot relative to my target docking position.
                        let next_dist_trend = airship_context
                            .next_pilot_dist_to_my_docking_pos_tracker.update(next_pilot.wpos.xy(), ctx.time.0)
                            .unwrap_or(DistanceTrend::Away);

                        // Track the moving average of the velocity of the next pilot ahead of my pilot but only
                        // if the velocity is greater than the expected simulated cruise speed minus a small tolerance.
                        // (i.e., when it can be expected that the next pilot is in the cruise phase).
                        if next_rate > airship_context.simulated_airship_speed - NEXT_PILOT_CRUISE_SPEED_TOLERANCE {
                            // Scale up the velocity so that the moving average can be done as an integer.
                            airship_context.next_pilot_average_velocity.add((next_rate as f64 * MOVING_AVERAGE_SCALE_FACTOR) as i64);
                        }

                        (next_pilot.wpos, next_dist_trend)
                    } else {
                        (Vec3::zero(), DistanceTrend::Away)
                    };

                    let my_pilot_distance_to_next_pilot = mypos.xy().distance_squared(next_pilot_wpos.xy());
                    let next_pilot_distance_to_docking_target = next_pilot_wpos.xy().distance_squared(current_approach.airship_pos.xy());

                    // What to check is based on the current avoidance mode.
                    let avoidance = match airship_context.avoid_mode {
                        AirshipAvoidanceMode::Stuck(..) => {
                            // currently stuck and backing out.
                            // Don't change anything until the airship position matches the backout position.
                            // The current_backout_pos() will return None when the backout position is reached.
                            if let Some(stuck_tracker) = &mut airship_context.my_stuck_tracker
                                && stuck_tracker.is_stuck(ctx, &mypos, &pos_tracker_target_loc)
                                && let Some(backout_pos) = stuck_tracker.current_backout_pos(ctx)
                            {
                                AirshipAvoidanceMode::Stuck(backout_pos)
                            } else {
                                #[cfg(debug_assertions)]
                                debug!("{:?} unstuck at pos: {} {}",
                                    ctx.npc_id,
                                    mypos.x as i32, mypos.y as i32,
                                );

                                AirshipAvoidanceMode::None
                            }
                        }
                        AirshipAvoidanceMode::Hold(..) => {
                            // currently holding position.
                            // It is assumed that the airship can't get stuck while holding.
                            // Continue holding until the next pilot is moving away from the docking position.
                            if next_pilot_dist_trend != DistanceTrend::Away {
                                airship_context.avoid_mode
                            } else {
                                AirshipAvoidanceMode::None
                            }
                        }
                        AirshipAvoidanceMode::Slow(..) => {
                            // currently slowed down
                            // My pilot could get stuck or reach the hold criteria while slowed down.
                            if let Some(stuck_tracker) = &mut airship_context.my_stuck_tracker
                                && stuck_tracker.is_stuck(ctx, &mypos, &pos_tracker_target_loc)
                                && let Some(backout_pos) = stuck_tracker.current_backout_pos(ctx)
                            {
                                AirshipAvoidanceMode::Stuck(backout_pos)
                            } else if should_hold(
                                my_pilot_distance_to_next_pilot,
                                next_pilot_distance_to_docking_target,
                                &next_pilot_dist_trend,
                            ) {
                                airship_context.did_hold = true;
                                AirshipAvoidanceMode::Hold(
                                    mypos,
                                    // hold with the airship pointing at the dock
                                    Dir::from_unnormalized((current_approach.dock_center - mypos.xy()).with_z(0.0))
                                        .unwrap_or_default(),
                                )
                            } else {
                                // Since my pilot is already slowed to slightly slower than the next pilot,
                                // it's possible that the distance between my pilot and the next pilot could increase
                                // to above the slow-down threshold, but it's more likely that the next pilot has to dock
                                // and then start moving away from the docking position before my pilot can
                                // resume normal speed. If my pilot did not slow down enough, it's possible that the
                                // next pilot will still be at the dock when my pilot arrives, and that is taken
                                // care of by the Hold avoidance mode.
                                // Check if the distance to the next pilot has increased to above Airships::AIRSHIP_SPACING,
                                // otherwise continue the slow-down until the next pilot is moving away from the docking position.
                                if next_pilot_dist_trend == DistanceTrend::Away || my_pilot_distance_to_next_pilot > Airships::AIRSHIP_SPACING.powi(2) {
                                    AirshipAvoidanceMode::None
                                } else {
                                    // continue slow down
                                    airship_context.avoid_mode
                                }
                            }
                        }
                        AirshipAvoidanceMode::None => {
                            // not currently avoiding or stuck. Check for all conditions.
                            let next_pilot_avg_velocity = (airship_context.next_pilot_average_velocity.average().unwrap_or(0) as f64 / MOVING_AVERAGE_SCALE_FACTOR) as f32;
                            // Check if stuck
                            if let Some(stuck_tracker) = &mut airship_context.my_stuck_tracker
                                && stuck_tracker.is_stuck(ctx, &mypos, &pos_tracker_target_loc)
                                && let Some(backout_pos) = stuck_tracker.current_backout_pos(ctx)
                            {
                                #[cfg(debug_assertions)]
                                debug!("{:?} stuck at pos: {} {}",
                                    ctx.npc_id,
                                    mypos.x as i32, mypos.y as i32,
                                );
                                AirshipAvoidanceMode::Stuck(backout_pos)
                                // Check if hold criteria are met.
                            } else if should_hold(
                                my_pilot_distance_to_next_pilot,
                                next_pilot_distance_to_docking_target,
                                &next_pilot_dist_trend,
                            ) {
                                airship_context.did_hold = true;
                                AirshipAvoidanceMode::Hold(
                                    mypos,
                                    Dir::from_unnormalized((current_approach.dock_center - mypos.xy()).with_z(0.0))
                                        .unwrap_or_default(),
                                )
                                // Check if slow down criteria are met.
                            } else if should_slow_down(
                                &mypos.xy(),
                                my_velocity,
                                my_distance_to_docking_target,
                                &next_pilot_wpos.xy(),
                                next_pilot_avg_velocity,
                                next_pilot_distance_to_docking_target,
                                &next_pilot_dist_trend
                            ) {
                                // My pilot is getting close to the next pilot ahead and should slow down.
                                // If simulated, slow to CLOSE_AIRSHIP_SPEED_FACTOR.
                                // If loaded, slow to a percentage of the next pilot's average velocity.
                                let mut new_speed_factor = if matches!(ctx.npc.mode, SimulationMode::Simulated) {
                                    // In simulated mode, the next pilot's average velocity is not updated,
                                    // so we use the simulated airship speed.
                                    CLOSE_AIRSHIP_SPEED_FACTOR
                                } else {
                                    // loaded mode, adjust my speed factor based on a percentage of
                                    // the next pilot's average velocity.
                                    let target_velocity: f32 = next_pilot_avg_velocity * CLOSE_AIRSHIP_SPEED_FACTOR;

                                    // TODO: Document the speed factor calculation in the airship evolutions RFC.
                                    // Set my speed factor according to the target velocity.
                                    // speed_factor = (0.5 * air_density * velocity ^ 2 * reference_area)/(thrust * 0.9)
                                    //              = 0.45 * air_density * velocity ^ 2 * reference_area / thrust
                                    // where:
                                    //      air_density is a constant
                                    //      reference_area is a constant per airship model
                                    //      thrust is a constant per airship model
                                    // The airship_context.speed_factor_conversion_factor is a constant value equal to
                                    // 0.45 * air_density * reference_area / thrust
                                    // and the estimated speed factor is calculated as:
                                    // speed_factor = airship_context.speed_factor_conversion_factor * target_velocity ^ 2
                                    let new_sf = airship_context.speed_factor_conversion_factor * target_velocity.powi(2);
                                    if new_sf > 0.0 {
                                        #[cfg(debug_assertions)]
                                        debug!(
                                            "Pilot {:?}: Adjusting speed factor to {}, next pilot avg velocity: {}, * speed_factor = {}",
                                            ctx.npc_id,
                                            new_sf,
                                            next_pilot_avg_velocity,
                                            target_velocity,
                                        );
                                    }
                                    new_sf
                                };

                                if new_speed_factor < 0.05 {
                                    warn!("Pilot {:?} calculated slow down speed factor is too low, clamping to 0.05", ctx.npc_id);
                                    new_speed_factor = 0.05;
                                }
                                AirshipAvoidanceMode::Slow(
                                    new_speed_factor
                                )
                            } else {
                                AirshipAvoidanceMode::None
                            }
                        }
                    };

                    // If the avoidance mode has changed, update the airship context.
                    if avoidance != airship_context.avoid_mode {
                        airship_context.avoid_mode = avoidance;
                    }
                }
            } else {
                // counting down
                airship_context.avoidance_timer = remaining.unwrap_or(radar_interval);
            }

            // Every time through the loop, move the airship according to the avoidance mode.
            match airship_context.avoid_mode {
                AirshipAvoidanceMode::Stuck(backout_pos) => {
                    // Unstick the airship
                    ctx.controller.do_goto_with_height_and_dir(
                        backout_pos,
                        1.5,
                        None,
                        None,
                        FlightMode::Braking(BrakingMode::Normal),
                    );
                }
                AirshipAvoidanceMode::Hold(hold_pos, hold_dir) => {
                    // Hold position
                    airship_context.hold_timer -= ctx.dt;
                    if airship_context.hold_timer <= 0.0 {
                        if !airship_context.hold_announced {
                            airship_context.hold_announced = true;
                            ctx.controller
                                .say(None, Content::localized("npc-speech-pilot-announce_hold"));
                        } else {
                            ctx.controller
                                .say(None, Content::localized("npc-speech-pilot-continue_hold"));
                        }
                        airship_context.hold_timer = ctx.rng.random_range(10.0..20.0);
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
                        0.25,
                        None,
                        Some(hold_dir),
                        FlightMode::Braking(BrakingMode::Normal),
                    );
                }
                AirshipAvoidanceMode::Slow(speed_factor) => {
                    // Slow down mode. Only change from normal movement is the speed factor.
                    fly_inner_default_goto(ctx, wpos, speed_factor, height_offset,
                        with_terrain_following, direction_override, flight_mode,
                    );
                }
                AirshipAvoidanceMode::None => {
                    // Normal movement, no avoidance.
                    fly_inner_default_goto(ctx, wpos, initial_speed_factor, height_offset,
                        with_terrain_following, direction_override, flight_mode,
                    );
                }
            }
        },
    )
    .repeat()
    .boxed()
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

/// The NPC is the airship captain. This action defines the flight loop for the
/// airship. The captain NPC is autonomous and will fly the airship along the
/// assigned route. The routes are established and assigned to the captain NPCs
/// when the world is generated.
pub fn pilot_airship<S: State>() -> impl Action<S> {
    now(move |ctx, route_context: &mut AirshipRouteContext| {
        // get the assigned route and start leg indexes
        if let Some((route_index, start_leg_index)) = ctx.state.data().airship_sim.assigned_routes.get(&ctx.npc_id)
        {
            // ----- Server startup processing -----
            // If route_context.route_index is the default value (usize::MAX) it means the server has just started.

            if route_context.route_index == usize::MAX {
                // This block should only run once, when the server starts up.
                // Set up the route context fixed values.
                route_context.route_index = *route_index;
                route_context.next_leg = *start_leg_index;
                if let Some (next_pilot) = ctx.state.data().airship_sim.next_pilot(*route_index, ctx.npc_id) {
                    route_context.next_pilot_id = next_pilot;
                } else {
                    route_context.next_pilot_id = NpcId::default();
                }
                let (max_speed, reference_area, thrust) =
                    ctx.state.data().npcs.mounts.get_mount_link(ctx.npc_id)
                        .map(|mount_link|
                            ctx.state.data().npcs.get(mount_link.mount)
                                .map(|airship| {
                                    (airship.body.max_speed_approx(),
                                     airship.body.parasite_drag(1.0),
                                     airship.body.fly_thrust().unwrap_or_default())
                                })
                                .unwrap_or_default()
                        ).or({
                            // If the mount link is not found, use a default speed.
                            Some((0.0, 1.0, 1.0))
                        })
                        .unwrap_or((0.0, 1.0, 1.0));
                route_context.simulated_airship_speed = max_speed;
                route_context.speed_factor_conversion_factor = 0.45 * AIR_DENSITY * reference_area / thrust;
                route_context.my_rate_tracker = Some(RateTracker::default());
                route_context.next_pilot_rate_tracker_ = Some(RateTracker::default());

                #[cfg(debug_assertions)]
                {
                    let current_approach = ctx.world.civs().airships.approach_for_route_and_leg(
                        route_context.route_index,
                        route_context.next_leg,
                    );
                    debug!(
                        "Server startup, airship pilot {:?} starting on route {} and leg {}, target dock: {} {}, following pilot {:?}",
                        ctx.npc_id,
                        route_context.route_index,
                        route_context.next_leg,
                        current_approach.airship_pos.x as i32, current_approach.airship_pos.y as i32,
                        route_context.next_pilot_id
                    );
                }
                if route_context.next_pilot_id == NpcId::default() {
                    tracing::error!("Pilot {:?} has no next pilot to follow.", ctx.npc_id);
                }
            }

            // ----- Each time entering pilot_airship -----

            // set the approach data for the current leg
            // Needed: docking position and direction.
            route_context.current_leg_approach = Some(ctx.world.civs().airships.approach_for_route_and_leg(
                route_context.route_index,
                route_context.next_leg,
            ));
            // Increment the leg index with wrap around
            route_context.next_leg = ctx.world.civs().airships.increment_route_leg(
                route_context.route_index,
                route_context.next_leg,
            );
            // set the approach data for the next leg
            // Needed: transition position for next leg.
            route_context.next_leg_approach = Some(ctx.world.civs().airships.approach_for_route_and_leg(
                route_context.route_index,
                route_context.next_leg,
            ));

            if route_context.current_leg_approach.is_none() ||
                route_context.next_leg_approach.is_none()
            {
                tracing::error!(
                    "Airship pilot {:?} approaches not found for route {} leg {}, stopping pilot_airship loop.",
                    ctx.npc_id,
                    route_context.route_index,
                    route_context.next_leg
                );
                return finish().map(|_, _| ()).boxed();
            }

            // unwrap is safe
            let current_approach = route_context.current_leg_approach.unwrap();
            let next_approach = route_context.next_leg_approach.unwrap();

            let next_leg_cruise_dir = (next_approach.approach_transition_pos.xy() - current_approach.airship_pos.xy()).normalized();
            route_context.next_leg_cruise_checkpoint_pos = (next_approach.approach_transition_pos - next_leg_cruise_dir * CRUISE_CHECKPOINT_DISTANCE).with_z(next_approach.approach_transition_pos.z);
            // The terrain height at the cruise checkpoint may be different from the terrain height at the docking position.
            // The approach_target_pos function will sample the terrain around the cruise checkpoint and return the position with z adjusted
            // to the maximum terrain height around the cruise checkpoint plus the approach cruise height.
            route_context.next_leg_cruise_checkpoint_pos = approach_target_pos(ctx, route_context.next_leg_cruise_checkpoint_pos.xy(), route_context.next_leg_cruise_checkpoint_pos.z, next_approach.height);

            // Track the next pilot distance trend relative to my current approach docking position.
            route_context.next_pilot_dist_to_my_docking_pos_tracker.reset(current_approach.airship_pos.xy());

            // Use a function to determine the speed factor based on the simulation mode. The simulation mode
            // could change at any time as world chunks are loaded or unloaded.
            let speed_factor_fn = |sim_mode: SimulationMode, speed_factor: f32| {
                // The speed factor for simulated airships is always 1.0
                if matches!(sim_mode, SimulationMode::Simulated) {
                    speed_factor.powf(0.3)
                } else {
                    speed_factor
                }
            };

            // ----- Phases of flight - One Leg of the Route -----

            // At this point, the airship is somewhere in the cruise phase of the current route leg.
            // Normally, the airship will be at the cruise checkpoint, fairly close to the docking site.
            // When the server first starts, the airship will be at its spawn point, somewhere along the route
            // and heading for the cruise checkpoint.

            // Fly 2D to Destination Transition Point with frequent radar checks
            fly_airship(
                AirshipFlightPhase::ApproachCruise,
                current_approach.approach_transition_pos,
                50.0,
                speed_factor_fn(ctx.npc.mode, 1.0),
                current_approach.height,
                true,
                None,
                FlightMode::FlyThrough,
                true,
                Duration::from_secs_f32(1.0),
            )
            .then(
                // Fly 3D to directly above the docking position, full PID control
                fly_airship(
                    AirshipFlightPhase::Transition,
                    current_approach.airship_pos + Vec3::unit_z() * current_approach.height,
                    20.0,
                    speed_factor_fn(ctx.npc.mode, 0.4),
                    current_approach.height,
                    true,
                    Some(current_approach.airship_direction),
                    FlightMode::Braking(BrakingMode::Normal),
                    false,
                    Duration::from_secs_f32(2.0),
            ))
            // Descend and Dock
            //    Docking
            //      Drop to 125 blocks above the dock with slightly looser controller settings
            //      then to the docking position with full precision control.
            //      This helps to ensure that the airship docks vertically and avoids collisions
            //      with other airships and the dock. The speed_factor is high to
            //      give a strong response to the PID controller. The speed_factor is reduced
            //      once docked to stop the airship propellers from rotating.
            //      Vary the timeout to get variation in the docking sequence.
            .then(
                just(|ctx: &mut NpcCtx, _| {
                    log_airship_position(ctx, &AirshipFlightPhase::Transition);
                }))

            .then(
                // descend to 125 blocks above the dock
                just(move |ctx, _| {
                    ctx.controller
                        .do_goto_with_height_and_dir(
                            current_approach.airship_pos + Vec3::unit_z() * 125.0,
                            speed_factor_fn(ctx.npc.mode, 0.9),
                            None,
                            Some(current_approach.airship_direction),
                            FlightMode::Braking(BrakingMode::Normal),
                        );
                })
                .repeat()
                .stop_if(timeout(ctx.rng.random_range(12.5..16.0) * (current_approach.height as f64 / Airships::CRUISE_HEIGHTS[0] as f64) * 1.3)))
            .then(
                just(|ctx: &mut NpcCtx, _| {
                    log_airship_position(ctx, &AirshipFlightPhase::Transition);
                }))
            .then(
                // descend to just above the docking position
                just(move |ctx: &mut NpcCtx, _| {
                    ctx.controller
                        .do_goto_with_height_and_dir(
                            current_approach.airship_pos + Vec3::unit_z() * 20.0,
                            speed_factor_fn(ctx.npc.mode, 0.9),
                            None,
                            Some(current_approach.airship_direction),
                            FlightMode::Braking(BrakingMode::Precise),
                        );
                })
                .repeat()
                .stop_if(timeout(ctx.rng.random_range(6.5..8.0))))
            // Announce arrival
            .then(just(|ctx: &mut NpcCtx, _| {
                log_airship_position(ctx, &AirshipFlightPhase::Docked);
                ctx.controller
                    .say(None, Content::localized("npc-speech-pilot-landed"));
            }))

            // Docked - Wait at Dock
            .then(
                now(move |ctx, route_context:&mut AirshipRouteContext| {
                    // The extra time to hold at the dock is a route_context variable.
                    // Adjust the extra time based on the airship's behavior during the previous
                    // flight. If the airship had to hold position, add 30 seconds to the dock time.
                    // If the airship did not hold position, subtract 45 seconds from the dock time
                    // because we want to reverse the extra penalty faster than building it up in case
                    // the airship transitions to simulation mode.
                    // If the airship had to slow down add 15 and if not subtract 20.
                    if route_context.did_hold {
                        route_context.extra_hold_dock_time += 30.0;
                    } else if route_context.extra_hold_dock_time > 45.0 {
                        route_context.extra_hold_dock_time -= 45.0;
                    } else {
                        route_context.extra_hold_dock_time = 0.0;
                    }

                    let docking_time = route_context.extra_hold_dock_time + Airships::docking_duration();
                    #[cfg(debug_assertions)]
                    {
                        if route_context.did_hold || docking_time > Airships::docking_duration() {
                            let docked_site_name = ctx.index.sites.get(current_approach.site_id).name().to_string();
                            debug!("{}, Docked at {}, did_hold:{}, extra_hold_dock_time:{}, docking_time:{}", format!("{:?}", ctx.npc_id), docked_site_name, route_context.did_hold, route_context.extra_hold_dock_time, docking_time);
                        }
                    }
                    route_context.did_hold = false;

                    just(move |ctx, _| {
                        ctx.controller
                        .do_goto_with_height_and_dir(
                            current_approach.airship_pos,
                            speed_factor_fn(ctx.npc.mode, 0.75),
                            None,
                            Some(current_approach.airship_direction),
                            FlightMode::Braking(BrakingMode::Precise),
                        );
                    })
                    .repeat()
                    .stop_if(timeout(ctx.rng.random_range(10.0..16.0)))
                    // While waiting, every now and then announce where the airship is going next.
                    .then(
                        just(move |ctx, _| {
                            // get the name of the site the airship is going to next.
                            // The route_context.current_approach_index has not been switched yet,
                            // so the index is the opposite of the current approach index.
                            let next_site_name = ctx.index.sites.get(next_approach.site_id).name().to_string();
                            ctx.controller.say(
                                None,
                                Content::localized_with_args("npc-speech-pilot-announce_next", [
                                (
                                    "dir",
                                    Direction::from_dir((next_approach.approach_transition_pos - ctx.npc.wpos).xy()).localize_npc(),
                                ),
                                ("dst", Content::Plain(next_site_name.to_string())),
                                ]),
                            );
                        })
                    )
                    .repeat()
                    .stop_if(timeout(docking_time as f64))
                })
            ).then(
                // Announce takeoff
                just(move |ctx, route_context:&mut AirshipRouteContext| {
                    ctx.controller.say(
                    None,
                        Content::localized_with_args("npc-speech-pilot-takeoff", [
                            ("src", Content::Plain(ctx.index.sites.get(current_approach.site_id).name().to_string())),
                            ("dst", Content::Plain(ctx.index.sites.get(next_approach.site_id).name().to_string())),
                        ]),
                    );
                    // This is when the airship target docking position changes to the next approach.
                    // Reset the next pilot distance trend tracker.
                    route_context.next_pilot_dist_to_my_docking_pos_tracker.reset(next_approach.airship_pos.xy());
                    log_airship_position(ctx, &AirshipFlightPhase::Ascent);
                })
            ).then(
                // Take off, full PID control
                fly_airship(
                    AirshipFlightPhase::Ascent,
                    current_approach.airship_pos + Vec3::unit_z() * 100.0,
                    20.0,
                    speed_factor_fn(ctx.npc.mode, 0.2),
                    0.0,
                    false,
                    Some(current_approach.airship_direction),
                    FlightMode::Braking(BrakingMode::Normal),
                    false,
                    Duration::from_secs_f32(120.0),
                )
            ).then(
                // Ascend to Cruise Alt, full PID control
                fly_airship(
                    AirshipFlightPhase::Ascent,
                    current_approach.airship_pos + Vec3::unit_z() * current_approach.height,
                    20.0,
                    speed_factor_fn(ctx.npc.mode, 0.4),
                    0.0,
                    false,
                    Some(Dir::from_unnormalized((next_approach.approach_transition_pos - ctx.npc.wpos).xy().with_z(0.0)).unwrap_or_default()),
                    FlightMode::Braking(BrakingMode::Normal),
                    false,
                    Duration::from_secs_f32(120.0),
                )
            ).then(
                // Fly 2D to Destination Cruise Checkpoint with infrequent radar checks
                fly_airship(
                    AirshipFlightPhase::DepartureCruise,
                    route_context.next_leg_cruise_checkpoint_pos,
                    50.0,
                    speed_factor_fn(ctx.npc.mode, 1.0),
                    next_approach.height,
                    true,
                    None,
                    FlightMode::FlyThrough,
                    true,
                    Duration::from_secs_f32(5.0),
                )
            )
            .map(|_, _| ()).boxed()
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

#[cfg(feature = "airship_log")]
/// Get access to the global airship logger and log an airship position.
fn log_airship_position(ctx: &NpcCtx, phase: &AirshipFlightPhase) {
    if let Ok(mut logger) = airship_logger() {
        logger.log_position(
            ctx.npc_id,
            ctx.index.seed,
            phase,
            ctx.time.0,
            ctx.npc.wpos,
            matches!(ctx.npc.mode, SimulationMode::Loaded),
        );
    } else {
        warn!("Failed to log airship position for {:?}", ctx.npc_id);
    }
}

#[cfg(not(feature = "airship_log"))]
/// When the logging feature is not enabled, this should become a no-op.
fn log_airship_position(_: &NpcCtx, _: &AirshipFlightPhase) {}

#[cfg(test)]
mod tests {
    use super::{DistanceTrend, DistanceTrendTracker, MovingAverage};
    use vek::*;

    #[test]
    fn moving_average_test() {
        let mut ma: MovingAverage<f32, 5, 3> = MovingAverage::default();
        ma.add(1.0);
        ma.add(2.0);
        ma.add(3.0);
        ma.add(4.0);
        ma.add(5.0);
        assert_eq!(ma.average().unwrap(), 3.0);

        ma.add(6.0); // This will remove the first value (1.0)
        assert_eq!(ma.average().unwrap(), 4.0);

        ma.add(7.0); // This will remove the second value (2.0)
        assert_eq!(ma.average().unwrap(), 5.0);

        ma.add(8.0); // This will remove the third value (3.0)
        assert_eq!(ma.average().unwrap(), 6.0);

        ma.add(9.0); // This will remove the fourth value (4.0)
        assert_eq!(ma.average().unwrap(), 7.0);

        ma.add(10.0); // This will remove the fifth value (5.0)
        assert_eq!(ma.average().unwrap(), 8.0);

        let mut ma2: MovingAverage<i64, 5, 3> = MovingAverage::default();
        ma2.add((1000.0f32 / 1000.0) as i64);
        ma2.add((2000.0f32 / 1000.0) as i64);
        ma2.add((3000.0f32 / 1000.0) as i64);
        ma2.add((4000.0f32 / 1000.0) as i64);
        ma2.add((5000.0f32 / 1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 3);

        ma2.add((6000.0f32 / 1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 4);

        ma2.add((7000.0f32 / 1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 5);

        ma2.add((8000.0f32 / 1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 6);

        ma2.add((9000.0f32 / 1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 7);

        ma2.add((10000.0f32 / 1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 8);

        let mut ma3: MovingAverage<i64, 5, 3> = MovingAverage::default();
        ma3.add((20.99467f32 * 10000.0) as i64);
        ma3.add((20.987871f32 * 10000.0) as i64);
        ma3.add((20.69861f32 * 10000.0) as i64);
        ma3.add((20.268217f32 * 10000.0) as i64);
        ma3.add((20.230164f32 * 10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.6358).abs() < 0.0001);

        ma3.add((20.48151f32 * 10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.5332).abs() < 0.0001);

        ma3.add((20.568598f32 * 10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.4493).abs() < 0.0001);

        ma3.add((20.909971f32 * 10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.4916).abs() < 0.0001);

        ma3.add((21.014437f32 * 10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.6408).abs() < 0.0001);

        ma3.add((20.62308f32 * 10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.7194).abs() < 0.0001);
    }

    #[test]
    fn distance_trend_tracker_test() {
        let mut tracker: DistanceTrendTracker<0> = DistanceTrendTracker::default();
        tracker.reset(Vec2::new(0.0, 0.0));
        assert!(tracker.update(Vec2::new(1.0, 0.0), 1.0).is_none());
        assert!(tracker.update(Vec2::new(2.0, 0.0), 2.0).is_none());
        assert!(matches!(
            tracker.update(Vec2::new(3.0, 0.0), 3.0).unwrap(),
            DistanceTrend::Away
        ));
        assert!(matches!(
            tracker.update(Vec2::new(4.0, 0.0), 4.0).unwrap(),
            DistanceTrend::Away
        ));
        assert!(matches!(
            tracker.update(Vec2::new(5.0, 0.0), 5.0).unwrap(),
            DistanceTrend::Away
        ));

        tracker.reset(Vec2::new(0.0, 0.0));
        assert!(tracker.update(Vec2::new(5.0, 0.0), 1.0).is_none());
        assert!(tracker.update(Vec2::new(4.0, 0.0), 2.0).is_none());
        assert!(matches!(
            tracker.update(Vec2::new(3.0, 0.0), 3.0).unwrap(),
            DistanceTrend::Towards
        ));
        assert!(matches!(
            tracker.update(Vec2::new(2.0, 0.0), 4.0).unwrap(),
            DistanceTrend::Towards
        ));
        assert!(matches!(
            tracker.update(Vec2::new(1.0, 0.0), 5.0).unwrap(),
            DistanceTrend::Towards
        ));
        assert!(matches!(
            tracker.update(Vec2::new(0.0, 0.0), 6.0).unwrap(),
            DistanceTrend::Towards
        ));
        assert!(matches!(
            tracker.update(Vec2::new(-1.0, 0.0), 7.0).unwrap(),
            DistanceTrend::Towards
        ));
        assert!(matches!(
            tracker.update(Vec2::new(-2.0, 0.0), 8.0).unwrap(),
            DistanceTrend::Away
        ));
        assert!(matches!(
            tracker.update(Vec2::new(-3.0, 0.0), 9.0).unwrap(),
            DistanceTrend::Away
        ));

        let mut tracker2: DistanceTrendTracker<5> = DistanceTrendTracker::default();
        assert!(tracker2.update(Vec2::new(100.0, 100.0), 10.0).is_none());
        assert!(tracker2.update(Vec2::new(100.0, 200.0), 20.0).is_none());
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 300.0), 30.0).unwrap(),
            DistanceTrend::Away
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 400.0), 40.0).unwrap(),
            DistanceTrend::Away
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 500.0), 50.0).unwrap(),
            DistanceTrend::Away
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 490.0), 60.0).unwrap(),
            DistanceTrend::Away
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 505.0), 70.0).unwrap(),
            DistanceTrend::Neutral
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 500.0), 80.0).unwrap(),
            DistanceTrend::Neutral
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 495.0), 90.0).unwrap(),
            DistanceTrend::Neutral
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 500.0), 100.0).unwrap(),
            DistanceTrend::Neutral
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 495.0), 110.0).unwrap(),
            DistanceTrend::Neutral
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 500.0), 120.0).unwrap(),
            DistanceTrend::Neutral
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 505.0), 130.0).unwrap(),
            DistanceTrend::Neutral
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 500.0), 140.0).unwrap(),
            DistanceTrend::Neutral
        ));
        assert!(matches!(
            tracker2.update(Vec2::new(100.0, 505.0), 150.0).unwrap(),
            DistanceTrend::Neutral
        ));
    }
}
