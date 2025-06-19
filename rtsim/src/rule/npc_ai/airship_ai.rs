use crate::{
    ai::{finish, just, now, predicate::timeout, Action, NpcCtx, State},
    data::{npc::SimulationMode, Npc},
};
use common::{
    comp::{
        Content,
        agent::{BrakingMode, FlightMode},
        compass::Direction,
    },
    rtsim::NpcId,
    store::Id,
    util::Dir,
};
use num_traits::cast::FromPrimitive;
use rand::prelude::*;
use rmp_serde::config;
use std::{
    cmp::Ordering, collections::VecDeque, sync::Mutex, thread::current, time::Duration
};
use vek::*;
use world::{
    civ::airship_travel::{AirshipDockingApproach, Airships},
    site::Site,
    util::{DHashMap, CARDINALS},
};

#[cfg(debug_assertions)] use tracing::debug;

const CLOSE_TO_DOCKING_SITE_DISTANCE: f32 = 225.0f32 * 225.0f32;
const SPEED_ADJUST_CLOSE_AIRSHIP_DISTANCE: f32 = 2500.0f32 * 2500.0f32;
const SOMEWHAT_CLOSE_AIRSHIP_DISTANCE: f32 = 1000.0f32 * 1000.0f32;
const PASSING_CLOSE_AIRSHIP_DISTANCE: f32 = 700.0f32 * 700.0f32;
const VERY_CLOSE_AIRSHIP_DISTANCE: f32 = 400.0f32 * 400.0f32;
const SLOW_DOWN_SPEED_MULTIPLIER: f32 = 0.3;
const CLOSE_AIRSHIP_SPEED_FACTOR: f32 = 0.9;
const CRUISE_CHECKPOINT_DISTANCE: f32 = 800.0;
const NEXT_PILOT_CRUISE_SPEED_TOLERANCE: f32 = 2.0;
const MOVING_AVERAGE_SCALE_FACTOR: f64 = 10000.0;
const NEXT_PILOT_MOVING_VELOCITY_AVERAGE_CAPACITY: usize = 5;
const NEXT_PILOT_MOVING_VELOCITY_AVERAGE_MIN_SIZE: usize = 3;
const NEXT_PILOT_MOVING_DIST_AVERAGE_CAPACITY: usize = 3;
const NEXT_PILOT_MOVING_DIST_AVERAGE_MIN_SIZE: usize = 2;
const NEXT_PILOT_MOVING_DIST_TRACKER_THRESHOLD: usize = 10;

/// Airships can slow down or hold position to avoid collisions with other
/// airships.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
enum AirshipAvoidanceMode {
    #[default]
    None,
    Hold(Vec3<f32>, Dir),
    SlowDown,
    Stuck(Vec3<f32>),
}

// Context data for piloting the airship.
// This is the context data for the pilot_airship action.
#[derive(Debug, Clone)]
struct AirshipRouteContext {
    // The route index (index into the outer vec of airships.routes)
    route_index: usize,
    // The expected airship speed when simulated.
    simulated_airship_speed: f32,
    // The next route leg index.
    next_leg: usize,
    // The NpcId of the captain ahead of this one.
    next_pilot_id: NpcId,
    // The current approach.
    current_leg_approach: Option<AirshipDockingApproach>,
    // The next approach.
    next_leg_approach: Option<AirshipDockingApproach>,
    // A point short of the approach transition point where the frequency of avoidance checks increases.
    next_leg_cruise_checkpoint_pos: Vec3<f32>,
    
    // Avoidance Data

    // For tracking the airship's position history to determine if the airship is stuck.
    my_stuck_tracker: Option<StuckAirshipTracker>,
    // For tracking airship velocity towards the approach transition point.
    my_rate_tracker: Option<RateTracker>,
    // For tracking the next pilot's velocity towards the approach transition point.
    next_pilot_rate_tracker_: Option<RateTracker>,
    // Timer for checking the airship trackers.
    avoidance_timer: Duration,
    // Timer used when holding, either on approach or at the dock.
    hold_timer: f32,
    // Whether the initial hold message has been sent to the client.
    hold_announced: bool, 
    // The original speed factor passed to the fly_airship action.
    speed_factor: f32,
    // The moving average of the next pilot's velocity during cruise phase.
    next_pilot_average_velocity: MovingAverage<i64, NEXT_PILOT_MOVING_VELOCITY_AVERAGE_CAPACITY, NEXT_PILOT_MOVING_VELOCITY_AVERAGE_MIN_SIZE>,
    // The moving average of the next pilot's distance from my current docking position target pos.
    next_pilot_dist_to_my_docking_pos_tracker: DistanceTrendTracker<NEXT_PILOT_MOVING_DIST_TRACKER_THRESHOLD>,
    // The current avoidance mode for the airship.
    avoid_mode: AirshipAvoidanceMode,
    // Whether the airship had to hold during the last flight.
    did_hold: bool,
    // number of times the airship had to slow down during the last flight.
    slow_count: u32,
    // The extra docking time due to holding.
    extra_hold_dock_time: f32,
    // The extra docking time due to holding.
    extra_slowdown_dock_time: f32,
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
            my_stuck_tracker: None,
            my_rate_tracker: None,
            next_pilot_rate_tracker_: None,
            avoidance_timer: Duration::default(),
            hold_timer: 0.0,
            hold_announced: false,
            speed_factor: 1.0,
            next_pilot_average_velocity: MovingAverage::default(),
            next_pilot_dist_to_my_docking_pos_tracker: DistanceTrendTracker::default(),
            avoid_mode: AirshipAvoidanceMode::default(),
            did_hold: false,
            slow_count: 0,
            extra_hold_dock_time: 0.0,
            extra_slowdown_dock_time: 0.0,
        }
    }
}

/// Tracks the airship position history.
/// Used for determining if an airship is stuck.
#[derive(Debug, Default, Clone)]
struct StuckAirshipTracker {
    // The airship's position history. Used for determining if the airship is stuck in one place.
    pos_history: Vec<Vec3<f32>>,
    // The route to follow for backing out of a stuck position.
    backout_route: Vec<Vec3<f32>>,
}

impl StuckAirshipTracker {
    const BACKOUT_TARGET_DIST: f32 = 50.0;
    const MAX_POS_HISTORY_SIZE: usize = 5;

    // Add a new position to the position history, maintaining a fixed size.
    fn add_position(&mut self, new_pos: Vec3<f32>) {
        if self.pos_history.len() >= StuckAirshipTracker::MAX_POS_HISTORY_SIZE {
            self.pos_history.remove(0);
        }
        self.pos_history.push(new_pos);
    }

    // Check if the airship is stuck in one place.
    fn is_stuck(&mut self, ctx: &mut NpcCtx, target_pos: &Vec2<f32>) -> bool {
        // The position history must be full to determine if the airship is stuck.
        if self.pos_history.len() == StuckAirshipTracker::MAX_POS_HISTORY_SIZE
            && self.backout_route.is_empty()
        {
            if let Some(last_pos) = self.pos_history.last() {
                // If all the positions in the history are within 10 of the last position,
                if self
                    .pos_history
                    .iter()
                    .all(|pos| pos.distance_squared(*last_pos) < 10.0)
                {
                    // Airship is stuck on some obstacle.
                    // If current position is near the ground, backout while gaining altitude
                    // else backout at the current height and then ascend to clear the obstacle.
                    // This function is only called in cruise phase, so the direction to backout
                    // will be the opposite of the direction from the current pos to the approach
                    // initial pos.
                    if let Some(backout_dir) = (ctx.npc.wpos.xy() - target_pos)
                        .with_z(0.0)
                        .try_normalized()
                    {
                        let ground = ctx
                            .world
                            .sim()
                            .get_surface_alt_approx(last_pos.xy().map(|e| e as i32));
                        let mut backout_pos = ctx.npc.wpos + backout_dir * 100.0;
                        if (ctx.npc.wpos.z - ground).abs() < 10.0 {
                            backout_pos.z += 50.0;
                        }
                        self.backout_route =
                            vec![backout_pos, backout_pos + Vec3::unit_z() * 200.0];
                        // The airship is stuck.
                        #[cfg(debug_assertions)]
                        debug!(
                            "Airship {} Stuck! at {} {} {}, backout_dir:{:?}, backout_pos:{:?}",
                            format!("{:?}", ctx.npc_id),
                            ctx.npc.wpos.x, ctx.npc.wpos.y, ctx.npc.wpos.z,
                            backout_dir,
                            backout_pos
                        );
                    }
                }
            }
        }
        !self.backout_route.is_empty()
    }

    fn next_backout_pos(&mut self, ctx: &mut NpcCtx) -> Option<Vec3<f32>> {
        if let Some(pos) = self.backout_route.first().cloned() {
            if ctx.npc.wpos.distance_squared(pos) < StuckAirshipTracker::BACKOUT_TARGET_DIST.powi(2) {
                self.backout_route.remove(0);
            }
            Some(pos)
        } else {
            self.pos_history.clear();
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
struct RateTracker {
    // ref_pos: Vec2<f32>,
    last_pos: Option<Vec2<f32>>,
    last_time: f32,
}

impl RateTracker {
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
enum DistanceTrend {
    Towards,
    Away,
    Neutral,
}

/// Tracks airship distance trend from a fixed position. Used for airship
/// traffic control. The parameter C is the threshold for determining if the
/// trend is stable (i.e., not increasing or decreasing too much).
#[derive(Debug, Default, Clone)]
struct DistanceTrendTracker<const C: usize> {
    fixed_pos: Vec2<f32>,
    avg_rate: MovingAverage<f64, NEXT_PILOT_MOVING_DIST_AVERAGE_CAPACITY, NEXT_PILOT_MOVING_DIST_AVERAGE_MIN_SIZE>,
    prev_dist: Option<f32>,
    prev_time: f64,
}

impl<const C: usize> DistanceTrendTracker<C> {
    fn update(&mut self, pos: Vec2<f32>, time: f64) -> Option<DistanceTrend> {
        let current_dist = pos.distance(self.fixed_pos);
        if let Some(prev) = self.prev_dist {
            // rate is blocks per second
            // Greater than 0 means the distance is increasing (going away from the target pos).
            // Near zero means the airship is stationary or moving perpendicular to the target pos.
            let rate = (current_dist - prev) as f64 / (time - self.prev_time);
            self.prev_dist = Some(current_dist);
            self.prev_time = time;

            self.avg_rate.add(rate);
            if let Some(avg) = self.avg_rate.average() {
                if avg > C as f64 {
                    //$$$ debug!("DistanceTrendTracker: Away avg: {}", avg);
                    Some(DistanceTrend::Away)
                } else if avg < -(C as f64) {
                    //$$$ debug!("DistanceTrendTracker: Towards avg: {}", avg);
                    Some(DistanceTrend::Towards)
                } else {
                    //$$$ debug!("DistanceTrendTracker: Neutral avg: {}", avg);
                    Some(DistanceTrend::Neutral)
                }
            } else {
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
    T: Default + FromPrimitive + std::ops::AddAssign + std::ops::Sub<Output = T> + std::ops::Div<Output = T> + Copy,
{
    values: VecDeque<T>,
    sum: T,
}

impl<T, const S: usize, const N: usize> MovingAverage<T, S, N>
where
    T: Default + FromPrimitive + std::ops::AddAssign + std::ops::Sub<Output = T> + std::ops::Div<Output = T> + Copy,
{
    fn add(&mut self, value: T) {
        if self.values.len() == S {
            if let Some(old_value) = self.values.pop_front() {
                self.sum = self.sum - old_value;
            }
        }
        self.values.push_back(value);
        self.sum += value;
    }

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
    T: Default + FromPrimitive + std::ops::AddAssign + std::ops::Sub<Output = T> + std::ops::Div<Output = T> + Copy,
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
enum AirshipFlightPhase {
    Ascent,
    DepartureCruise,
    ApproachCruise,
    Transition,
    #[default]
    Docked,
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
    now(move |ctx, airship_context: &mut AirshipRouteContext| {
        // let debug_npc = {
        //     let npc_id_str = format!("{:?}", ctx.npc_id);
        //     npc_id_str == "NpcId(1920v3)"
        // };
        // if debug_npc {
        //     if phase == AirshipFlightPhase::Cruise {
        //         debug!("My pilot: cruise phase, reset speed factor")
        //     }
        // }
        airship_context.avoidance_timer = radar_interval;
        airship_context.avoid_mode = AirshipAvoidanceMode::None;
        match phase {
            AirshipFlightPhase::DepartureCruise => {
                // Reset my pos tracker
                airship_context.my_stuck_tracker = Some(StuckAirshipTracker::default());
                // Reset the speed factor
                airship_context.speed_factor = initial_speed_factor;
            }
            AirshipFlightPhase::ApproachCruise => {
                // Leave speed factor as is.
            }
            _ => {
                // Reset the speed factor
                airship_context.speed_factor = initial_speed_factor;
            }
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

            let debug_npc = {
                let npc_id_str = format!("{:?}", ctx.npc_id);
                npc_id_str == "NpcId(1920v3)____"
            };

            // It's safe to unwrap here, this function is not called if either airship_context.current_leg_approach
            // or airship_context.next_leg_approach are None. The pos_tracker_target_loc is for determining the reverse
            // direction for 'unsticking' the airship if it gets stuck in one place; In DepartureCruise phase, it is the
            // next_leg_cruise_checkpoint_pos and in ApproachCruise phase, it is the transition pos of the current destination.
            let (current_approach, pos_tracker_target_loc) = 
                match phase {
                    AirshipFlightPhase::DepartureCruise => (&airship_context.next_leg_approach.unwrap(), airship_context.next_leg_cruise_checkpoint_pos.xy()),
                    _ => (&airship_context.current_leg_approach.unwrap(), airship_context.current_leg_approach.unwrap().approach_transition_pos.xy()),
                };

            // decrement the avoidance timer
            let remaining = airship_context
                .avoidance_timer
                .checked_sub(Duration::from_secs_f32(ctx.dt));
            // If it's time for avoidance checks (the timer is always counting down)..
            // The collision avoidance checks are not done every tick (no dt required), only
            // every 1-5 seconds depending on flight phase.
            if remaining.is_none() {
                // reset the timer
                airship_context.avoidance_timer = radar_interval;
                // If actually doing avoidance checks..
                if with_collision_avoidance {

                    let mypos = ctx.npc.wpos;

                    if matches!(phase, AirshipFlightPhase::DepartureCruise | AirshipFlightPhase::ApproachCruise) {
                        // The last phase of the flight loop is DepartureCruise, heading to the
                        // next_leg_cruise_checkpoint_pos. The first phase of the next flight loop is ApproachCruise,
                        // heading to the current_leg_approach approach_transition_pos. Position and Rate tracking
                        // are done in both cruise phases. The position tracker is used to determine if the airship
                        // is stuck in one place. Rate tracking is used to adjust the velocity of Loaded airships
                        // relative to the pilot immediately ahead on the route.

                        // Check if the airship is stuck (not moving) but only if it's not holding position.
                        if !matches!(airship_context.avoid_mode, AirshipAvoidanceMode::Hold(..))
                            && let Some(stuck_tracker) = &mut airship_context.my_stuck_tracker
                        {
                            stuck_tracker.add_position(mypos);
                            // Check if the airship is stuck in one place.
                            if stuck_tracker.is_stuck(ctx, &pos_tracker_target_loc)
                                && let Some(backout_pos) = stuck_tracker.next_backout_pos(ctx)
                            {
                                airship_context.avoid_mode =
                                    AirshipAvoidanceMode::Stuck(backout_pos);
                            } else {
                                // If the mode was stuck, but the airship is no longer stuck, clear
                                // the mode.
                                if matches!(
                                    airship_context.avoid_mode,
                                    AirshipAvoidanceMode::Stuck(..)
                                ) {
                                    airship_context.avoid_mode = AirshipAvoidanceMode::None;
                                }
                            }
                        }
                        
                        if let Some(my_rate_tracker) = &mut airship_context.my_rate_tracker
                        {
                            let my_rate = my_rate_tracker.update(
                                mypos.xy(),
                                ctx.time.0 as f32,
                            );                            

                            if let Some(next_pilot_rate_tracker) = &mut airship_context.next_pilot_rate_tracker_
                                && let Some(next_pilot) = ctx.state.data().npcs.get(airship_context.next_pilot_id)
                            {
                                let next_pilot_rate = next_pilot_rate_tracker.update(
                                    next_pilot.wpos.xy(),
                                    ctx.time.0 as f32,
                                );
                                let next_pilot_dist_trend = airship_context
                                    .next_pilot_dist_to_my_docking_pos_tracker.update(next_pilot.wpos.xy(), ctx.time.0)
                                    .unwrap_or(DistanceTrend::Away);

                                // Track the moving average of the velocity of the next pilot ahead of my pilot but only
                                // if the velocity is greater than the expected simulated cruise speed minus a small tolerance.
                                // (i.e., when it can be expected that the next pilot is in the cruise phase).
                                if next_pilot_rate > airship_context.simulated_airship_speed - NEXT_PILOT_CRUISE_SPEED_TOLERANCE {
                                    // Scale up the velocity so that the moving average can be done as an integer.
                                    airship_context.next_pilot_average_velocity.add((next_pilot_rate as f64 * MOVING_AVERAGE_SCALE_FACTOR) as i64);
                                }
                            
                                // If not currently avoiding the airship ahead
                                if airship_context.avoid_mode == AirshipAvoidanceMode::None && my_rate > 0.0 {                                    
                                    // If 'my' pilot is Loaded (not Simulated), and there's enough data to
                                    // estimate the cruising speed of the pilot ahead, and my pilot is relatively close
                                    // to the pilot ahead, then adjust my speed factor match that cruising speed.
                                    // 'relatively close' cannot be too far away because the pilot ahead could be close
                                    // but moving in the opposite direction if the route legs go in somewhat opposite
                                    // directions.                                                
                                    let next_pilot_avg_velocity_i64 = airship_context.next_pilot_average_velocity.average().unwrap_or(0) ;
                                    if next_pilot_avg_velocity_i64 > 0
                                        && ctx.npc.wpos.xy().distance_squared(next_pilot.wpos.xy()) < SPEED_ADJUST_CLOSE_AIRSHIP_DISTANCE 
                                        && next_pilot_dist_trend == DistanceTrend::Towards
                                    {
                                        debug!("{:?} pos: {} {}, docking pos: {} {}, tracker pos: {} {}, next pilot {:?} pos: {} {}, dist: {}, trend: {:?}",
                                            ctx.npc_id,
                                            mypos.x as i32, mypos.y as i32,
                                            airship_context.current_leg_approach.unwrap().airship_pos.x as i32, airship_context.current_leg_approach.unwrap().airship_pos.y as i32,
                                            airship_context.next_pilot_dist_to_my_docking_pos_tracker.fixed_pos.x as i32, airship_context.next_pilot_dist_to_my_docking_pos_tracker.fixed_pos.y as i32,
                                            airship_context.next_pilot_id,
                                            next_pilot.wpos.x as i32, next_pilot.wpos.y as i32,
                                            next_pilot.wpos.xy().distance(airship_context.next_pilot_dist_to_my_docking_pos_tracker.fixed_pos),
                                            next_pilot_dist_trend
                                        );
                                        // scale the next pilot's average velocity back down to a float
                                        let next_pilot_avg_velocity = (next_pilot_avg_velocity_i64 as f64 / MOVING_AVERAGE_SCALE_FACTOR) as f32;
                                        // My pilot is getting close to the next pilot ahead,
                                        // adjust my speed factor based on a percentage of the next pilot's average velocity.
                                        let target_velocity = next_pilot_avg_velocity * CLOSE_AIRSHIP_SPEED_FACTOR;
                                        // TODO: fix this so it's not hardcoded based on the airship physics.
                                        // This formula was derived emperically by testing the airship velocity
                                        // at different speed_factors, which is based on the mass of the airship
                                        // and the thrust from common::states:utils::fly_thrust(). The current coefficients
                                        // are based on an estimated speed of 25 blocks per second.
                                        let new_speed_factor = -2.0173430f32 + 
                                            0.3885298f32 * target_velocity +
                                            -0.0221485f32 * target_velocity.powi(2) +
                                            0.0004694f32 * target_velocity.powi(3);
                                        if new_speed_factor > 0.0 {
                                            airship_context.speed_factor = new_speed_factor.min(1.05);
                                            debug!(
                                                "Pilot {:?}: Adjusting speed factor to {}, next pilot avg velocity: {}, * speed_factor: {}",
                                                ctx.npc_id,
                                                airship_context.speed_factor.min(1.05),
                                                next_pilot_avg_velocity,
                                                target_velocity,
                                            );
                                            if debug_npc {
                                                debug!(
                                                    "My pilot: Adjusting speed factor to {}, next pilot avg velocity: {}",
                                                    airship_context.speed_factor,
                                                    next_pilot_avg_velocity
                                                );
                                            }
                                        } else {
                                            // If the new speed factor is negative, don't change it.
                                            if debug_npc {
                                                debug!(
                                                    "My pilot: Not adjusting speed factor, new speed factor is negative: {}",
                                                    new_speed_factor
                                                );
                                            }
                                        }
                                    } else {
                                        // Resume normal speed factor.
                                        if (airship_context.speed_factor - initial_speed_factor).abs() > f32::EPSILON {
                                            // If the speed factor was adjusted, reset it to the initial value.
                                            debug!(
                                                "Pilot {:?}: Resetting speed factor to initial value: {}",
                                                ctx.npc_id,
                                                initial_speed_factor
                                            );
                                            airship_context.speed_factor = initial_speed_factor;
                                        }
                                    }
                                    if debug_npc {
                                        debug!("My pilot velocity/next pilot velocity {}, {}, speed_factor: {}", my_rate, next_pilot_rate, airship_context.speed_factor);
                                    }
                                } else {
                                    if debug_npc {
                                        debug!("My pilot velocity/next pilot velocity {}, {}, speed_factor: {}", my_rate, next_pilot_rate, airship_context.speed_factor);
                                    }
                                }
                            } else {
                                if debug_npc {
                                    debug!("My pilot velocity: {}", my_rate);
                                }
                            }
                        }
                    }

                    // Continue with the avoidance logic only if the airship is not stuck, and there is a pilot ahead.
                    if !matches!(airship_context.avoid_mode, AirshipAvoidanceMode::Stuck(..))
                        && let Some(next_pilot) = ctx.state.data().npcs.get(airship_context.next_pilot_id)
                    {                            
                        // Get the avoidance mode relative to the airship ahead on the route.
                        let avoidance = {
                            // if let Some(pilot) = ctx.state.data().npcs.get(airship_context.next_pilot) {
                            let pilot_wpos = next_pilot.wpos;
                            /*
                                The basic logic:
                                If the other airship is near my docking position
                                    If I'm really close to the other airship
                                        hold position
                                    else if I'm close to the other airship
                                        slow down
                                    else
                                        do nothing
                                    end
                                else if I'm close to the other airship
                                    slow down
                                else
                                    do nothing
                                end

                                d1 = pilot ahead distance from the current approach docking position
                                d2 = distance between my position and the position of the pilot ahead

                                If d1 < CLOSE_TO_DOCKING_SITE_DISTANCE
                                    The other airship is near the dock
                                    If d2 < VERY_CLOSE_AIRSHIP_DISTANCE
                                        hold position
                                    else if d2 < SOMEWHAT_CLOSE_AIRSHIP_DISTANCE
                                        slow down
                                    else
                                        do nothing
                                    end
                                else if d2 < PASSING_CLOSE_AIRSHIP_DISTANCE
                                    slow down
                                else
                                    do nothing
                                end                                            
                            */
                            let d1 = if phase == AirshipFlightPhase::DepartureCruise {
                                pilot_wpos.xy().distance_squared(airship_context.next_leg_cruise_checkpoint_pos.xy())
                            } else {
                                pilot_wpos.xy().distance_squared(current_approach.approach_transition_pos.xy())
                            };
                            let d2 = mypos.xy().distance_squared(pilot_wpos.xy());
                            // if debug_npc {
                            //     debug!("My Pilot d1:{}, d2:{}", d1.sqrt(), d2.sqrt());
                            // }
                            // Once holding, move the hold criteria outwards so that the airship
                            // doesn't stop holding mode due to osillations in the hold position.
                            let close_dist_adjustment = if matches!(airship_context.avoid_mode, AirshipAvoidanceMode::Hold(..)) {
                                50.0f32.powi(2)
                            } else {
                                0.0
                            };

                            if d1 < CLOSE_TO_DOCKING_SITE_DISTANCE {
                                if d2 < VERY_CLOSE_AIRSHIP_DISTANCE + close_dist_adjustment {
                                    AirshipAvoidanceMode::Hold(
                                        mypos,
                                        Dir::from_unnormalized((current_approach.dock_center - mypos.xy()).with_z(0.0))
                                            .unwrap_or_default(),
                                    )
                                } else if d2 < SOMEWHAT_CLOSE_AIRSHIP_DISTANCE {
                                    AirshipAvoidanceMode::SlowDown
                                } else {
                                    AirshipAvoidanceMode::None
                                }
                            } else if d2 < PASSING_CLOSE_AIRSHIP_DISTANCE {
                                AirshipAvoidanceMode::SlowDown
                            } else {
                                AirshipAvoidanceMode::None
                            }
                        };
                        
                        if matches!(avoidance, AirshipAvoidanceMode::Hold(..)){
                            // Don't reenter hold mode
                            if !matches!(
                                airship_context.avoid_mode,
                                AirshipAvoidanceMode::Hold(..)
                            ) {
                                airship_context.did_hold = true;
                                airship_context.avoid_mode = avoidance;
                                airship_context.hold_timer = ctx.rng.gen_range(4.0..7.0);
                                airship_context.hold_announced = false;
                                debug!(
                                    "pilot {:?}: Hold position at {:?}, hold timer: {}",
                                    ctx.npc_id,
                                    mypos,
                                    airship_context.hold_timer
                                );
                                if debug_npc {
                                    debug!(
                                        "My pilot: Hold position at {:?}, hold timer: {}",
                                        mypos,
                                        airship_context.hold_timer
                                    );
                                }
                            }
                        } else if matches!(avoidance, AirshipAvoidanceMode::SlowDown) {
                            // Don't reenter slow down mode
                            if !matches!(
                                airship_context.avoid_mode,
                                AirshipAvoidanceMode::SlowDown
                            ) {
                                airship_context.slow_count += 1;
                                airship_context.avoid_mode = AirshipAvoidanceMode::SlowDown;
                                airship_context.speed_factor = initial_speed_factor * SLOW_DOWN_SPEED_MULTIPLIER;
                                if debug_npc {
                                    debug!(
                                        "My pilot: Slow down mode, count: {}, speed factor: {}",
                                        airship_context.slow_count,
                                        airship_context.speed_factor
                                    );
                                }
                            }
                        } else {
                            // Clear the avoidance mode if it was set to hold or slow down.
                            if debug_npc && !matches!(
                                airship_context.avoid_mode,
                                AirshipAvoidanceMode::None
                            ) {
                                debug!(
                                    "My pilot: No avoidance mode, clearing from {:?}",
                                    airship_context.avoid_mode
                                );
                            }
                            airship_context.avoid_mode = AirshipAvoidanceMode::None;
                        }
                    }
                    
                } else {
                    airship_context.avoid_mode = AirshipAvoidanceMode::None;
                    airship_context.speed_factor = initial_speed_factor;
                }
            } else {
                airship_context.avoidance_timer = remaining.unwrap_or(radar_interval);
            }

            // Handle moving the airship based on avoidance mode.
            if let AirshipAvoidanceMode::Stuck(unstick_target) = airship_context.avoid_mode {
                // Unstick the airship
                ctx.controller.do_goto_with_height_and_dir(
                    unstick_target,
                    1.5,
                    None,
                    None,
                    FlightMode::Braking(BrakingMode::Normal),
                );
            } else if let AirshipAvoidanceMode::Hold(hold_pos, hold_dir) =
                airship_context.avoid_mode
            {
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
                    airship_context.hold_timer = ctx.rng.gen_range(10.0..20.0);
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
            } else {
                // Not holding or stuck.
                // Use terrain height offset if specified.
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
                route_context.simulated_airship_speed =
                    ctx.state.data().npcs.mounts.get_mount_link(ctx.npc_id)
                        .map(|mount_link|
                            ctx.state.data().npcs.get(mount_link.mount)
                                .map(|airship| {
                                    airship.body.max_speed_approx()
                                })
                                .unwrap_or_default()
                        ).or_else(|| {
                            // If the mount link is not found, use a default speed.
                            Some(0.0)
                        })
                        .unwrap_or(0.0);
                route_context.my_rate_tracker = Some(RateTracker::default());
                route_context.next_pilot_rate_tracker_ = Some(RateTracker::default());

                if cfg!(debug_assertions) {
                    let current_approach = ctx.world.civs().airships.approach_for_route_and_leg(
                        route_context.route_index,
                        route_context.next_leg,
                    );
                    // let next_leg_index = ctx.world.civs().airships.increment_route_leg(
                    //         route_context.route_index,
                    //         route_context.next_leg,
                    //     );
                    // let next_approach = ctx.world.civs().airships.approach_for_route_and_leg(
                    //     route_context.route_index,
                    //     next_leg_index,
                    //     ).unwrap();
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

            // Track the next pilot distance trend from my current approach docking position.
            route_context.next_pilot_dist_to_my_docking_pos_tracker.reset(current_approach.airship_pos.xy());

            // Use a function to determine the speed factor based on the simulation mode. The simulation mode
            // could change at any time as world chunks are loaded or unloaded.
            let speed_factor_fn = |sim_mode: SimulationMode, speed_factor: f32| {
                // The speed factor for simulated airships is always 1.0
                if matches!(sim_mode, SimulationMode::Simulated) {
                    1.0
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
                .stop_if(timeout(ctx.rng.gen_range(13.0..17.0) * (current_approach.height as f64 / Airships::CRUISE_HEIGHTS[0] as f64) * 1.3)))
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
                .stop_if(timeout(ctx.rng.gen_range(6.0..8.0))))
            // Announce arrival
            .then(just(|ctx: &mut NpcCtx, _| {
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
                    if route_context.slow_count > 0 {
                        route_context.extra_slowdown_dock_time += 15.0;
                    } else if route_context.extra_slowdown_dock_time > 20.0 {
                        route_context.extra_slowdown_dock_time -= 20.0;
                    } else {
                        route_context.extra_slowdown_dock_time = 0.0;
                    }

                    let docking_time = route_context.extra_hold_dock_time + route_context.extra_slowdown_dock_time + Airships::docking_duration();
                    #[cfg(debug_assertions)]
                    {
                        if route_context.did_hold || route_context.slow_count > 0 || docking_time > Airships::docking_duration() {
                            let docked_site_name = ctx.index.sites.get(current_approach.site_id).name().to_string();
                            debug!("{}, Docked at {}, did_hold:{}, slow_count:{}, extra_hold_dock_time:{}, extra_slowdown_dock_time:{}, docking_time:{}", format!("{:?}", ctx.npc_id), docked_site_name, route_context.did_hold, route_context.slow_count, route_context.extra_hold_dock_time, route_context.extra_slowdown_dock_time, docking_time);
                        }
                    }
                    route_context.did_hold = false;
                    route_context.slow_count = 0;

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
                    .stop_if(timeout(ctx.rng.gen_range(10.0..16.0)))
                    // While waiting, every now and then announce where the airship is going next.
                    .then(
                        just(move |ctx, route_context:&mut AirshipRouteContext| {
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

#[cfg(test)]
mod tests {
    use super::{MovingAverage, DistanceTrend, DistanceTrendTracker};
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
        ma2.add((1000.0f32/1000.0) as i64);
        ma2.add((2000.0f32/1000.0) as i64);
        ma2.add((3000.0f32/1000.0) as i64);
        ma2.add((4000.0f32/1000.0) as i64);
        ma2.add((5000.0f32/1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 3);

        ma2.add((6000.0f32/1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 4);

        ma2.add((7000.0f32/1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 5);

        ma2.add((8000.0f32/1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 6);

        ma2.add((9000.0f32/1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 7);

        ma2.add((10000.0f32/1000.0) as i64);
        assert_eq!(ma2.average().unwrap(), 8);

        let mut ma3: MovingAverage<i64, 5, 3> = MovingAverage::default();
        ma3.add((20.99467f32*10000.0) as i64);
        ma3.add((20.987871f32*10000.0) as i64);
        ma3.add((20.69861f32*10000.0) as i64);
        ma3.add((20.268217f32*10000.0) as i64);
        ma3.add((20.230164f32*10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.6358).abs() < 0.0001);

        ma3.add((20.48151f32*10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.5332).abs() < 0.0001);

        ma3.add((20.568598f32*10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.4493).abs() < 0.0001);

        ma3.add((20.909971f32*10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.4916).abs() < 0.0001);

        ma3.add((21.014437f32*10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.6408).abs() < 0.0001);

        ma3.add((20.62308f32*10000.0) as i64);
        assert!((ma3.average().unwrap() as f64 / 10000.0 - 20.7194).abs() < 0.0001);
    }

    #[test]
    fn distance_trend_tracker_test() {
        let mut tracker: DistanceTrendTracker<0> = DistanceTrendTracker::default();
        tracker.reset(Vec2::new(0.0, 0.0));
        assert!(tracker.update(Vec2::new(1.0, 0.0), 1.0).is_none());
        assert!(tracker.update(Vec2::new(2.0, 0.0), 2.0).is_none());
        assert!(matches!(tracker.update(Vec2::new(3.0, 0.0), 3.0).unwrap(), DistanceTrend::Away));
        assert!(matches!(tracker.update(Vec2::new(4.0, 0.0), 4.0).unwrap(), DistanceTrend::Away));
        assert!(matches!(tracker.update(Vec2::new(5.0, 0.0), 5.0).unwrap(), DistanceTrend::Away));

        tracker.reset(Vec2::new(0.0, 0.0));
        assert!(tracker.update(Vec2::new(5.0, 0.0), 1.0).is_none());
        assert!(tracker.update(Vec2::new(4.0, 0.0), 2.0).is_none());
        assert!(matches!(tracker.update(Vec2::new(3.0, 0.0), 3.0).unwrap(), DistanceTrend::Towards));
        assert!(matches!(tracker.update(Vec2::new(2.0, 0.0), 4.0).unwrap(), DistanceTrend::Towards));
        assert!(matches!(tracker.update(Vec2::new(1.0, 0.0), 5.0).unwrap(), DistanceTrend::Towards));
        assert!(matches!(tracker.update(Vec2::new(0.0, 0.0), 6.0).unwrap(), DistanceTrend::Towards));
        assert!(matches!(tracker.update(Vec2::new(-1.0, 0.0), 7.0).unwrap(), DistanceTrend::Towards));
        assert!(matches!(tracker.update(Vec2::new(-2.0, 0.0), 8.0).unwrap(), DistanceTrend::Away));
        assert!(matches!(tracker.update(Vec2::new(-3.0, 0.0), 9.0).unwrap(), DistanceTrend::Away));

        let mut tracker2: DistanceTrendTracker<5> = DistanceTrendTracker::default();
        assert!(tracker2.update(Vec2::new(100.0, 100.0), 10.0).is_none());
        assert!(tracker2.update(Vec2::new(100.0, 200.0), 20.0).is_none());
        // $$$
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 300.0), 30.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 400.0), 40.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 500.0), 50.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 490.0), 60.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 505.0), 70.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 500.0), 80.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 495.0), 90.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 500.0), 100.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 495.0), 110.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 500.0), 120.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 505.0), 130.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 500.0), 140.0).unwrap(), tracker2.avg_rate.average().unwrap());
        println!("{:?} {}", tracker2.update(Vec2::new(100.0, 505.0), 150.0).unwrap(), tracker2.avg_rate.average().unwrap());
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 300.0), 3.0).unwrap(), DistanceTrend::Away));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 400.0), 4.0).unwrap(), DistanceTrend::Away));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 500.0), 5.0).unwrap(), DistanceTrend::Away));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 490.0), 6.0).unwrap(), DistanceTrend::Away));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 505.0), 7.0).unwrap(), DistanceTrend::Away));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 500.0), 8.0).unwrap(), DistanceTrend::Neutral));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 495.0), 9.0).unwrap(), DistanceTrend::Neutral));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 500.0), 10.0).unwrap(), DistanceTrend::Neutral));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 495.0), 11.0).unwrap(), DistanceTrend::Neutral));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 500.0), 12.0).unwrap(), DistanceTrend::Neutral));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 505.0), 13.0).unwrap(), DistanceTrend::Neutral));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 500.0), 14.0).unwrap(), DistanceTrend::Neutral));
        // assert!(matches!(tracker2.update(Vec2::new(100.0, 505.0), 15.0).unwrap(), DistanceTrend::Neutral));
    }

}
